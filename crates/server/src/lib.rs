//! # Orbit Server
//!
//! Hosted HTTP and WebSocket control-plane surface for Orbit.

use std::collections::{HashMap, VecDeque};
use std::env;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use orbit_events::{
    AppliedOrphanPolicy, ApprovalRequestedEventPayload, ApprovalResolvedEventPayload,
    ConnectorEventPayload, ConnectorEventRequest, ConnectorInteractionRequest,
    ConnectorInteractionResponse, EventEnvelope, EventIdentifiers, HostedEventName,
    HostedEventStatus, HostedEventTopic, HostedTaskEventSummary, LaneSignalEventPayload,
    TaskRoutedEventPayload, TerminalEventPayload,
};
use orbit_orchestrator::{
    plan_work_item, LaneRole, WorkItem, WorkItemContext, WorkItemPriority, WorkItemSource,
};
use orbit_runtime::task_registry::{Task, TaskRegistry, TaskRegistrySnapshot, TaskStatus};
use orbit_runtime::worker_boot::{
    WorkerEventKind, WorkerEventPayload, WorkerRegistry, WorkerStatus,
};
use orbit_tools::{
    cancel_hosted_agent_with_locator, hosted_agent_status_with_locator, launch_hosted_agent,
    HostedAgentCancellationSource, HostedAgentLaunchRequest, HostedAgentLocator,
    HostedAgentStatusSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::broadcast;

const DEFAULT_EVENT_REPLAY_LIMIT: usize = 100;
const ORPHANED_WORKER_STATUS: &str = "orphaned";
const DEFAULT_ORPHAN_APPROVAL_DELAY: Duration = Duration::from_secs(0);

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrphanPolicy {
    approval_delay: Duration,
    auto_retry_after: Option<Duration>,
    auto_cancel_after: Option<Duration>,
}

fn applied_orphan_policy_from_default(policy: &OrphanPolicy) -> AppliedOrphanPolicy {
    AppliedOrphanPolicy {
        source: "default".to_string(),
        match_repository: None,
        match_source: None,
        match_priority: None,
        approval_delay_secs: policy.approval_delay.as_secs(),
        auto_retry_after_secs: policy.auto_retry_after.map(|duration| duration.as_secs()),
        auto_cancel_after_secs: policy.auto_cancel_after.map(|duration| duration.as_secs()),
    }
}

fn applied_orphan_policy_from_rule(
    rule: &OrphanPolicyRule,
    policy: &OrphanPolicy,
) -> AppliedOrphanPolicy {
    AppliedOrphanPolicy {
        source: "rule".to_string(),
        match_repository: rule.repository.clone(),
        match_source: rule.source.clone(),
        match_priority: rule.priority.clone(),
        approval_delay_secs: policy.approval_delay.as_secs(),
        auto_retry_after_secs: policy.auto_retry_after.map(|duration| duration.as_secs()),
        auto_cancel_after_secs: policy.auto_cancel_after.map(|duration| duration.as_secs()),
    }
}

impl From<&OrphanPolicyRule> for OrphanPolicyRuleResponse {
    fn from(rule: &OrphanPolicyRule) -> Self {
        Self {
            repository: rule.repository.clone(),
            source: rule.source.clone(),
            priority: rule.priority.clone(),
            approval_delay_secs: rule.approval_delay_secs,
            auto_retry_after_secs: rule.auto_retry_after_secs,
            auto_cancel_after_secs: rule.auto_cancel_after_secs,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct OrphanPolicyRule {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    approval_delay_secs: Option<u64>,
    #[serde(default)]
    auto_retry_after_secs: Option<u64>,
    #[serde(default)]
    auto_cancel_after_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub event_replay_limit: usize,
    pub lane_transport_kind: LaneTransportKind,
    pub reconcile_interval: Option<Duration>,
    pub state_file: Option<PathBuf>,
    pub orphan_approval_delay: Duration,
    pub orphan_auto_retry_after: Option<Duration>,
    pub orphan_auto_cancel_after: Option<Duration>,
    orphan_policy_rules: Vec<OrphanPolicyRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneTransportKind {
    InMemory,
    ToolsAgent,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8788),
            event_replay_limit: DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport_kind: LaneTransportKind::InMemory,
            reconcile_interval: Some(Duration::from_secs(15)),
            state_file: default_server_state_file(),
            orphan_approval_delay: DEFAULT_ORPHAN_APPROVAL_DELAY,
            orphan_auto_retry_after: None,
            orphan_auto_cancel_after: None,
            orphan_policy_rules: Vec::new(),
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut config = Self::default();

        if let Ok(host) = env::var("ORBIT_SERVER_HOST") {
            let ip: IpAddr = host.parse()?;
            config.bind_addr = SocketAddr::new(ip, config.bind_addr.port());
        }

        if let Ok(port) = env::var("ORBIT_SERVER_PORT") {
            let port: u16 = port.parse()?;
            config.bind_addr = SocketAddr::new(config.bind_addr.ip(), port);
        }

        if let Ok(limit) = env::var("ORBIT_SERVER_EVENT_REPLAY_LIMIT") {
            let limit: usize = limit.parse()?;
            config.event_replay_limit = limit.max(1);
        }

        if let Ok(kind) = env::var("ORBIT_SERVER_LANE_TRANSPORT") {
            config.lane_transport_kind = match kind.trim().to_ascii_lowercase().as_str() {
                "tools-agent" | "tools_agent" | "agent" => LaneTransportKind::ToolsAgent,
                _ => LaneTransportKind::InMemory,
            };
        }

        if let Ok(interval) = env::var("ORBIT_SERVER_RECONCILE_INTERVAL_SECS") {
            let interval: u64 = interval.parse()?;
            config.reconcile_interval = (interval > 0).then(|| Duration::from_secs(interval));
        }

        if let Ok(path) = env::var("ORBIT_SERVER_STATE_FILE") {
            let path = path.trim();
            config.state_file = (!path.is_empty()).then(|| PathBuf::from(path));
        }

        if let Ok(delay) = env::var("ORBIT_SERVER_ORPHAN_APPROVAL_DELAY_SECS") {
            let delay: u64 = delay.parse()?;
            config.orphan_approval_delay = Duration::from_secs(delay);
        }

        if let Ok(delay) = env::var("ORBIT_SERVER_ORPHAN_AUTO_RETRY_SECS") {
            let delay: u64 = delay.parse()?;
            config.orphan_auto_retry_after = (delay > 0).then(|| Duration::from_secs(delay));
        }

        if let Ok(delay) = env::var("ORBIT_SERVER_ORPHAN_AUTO_CANCEL_SECS") {
            let delay: u64 = delay.parse()?;
            config.orphan_auto_cancel_after = (delay > 0).then(|| Duration::from_secs(delay));
        }

        if let Ok(rules) = env::var("ORBIT_SERVER_ORPHAN_POLICY_RULES") {
            let rules = rules.trim();
            if !rules.is_empty() {
                config.orphan_policy_rules = serde_json::from_str(rules).map_err(|error| {
                    format!("invalid ORBIT_SERVER_ORPHAN_POLICY_RULES: {error}")
                })?;
            }
        }

        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub struct ServerState {
    started_at: Instant,
    tasks: TaskRegistry,
    lane_transport: Arc<dyn LaneWorkerTransport>,
    contexts: Arc<Mutex<HashMap<String, HostedTaskContext>>>,
    persistence: Option<Arc<ServerStatePersistence>>,
    event_sender: broadcast::Sender<EventEnvelope>,
    event_history: Arc<Mutex<VecDeque<EventEnvelope>>>,
    event_replay_limit: usize,
    orphan_approval_delay: Duration,
    orphan_auto_retry_after: Option<Duration>,
    orphan_auto_cancel_after: Option<Duration>,
    orphan_policy_rules: Vec<OrphanPolicyRule>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(DEFAULT_EVENT_REPLAY_LIMIT)
    }
}

impl ServerState {
    #[must_use]
    pub fn new(event_replay_limit: usize) -> Self {
        Self::new_with_transport_kind_and_state_file(
            event_replay_limit,
            LaneTransportKind::InMemory,
            None,
        )
    }

    #[must_use]
    pub fn new_with_transport_kind(
        event_replay_limit: usize,
        lane_transport_kind: LaneTransportKind,
    ) -> Self {
        Self::new_with_transport_kind_and_state_file(event_replay_limit, lane_transport_kind, None)
    }

    #[must_use]
    pub fn new_with_transport_kind_and_state_file(
        event_replay_limit: usize,
        lane_transport_kind: LaneTransportKind,
        state_file: Option<PathBuf>,
    ) -> Self {
        let lane_transport: Arc<dyn LaneWorkerTransport> = match lane_transport_kind {
            LaneTransportKind::InMemory => Arc::new(InMemoryLaneWorkerTransport::new()),
            LaneTransportKind::ToolsAgent => Arc::new(HostedAgentLaneWorkerTransport),
        };
        Self::build(
            event_replay_limit,
            lane_transport,
            state_file,
            DEFAULT_ORPHAN_APPROVAL_DELAY,
            None,
            None,
            Vec::new(),
        )
    }

    #[must_use]
    fn new_with_transport_kind_state_file_and_policy(
        event_replay_limit: usize,
        lane_transport_kind: LaneTransportKind,
        state_file: Option<PathBuf>,
        orphan_approval_delay: Duration,
        orphan_auto_retry_after: Option<Duration>,
        orphan_auto_cancel_after: Option<Duration>,
        orphan_policy_rules: Vec<OrphanPolicyRule>,
    ) -> Self {
        let lane_transport: Arc<dyn LaneWorkerTransport> = match lane_transport_kind {
            LaneTransportKind::InMemory => Arc::new(InMemoryLaneWorkerTransport::new()),
            LaneTransportKind::ToolsAgent => Arc::new(HostedAgentLaneWorkerTransport),
        };
        Self::build(
            event_replay_limit,
            lane_transport,
            state_file,
            orphan_approval_delay,
            orphan_auto_retry_after,
            orphan_auto_cancel_after,
            orphan_policy_rules,
        )
    }

    #[must_use]
    fn build(
        event_replay_limit: usize,
        lane_transport: Arc<dyn LaneWorkerTransport>,
        state_file: Option<PathBuf>,
        orphan_approval_delay: Duration,
        orphan_auto_retry_after: Option<Duration>,
        orphan_auto_cancel_after: Option<Duration>,
        orphan_policy_rules: Vec<OrphanPolicyRule>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(event_replay_limit.max(1) * 4);
        let persistence = state_file.map(|path| Arc::new(ServerStatePersistence { path }));
        let (tasks, contexts, event_history) = persistence
            .as_ref()
            .and_then(|persistence| persistence.load().ok())
            .map(|snapshot| {
                (
                    TaskRegistry::from_snapshot(snapshot.tasks),
                    snapshot.contexts,
                    trim_event_history(snapshot.event_history, event_replay_limit.max(1)),
                )
            })
            .unwrap_or_else(|| {
                (
                    TaskRegistry::new(),
                    HashMap::new(),
                    VecDeque::with_capacity(event_replay_limit.max(1)),
                )
            });
        let state = Self {
            started_at: Instant::now(),
            tasks,
            lane_transport,
            contexts: Arc::new(Mutex::new(contexts)),
            persistence,
            event_sender,
            event_history: Arc::new(Mutex::new(event_history)),
            event_replay_limit: event_replay_limit.max(1),
            orphan_approval_delay,
            orphan_auto_retry_after,
            orphan_auto_cancel_after,
            orphan_policy_rules,
        };
        state.recover_loaded_state();
        state
    }

    #[cfg(test)]
    #[must_use]
    fn with_lane_transport(
        event_replay_limit: usize,
        lane_transport: Arc<dyn LaneWorkerTransport>,
    ) -> Self {
        Self::build(
            event_replay_limit,
            lane_transport,
            None,
            DEFAULT_ORPHAN_APPROVAL_DELAY,
            None,
            None,
            Vec::new(),
        )
    }

    #[cfg(test)]
    #[must_use]
    fn with_lane_transport_and_state_file(
        event_replay_limit: usize,
        lane_transport: Arc<dyn LaneWorkerTransport>,
        state_file: PathBuf,
    ) -> Self {
        Self::build(
            event_replay_limit,
            lane_transport,
            Some(state_file),
            DEFAULT_ORPHAN_APPROVAL_DELAY,
            None,
            None,
            Vec::new(),
        )
    }

    #[cfg(test)]
    #[must_use]
    fn with_lane_transport_and_policy(
        event_replay_limit: usize,
        lane_transport: Arc<dyn LaneWorkerTransport>,
        orphan_approval_delay: Duration,
        orphan_auto_retry_after: Option<Duration>,
        orphan_auto_cancel_after: Option<Duration>,
    ) -> Self {
        Self::build(
            event_replay_limit,
            lane_transport,
            None,
            orphan_approval_delay,
            orphan_auto_retry_after,
            orphan_auto_cancel_after,
            Vec::new(),
        )
    }

    #[cfg(test)]
    #[must_use]
    fn with_lane_transport_and_policy_rules(
        event_replay_limit: usize,
        lane_transport: Arc<dyn LaneWorkerTransport>,
        orphan_approval_delay: Duration,
        orphan_auto_retry_after: Option<Duration>,
        orphan_auto_cancel_after: Option<Duration>,
        orphan_policy_rules: Vec<OrphanPolicyRule>,
    ) -> Self {
        Self::build(
            event_replay_limit,
            lane_transport,
            None,
            orphan_approval_delay,
            orphan_auto_retry_after,
            orphan_auto_cancel_after,
            orphan_policy_rules,
        )
    }

    fn record_context(&self, task_id: &str, context: HostedTaskContext) {
        let mut contexts = self
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        contexts.insert(task_id.to_string(), context);
    }

    fn context_for(&self, task_id: &str) -> Option<HostedTaskContext> {
        let contexts = self
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        contexts.get(task_id).cloned()
    }

    fn broadcast_event(&self, event: EventEnvelope) {
        let event = self.hydrate_event_for_stream(&event);
        {
            let mut history = self
                .event_history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if history.len() == self.event_replay_limit {
                history.pop_front();
            }
            history.push_back(event.clone());
        }
        let _ = self.persist_state();
        let _ = self.event_sender.send(event);
    }

    fn replay_events(&self) -> Vec<EventEnvelope> {
        let history = self
            .event_history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        history.iter().cloned().collect()
    }

    fn replay_events_filtered(&self, query: &EventStreamQuery) -> Vec<EventEnvelope> {
        trim_matching_events(
            self.replay_events()
                .into_iter()
                .filter(|event| event_matches_query(self, event, query))
                .map(|event| self.hydrate_event_for_stream(&event))
                .collect(),
            query.limit,
        )
    }

    fn hydrate_event_for_stream(&self, event: &EventEnvelope) -> EventEnvelope {
        let Some(task_id) = event.task_id.as_deref() else {
            return event.clone();
        };
        let Some(context) = self.context_for(task_id) else {
            return event.clone();
        };

        EventEnvelope {
            payload: build_event_payload(
                &context,
                inferred_task_status_from_event(event),
                event.payload.clone(),
            ),
            ..event.clone()
        }
    }

    fn task_snapshot(&self, task: Task) -> HostedTaskSnapshot {
        let task_id = task.task_id.clone();
        HostedTaskSnapshot::from_task(task, self.context_for(&task_id))
    }

    fn persist_state(&self) -> Result<(), String> {
        let Some(persistence) = &self.persistence else {
            return Ok(());
        };
        persistence.save(&self.snapshot())
    }

    fn snapshot(&self) -> ServerStateSnapshot {
        let contexts = self
            .contexts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let event_history = self
            .event_history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect();
        ServerStateSnapshot {
            tasks: self.tasks.snapshot(),
            contexts,
            event_history,
        }
    }

    fn recover_loaded_state(&self) {
        reconcile_active_tasks(self);
    }
}

#[derive(Debug)]
struct ServerStatePersistence {
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerStateSnapshot {
    tasks: TaskRegistrySnapshot,
    contexts: HashMap<String, HostedTaskContext>,
    #[serde(default)]
    event_history: Vec<EventEnvelope>,
}

impl ServerStatePersistence {
    fn load(&self) -> Result<ServerStateSnapshot, String> {
        if !self.path.exists() {
            return Err(format!("state file not found: {}", self.path.display()));
        }
        let contents = std::fs::read_to_string(&self.path).map_err(|error| error.to_string())?;
        serde_json::from_str(&contents).map_err(|error| error.to_string())
    }

    fn save(&self, snapshot: &ServerStateSnapshot) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let tmp_path = self.path.with_extension("tmp");
        std::fs::write(
            &tmp_path,
            serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        std::fs::rename(&tmp_path, &self.path).map_err(|error| error.to_string())
    }
}

fn default_server_state_file() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    Some(cwd.join(".orbit-server").join("state.json"))
}

fn current_unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl OrphanPolicyRule {
    fn matches(&self, context: &HostedTaskContext) -> bool {
        matches_optional_filter(self.repository.as_deref(), context.repository.as_deref())
            && matches_optional_filter(self.source.as_deref(), context.source.as_deref())
            && matches_optional_filter(self.priority.as_deref(), context.priority.as_deref())
    }
}

fn task_matches_query(task: &HostedTaskSnapshot, query: &ListTasksQuery) -> bool {
    matches_optional_csv_filter(
        query.status.as_deref(),
        Some(hosted_task_status_label(task.status)),
    ) && matches_optional_filter(query.source.as_deref(), task.source.as_deref())
        && matches_optional_filter(query.user_id.as_deref(), task.user_id.as_deref())
        && matches_optional_filter(query.channel_id.as_deref(), task.channel_id.as_deref())
        && matches_optional_filter(query.thread_ts.as_deref(), task.thread_ts.as_deref())
        && matches_optional_filter(query.repository.as_deref(), task.repository.as_deref())
}

fn event_matches_query(
    state: &ServerState,
    event: &EventEnvelope,
    query: &EventStreamQuery,
) -> bool {
    if !matches_optional_csv_filter(query.task_id.as_deref(), event.task_id.as_deref())
        || !matches_optional_csv_filter(query.lane_id.as_deref(), event.lane_id.as_deref())
        || !matches_optional_csv_filter(
            query.topic.as_deref(),
            Some(hosted_event_topic_label(&event.topic)),
        )
        || !matches_optional_csv_filter(
            query.event.as_deref(),
            Some(hosted_event_name_label(&event.event)),
        )
        || !matches_optional_csv_filter(
            query.status.as_deref(),
            Some(hosted_event_status_label(&event.status)),
        )
    {
        return false;
    }

    if query.source.is_none()
        && query.user_id.is_none()
        && query.channel_id.is_none()
        && query.thread_ts.is_none()
        && query.repository.is_none()
    {
        return true;
    }

    let Some(task_id) = event.task_id.as_deref() else {
        return false;
    };
    let context = state.context_for(task_id).unwrap_or_default();

    matches_optional_csv_filter(query.source.as_deref(), context.source.as_deref())
        && matches_optional_csv_filter(query.user_id.as_deref(), context.user_id.as_deref())
        && matches_optional_csv_filter(query.channel_id.as_deref(), context.channel_id.as_deref())
        && matches_optional_csv_filter(query.thread_ts.as_deref(), context.thread_ts.as_deref())
        && matches_optional_csv_filter(
            query.repository.as_deref(),
            context.repository.as_deref().or(event.repo_id.as_deref()),
        )
}

fn matches_optional_filter(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => actual == Some(expected),
        None => true,
    }
}

fn matches_optional_csv_filter(expected: Option<&str>, actual: Option<&str>) -> bool {
    match expected {
        Some(expected) => expected
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|candidate| actual == Some(candidate)),
        None => true,
    }
}

fn hosted_task_status_label(status: HostedTaskStatus) -> &'static str {
    match status {
        HostedTaskStatus::Pending => "pending",
        HostedTaskStatus::Running => "running",
        HostedTaskStatus::Completed => "completed",
        HostedTaskStatus::Failed => "failed",
        HostedTaskStatus::Cancelled => "cancelled",
    }
}

fn hosted_event_topic_label(topic: &HostedEventTopic) -> &'static str {
    match topic {
        HostedEventTopic::Task => "task",
        HostedEventTopic::Lane => "lane",
        HostedEventTopic::Approval => "approval",
        HostedEventTopic::Memory => "memory",
        HostedEventTopic::Connector => "connector",
    }
}

fn hosted_event_name_label(event: &HostedEventName) -> &'static str {
    match event {
        HostedEventName::TaskCreated => "task.created",
        HostedEventName::TaskRouted => "task.routed",
        HostedEventName::TaskCancelled => "task.cancelled",
        HostedEventName::LaneStarted => "lane.started",
        HostedEventName::LaneBlocked => "lane.blocked",
        HostedEventName::LaneGreen => "lane.green",
        HostedEventName::LaneFailed => "lane.failed",
        HostedEventName::ApprovalRequested => "approval.requested",
        HostedEventName::ApprovalResolved => "approval.resolved",
        HostedEventName::MemoryCaptured => "memory.captured",
        HostedEventName::ConnectorEventReceived => "connector.event.received",
    }
}

fn hosted_event_status_label(status: &HostedEventStatus) -> &'static str {
    match status {
        HostedEventStatus::Pending => "pending",
        HostedEventStatus::Running => "running",
        HostedEventStatus::Blocked => "blocked",
        HostedEventStatus::Completed => "completed",
        HostedEventStatus::Failed => "failed",
        HostedEventStatus::Cancelled => "cancelled",
    }
}

fn trim_matching_events(
    mut events: Vec<EventEnvelope>,
    limit: Option<usize>,
) -> Vec<EventEnvelope> {
    let Some(limit) = limit else {
        return events;
    };
    if limit == 0 || events.len() <= limit {
        return events;
    }
    let start = events.len().saturating_sub(limit);
    events.drain(..start);
    events
}

fn build_event_payload(
    context: &HostedTaskContext,
    task_status: Option<&str>,
    payload: Option<Value>,
) -> Option<Value> {
    let mut object = match payload {
        Some(Value::Object(object)) => object,
        Some(value) => Map::from_iter([("value".to_string(), value)]),
        None => Map::new(),
    };

    let summary = HostedTaskEventSummary {
        task_status: task_status.map(str::to_string),
        source: context.source.clone(),
        user_id: context.user_id.clone(),
        channel_id: context.channel_id.clone(),
        thread_ts: context.thread_ts.clone(),
        approval_message_ts: context.approval_message_ts.clone(),
        repository: context.repository.clone(),
        branch: context.branch.clone(),
        priority: context.priority.clone(),
        plan_id: context.plan_id.clone(),
        plan_kind: context.plan_kind.clone(),
        work_item_id: context.work_item_id.clone(),
        worker_id: context.worker_id.clone(),
        worker_status: context.worker_status.clone(),
        orphan_policy: context.applied_orphan_policy.clone(),
    };
    let summary_object =
        match serde_json::to_value(summary).expect("task event summary should serialize") {
            Value::Object(object) => object,
            _ => unreachable!("task event summary serializes to an object"),
        };

    for (key, value) in summary_object {
        object.entry(key).or_insert(value);
    }

    Some(Value::Object(object))
}

fn serialize_event_extra<T: Serialize>(payload: T) -> Value {
    serde_json::to_value(payload).expect("event payload should serialize")
}

fn inferred_task_status_from_event(event: &EventEnvelope) -> Option<&'static str> {
    match event.event {
        HostedEventName::TaskCreated => Some("pending"),
        HostedEventName::TaskRouted | HostedEventName::LaneStarted => Some("running"),
        HostedEventName::LaneBlocked | HostedEventName::ApprovalRequested => Some("pending"),
        HostedEventName::ApprovalResolved => match event.status {
            HostedEventStatus::Pending
            | HostedEventStatus::Running
            | HostedEventStatus::Blocked => Some("running"),
            HostedEventStatus::Completed => Some("completed"),
            HostedEventStatus::Failed => Some("failed"),
            HostedEventStatus::Cancelled => Some("cancelled"),
        },
        HostedEventName::LaneGreen => Some("completed"),
        HostedEventName::LaneFailed => Some("failed"),
        HostedEventName::TaskCancelled => Some("cancelled"),
        HostedEventName::MemoryCaptured | HostedEventName::ConnectorEventReceived => None,
    }
}

fn effective_orphan_policy(
    state: &ServerState,
    context: &HostedTaskContext,
) -> (OrphanPolicy, AppliedOrphanPolicy) {
    let mut policy = OrphanPolicy {
        approval_delay: state.orphan_approval_delay,
        auto_retry_after: state.orphan_auto_retry_after,
        auto_cancel_after: state.orphan_auto_cancel_after,
    };
    let mut applied = applied_orphan_policy_from_default(&policy);

    if let Some(rule) = state
        .orphan_policy_rules
        .iter()
        .find(|rule| rule.matches(context))
    {
        if let Some(delay_secs) = rule.approval_delay_secs {
            policy.approval_delay = Duration::from_secs(delay_secs);
        }
        if let Some(delay_secs) = rule.auto_retry_after_secs {
            policy.auto_retry_after = (delay_secs > 0).then(|| Duration::from_secs(delay_secs));
        }
        if let Some(delay_secs) = rule.auto_cancel_after_secs {
            policy.auto_cancel_after = (delay_secs > 0).then(|| Duration::from_secs(delay_secs));
        }
        applied = applied_orphan_policy_from_rule(rule, &policy);
    }

    (policy, applied)
}

fn trim_event_history(event_history: Vec<EventEnvelope>, limit: usize) -> VecDeque<EventEnvelope> {
    let limit = limit.max(1);
    let len = event_history.len();
    let start = len.saturating_sub(limit);
    event_history.into_iter().skip(start).collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostedTaskContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_message_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphaned_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphaned_approval_requested_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_auto_retry_attempted_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_orphan_policy: Option<AppliedOrphanPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_manifest_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostedTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl HostedTaskStatus {
    fn from_runtime(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Created => Self::Pending,
            TaskStatus::Running => Self::Running,
            TaskStatus::Completed => Self::Completed,
            TaskStatus::Failed => Self::Failed,
            TaskStatus::Stopped => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostedTaskSnapshot {
    pub task_id: String,
    pub status: HostedTaskStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_message_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_policy: Option<AppliedOrphanPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_status: Option<String>,
}

impl HostedTaskSnapshot {
    fn from_task(task: Task, context: Option<HostedTaskContext>) -> Self {
        let status = HostedTaskStatus::from_runtime(task.status);
        let result = (!task.output.is_empty()).then_some(task.output.clone());
        let error = (status == HostedTaskStatus::Failed).then_some(task.output.clone());
        let context = context.unwrap_or_default();

        Self {
            task_id: task.task_id,
            status,
            created_at: task.created_at,
            updated_at: task.updated_at,
            prompt: task.prompt,
            description: task.description,
            result,
            error,
            lane_id: context.lane_id,
            source: context.source,
            user_id: context.user_id,
            channel_id: context.channel_id,
            thread_ts: context.thread_ts,
            approval_message_ts: context.approval_message_ts,
            orphan_policy: context.applied_orphan_policy,
            repository: context.repository,
            branch: context.branch,
            priority: context.priority,
            plan_id: context.plan_id,
            plan_kind: context.plan_kind,
            work_item_id: context.work_item_id,
            worker_id: context.worker_id,
            worker_status: context.worker_status,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub prompt: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub permission_mode: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub priority: Option<String>,
    pub source: Option<String>,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub thread_ts: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub source: Option<String>,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub thread_ts: Option<String>,
    pub repository: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EventStreamQuery {
    pub task_id: Option<String>,
    pub lane_id: Option<String>,
    pub topic: Option<String>,
    pub event: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub thread_ts: Option<String>,
    pub repository: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OrphanPolicyQuery {
    pub repository: Option<String>,
    pub source: Option<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteTaskRequest {
    pub finish_reason: String,
    #[serde(default)]
    pub tokens_output: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveApprovalRequest {
    pub approval_kind: String,
    pub action: String,
    pub resolved_by: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateTaskContextRequest {
    pub source: Option<String>,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub thread_ts: Option<String>,
    pub approval_message_ts: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub task_id: String,
    pub status: HostedTaskStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskRuntimeResponse {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_policy: Option<AppliedOrphanPolicy>,
    #[serde(rename = "hostedAgent", skip_serializing_if = "Option::is_none")]
    pub hosted_agent: Option<HostedAgentStatusSnapshot>,
}

#[derive(Debug, Serialize)]
pub struct OrphanPolicyRuleResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_delay_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_retry_after_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_cancel_after_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct OrphanPolicyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<OrphanPolicyQuery>,
    pub default_policy: AppliedOrphanPolicy,
    pub effective_policy: AppliedOrphanPolicy,
    pub configured_rules: Vec<OrphanPolicyRuleResponse>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub commit: String,
    pub build_time: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub system: SystemStatus,
    pub tasks: TaskCounters,
}

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub status: &'static str,
    pub version: String,
    pub uptime: u64,
}

#[derive(Debug, Serialize)]
pub struct TaskCounters {
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn app(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/version", get(version))
        .route("/v1/policies/orphans", get(get_orphan_policy))
        .route("/v1/tasks", get(list_tasks).post(create_task))
        .route("/v1/tasks/:task_id", get(get_task))
        .route("/v1/tasks/:task_id/context", post(update_task_context))
        .route("/v1/tasks/:task_id/runtime", get(get_task_runtime))
        .route("/v1/tasks/:task_id/cancel", post(cancel_task))
        .route("/v1/tasks/:task_id/approval", post(resolve_approval))
        .route("/v1/tasks/:task_id/reconcile", post(reconcile_task))
        .route("/v1/tasks/:task_id/complete", post(complete_task))
        .route("/v1/events/ws", get(events_ws))
        .route(
            "/v1/connectors/:connector/interactions",
            post(connector_interaction),
        )
        .route("/v1/connectors/:connector/events", post(connector_event))
        .with_state(state)
}

pub async fn serve(config: ServerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(ServerState::new_with_transport_kind_state_file_and_policy(
        config.event_replay_limit,
        config.lane_transport_kind,
        config.state_file.clone(),
        config.orphan_approval_delay,
        config.orphan_auto_retry_after,
        config.orphan_auto_cancel_after,
        config.orphan_policy_rules.clone(),
    ));
    if let Some(interval) = config.reconcile_interval {
        let reconcile_state = state.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                reconcile_active_tasks(reconcile_state.as_ref());
            }
        });
    }
    let app = app(state);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    println!("orbit-server listening on http://{}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn status(State(state): State<Arc<ServerState>>) -> Json<StatusResponse> {
    reconcile_active_tasks(state.as_ref());
    let tasks = state.tasks.list(None);
    let counters = TaskCounters {
        total_tasks: tasks.len(),
        active_tasks: tasks
            .iter()
            .filter(|task| matches!(task.status, TaskStatus::Created | TaskStatus::Running))
            .count(),
        completed_tasks: tasks
            .iter()
            .filter(|task| matches!(task.status, TaskStatus::Completed))
            .count(),
        failed_tasks: tasks
            .iter()
            .filter(|task| matches!(task.status, TaskStatus::Failed))
            .count(),
    };

    Json(StatusResponse {
        system: SystemStatus {
            status: "healthy",
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: state.started_at.elapsed().as_secs(),
        },
        tasks: counters,
    })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
    })
}

async fn get_orphan_policy(
    Query(query): Query<OrphanPolicyQuery>,
    State(state): State<Arc<ServerState>>,
) -> Json<OrphanPolicyResponse> {
    let preview_context = HostedTaskContext {
        source: query.source.clone(),
        repository: query.repository.clone(),
        priority: query.priority.clone(),
        ..HostedTaskContext::default()
    };
    let default_policy = applied_orphan_policy_from_default(&OrphanPolicy {
        approval_delay: state.orphan_approval_delay,
        auto_retry_after: state.orphan_auto_retry_after,
        auto_cancel_after: state.orphan_auto_cancel_after,
    });
    let (_, effective_policy) = effective_orphan_policy(state.as_ref(), &preview_context);

    Json(OrphanPolicyResponse {
        preview: (query.repository.is_some() || query.source.is_some() || query.priority.is_some())
            .then_some(query),
        default_policy,
        effective_policy,
        configured_rules: state
            .orphan_policy_rules
            .iter()
            .map(OrphanPolicyRuleResponse::from)
            .collect(),
    })
}

async fn create_task(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<CreateTaskResponse>, AppError> {
    let task = create_task_internal(state.as_ref(), request)?;
    let snapshot = state.task_snapshot(task);
    Ok(Json(CreateTaskResponse {
        task_id: snapshot.task_id,
        status: snapshot.status,
        message: "task created".to_string(),
        lane_id: snapshot.lane_id,
        plan_kind: snapshot.plan_kind,
        worker_id: snapshot.worker_id,
        worker_status: snapshot.worker_status,
    }))
}

async fn list_tasks(
    Query(query): Query<ListTasksQuery>,
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<HostedTaskSnapshot>> {
    reconcile_active_tasks(state.as_ref());
    let mut tasks = state
        .tasks
        .list(None)
        .into_iter()
        .map(|task| state.task_snapshot(task))
        .filter(|task| task_matches_query(task, &query))
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    if let Some(limit) = query.limit {
        tasks.truncate(limit);
    }
    Json(tasks)
}

async fn get_task(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    let _ = reconcile_task_from_artifacts(state.as_ref(), &task_id);
    let task = state
        .tasks
        .get(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    Ok(Json(state.task_snapshot(task)))
}

async fn update_task_context(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpdateTaskContextRequest>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    let task = state
        .tasks
        .get(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let mut context = state.context_for(&task_id).unwrap_or_default();

    if let Some(source) = request.source {
        context.source = Some(source);
    }
    if let Some(user_id) = request.user_id {
        context.user_id = Some(user_id);
    }
    if let Some(channel_id) = request.channel_id {
        context.channel_id = Some(channel_id);
    }
    if let Some(thread_ts) = request.thread_ts {
        context.thread_ts = Some(thread_ts);
    }
    if let Some(approval_message_ts) = request.approval_message_ts {
        context.approval_message_ts = Some(approval_message_ts);
    }

    state.record_context(&task_id, context);
    state.persist_state().map_err(AppError::internal)?;
    Ok(Json(state.task_snapshot(task)))
}

async fn reconcile_task(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    reconcile_task_from_artifacts(state.as_ref(), &task_id)?;
    let task = state
        .tasks
        .get(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    Ok(Json(state.task_snapshot(task)))
}

async fn get_task_runtime(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<TaskRuntimeResponse>, AppError> {
    let _ = reconcile_task_from_artifacts(state.as_ref(), &task_id);
    let context = state
        .context_for(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let hosted_agent = hosted_agent_runtime_snapshot(&context);
    Ok(Json(TaskRuntimeResponse {
        task_id,
        worker_id: context.worker_id,
        worker_status: context.worker_status,
        manifest_file: context.worker_manifest_file,
        output_file: context.worker_output_file,
        orphan_policy: context.applied_orphan_policy,
        hosted_agent,
    }))
}

#[derive(Debug, Deserialize)]
struct HostedAgentManifestSnapshot {
    #[serde(default)]
    status: String,
    #[serde(rename = "derivedState", default)]
    derived_state: String,
    #[serde(default)]
    error: Option<String>,
}

fn hosted_agent_runtime_snapshot(context: &HostedTaskContext) -> Option<HostedAgentStatusSnapshot> {
    let worker_id = context.worker_id.as_deref()?;
    let snapshot = hosted_agent_status_with_locator(&HostedAgentLocator {
        agent_id: Some(worker_id.to_string()),
        manifest_file: context.worker_manifest_file.clone(),
        output_file: context.worker_output_file.clone(),
        hosted_task_id: None,
    });
    snapshot.found.then_some(snapshot)
}

fn reconcile_active_tasks(state: &ServerState) {
    let active_task_ids = state
        .tasks
        .list(None)
        .into_iter()
        .filter(|task| matches!(task.status, TaskStatus::Created | TaskStatus::Running))
        .map(|task| task.task_id)
        .collect::<Vec<_>>();

    for task_id in active_task_ids {
        let _ = reconcile_task_from_artifacts(state, &task_id);
    }
}

fn reconcile_task_from_artifacts(state: &ServerState, task_id: &str) -> Result<bool, AppError> {
    let task = state
        .tasks
        .get(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    if !matches!(task.status, TaskStatus::Created | TaskStatus::Running) {
        return Ok(false);
    }

    let mut context = match state.context_for(task_id) {
        Some(context) => context,
        None => return Ok(false),
    };
    let Some(manifest_file) = context.worker_manifest_file.clone() else {
        return Ok(false);
    };
    let manifest = read_hosted_agent_manifest(&manifest_file).map_err(AppError::internal)?;
    let normalized_status = manifest.status.trim().to_ascii_lowercase();

    match normalized_status.as_str() {
        "cancelled" => {
            state.tasks.stop(task_id).map_err(AppError::bad_request)?;
            context.worker_status = Some(WorkerStatus::Finished.to_string());
            context.orphaned_at = None;
            context.orphaned_approval_requested_at = None;
            context.orphan_auto_retry_attempted_at = None;
            context.applied_orphan_policy = None;
            state.record_context(task_id, context.clone());
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::TaskCancelled,
                HostedEventStatus::Cancelled,
                HostedEventTopic::Task,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task_id.to_string()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("cancelled"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        detail: Some(
                            "task cancellation restored from hosted agent manifest".to_string(),
                        ),
                        reconciled: Some(true),
                        derived_state: Some(manifest.derived_state.clone()),
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
            state.persist_state().map_err(AppError::internal)?;
            Ok(true)
        }
        "completed" => {
            if let Some(result) = context
                .worker_output_file
                .as_deref()
                .map(|path| read_agent_output_section(path, "### Final response"))
                .transpose()
                .map_err(AppError::internal)?
                .flatten()
            {
                state
                    .tasks
                    .append_output(task_id, result.trim())
                    .map_err(AppError::internal)?;
            }
            state
                .tasks
                .set_status(task_id, TaskStatus::Completed)
                .map_err(AppError::internal)?;
            context.worker_status = Some(WorkerStatus::Finished.to_string());
            context.orphaned_at = None;
            context.orphaned_approval_requested_at = None;
            context.orphan_auto_retry_attempted_at = None;
            context.applied_orphan_policy = None;
            state.record_context(task_id, context.clone());
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneGreen,
                HostedEventStatus::Completed,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task_id.to_string()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("completed"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        finish_reason: Some("manifest_reconcile".to_string()),
                        tokens_output: Some(0),
                        reconciled: Some(true),
                        derived_state: Some(manifest.derived_state.clone()),
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
            state.persist_state().map_err(AppError::internal)?;
            Ok(true)
        }
        "failed" => {
            state
                .tasks
                .set_status(task_id, TaskStatus::Failed)
                .map_err(AppError::internal)?;
            let failure_error = manifest.error.clone().or_else(|| {
                context
                    .worker_output_file
                    .as_deref()
                    .map(|path| read_agent_output_section(path, "### Error"))
                    .transpose()
                    .ok()
                    .flatten()
                    .flatten()
            });
            if let Some(error) = failure_error
                .as_deref()
                .filter(|error| !error.trim().is_empty())
            {
                state
                    .tasks
                    .append_output(task_id, error.trim())
                    .map_err(AppError::internal)?;
            }
            context.worker_status = Some(WorkerStatus::Failed.to_string());
            context.orphaned_at = None;
            context.orphaned_approval_requested_at = None;
            context.orphan_auto_retry_attempted_at = None;
            context.applied_orphan_policy = None;
            state.record_context(task_id, context.clone());
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneFailed,
                HostedEventStatus::Failed,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task_id.to_string()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("failed"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        finish_reason: Some("manifest_reconcile".to_string()),
                        tokens_output: Some(0),
                        error: failure_error,
                        reconciled: Some(true),
                        derived_state: Some(manifest.derived_state.clone()),
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
            state.persist_state().map_err(AppError::internal)?;
            Ok(true)
        }
        _ => {
            let Some(runtime) = hosted_agent_runtime_snapshot(&context) else {
                return Ok(false);
            };
            if !runtime.orphaned {
                return Ok(false);
            }
            let (policy, applied_policy) = effective_orphan_policy(state, &context);
            let now = current_unix_timestamp_secs();
            let first_orphaned_at = context.orphaned_at.unwrap_or(now);
            let orphaned_for_secs = now.saturating_sub(first_orphaned_at);
            let mut changed = false;

            if context.applied_orphan_policy.as_ref() != Some(&applied_policy) {
                context.applied_orphan_policy = Some(applied_policy.clone());
                changed = true;
            }

            if context.orphaned_at.is_none() {
                context.orphaned_at = Some(first_orphaned_at);
                changed = true;
            }

            if context.worker_status.as_deref() != Some(ORPHANED_WORKER_STATUS) {
                context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
                state.record_context(task_id, context.clone());
                state.broadcast_event(EventEnvelope::new(
                    HostedEventName::LaneBlocked,
                    HostedEventStatus::Blocked,
                    HostedEventTopic::Lane,
                    EventIdentifiers {
                        repo_id: context.repository.clone(),
                        lane_id: context.lane_id.clone(),
                        task_id: Some(task_id.to_string()),
                        ..EventIdentifiers::default()
                    },
                    build_event_payload(
                        &context,
                        Some("pending"),
                        Some(serialize_event_extra(LaneSignalEventPayload {
                            role: None,
                            description: None,
                            detail: Some(
                                "hosted agent manifest is non-terminal but no live control is attached"
                                    .to_string(),
                            ),
                            transport: None,
                            extra: Map::from_iter([
                                ("reconciled".to_string(), json!(true)),
                                ("orphaned".to_string(), json!(true)),
                                ("orphaned_at".to_string(), json!(first_orphaned_at)),
                                (
                                    "orphaned_for_secs".to_string(),
                                    json!(orphaned_for_secs),
                                ),
                                ("derived_state".to_string(), json!(runtime.derived_state)),
                                ("status".to_string(), json!(runtime.status)),
                            ]),
                        })),
                    ),
                    None,
                ));
                changed = true;
            }

            if let Some(auto_retry_after) = policy.auto_retry_after {
                if orphaned_for_secs >= auto_retry_after.as_secs()
                    && context.orphan_auto_retry_attempted_at.is_none()
                {
                    context.orphan_auto_retry_attempted_at = Some(now);
                    state.record_context(task_id, context);
                    let _ = retry_task_lane_internal(state, task_id, true)?;
                    return Ok(true);
                }
            }

            if let Some(auto_cancel_after) = policy.auto_cancel_after {
                if orphaned_for_secs >= auto_cancel_after.as_secs() {
                    state.record_context(task_id, context);
                    let _ = cancel_task_internal_with_detail(
                        state,
                        task_id,
                        Some(format!(
                            "task auto-cancelled after orphan timeout ({}s)",
                            orphaned_for_secs
                        )),
                    )?;
                    return Ok(true);
                }
            }

            if orphaned_for_secs >= policy.approval_delay.as_secs()
                && context.orphaned_approval_requested_at.is_none()
            {
                context.orphaned_approval_requested_at = Some(now);
                state.record_context(task_id, context.clone());
                state.broadcast_event(EventEnvelope::new(
                    HostedEventName::ApprovalRequested,
                    HostedEventStatus::Pending,
                    HostedEventTopic::Approval,
                    EventIdentifiers {
                        repo_id: context.repository.clone(),
                        lane_id: context.lane_id.clone(),
                        task_id: Some(task_id.to_string()),
                        ..EventIdentifiers::default()
                    },
                    build_event_payload(
                        &context,
                        Some("pending"),
                        Some(serialize_event_extra(ApprovalRequestedEventPayload {
                            approval_kind: "orphaned_hosted_agent".to_string(),
                            reason: Some(
                                "hosted agent lost live control and needs operator review"
                                    .to_string(),
                            ),
                            detail: Some(
                                "hosted agent manifest is non-terminal but no live control is attached"
                                    .to_string(),
                            ),
                            extra: Map::from_iter([
                                ("orphaned".to_string(), json!(true)),
                                ("orphaned_at".to_string(), json!(first_orphaned_at)),
                                (
                                    "orphaned_for_secs".to_string(),
                                    json!(orphaned_for_secs),
                                ),
                            ]),
                        })),
                    ),
                    None,
                ));
                changed = true;
            }

            if changed {
                state.record_context(task_id, context);
                state.persist_state().map_err(AppError::internal)?;
            }
            Ok(changed)
        }
    }
}

fn read_hosted_agent_manifest(path: &str) -> Result<HostedAgentManifestSnapshot, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&contents).map_err(|error| error.to_string())
}

fn read_agent_output_section(path: &str, heading: &str) -> Result<Option<String>, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(extract_markdown_section(&contents, heading))
}

fn extract_markdown_section(contents: &str, heading: &str) -> Option<String> {
    let start = contents.find(heading)?;
    let section = &contents[start + heading.len()..];
    let section = section.trim_start_matches('\n');
    let end = section
        .find("\n### ")
        .or_else(|| section.find("\n## "))
        .unwrap_or(section.len());
    let section = section[..end].trim();
    (!section.is_empty()).then(|| section.to_string())
}

async fn cancel_task(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    let snapshot = cancel_task_internal(state.as_ref(), &task_id)?;
    Ok(Json(snapshot))
}

fn cancel_task_internal(
    state: &ServerState,
    task_id: &str,
) -> Result<HostedTaskSnapshot, AppError> {
    cancel_task_internal_with_detail(state, task_id, None)
}

fn cancel_task_internal_with_detail(
    state: &ServerState,
    task_id: &str,
    detail_override: Option<String>,
) -> Result<HostedTaskSnapshot, AppError> {
    let task = state
        .tasks
        .get(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    if matches!(
        task.status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
    ) {
        return Err(AppError::bad_request(format!(
            "task {task_id} is already in terminal state: {}",
            task.status
        )));
    }

    let mut context = state
        .context_for(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let cancellation = if let Some(worker_id) = context.worker_id.clone() {
        let locator = HostedAgentLocator {
            agent_id: Some(worker_id.clone()),
            manifest_file: context.worker_manifest_file.clone(),
            output_file: context.worker_output_file.clone(),
            hosted_task_id: Some(task_id.to_string()),
        };
        state
            .lane_transport
            .cancel(&worker_id, Some(&locator))
            .map_err(AppError::internal)?
    } else {
        LaneCancellationResult {
            worker_status: None,
            clear_worker_status: false,
            detail: Some("task had no active worker".to_string()),
        }
    };

    state.tasks.stop(task_id).map_err(AppError::bad_request)?;
    if cancellation.clear_worker_status {
        context.worker_status = None;
    } else if let Some(worker_status) = cancellation.worker_status {
        context.worker_status = Some(worker_status.to_string());
    }
    context.orphaned_at = None;
    context.orphaned_approval_requested_at = None;
    context.orphan_auto_retry_attempted_at = None;
    context.applied_orphan_policy = None;
    state.record_context(task_id, context.clone());
    state.broadcast_event(EventEnvelope::new(
        HostedEventName::TaskCancelled,
        HostedEventStatus::Cancelled,
        HostedEventTopic::Task,
        EventIdentifiers {
            repo_id: context.repository.clone(),
            lane_id: context.lane_id.clone(),
            task_id: Some(task_id.to_string()),
            ..EventIdentifiers::default()
        },
        build_event_payload(
            &context,
            Some("cancelled"),
            Some(serialize_event_extra(TerminalEventPayload {
                detail: detail_override.or(cancellation.detail),
                extra: Map::new(),
                ..TerminalEventPayload::default()
            })),
        ),
        None,
    ));
    state.persist_state().map_err(AppError::internal)?;

    let task = state
        .tasks
        .get(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    Ok(state.task_snapshot(task))
}

fn retry_task_lane_internal(
    state: &ServerState,
    task_id: &str,
    preserve_auto_retry_attempted_at: bool,
) -> Result<HostedTaskSnapshot, AppError> {
    let task = state
        .tasks
        .get(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let mut refreshed_context = state
        .context_for(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let primary_role = lane_role_from_plan_kind(refreshed_context.plan_kind.as_deref());
    let primary_description = refreshed_context.plan_kind.clone();
    let auto_retry_attempted_at = refreshed_context.orphan_auto_retry_attempted_at;

    refreshed_context.worker_status = None;
    refreshed_context.worker_id = None;
    refreshed_context.worker_manifest_file = None;
    refreshed_context.worker_output_file = None;
    refreshed_context.orphaned_at = None;
    refreshed_context.orphaned_approval_requested_at = None;
    refreshed_context.applied_orphan_policy = None;
    if preserve_auto_retry_attempted_at {
        refreshed_context.orphan_auto_retry_attempted_at = auto_retry_attempted_at;
    } else {
        refreshed_context.orphan_auto_retry_attempted_at = None;
    }
    state.record_context(task_id, refreshed_context.clone());
    bootstrap_task_lane(
        state,
        &task,
        &mut refreshed_context,
        primary_role,
        primary_description.as_deref(),
    )?;
    let reloaded = state
        .tasks
        .get(task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    Ok(state.task_snapshot(reloaded))
}

async fn resolve_approval(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ResolveApprovalRequest>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    if request.approval_kind != "orphaned_hosted_agent" {
        return Err(AppError::bad_request(format!(
            "unsupported approval kind: {}",
            request.approval_kind
        )));
    }
    if request.action != "cancel" && request.action != "retry" {
        return Err(AppError::bad_request(format!(
            "unsupported approval action for orphaned hosted agent: {}",
            request.action
        )));
    }

    let context = state
        .context_for(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    if context.worker_status.as_deref() != Some(ORPHANED_WORKER_STATUS) {
        return Err(AppError::bad_request(format!(
            "task {task_id} is not awaiting orphaned hosted agent approval"
        )));
    }

    let snapshot = if request.action == "cancel" {
        cancel_task_internal(state.as_ref(), &task_id)?
    } else {
        retry_task_lane_internal(state.as_ref(), &task_id, true)?
    };
    let refreshed_context = state
        .context_for(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    state.broadcast_event(EventEnvelope::new(
        HostedEventName::ApprovalResolved,
        HostedEventStatus::Completed,
        HostedEventTopic::Approval,
        EventIdentifiers {
            repo_id: refreshed_context.repository.clone(),
            lane_id: refreshed_context.lane_id.clone(),
            task_id: Some(task_id.clone()),
            ..EventIdentifiers::default()
        },
        build_event_payload(
            &refreshed_context,
            Some(hosted_task_status_label(snapshot.status)),
            Some(serialize_event_extra(ApprovalResolvedEventPayload {
                approval_kind: request.approval_kind,
                action: request.action,
                resolved_by: request.resolved_by,
                reason: request.reason,
                extra: Map::new(),
            })),
        ),
        None,
    ));
    state.persist_state().map_err(AppError::internal)?;

    Ok(Json(snapshot))
}

async fn complete_task(
    Path(task_id): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<CompleteTaskRequest>,
) -> Result<Json<HostedTaskSnapshot>, AppError> {
    let mut context = state
        .context_for(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    let worker_id = context
        .worker_id
        .clone()
        .ok_or_else(|| AppError::bad_request(format!("task {task_id} has no active worker")))?;

    let completion = state
        .lane_transport
        .observe_completion(&worker_id, &request.finish_reason, request.tokens_output)
        .map_err(AppError::internal)?;

    if let Some(result) = request
        .result
        .as_deref()
        .filter(|result| !result.trim().is_empty())
    {
        state
            .tasks
            .append_output(&task_id, result)
            .map_err(AppError::internal)?;
    }

    context.worker_status = Some(completion.worker_status.to_string());
    state.record_context(&task_id, context.clone());
    let failure_error = request.error.clone().or(completion.error.clone());

    match completion.worker_status {
        WorkerStatus::Finished => {
            state
                .tasks
                .set_status(&task_id, TaskStatus::Completed)
                .map_err(AppError::internal)?;
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneGreen,
                HostedEventStatus::Completed,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task_id.clone()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("completed"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        finish_reason: Some(request.finish_reason.clone()),
                        tokens_output: Some(request.tokens_output),
                        result: request.result.clone(),
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
        }
        WorkerStatus::Failed => {
            state
                .tasks
                .set_status(&task_id, TaskStatus::Failed)
                .map_err(AppError::internal)?;
            if let Some(error) = failure_error.as_deref() {
                state
                    .tasks
                    .append_output(&task_id, error)
                    .map_err(AppError::internal)?;
            }
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneFailed,
                HostedEventStatus::Failed,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task_id.clone()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("failed"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        finish_reason: Some(request.finish_reason.clone()),
                        tokens_output: Some(request.tokens_output),
                        result: request.result.clone(),
                        error: failure_error,
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
        }
        status => {
            return Err(AppError::internal(format!(
                "unexpected worker completion status: {status}"
            )));
        }
    }
    state.persist_state().map_err(AppError::internal)?;

    let task = state
        .tasks
        .get(&task_id)
        .ok_or_else(|| AppError::not_found(format!("task not found: {task_id}")))?;
    Ok(Json(state.task_snapshot(task)))
}

async fn events_ws(
    ws: WebSocketUpgrade,
    Query(query): Query<EventStreamQuery>,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| stream_events(socket, state, query))
}

async fn connector_interaction(
    Path(connector): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ConnectorInteractionRequest>,
) -> Json<ConnectorInteractionResponse> {
    if request.action == "orphaned_hosted_agent.cancel"
        || request.action == "orphaned_hosted_agent.retry"
    {
        if let Some(task_id) = request.value.as_deref() {
            let action = if request.action.ends_with(".retry") {
                "retry"
            } else {
                "cancel"
            };
            match resolve_approval(
                Path(task_id.to_string()),
                State(state),
                Json(ResolveApprovalRequest {
                    approval_kind: "orphaned_hosted_agent".to_string(),
                    action: action.to_string(),
                    resolved_by: request.user_id.clone(),
                    reason: Some("resolved from Slack interaction".to_string()),
                }),
            )
            .await
            {
                Ok(Json(snapshot)) => {
                    return Json(ConnectorInteractionResponse {
                        blocks: vec![json!({
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!(
                                    "Approval resolved. Task `{}` applied orphaned hosted-agent action `{}`.",
                                    snapshot.task_id,
                                    action
                                ),
                            },
                        })],
                    });
                }
                Err(error) => {
                    return Json(ConnectorInteractionResponse {
                        blocks: vec![json!({
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("Approval resolution failed: {}", error.message),
                            },
                        })],
                    });
                }
            }
        }
    }

    let detail = request.value.as_deref().unwrap_or("No value supplied");
    Json(ConnectorInteractionResponse {
        blocks: vec![json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("Interaction `{}` received. {}", request.action, detail),
            },
            "context": request.context,
            "user_id": request.user_id,
            "source": connector,
        })],
    })
}

async fn connector_event(
    Path(connector): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ConnectorEventRequest>,
) -> StatusCode {
    let payload =
        ConnectorEventPayload::new(connector, request.event_type, request.user_id, request.data);
    state.broadcast_event(EventEnvelope::new(
        HostedEventName::ConnectorEventReceived,
        HostedEventStatus::Completed,
        HostedEventTopic::Connector,
        EventIdentifiers::default(),
        Some(serde_json::to_value(payload).expect("connector event payload should serialize")),
        None,
    ));
    StatusCode::ACCEPTED
}

async fn stream_events(socket: WebSocket, state: Arc<ServerState>, query: EventStreamQuery) {
    let (mut sender, mut receiver) = socket.split();

    for event in state.replay_events_filtered(&query) {
        if send_event(&state, &mut sender, &event).await.is_err() {
            return;
        }
    }

    let mut rx = state.event_sender.subscribe();
    loop {
        tokio::select! {
            message = receiver.next() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
            event = rx.recv() => match event {
                Ok(event) => {
                    if !event_matches_query(&state, &event, &query) {
                        continue;
                    }
                    if send_event(&state, &mut sender, &event).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

async fn send_event(
    state: &ServerState,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    event: &EventEnvelope,
) -> Result<(), ()> {
    let payload = serde_json::to_string(&state.hydrate_event_for_stream(event)).map_err(|_| ())?;
    sender.send(Message::Text(payload)).await.map_err(|_| ())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneBootstrapResult {
    worker_id: String,
    worker_status: WorkerStatus,
    signals: Vec<LaneTransportSignal>,
    artifacts: Option<LaneWorkerArtifacts>,
}

#[derive(Debug, Clone)]
struct LaneExecutionRequest {
    task_id: String,
    prompt: String,
    repository: Option<String>,
    lane_role: Option<LaneRole>,
    model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneCompletionResult {
    worker_status: WorkerStatus,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneCancellationResult {
    worker_status: Option<WorkerStatus>,
    clear_worker_status: bool,
    detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneWorkerArtifacts {
    manifest_file: String,
    output_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneTransportSignal {
    kind: LaneTransportSignalKind,
    detail: Option<String>,
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaneTransportSignalKind {
    Running,
    Blocked,
    Failed,
}

trait LaneWorkerTransport: Send + Sync + fmt::Debug {
    fn bootstrap(&self, request: &LaneExecutionRequest) -> Result<LaneBootstrapResult, String>;

    fn cancel(
        &self,
        worker_id: &str,
        locator: Option<&HostedAgentLocator>,
    ) -> Result<LaneCancellationResult, String>;

    fn observe_completion(
        &self,
        worker_id: &str,
        finish_reason: &str,
        tokens_output: u64,
    ) -> Result<LaneCompletionResult, String>;
}

#[derive(Debug, Default)]
struct InMemoryLaneWorkerTransport {
    workers: WorkerRegistry,
}

impl InMemoryLaneWorkerTransport {
    fn new() -> Self {
        Self {
            workers: WorkerRegistry::new(),
        }
    }
}

impl LaneWorkerTransport for InMemoryLaneWorkerTransport {
    fn bootstrap(&self, request: &LaneExecutionRequest) -> Result<LaneBootstrapResult, String> {
        let cwd = request.repository.as_deref().unwrap_or(".");
        let worker = self.workers.create(cwd, &[], true);
        let worker_id = worker.worker_id.clone();

        let ready_worker = self.workers.observe(&worker_id, "Ready for input\n>")?;
        if ready_worker.status != WorkerStatus::ReadyForPrompt {
            return Err(format!(
                "worker did not reach ready state: {}",
                ready_worker.status
            ));
        }

        let running_worker = self
            .workers
            .send_prompt(&worker_id, Some(&request.prompt))?;

        Ok(LaneBootstrapResult {
            worker_id,
            worker_status: running_worker.status,
            signals: running_worker
                .events
                .iter()
                .filter_map(signal_from_worker_event)
                .collect(),
            artifacts: None,
        })
    }

    fn observe_completion(
        &self,
        worker_id: &str,
        finish_reason: &str,
        tokens_output: u64,
    ) -> Result<LaneCompletionResult, String> {
        let worker = self
            .workers
            .observe_completion(worker_id, finish_reason, tokens_output)?;
        Ok(LaneCompletionResult {
            worker_status: worker.status,
            error: worker
                .last_error
                .as_ref()
                .map(|error| error.message.clone()),
        })
    }

    fn cancel(
        &self,
        worker_id: &str,
        _locator: Option<&HostedAgentLocator>,
    ) -> Result<LaneCancellationResult, String> {
        let worker = self.workers.terminate(worker_id)?;
        Ok(LaneCancellationResult {
            worker_status: Some(worker.status),
            clear_worker_status: false,
            detail: worker.events.last().and_then(|event| event.detail.clone()),
        })
    }
}

#[derive(Debug, Default)]
struct HostedAgentLaneWorkerTransport;

impl LaneWorkerTransport for HostedAgentLaneWorkerTransport {
    fn bootstrap(&self, request: &LaneExecutionRequest) -> Result<LaneBootstrapResult, String> {
        let handle = launch_hosted_agent(HostedAgentLaunchRequest {
            description: request
                .repository
                .as_ref()
                .map(|repository| format!("Hosted task {} for {}", request.task_id, repository))
                .unwrap_or_else(|| format!("Hosted task {}", request.task_id)),
            prompt: request.prompt.clone(),
            subagent_type: Some(hosted_subagent_type(request.lane_role).to_string()),
            name: Some(format!("lane-{}", request.task_id)),
            model: request.model.clone(),
            hosted_task_id: Some(request.task_id.clone()),
        })?;

        Ok(LaneBootstrapResult {
            worker_id: handle.agent_id.clone(),
            worker_status: WorkerStatus::Running,
            signals: vec![LaneTransportSignal {
                kind: LaneTransportSignalKind::Running,
                detail: Some(format!("hosted agent {} spawned", handle.name)),
                payload: Some(json!({
                    "agent_id": handle.agent_id,
                    "agent_name": handle.name,
                    "manifest_file": handle.manifest_file,
                    "output_file": handle.output_file,
                    "status": handle.status,
                })),
            }],
            artifacts: Some(LaneWorkerArtifacts {
                manifest_file: handle.manifest_file,
                output_file: handle.output_file,
            }),
        })
    }

    fn observe_completion(
        &self,
        _worker_id: &str,
        _finish_reason: &str,
        _tokens_output: u64,
    ) -> Result<LaneCompletionResult, String> {
        Err("hosted agent lanes report completion asynchronously via callback".to_string())
    }

    fn cancel(
        &self,
        worker_id: &str,
        locator: Option<&HostedAgentLocator>,
    ) -> Result<LaneCancellationResult, String> {
        let cancellation =
            cancel_hosted_agent_with_locator(locator.unwrap_or(&HostedAgentLocator {
                agent_id: Some(worker_id.to_string()),
                ..HostedAgentLocator::default()
            }));
        match cancellation.source {
            HostedAgentCancellationSource::LiveControl
            | HostedAgentCancellationSource::AlreadyTerminal => Ok(LaneCancellationResult {
                worker_status: Some(WorkerStatus::Finished),
                clear_worker_status: false,
                detail: Some(cancellation.detail),
            }),
            HostedAgentCancellationSource::ManifestFallback => Ok(LaneCancellationResult {
                worker_status: None,
                clear_worker_status: true,
                detail: Some(format!(
                    "{}; live executor was not attached",
                    cancellation.detail
                )),
            }),
            HostedAgentCancellationSource::NotFound => Err(cancellation.detail),
        }
    }
}

fn create_task_internal(state: &ServerState, request: CreateTaskRequest) -> Result<Task, AppError> {
    if request.prompt.trim().is_empty() {
        return Err(AppError::bad_request("prompt must not be empty"));
    }

    let description = request.repository.as_deref();
    let task = state.tasks.create(&request.prompt, description);
    let task_id = task.task_id.clone();
    let source = map_work_item_source(request.source.as_deref());
    let priority = map_priority(request.priority.as_deref());
    let work_item = WorkItem::new(
        request.prompt.clone(),
        source,
        request.repository.clone(),
        request.branch.clone(),
        priority,
        build_work_item_context(&request),
    );
    let plan = plan_work_item(work_item);
    let primary_lane = plan.lanes.first().cloned();
    let lane_id = primary_lane.as_ref().map(|lane| lane.lane_id.clone());
    let plan_kind = primary_lane
        .as_ref()
        .map(|lane| lane_role_label(lane.role).to_string());
    let mut context = HostedTaskContext {
        source: request.source,
        user_id: request.user_id,
        channel_id: request.channel_id,
        thread_ts: request.thread_ts,
        approval_message_ts: None,
        orphaned_at: None,
        orphaned_approval_requested_at: None,
        orphan_auto_retry_attempted_at: None,
        applied_orphan_policy: None,
        repository: request.repository,
        branch: request.branch,
        priority: request.priority,
        plan_id: Some(plan.plan_id.clone()),
        plan_kind: plan_kind.clone(),
        work_item_id: Some(plan.work_item.work_item_id.clone()),
        lane_id: lane_id.clone(),
        worker_id: None,
        worker_status: None,
        worker_manifest_file: None,
        worker_output_file: None,
        model: request.model,
        provider: request.provider,
        permission_mode: request.permission_mode,
        allowed_tools: request.allowed_tools.unwrap_or_default(),
    };
    state.record_context(&task_id, context.clone());
    if let Some(lane_id) = lane_id.as_deref() {
        state
            .tasks
            .assign_team(&task_id, lane_id)
            .map_err(AppError::internal)?;
    }
    state.persist_state().map_err(AppError::internal)?;

    state.broadcast_event(EventEnvelope::new(
        HostedEventName::TaskCreated,
        HostedEventStatus::Pending,
        HostedEventTopic::Task,
        EventIdentifiers {
            repo_id: context.repository.clone(),
            task_id: Some(task_id.clone()),
            ..EventIdentifiers::default()
        },
        build_event_payload(
            &context,
            Some("pending"),
            Some(json!({
                "status": HostedTaskStatus::Pending,
                "prompt": task.prompt,
                "source": context.source,
                "priority": context.priority,
            })),
        ),
        None,
    ));

    state.broadcast_event(EventEnvelope::new(
        HostedEventName::TaskRouted,
        HostedEventStatus::Running,
        HostedEventTopic::Task,
        EventIdentifiers {
            repo_id: context.repository.clone(),
            lane_id: lane_id.clone(),
            task_id: Some(task_id.clone()),
            ..EventIdentifiers::default()
        },
        build_event_payload(
            &context,
            Some("pending"),
            Some(serialize_event_extra(TaskRoutedEventPayload {
                lane_count: plan.lanes.len(),
            })),
        ),
        None,
    ));

    let primary_role = primary_lane.as_ref().map(|lane| lane.role);
    let primary_description = primary_lane.as_ref().map(|lane| lane.description.clone());
    bootstrap_task_lane(
        state,
        &task,
        &mut context,
        primary_role,
        primary_description.as_deref(),
    )?;

    state
        .tasks
        .get(&task_id)
        .ok_or_else(|| AppError::internal("task was created but could not be reloaded"))
}

fn bootstrap_task_lane(
    state: &ServerState,
    task: &Task,
    context: &mut HostedTaskContext,
    primary_role: Option<LaneRole>,
    primary_description: Option<&str>,
) -> Result<(), AppError> {
    let lane_request = LaneExecutionRequest {
        task_id: task.task_id.clone(),
        prompt: task.prompt.clone(),
        repository: context.repository.clone(),
        lane_role: primary_role,
        model: context.model.clone(),
    };

    match state
        .lane_transport
        .bootstrap(&lane_request)
        .map_err(AppError::internal)
    {
        Ok(bootstrap) => {
            context.worker_id = Some(bootstrap.worker_id.clone());
            context.worker_status = Some(bootstrap.worker_status.to_string());
            context.worker_manifest_file = bootstrap
                .artifacts
                .as_ref()
                .map(|artifacts| artifacts.manifest_file.clone());
            context.worker_output_file = bootstrap
                .artifacts
                .as_ref()
                .map(|artifacts| artifacts.output_file.clone());
            context.orphaned_at = None;
            context.orphaned_approval_requested_at = None;
            context.applied_orphan_policy = None;
            state.record_context(&task.task_id, context.clone());

            emit_lane_transport_signals(
                state,
                context,
                &task.task_id,
                primary_role,
                primary_description,
                &bootstrap.signals,
            );

            if bootstrap.worker_status == WorkerStatus::Running {
                state
                    .tasks
                    .set_status(&task.task_id, TaskStatus::Running)
                    .map_err(AppError::internal)?;
            }
            state.persist_state().map_err(AppError::internal)?;
            Ok(())
        }
        Err(error) => {
            context.worker_status = Some("failed".to_string());
            context.orphaned_at = None;
            context.orphaned_approval_requested_at = None;
            context.applied_orphan_policy = None;
            state.record_context(&task.task_id, context.clone());
            state
                .tasks
                .set_status(&task.task_id, TaskStatus::Failed)
                .map_err(AppError::internal)?;
            state
                .tasks
                .append_output(&task.task_id, &error.message)
                .map_err(AppError::internal)?;
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneFailed,
                HostedEventStatus::Failed,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    repo_id: context.repository.clone(),
                    lane_id: context.lane_id.clone(),
                    task_id: Some(task.task_id.clone()),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    context,
                    Some("failed"),
                    Some(serialize_event_extra(TerminalEventPayload {
                        error: Some(error.message.clone()),
                        extra: Map::new(),
                        ..TerminalEventPayload::default()
                    })),
                ),
                None,
            ));
            state.persist_state().map_err(AppError::internal)?;
            Ok(())
        }
    }
}

fn signal_from_worker_event(
    event: &orbit_runtime::worker_boot::WorkerEvent,
) -> Option<LaneTransportSignal> {
    let kind = match event.kind {
        WorkerEventKind::Running => LaneTransportSignalKind::Running,
        WorkerEventKind::TrustRequired | WorkerEventKind::PromptReplayArmed => {
            LaneTransportSignalKind::Blocked
        }
        WorkerEventKind::PromptMisdelivery | WorkerEventKind::Failed => {
            LaneTransportSignalKind::Failed
        }
        _ => return None,
    };

    Some(LaneTransportSignal {
        kind,
        detail: event.detail.clone(),
        payload: event
            .payload
            .as_ref()
            .map(worker_payload_to_json)
            .transpose()
            .ok()
            .flatten(),
    })
}

fn worker_payload_to_json(
    payload: &WorkerEventPayload,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(payload)
}

fn emit_lane_transport_signals(
    state: &ServerState,
    context: &HostedTaskContext,
    task_id: &str,
    primary_role: Option<LaneRole>,
    primary_description: Option<&str>,
    signals: &[LaneTransportSignal],
) {
    for signal in signals {
        let event_name = match signal.kind {
            LaneTransportSignalKind::Running => HostedEventName::LaneStarted,
            LaneTransportSignalKind::Blocked => HostedEventName::LaneBlocked,
            LaneTransportSignalKind::Failed => HostedEventName::LaneFailed,
        };
        let status = match signal.kind {
            LaneTransportSignalKind::Running => HostedEventStatus::Running,
            LaneTransportSignalKind::Blocked => HostedEventStatus::Blocked,
            LaneTransportSignalKind::Failed => HostedEventStatus::Failed,
        };
        let task_status = match signal.kind {
            LaneTransportSignalKind::Running => Some("running"),
            LaneTransportSignalKind::Blocked => Some("pending"),
            LaneTransportSignalKind::Failed => Some("failed"),
        };

        let payload = serialize_event_extra(LaneSignalEventPayload {
            role: primary_role.map(lane_role_label).map(str::to_string),
            description: primary_description.map(str::to_string),
            detail: signal.detail.clone(),
            transport: signal.payload.clone(),
            extra: Map::new(),
        });

        state.broadcast_event(EventEnvelope::new(
            event_name,
            status,
            HostedEventTopic::Lane,
            EventIdentifiers {
                repo_id: context.repository.clone(),
                lane_id: context.lane_id.clone(),
                task_id: Some(task_id.to_string()),
                ..EventIdentifiers::default()
            },
            build_event_payload(context, task_status, Some(payload)),
            None,
        ));
    }
}

fn build_work_item_context(request: &CreateTaskRequest) -> WorkItemContext {
    let mut metadata = std::collections::HashMap::new();
    insert_metadata(&mut metadata, "source", request.source.clone());
    insert_metadata(&mut metadata, "user_id", request.user_id.clone());
    insert_metadata(&mut metadata, "channel_id", request.channel_id.clone());
    insert_metadata(&mut metadata, "thread_ts", request.thread_ts.clone());
    insert_metadata(&mut metadata, "model", request.model.clone());
    insert_metadata(&mut metadata, "provider", request.provider.clone());
    insert_metadata(
        &mut metadata,
        "permission_mode",
        request.permission_mode.clone(),
    );
    if let Some(allowed_tools) = request.allowed_tools.clone() {
        if !allowed_tools.is_empty() {
            metadata.insert("allowed_tools".to_string(), allowed_tools.join(","));
        }
    }
    WorkItemContext { metadata }
}

fn insert_metadata(
    metadata: &mut std::collections::HashMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), value);
    }
}

fn map_work_item_source(source: Option<&str>) -> WorkItemSource {
    match source.unwrap_or_default().to_ascii_lowercase().as_str() {
        "slack" => WorkItemSource::Slack,
        "linear" => WorkItemSource::Linear,
        "github" => WorkItemSource::Github,
        "cron" => WorkItemSource::Cron,
        "webhook" => WorkItemSource::Webhook,
        "api" => WorkItemSource::Unknown,
        _ => WorkItemSource::Unknown,
    }
}

fn map_priority(priority: Option<&str>) -> WorkItemPriority {
    match priority.unwrap_or("medium").to_ascii_lowercase().as_str() {
        "low" => WorkItemPriority::Low,
        "high" => WorkItemPriority::High,
        _ => WorkItemPriority::Medium,
    }
}

fn lane_role_label(role: LaneRole) -> &'static str {
    match role {
        LaneRole::Triager => "triage",
        LaneRole::Planner => "planning",
        LaneRole::Implementer => "implementation",
        LaneRole::Reviewer => "review",
        LaneRole::Verifier => "verification",
        LaneRole::Release => "release",
        LaneRole::Maintenance => "maintenance",
    }
}

fn lane_role_from_plan_kind(plan_kind: Option<&str>) -> Option<LaneRole> {
    match plan_kind? {
        "triage" => Some(LaneRole::Triager),
        "planning" => Some(LaneRole::Planner),
        "implementation" => Some(LaneRole::Implementer),
        "review" => Some(LaneRole::Reviewer),
        "verification" => Some(LaneRole::Verifier),
        "release" => Some(LaneRole::Release),
        "maintenance" => Some(LaneRole::Maintenance),
        _ => None,
    }
}

fn hosted_subagent_type(role: Option<LaneRole>) -> &'static str {
    match role {
        Some(LaneRole::Planner) => "Plan",
        Some(LaneRole::Reviewer) | Some(LaneRole::Verifier) | Some(LaneRole::Triager) => {
            "Verification"
        }
        _ => "general-purpose",
    }
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::Request;
    use std::fs;
    use std::path::PathBuf;
    use tower::util::ServiceExt;

    #[derive(Debug)]
    struct FailingLaneWorkerTransport;

    impl LaneWorkerTransport for FailingLaneWorkerTransport {
        fn bootstrap(
            &self,
            _request: &LaneExecutionRequest,
        ) -> Result<LaneBootstrapResult, String> {
            Err("simulated lane transport failure".to_string())
        }

        fn cancel(
            &self,
            _worker_id: &str,
            _locator: Option<&HostedAgentLocator>,
        ) -> Result<LaneCancellationResult, String> {
            Err("simulated lane transport failure".to_string())
        }

        fn observe_completion(
            &self,
            _worker_id: &str,
            _finish_reason: &str,
            _tokens_output: u64,
        ) -> Result<LaneCompletionResult, String> {
            Err("simulated lane transport failure".to_string())
        }
    }

    #[derive(Debug)]
    struct BlockedLaneWorkerTransport;

    impl LaneWorkerTransport for BlockedLaneWorkerTransport {
        fn bootstrap(
            &self,
            _request: &LaneExecutionRequest,
        ) -> Result<LaneBootstrapResult, String> {
            Ok(LaneBootstrapResult {
                worker_id: "worker_blocked".to_string(),
                worker_status: WorkerStatus::ReadyForPrompt,
                signals: vec![LaneTransportSignal {
                    kind: LaneTransportSignalKind::Blocked,
                    detail: Some("trust prompt detected".to_string()),
                    payload: Some(json!({
                        "type": "trust_prompt",
                        "cwd": "repo-blocked",
                        "resolution": null
                    })),
                }],
                artifacts: None,
            })
        }

        fn cancel(
            &self,
            _worker_id: &str,
            _locator: Option<&HostedAgentLocator>,
        ) -> Result<LaneCancellationResult, String> {
            Ok(LaneCancellationResult {
                worker_status: Some(WorkerStatus::Finished),
                clear_worker_status: false,
                detail: Some("blocked worker cancelled".to_string()),
            })
        }

        fn observe_completion(
            &self,
            _worker_id: &str,
            _finish_reason: &str,
            _tokens_output: u64,
        ) -> Result<LaneCompletionResult, String> {
            Ok(LaneCompletionResult {
                worker_status: WorkerStatus::Finished,
                error: None,
            })
        }
    }

    #[derive(Debug)]
    struct ArtifactLaneWorkerTransport {
        worker_id: String,
        manifest_file: String,
        output_file: String,
    }

    impl LaneWorkerTransport for ArtifactLaneWorkerTransport {
        fn bootstrap(
            &self,
            _request: &LaneExecutionRequest,
        ) -> Result<LaneBootstrapResult, String> {
            Ok(LaneBootstrapResult {
                worker_id: self.worker_id.clone(),
                worker_status: WorkerStatus::Running,
                signals: vec![LaneTransportSignal {
                    kind: LaneTransportSignalKind::Running,
                    detail: Some("hosted artifact lane spawned".to_string()),
                    payload: None,
                }],
                artifacts: Some(LaneWorkerArtifacts {
                    manifest_file: self.manifest_file.clone(),
                    output_file: self.output_file.clone(),
                }),
            })
        }

        fn cancel(
            &self,
            _worker_id: &str,
            _locator: Option<&HostedAgentLocator>,
        ) -> Result<LaneCancellationResult, String> {
            Ok(LaneCancellationResult {
                worker_status: Some(WorkerStatus::Finished),
                clear_worker_status: false,
                detail: Some("artifact lane cancellation is control-plane only".to_string()),
            })
        }

        fn observe_completion(
            &self,
            _worker_id: &str,
            _finish_reason: &str,
            _tokens_output: u64,
        ) -> Result<LaneCompletionResult, String> {
            Err("artifact lane completion is callback-only".to_string())
        }
    }

    #[derive(Debug)]
    struct ManifestBackedCancellationLaneWorkerTransport {
        worker_id: String,
        manifest_file: String,
        output_file: String,
    }

    impl LaneWorkerTransport for ManifestBackedCancellationLaneWorkerTransport {
        fn bootstrap(
            &self,
            _request: &LaneExecutionRequest,
        ) -> Result<LaneBootstrapResult, String> {
            Ok(LaneBootstrapResult {
                worker_id: self.worker_id.clone(),
                worker_status: WorkerStatus::Running,
                signals: vec![LaneTransportSignal {
                    kind: LaneTransportSignalKind::Running,
                    detail: Some("hosted lane restored without live executor".to_string()),
                    payload: None,
                }],
                artifacts: Some(LaneWorkerArtifacts {
                    manifest_file: self.manifest_file.clone(),
                    output_file: self.output_file.clone(),
                }),
            })
        }

        fn cancel(
            &self,
            _worker_id: &str,
            _locator: Option<&HostedAgentLocator>,
        ) -> Result<LaneCancellationResult, String> {
            Ok(LaneCancellationResult {
                worker_status: None,
                clear_worker_status: true,
                detail: Some(
                    "hosted agent cancellation restored from manifest; live executor was not attached"
                        .to_string(),
                ),
            })
        }

        fn observe_completion(
            &self,
            _worker_id: &str,
            _finish_reason: &str,
            _tokens_output: u64,
        ) -> Result<LaneCompletionResult, String> {
            Err("manifest-backed lane completion is callback-only".to_string())
        }
    }

    #[derive(Debug)]
    struct RetryableLaneWorkerTransport {
        prefix: String,
        counter: Mutex<u64>,
    }

    impl RetryableLaneWorkerTransport {
        fn new(prefix: &str) -> Self {
            Self {
                prefix: prefix.to_string(),
                counter: Mutex::new(0),
            }
        }
    }

    impl LaneWorkerTransport for RetryableLaneWorkerTransport {
        fn bootstrap(
            &self,
            _request: &LaneExecutionRequest,
        ) -> Result<LaneBootstrapResult, String> {
            let mut counter = self.counter.lock().expect("retry counter lock poisoned");
            *counter += 1;
            let worker_id = format!("{}-{}", self.prefix, *counter);
            Ok(LaneBootstrapResult {
                worker_id: worker_id.clone(),
                worker_status: WorkerStatus::Running,
                signals: vec![LaneTransportSignal {
                    kind: LaneTransportSignalKind::Running,
                    detail: Some(format!("retryable worker {worker_id} started")),
                    payload: None,
                }],
                artifacts: Some(LaneWorkerArtifacts {
                    manifest_file: format!("{worker_id}.json"),
                    output_file: format!("{worker_id}.md"),
                }),
            })
        }

        fn cancel(
            &self,
            _worker_id: &str,
            _locator: Option<&HostedAgentLocator>,
        ) -> Result<LaneCancellationResult, String> {
            Ok(LaneCancellationResult {
                worker_status: Some(WorkerStatus::Finished),
                clear_worker_status: false,
                detail: Some("retryable worker cancelled".to_string()),
            })
        }

        fn observe_completion(
            &self,
            _worker_id: &str,
            _finish_reason: &str,
            _tokens_output: u64,
        ) -> Result<LaneCompletionResult, String> {
            Err("retryable lane completion is callback-only".to_string())
        }
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_hosted_agent_manifest(
        path: &PathBuf,
        output_path: &PathBuf,
        status: &str,
        error: Option<&str>,
    ) {
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "agentId": path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("agent-test"),
                "name": "test-agent",
                "description": "test hosted agent artifact",
                "subagentType": "Explore",
                "model": "claude-opus-4-6",
                "status": status,
                "outputFile": output_path.display().to_string(),
                "manifestFile": path.display().to_string(),
                "createdAt": "2026-04-08T00:00:00Z",
                "startedAt": "2026-04-08T00:00:00Z",
                "laneEvents": [],
                "currentBlocker": null,
                "hostedTaskId": null,
                "derivedState": if status == "completed" { "finished_cleanable" } else { "truly_idle" },
                "completedAt": if matches!(status, "completed" | "failed" | "cancelled") {
                    Some("2026-04-08T00:00:01Z")
                } else {
                    None::<&str>
                },
                "error": error,
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn write_hosted_agent_output(
        path: &PathBuf,
        final_response: Option<&str>,
        error: Option<&str>,
    ) {
        let mut contents = String::from("# Agent Task\n");
        if let Some(final_response) = final_response {
            contents.push_str("\n### Final response\n\n");
            contents.push_str(final_response);
            contents.push('\n');
        }
        if let Some(error) = error {
            contents.push_str("\n### Error\n\n");
            contents.push_str(error);
            contents.push('\n');
        }
        fs::write(path, contents).unwrap();
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = app(Arc::new(ServerState::default()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_task_persists_and_gets_exposed() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Investigate flaky test",
                            "repository": "repo-a",
                            "source": "slack",
                            "user_id": "U123",
                            "channel_id": "C456"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created = state.tasks.list(None);
        assert_eq!(created.len(), 1);

        let task_id = created[0].task_id.clone();
        let get_response = router
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["lane_id"].as_str().map(str::is_empty), Some(false));
        assert_eq!(
            snapshot["plan_kind"].as_str().map(str::is_empty),
            Some(false)
        );
        assert_eq!(
            snapshot["worker_id"].as_str().map(str::is_empty),
            Some(false)
        );
        assert_eq!(snapshot["worker_status"], "running");

        let events = state.replay_events();
        let created_event = events
            .iter()
            .find(|event| event.event == HostedEventName::TaskCreated)
            .expect("task created event should be emitted");
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("task_status")),
            Some(&json!("pending"))
        );
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("source")),
            Some(&json!("slack"))
        );
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("channel_id")),
            Some(&json!("C456"))
        );

        let started_event = events
            .iter()
            .find(|event| event.event == HostedEventName::LaneStarted)
            .expect("lane started event should be emitted");
        assert_eq!(
            started_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("task_status")),
            Some(&json!("running"))
        );
        assert_eq!(
            started_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("repository")),
            Some(&json!("repo-a"))
        );
        assert!(started_event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("plan_kind"))
            .and_then(|value| value.as_str())
            .is_some());
    }

    #[tokio::test]
    async fn update_task_context_persists_slack_thread_anchor() {
        let dir = temp_test_dir("orbit-server-context-update");
        let state_file = dir.join("state.json");
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist Slack thread anchor",
                            "source": "slack",
                            "channel_id": "C123",
                            "user_id": "U123"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let update_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/context"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "thread_ts": "1712345678.999999",
                            "channel_id": "C123",
                            "approval_message_ts": "1712345679.111111"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_response.status(), StatusCode::OK);
        let update_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: serde_json::Value = serde_json::from_slice(&update_body).unwrap();
        assert_eq!(updated["thread_ts"], "1712345678.999999");
        assert_eq!(updated["channel_id"], "C123");
        assert_eq!(updated["approval_message_ts"], "1712345679.111111");

        let restored = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let restored_context = restored
            .context_for(&task_id)
            .expect("context should reload from state file");
        assert_eq!(
            restored_context.thread_ts.as_deref(),
            Some("1712345678.999999")
        );
        assert_eq!(restored_context.channel_id.as_deref(), Some("C123"));
        assert_eq!(
            restored_context.approval_message_ts.as_deref(),
            Some("1712345679.111111")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn list_tasks_filters_by_slack_context() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        for payload in [
            json!({
                "prompt": "First Slack task",
                "source": "slack",
                "channel_id": "C-match",
                "thread_ts": "T-match",
                "user_id": "U-1"
            }),
            json!({
                "prompt": "Second Slack task",
                "source": "slack",
                "channel_id": "C-other",
                "thread_ts": "T-other",
                "user_id": "U-2"
            }),
            json!({
                "prompt": "API task",
                "source": "api",
                "repository": "repo-api"
            }),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/tasks")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let list_response = router
            .oneshot(
                Request::builder()
                    .uri("/v1/tasks?source=slack&channel_id=C-match&thread_ts=T-match")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);
        let body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshots: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tasks = snapshots
            .as_array()
            .expect("list response should be an array");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["prompt"], "First Slack task");
        assert_eq!(tasks[0]["source"], "slack");
        assert_eq!(tasks[0]["channel_id"], "C-match");
        assert_eq!(tasks[0]["thread_ts"], "T-match");
    }

    #[tokio::test]
    async fn list_tasks_supports_csv_status_filters() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let cancelled_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Slack cancelled task",
                            "source": "slack",
                            "channel_id": "C-active"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cancelled_response.status(), StatusCode::OK);
        let cancelled_body = to_bytes(cancelled_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let cancelled_task: serde_json::Value = serde_json::from_slice(&cancelled_body).unwrap();
        let cancelled_task_id = cancelled_task["task_id"]
            .as_str()
            .expect("task response should include task_id");

        let cancel_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{cancelled_task_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cancel_response.status(), StatusCode::OK);

        let active_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Slack active task",
                            "source": "slack",
                            "channel_id": "C-active"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(active_response.status(), StatusCode::OK);

        let list_response = router
            .oneshot(
                Request::builder()
                    .uri("/v1/tasks?source=slack&status=pending,running")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);
        let body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshots: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tasks = snapshots
            .as_array()
            .expect("list response should be an array");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["prompt"], "Slack active task");
        assert_eq!(tasks[0]["status"], "running");
    }

    #[tokio::test]
    async fn orphan_policy_endpoint_returns_default_policy_and_rules() {
        let state = Arc::new(ServerState::with_lane_transport_and_policy_rules(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::default()),
            Duration::from_secs(120),
            Some(Duration::from_secs(60)),
            Some(Duration::from_secs(600)),
            vec![OrphanPolicyRule {
                repository: Some("repo-ops".to_string()),
                source: Some("slack".to_string()),
                priority: Some("high".to_string()),
                approval_delay_secs: Some(30),
                auto_retry_after_secs: Some(15),
                auto_cancel_after_secs: Some(300),
            }],
        ));
        let router = app(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/v1/policies/orphans")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["default_policy"]["source"], "default");
        assert_eq!(snapshot["default_policy"]["approval_delay_secs"], 120);
        assert_eq!(snapshot["default_policy"]["auto_retry_after_secs"], 60);
        assert_eq!(snapshot["default_policy"]["auto_cancel_after_secs"], 600);
        assert_eq!(snapshot["effective_policy"]["source"], "default");
        assert_eq!(snapshot["configured_rules"][0]["repository"], "repo-ops");
        assert_eq!(snapshot["configured_rules"][0]["source"], "slack");
        assert_eq!(snapshot["configured_rules"][0]["priority"], "high");
        assert_eq!(snapshot["configured_rules"][0]["approval_delay_secs"], 30);
    }

    #[tokio::test]
    async fn orphan_policy_endpoint_previews_scoped_rule_match() {
        let state = Arc::new(ServerState::with_lane_transport_and_policy_rules(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::default()),
            Duration::from_secs(300),
            None,
            None,
            vec![OrphanPolicyRule {
                repository: Some("repo-fast-policy".to_string()),
                source: Some("api".to_string()),
                priority: None,
                approval_delay_secs: Some(0),
                auto_retry_after_secs: Some(30),
                auto_cancel_after_secs: None,
            }],
        ));
        let router = app(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/v1/policies/orphans?repository=repo-fast-policy&source=api")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["preview"]["repository"], "repo-fast-policy");
        assert_eq!(snapshot["preview"]["source"], "api");
        assert_eq!(snapshot["effective_policy"]["source"], "rule");
        assert_eq!(
            snapshot["effective_policy"]["match_repository"],
            "repo-fast-policy"
        );
        assert_eq!(snapshot["effective_policy"]["approval_delay_secs"], 0);
        assert_eq!(snapshot["effective_policy"]["auto_retry_after_secs"], 30);
        assert!(snapshot["effective_policy"]["auto_cancel_after_secs"].is_null());
    }

    #[tokio::test]
    async fn failing_lane_transport_marks_task_failed() {
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(FailingLaneWorkerTransport),
        ));
        let app = app(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Ship the release train",
                            "repository": "repo-b",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(created["status"], "failed");
        assert_eq!(created["worker_status"], "failed");

        let tasks = state.tasks.list(None);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Failed);
        assert!(tasks[0].output.contains("simulated lane transport failure"));
    }

    #[tokio::test]
    async fn blocked_lane_transport_emits_lane_blocked_event() {
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(BlockedLaneWorkerTransport),
        ));
        let app = app(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Needs repository approval",
                            "repository": "repo-blocked",
                            "source": "slack"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(created["status"], "pending");
        assert_eq!(created["worker_status"], "ready_for_prompt");

        let tasks = state.tasks.list(None);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Created);

        let blocked_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::LaneBlocked)
            .expect("lane blocked event should be emitted");
        assert_eq!(blocked_event.status, HostedEventStatus::Blocked);
        assert_eq!(blocked_event.repo_id.as_deref(), Some("repo-blocked"));
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("detail"))
                .and_then(|value| value.as_str()),
            Some("trust prompt detected")
        );
    }

    #[tokio::test]
    async fn cancel_task_stops_running_worker_and_emits_task_cancelled() {
        let state = Arc::new(ServerState::default());
        let app = app(state.clone());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Cancel the running worker",
                            "repository": "repo-cancel",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let cancel_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(cancel_response.status(), StatusCode::OK);
        let cancel_body = to_bytes(cancel_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&cancel_body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");
        assert_eq!(snapshot["worker_status"], "finished");

        let cancelled_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::TaskCancelled)
            .expect("task cancelled event should be emitted");
        assert_eq!(cancelled_event.status, HostedEventStatus::Cancelled);
        assert_eq!(cancelled_event.repo_id.as_deref(), Some("repo-cancel"));
    }

    #[tokio::test]
    async fn cancel_task_marks_hosted_lane_cancelled_even_without_transport_termination() {
        let dir = temp_test_dir("orbit-server-cancel-hosted");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-cancel".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let app = app(state.clone());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Cancel the hosted lane",
                            "repository": "repo-hosted-cancel",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let cancel_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(cancel_response.status(), StatusCode::OK);
        let cancel_body = to_bytes(cancel_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&cancel_body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");
        assert_eq!(snapshot["worker_status"], "finished");

        let cancelled_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::TaskCancelled)
            .expect("task cancelled event should be emitted");
        assert_eq!(cancelled_event.status, HostedEventStatus::Cancelled);
        assert_eq!(
            cancelled_event.repo_id.as_deref(),
            Some("repo-hosted-cancel")
        );
        assert!(cancelled_event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("detail"))
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("control-plane only"));

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn get_task_runtime_reports_hosted_agent_status() {
        let dir = temp_test_dir("orbit-server-runtime-hosted");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-runtime".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let app = app(state.clone());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Inspect hosted runtime state",
                            "repository": "repo-runtime",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();
        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let runtime_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}/runtime"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(runtime_response.status(), StatusCode::OK);
        let body = to_bytes(runtime_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["worker_id"], "agent-hosted-runtime");
        assert_eq!(snapshot["worker_status"], ORPHANED_WORKER_STATUS);
        assert_eq!(snapshot["hostedAgent"]["found"], true);
        assert_eq!(snapshot["hostedAgent"]["liveControl"], false);
        assert_eq!(snapshot["hostedAgent"]["orphaned"], true);
        assert_eq!(snapshot["hostedAgent"]["status"], "running");
        assert_eq!(snapshot["orphan_policy"]["source"], "default");
        assert_eq!(snapshot["orphan_policy"]["approval_delay_secs"], 0);
        assert_eq!(
            snapshot["hostedAgent"]["manifestFile"],
            manifest_file.display().to_string()
        );
        assert_eq!(
            snapshot["hostedAgent"]["outputFile"],
            output_file.display().to_string()
        );
        assert_eq!(
            snapshot["hostedAgent"]["detail"],
            "hosted agent manifest restored from locator path"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn complete_task_marks_task_completed_and_emits_lane_green() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Complete the release checklist",
                            "repository": "repo-green",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let complete_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/complete"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "finish_reason": "stop",
                            "tokens_output": 128,
                            "result": "release completed"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(complete_response.status(), StatusCode::OK);
        let complete_body = to_bytes(complete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&complete_body).unwrap();
        assert_eq!(snapshot["status"], "completed");
        assert_eq!(snapshot["worker_status"], "finished");

        let green_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::LaneGreen)
            .expect("lane green event should be emitted");
        assert_eq!(green_event.status, HostedEventStatus::Completed);
        assert_eq!(green_event.repo_id.as_deref(), Some("repo-green"));
        assert_eq!(
            green_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("finish_reason"))
                .and_then(|value| value.as_str()),
            Some("stop")
        );
    }

    #[tokio::test]
    async fn complete_task_classifies_provider_failure_and_marks_task_failed() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Finish with provider degradation",
                            "repository": "repo-red",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let complete_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/complete"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "finish_reason": "unknown",
                            "tokens_output": 0
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(complete_response.status(), StatusCode::OK);
        let complete_body = to_bytes(complete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&complete_body).unwrap();
        assert_eq!(snapshot["status"], "failed");
        assert_eq!(snapshot["worker_status"], "failed");
        assert!(snapshot["error"]
            .as_str()
            .unwrap()
            .contains("provider degraded"));

        let failed_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::LaneFailed)
            .expect("lane failed event should be emitted");
        assert_eq!(failed_event.status, HostedEventStatus::Failed);
        assert_eq!(failed_event.repo_id.as_deref(), Some("repo-red"));
        assert_eq!(
            failed_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("finish_reason"))
                .and_then(|value| value.as_str()),
            Some("unknown")
        );
    }

    #[tokio::test]
    async fn get_task_reconciles_completed_hosted_agent_from_manifest() {
        let dir = temp_test_dir("orbit-server-reconcile-success");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-success".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Recover completion from hosted agent artifacts",
                            "repository": "repo-hosted-green",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "completed", None);
        write_hosted_agent_output(&output_file, Some("Recovered final answer"), None);

        let get_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "completed");
        assert_eq!(snapshot["worker_status"], "finished");
        assert!(snapshot["result"]
            .as_str()
            .unwrap()
            .contains("Recovered final answer"));

        let green_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::LaneGreen)
            .last()
            .expect("reconciled lane green event should be emitted");
        assert_eq!(green_event.status, HostedEventStatus::Completed);
        assert_eq!(green_event.repo_id.as_deref(), Some("repo-hosted-green"));
        assert_eq!(
            green_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("reconciled"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn reconcile_endpoint_marks_failed_hosted_agent_from_manifest() {
        let dir = temp_test_dir("orbit-server-reconcile-failed");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-failed".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Recover failure from hosted agent artifacts",
                            "repository": "repo-hosted-red",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(
            &manifest_file,
            &output_file,
            "failed",
            Some("agent crashed before callback"),
        );
        write_hosted_agent_output(&output_file, None, Some("agent crashed before callback"));

        let reconcile_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "failed");
        assert_eq!(snapshot["worker_status"], "failed");
        assert!(snapshot["error"]
            .as_str()
            .unwrap()
            .contains("agent crashed before callback"));

        let failed_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::LaneFailed)
            .last()
            .expect("reconciled lane failed event should be emitted");
        assert_eq!(failed_event.status, HostedEventStatus::Failed);
        assert_eq!(failed_event.repo_id.as_deref(), Some("repo-hosted-red"));
        assert_eq!(
            failed_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("reconciled"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn reconcile_endpoint_marks_orphaned_hosted_agent_as_blocked() {
        let dir = temp_test_dir("orbit-server-reconcile-orphaned");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphaned".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Recover orphaned hosted agent from artifacts",
                            "repository": "repo-hosted-orphaned",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let reconcile_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["worker_status"], ORPHANED_WORKER_STATUS);

        let blocked_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::LaneBlocked)
            .last()
            .expect("reconciled lane blocked event should be emitted");
        assert_eq!(blocked_event.status, HostedEventStatus::Blocked);
        assert_eq!(
            blocked_event.repo_id.as_deref(),
            Some("repo-hosted-orphaned")
        );
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("reconciled"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphaned"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphan_policy"))
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_str()),
            Some("default")
        );

        let approval_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .last()
            .expect("approval requested event should be emitted");
        assert_eq!(approval_event.status, HostedEventStatus::Pending);
        assert_eq!(
            approval_event.repo_id.as_deref(),
            Some("repo-hosted-orphaned")
        );
        assert_eq!(
            approval_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("approval_kind"))
                .and_then(|value| value.as_str()),
            Some("orphaned_hosted_agent")
        );
        assert_eq!(
            approval_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphaned"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            approval_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphan_policy"))
                .and_then(|value| value.get("approval_delay_secs"))
                .and_then(|value| value.as_u64()),
            Some(0)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn orphaned_hosted_agent_waits_for_approval_delay_and_does_not_duplicate_requests() {
        let dir = temp_test_dir("orbit-server-orphan-delay");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport_and_policy(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphan-delay".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            Duration::from_secs(300),
            None,
            None,
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Delay orphaned approval request",
                            "repository": "repo-hosted-orphan-delay",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let first_reconcile = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first_reconcile.status(), StatusCode::OK);

        let initial_approval_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(initial_approval_count, 0);

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("orphaned task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        orphaned_context.orphaned_at = Some(current_unix_timestamp_secs() - 301);
        state.record_context(&task_id, orphaned_context);

        let delayed_reconcile = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delayed_reconcile.status(), StatusCode::OK);

        let approval_events = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .collect::<Vec<_>>();
        assert_eq!(approval_events.len(), 1);

        let duplicate_reconcile = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(duplicate_reconcile.status(), StatusCode::OK);

        let duplicate_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(duplicate_count, 1);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn orphaned_hosted_agent_auto_cancels_after_timeout() {
        let dir = temp_test_dir("orbit-server-orphan-auto-cancel");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport_and_policy(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphan-auto-cancel".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            Duration::from_secs(300),
            None,
            Some(Duration::from_secs(60)),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Auto-cancel orphaned hosted agent",
                            "repository": "repo-hosted-orphan-auto-cancel",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("orphaned task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        orphaned_context.orphaned_at = Some(current_unix_timestamp_secs() - 61);
        state.record_context(&task_id, orphaned_context);

        let reconcile_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");

        let cancelled_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::TaskCancelled)
            .last()
            .expect("auto-cancelled task event should be emitted");
        assert_eq!(cancelled_event.status, HostedEventStatus::Cancelled);
        assert!(cancelled_event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("detail"))
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("auto-cancelled after orphan timeout"));

        let approval_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(approval_count, 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn orphaned_hosted_agent_auto_retries_once_before_requesting_approval() {
        let dir = temp_test_dir("orbit-server-orphan-auto-retry");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport_and_policy(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphan-auto-retry".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            Duration::from_secs(300),
            Some(Duration::from_secs(60)),
            None,
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Auto-retry orphaned hosted agent",
                            "repository": "repo-hosted-orphan-auto-retry",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("orphaned task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        orphaned_context.orphaned_at = Some(current_unix_timestamp_secs() - 61);
        state.record_context(&task_id, orphaned_context);

        let reconcile_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["worker_status"], "running");

        let started_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::LaneStarted)
            .count();
        assert!(started_count >= 2);

        let approval_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(approval_count, 0);

        let refreshed_context = state
            .context_for(&task_id)
            .expect("task context should exist after auto-retry");
        assert!(refreshed_context.orphan_auto_retry_attempted_at.is_some());
        assert_eq!(refreshed_context.orphaned_at, None);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn orphaned_hosted_agent_requests_approval_after_auto_retry_is_exhausted() {
        let dir = temp_test_dir("orbit-server-orphan-after-auto-retry");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport_and_policy(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphan-after-auto-retry".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            Duration::from_secs(120),
            Some(Duration::from_secs(60)),
            None,
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Fall back to approval after auto-retry",
                            "repository": "repo-hosted-orphan-after-auto-retry",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("orphaned task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        orphaned_context.orphaned_at = Some(current_unix_timestamp_secs() - 121);
        orphaned_context.orphan_auto_retry_attempted_at = Some(current_unix_timestamp_secs() - 61);
        state.record_context(&task_id, orphaned_context);

        let reconcile_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["worker_status"], ORPHANED_WORKER_STATUS);

        let approval_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(approval_count, 1);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn repo_scoped_orphan_policy_rule_overrides_global_defaults() {
        let dir = temp_test_dir("orbit-server-orphan-policy-rule");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport_and_policy_rules(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-orphan-policy-rule".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            Duration::from_secs(300),
            None,
            None,
            vec![OrphanPolicyRule {
                repository: Some("repo-fast-policy".to_string()),
                source: Some("api".to_string()),
                priority: None,
                approval_delay_secs: Some(0),
                auto_retry_after_secs: Some(30),
                auto_cancel_after_secs: None,
            }],
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Use repo-scoped orphan policy",
                            "repository": "repo-fast-policy",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("orphaned task context should exist");
        orphaned_context.orphaned_at = Some(current_unix_timestamp_secs() - 31);
        state.record_context(&task_id, orphaned_context);

        let reconcile_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconcile_response.status(), StatusCode::OK);
        let body = to_bytes(reconcile_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["worker_status"], "running");

        let approval_count = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalRequested)
            .count();
        assert_eq!(approval_count, 0);

        let refreshed_context = state
            .context_for(&task_id)
            .expect("task context should exist after repo-scoped auto-retry");
        assert!(refreshed_context.orphan_auto_retry_attempted_at.is_some());

        let blocked_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::LaneBlocked)
            .last()
            .expect("repo-scoped orphan policy should emit a blocked event");
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphan_policy"))
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_str()),
            Some("rule")
        );
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphan_policy"))
                .and_then(|value| value.get("match_repository"))
                .and_then(|value| value.as_str()),
            Some("repo-fast-policy")
        );
        assert_eq!(
            blocked_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("orphan_policy"))
                .and_then(|value| value.get("auto_retry_after_secs"))
                .and_then(|value| value.as_u64()),
            Some(30)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn resolve_orphaned_hosted_agent_approval_cancels_task() {
        let dir = temp_test_dir("orbit-server-approval-orphaned");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-approval".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Resolve orphaned hosted agent approval",
                            "repository": "repo-hosted-approval",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "running", None);
        write_hosted_agent_output(&output_file, None, None);

        let reconcile_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/reconcile"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reconcile_response.status(), StatusCode::OK);

        let resolve_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/approval"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "approval_kind": "orphaned_hosted_agent",
                            "action": "cancel",
                            "resolved_by": "U-ops",
                            "reason": "operator chose cancel"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resolve_response.status(), StatusCode::OK);
        let body = to_bytes(resolve_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");
        assert_eq!(snapshot["worker_status"], "finished");

        let resolved_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalResolved)
            .last()
            .expect("approval resolved event should be emitted");
        assert_eq!(resolved_event.status, HostedEventStatus::Completed);
        assert_eq!(
            resolved_event.repo_id.as_deref(),
            Some("repo-hosted-approval")
        );
        assert_eq!(
            resolved_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("action"))
                .and_then(|value| value.as_str()),
            Some("cancel")
        );
        assert_eq!(
            resolved_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("resolved_by"))
                .and_then(|value| value.as_str()),
            Some("U-ops")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn resolve_orphaned_hosted_agent_approval_retries_task_lane() {
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(RetryableLaneWorkerTransport::new("retry-worker")),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Retry orphaned hosted agent lane",
                            "repository": "repo-hosted-retry",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();
        assert_eq!(created["worker_id"], "retry-worker-1");
        assert_eq!(created["worker_status"], "running");

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        state.record_context(&task_id, orphaned_context);

        let resolve_response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/approval"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "approval_kind": "orphaned_hosted_agent",
                            "action": "retry",
                            "resolved_by": "U-ops",
                            "reason": "operator chose retry"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resolve_response.status(), StatusCode::OK);
        let body = to_bytes(resolve_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["worker_id"], "retry-worker-2");
        assert_eq!(snapshot["worker_status"], "running");

        let resolved_event = state
            .replay_events()
            .into_iter()
            .filter(|event| event.event == HostedEventName::ApprovalResolved)
            .last()
            .expect("approval resolved event should be emitted");
        assert_eq!(resolved_event.status, HostedEventStatus::Completed);
        assert_eq!(resolved_event.repo_id.as_deref(), Some("repo-hosted-retry"));
        assert_eq!(
            resolved_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("action"))
                .and_then(|value| value.as_str()),
            Some("retry")
        );
        assert_eq!(
            resolved_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("worker_id"))
                .and_then(|value| value.as_str()),
            Some("retry-worker-2")
        );
    }

    #[tokio::test]
    async fn connector_interaction_route_resolves_orphan_approval() {
        let state = Arc::new(ServerState::with_lane_transport(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(RetryableLaneWorkerTransport::new("retry-worker")),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Retry from slack connector interaction",
                            "repository": "repo-slack-connector",
                            "source": "slack"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let mut orphaned_context = state
            .context_for(&task_id)
            .expect("task context should exist");
        orphaned_context.worker_status = Some(ORPHANED_WORKER_STATUS.to_string());
        state.record_context(&task_id, orphaned_context);

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/connectors/slack/interactions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "action": "orphaned_hosted_agent.retry",
                            "value": task_id,
                            "user_id": "U-slack"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["blocks"][0]["type"], "section");
    }

    #[tokio::test]
    async fn connector_event_route_emits_connector_event() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/connectors/slack/events")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "type": "reaction_added",
                            "user_id": "U-slack",
                            "data": {
                                "reaction": "eyes"
                            }
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let connector_event = state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::ConnectorEventReceived)
            .expect("connector event should be emitted");
        assert_eq!(connector_event.status, HostedEventStatus::Completed);
        assert_eq!(
            connector_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("connector"))
                .and_then(|value| value.as_str()),
            Some("slack")
        );
        assert_eq!(
            connector_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("source"))
                .and_then(|value| value.as_str()),
            Some("slack")
        );
        assert_eq!(
            connector_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("type"))
                .and_then(|value| value.as_str()),
            Some("reaction_added")
        );
    }

    #[tokio::test]
    async fn server_state_persists_tasks_and_contexts_across_restart() {
        let dir = temp_test_dir("orbit-server-persist");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let state_file = dir.join("server-state.json");
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(ArtifactLaneWorkerTransport {
                worker_id: "agent-hosted-persist".to_string(),
                manifest_file: manifest_file.display().to_string(),
                output_file: output_file.display().to_string(),
            }),
            state_file.clone(),
        ));
        let app = app(state.clone());

        let create_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist me across restart",
                            "repository": "repo-persisted",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let restored = ServerState::new_with_transport_kind_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            LaneTransportKind::InMemory,
            Some(state_file),
        );
        let restored_task = restored.tasks.get(&task_id).expect("task should reload");
        let restored_context = restored
            .context_for(&task_id)
            .expect("context should reload");

        assert_eq!(restored_task.prompt, "Persist me across restart");
        assert_eq!(restored_task.status, TaskStatus::Running);
        assert_eq!(
            restored_task.team_id.as_deref(),
            restored_context.lane_id.as_deref()
        );
        assert_eq!(
            restored_context.worker_manifest_file.as_deref(),
            Some(manifest_file.display().to_string().as_str())
        );
        assert_eq!(
            restored_context.worker_output_file.as_deref(),
            Some(output_file.display().to_string().as_str())
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn server_state_file_restores_tasks_and_contexts_across_restart() {
        let dir = temp_test_dir("orbit-server-state-restore");
        let state_file = dir.join("state.json");
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let app = app(state.clone());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist me across restart",
                            "repository": "repo-persisted",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        assert!(state_file.exists());
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let restored = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let restored_task = restored
            .tasks
            .get(&task_id)
            .expect("task should reload from state file");
        assert_eq!(restored_task.status, TaskStatus::Running);

        let restored_context = restored
            .context_for(&task_id)
            .expect("context should reload from state file");
        assert_eq!(
            restored_context.repository.as_deref(),
            Some("repo-persisted")
        );
        assert_eq!(restored_context.source.as_deref(), Some("api"));
        assert_eq!(restored_context.worker_status.as_deref(), Some("running"));

        let restored_app = super::app(restored.clone());
        let get_response = restored_app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "running");
        assert_eq!(snapshot["repository"], "repo-persisted");
        assert_eq!(snapshot["worker_status"], "running");

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn server_state_file_restores_event_history_across_restart() {
        let dir = temp_test_dir("orbit-server-event-history");
        let state_file = dir.join("state.json");
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let app = app(state.clone());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist event history across restart",
                            "repository": "repo-events",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);

        let original_events = state.replay_events();
        assert!(original_events
            .iter()
            .any(|event| event.event == HostedEventName::TaskCreated));
        assert!(original_events
            .iter()
            .any(|event| event.event == HostedEventName::TaskRouted));
        let created_event = original_events
            .iter()
            .find(|event| event.event == HostedEventName::TaskCreated)
            .expect("task created event should exist");
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("task_status")),
            Some(&json!("pending"))
        );
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("repository")),
            Some(&json!("repo-events"))
        );
        assert_eq!(
            created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("source")),
            Some(&json!("api"))
        );

        let restored = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            Arc::new(InMemoryLaneWorkerTransport::new()),
            state_file.clone(),
        ));
        let restored_events = restored.replay_events();

        assert!(restored_events
            .iter()
            .any(|event| event.event == HostedEventName::TaskCreated));
        assert!(restored_events
            .iter()
            .any(|event| event.event == HostedEventName::TaskRouted));
        let restored_created_event = restored_events
            .iter()
            .find(|event| event.event == HostedEventName::TaskCreated)
            .expect("restored task created event should exist");
        assert_eq!(
            restored_created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("task_status")),
            Some(&json!("pending"))
        );
        assert_eq!(
            restored_created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("repository")),
            Some(&json!("repo-events"))
        );
        assert_eq!(
            restored_created_event
                .payload
                .as_ref()
                .and_then(|payload| payload.get("source")),
            Some(&json!("api"))
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_events_filtered_matches_task_context_and_event_fields() {
        let state = ServerState::default();
        state.record_context(
            "task-slack",
            HostedTaskContext {
                source: Some("slack".to_string()),
                repository: Some("repo-alpha".to_string()),
                channel_id: Some("C123".to_string()),
                thread_ts: Some("171234.56".to_string()),
                ..HostedTaskContext::default()
            },
        );
        state.record_context(
            "task-api",
            HostedTaskContext {
                source: Some("api".to_string()),
                repository: Some("repo-beta".to_string()),
                ..HostedTaskContext::default()
            },
        );

        state.broadcast_event(EventEnvelope::new(
            HostedEventName::TaskCreated,
            HostedEventStatus::Pending,
            HostedEventTopic::Task,
            EventIdentifiers {
                repo_id: Some("repo-alpha".to_string()),
                task_id: Some("task-slack".to_string()),
                ..EventIdentifiers::default()
            },
            build_event_payload(
                &state
                    .context_for("task-slack")
                    .expect("task context should exist"),
                Some("pending"),
                None,
            ),
            None,
        ));
        state.broadcast_event(EventEnvelope::new(
            HostedEventName::ApprovalRequested,
            HostedEventStatus::Pending,
            HostedEventTopic::Approval,
            EventIdentifiers {
                repo_id: Some("repo-alpha".to_string()),
                task_id: Some("task-slack".to_string()),
                lane_id: Some("lane-1".to_string()),
                ..EventIdentifiers::default()
            },
            build_event_payload(
                &state
                    .context_for("task-slack")
                    .expect("task context should exist"),
                Some("pending"),
                Some(json!({
                    "approval_kind": "orphaned_hosted_agent",
                })),
            ),
            None,
        ));
        state.broadcast_event(EventEnvelope::new(
            HostedEventName::ApprovalRequested,
            HostedEventStatus::Pending,
            HostedEventTopic::Approval,
            EventIdentifiers {
                repo_id: Some("repo-beta".to_string()),
                task_id: Some("task-api".to_string()),
                lane_id: Some("lane-2".to_string()),
                ..EventIdentifiers::default()
            },
            build_event_payload(
                &state
                    .context_for("task-api")
                    .expect("task context should exist"),
                Some("pending"),
                Some(json!({
                    "approval_kind": "orphaned_hosted_agent",
                })),
            ),
            None,
        ));

        let filtered = state.replay_events_filtered(&EventStreamQuery {
            topic: Some("approval".to_string()),
            event: Some("approval.requested".to_string()),
            status: Some("pending".to_string()),
            source: Some("slack".to_string()),
            repository: Some("repo-alpha".to_string()),
            channel_id: Some("C123".to_string()),
            thread_ts: Some("171234.56".to_string()),
            ..EventStreamQuery::default()
        });

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].task_id.as_deref(), Some("task-slack"));
        assert_eq!(filtered[0].lane_id.as_deref(), Some("lane-1"));
        assert_eq!(
            filtered[0]
                .payload
                .as_ref()
                .and_then(|payload| payload.get("task_status")),
            Some(&json!("pending"))
        );
        assert_eq!(
            filtered[0]
                .payload
                .as_ref()
                .and_then(|payload| payload.get("channel_id")),
            Some(&json!("C123"))
        );
        assert_eq!(
            filtered[0]
                .payload
                .as_ref()
                .and_then(|payload| payload.get("thread_ts")),
            Some(&json!("171234.56"))
        );
    }

    #[test]
    fn replay_events_filtered_applies_limit_after_matching() {
        let state = ServerState::default();
        state.record_context(
            "task-limit",
            HostedTaskContext {
                repository: Some("repo-limit".to_string()),
                ..HostedTaskContext::default()
            },
        );

        for (event, status, lane_id) in [
            (
                HostedEventName::TaskCreated,
                HostedEventStatus::Pending,
                None,
            ),
            (
                HostedEventName::TaskRouted,
                HostedEventStatus::Running,
                None,
            ),
            (
                HostedEventName::LaneStarted,
                HostedEventStatus::Running,
                Some("lane-1"),
            ),
        ] {
            let topic = match event {
                HostedEventName::TaskCreated | HostedEventName::TaskRouted => {
                    HostedEventTopic::Task
                }
                _ => HostedEventTopic::Lane,
            };
            let task_status = match event {
                HostedEventName::TaskCreated => Some("pending"),
                HostedEventName::TaskRouted | HostedEventName::LaneStarted => Some("running"),
                _ => None,
            };
            state.broadcast_event(EventEnvelope::new(
                event,
                status,
                topic,
                EventIdentifiers {
                    repo_id: Some("repo-limit".to_string()),
                    task_id: Some("task-limit".to_string()),
                    lane_id: lane_id.map(str::to_string),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &state
                        .context_for("task-limit")
                        .expect("task context should exist"),
                    task_status,
                    None,
                ),
                None,
            ));
        }

        let filtered = state.replay_events_filtered(&EventStreamQuery {
            task_id: Some("task-limit".to_string()),
            limit: Some(2),
            ..EventStreamQuery::default()
        });

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].event, HostedEventName::TaskRouted);
        assert_eq!(filtered[1].event, HostedEventName::LaneStarted);
    }

    #[test]
    fn replay_events_filtered_supports_comma_separated_stream_filters() {
        let state = ServerState::default();
        state.record_context(
            "task-one",
            HostedTaskContext {
                source: Some("slack".to_string()),
                channel_id: Some("C111".to_string()),
                ..HostedTaskContext::default()
            },
        );
        state.record_context(
            "task-two",
            HostedTaskContext {
                source: Some("slack".to_string()),
                channel_id: Some("C222".to_string()),
                ..HostedTaskContext::default()
            },
        );

        for (task_id, channel_id) in [("task-one", "C111"), ("task-two", "C222")] {
            let context = state
                .context_for(task_id)
                .expect("task context should exist");
            state.broadcast_event(EventEnvelope::new(
                HostedEventName::LaneStarted,
                HostedEventStatus::Running,
                HostedEventTopic::Lane,
                EventIdentifiers {
                    task_id: Some(task_id.to_string()),
                    lane_id: Some(format!("lane-{task_id}")),
                    ..EventIdentifiers::default()
                },
                build_event_payload(
                    &context,
                    Some("running"),
                    Some(json!({ "channel_id": channel_id })),
                ),
                None,
            ));
        }

        let filtered = state.replay_events_filtered(&EventStreamQuery {
            task_id: Some("task-one,task-two".to_string()),
            channel_id: Some("C111,C222".to_string()),
            source: Some("slack".to_string()),
            event: Some("lane.started".to_string()),
            ..EventStreamQuery::default()
        });

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].task_id.as_deref(), Some("task-one"));
        assert_eq!(filtered[1].task_id.as_deref(), Some("task-two"));
    }

    #[tokio::test]
    async fn state_file_restores_hosted_task_after_server_restart() {
        let dir = temp_test_dir("orbit-server-state-restore");
        let state_file = dir.join("server-state.json");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let lane_transport = Arc::new(ArtifactLaneWorkerTransport {
            worker_id: "agent-hosted-restored".to_string(),
            manifest_file: manifest_file.display().to_string(),
            output_file: output_file.display().to_string(),
        });
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport.clone(),
            state_file.clone(),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist this hosted task across restart",
                            "repository": "repo-hosted-persisted",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();
        assert!(state_file.exists());

        write_hosted_agent_manifest(&manifest_file, &output_file, "completed", None);
        write_hosted_agent_output(&output_file, Some("Recovered after restart"), None);

        drop(router);
        drop(state);

        let restored_state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport,
            state_file.clone(),
        ));
        let restored_app = super::app(restored_state);

        let get_response = restored_app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "completed");
        assert_eq!(snapshot["worker_status"], "finished");
        assert!(snapshot["result"]
            .as_str()
            .unwrap()
            .contains("Recovered after restart"));

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn state_file_reconciles_hosted_task_during_server_restart() {
        let dir = temp_test_dir("orbit-server-startup-reconcile");
        let state_file = dir.join("server-state.json");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let lane_transport = Arc::new(ArtifactLaneWorkerTransport {
            worker_id: "agent-hosted-startup-reconcile".to_string(),
            manifest_file: manifest_file.display().to_string(),
            output_file: output_file.display().to_string(),
        });
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport.clone(),
            state_file.clone(),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Reconcile hosted task during startup",
                            "repository": "repo-startup-reconcile",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(&manifest_file, &output_file, "completed", None);
        write_hosted_agent_output(&output_file, Some("Recovered during startup"), None);

        drop(router);
        drop(state);

        let restored_state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport,
            state_file.clone(),
        ));

        let restored_task = restored_state
            .tasks
            .get(&task_id)
            .expect("task should still exist after restart");
        assert_eq!(restored_task.status, TaskStatus::Completed);

        let restored_context = restored_state
            .context_for(&task_id)
            .expect("context should still exist after restart");
        assert_eq!(restored_context.worker_status.as_deref(), Some("finished"));

        let green_event = restored_state
            .replay_events()
            .into_iter()
            .find(|event| {
                event.event == HostedEventName::LaneGreen
                    && event.task_id.as_deref() == Some(task_id.as_str())
                    && event
                        .payload
                        .as_ref()
                        .and_then(|payload| payload.get("reconciled"))
                        .and_then(|value| value.as_bool())
                        == Some(true)
            })
            .expect("startup reconcile lane green event should be replayable");
        assert_eq!(green_event.status, HostedEventStatus::Completed);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn state_file_restores_cancelled_hosted_task_after_server_restart() {
        let dir = temp_test_dir("orbit-server-state-cancelled");
        let state_file = dir.join("server-state.json");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let lane_transport = Arc::new(ArtifactLaneWorkerTransport {
            worker_id: "agent-hosted-cancelled".to_string(),
            manifest_file: manifest_file.display().to_string(),
            output_file: output_file.display().to_string(),
        });
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport.clone(),
            state_file.clone(),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Persist cancelled hosted task across restart",
                            "repository": "repo-hosted-cancelled",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        write_hosted_agent_manifest(
            &manifest_file,
            &output_file,
            "cancelled",
            Some("sub-agent cancelled by control plane"),
        );
        write_hosted_agent_output(
            &output_file,
            None,
            Some("sub-agent cancelled by control plane"),
        );

        drop(router);
        drop(state);

        let restored_state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport,
            state_file.clone(),
        ));
        let restored_app = super::app(restored_state.clone());

        let get_response = restored_app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/tasks/{task_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");
        assert_eq!(snapshot["worker_status"], "finished");

        let cancelled_event = restored_state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::TaskCancelled)
            .expect("cancelled event should be emitted");
        assert_eq!(cancelled_event.status, HostedEventStatus::Cancelled);

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn cancel_after_restart_clears_worker_status_for_manifest_backed_hosted_lane() {
        let dir = temp_test_dir("orbit-server-cancel-restart-manifest");
        let state_file = dir.join("server-state.json");
        let manifest_file = dir.join("agent.json");
        let output_file = dir.join("agent.md");
        let lane_transport = Arc::new(ManifestBackedCancellationLaneWorkerTransport {
            worker_id: "agent-hosted-restart-cancel".to_string(),
            manifest_file: manifest_file.display().to_string(),
            output_file: output_file.display().to_string(),
        });
        let state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport.clone(),
            state_file.clone(),
        ));
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Cancel hosted task after restart without live executor",
                            "repository": "repo-hosted-restart-cancel",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();
        assert_eq!(created["worker_status"], "running");

        drop(router);
        drop(state);

        let restored_state = Arc::new(ServerState::with_lane_transport_and_state_file(
            DEFAULT_EVENT_REPLAY_LIMIT,
            lane_transport,
            state_file.clone(),
        ));
        let restored_app = super::app(restored_state.clone());

        let cancel_response = restored_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(cancel_response.status(), StatusCode::OK);
        let cancel_body = to_bytes(cancel_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_slice(&cancel_body).unwrap();
        assert_eq!(snapshot["status"], "cancelled");
        assert!(snapshot["worker_status"].is_null());

        let cancelled_event = restored_state
            .replay_events()
            .into_iter()
            .find(|event| event.event == HostedEventName::TaskCancelled)
            .expect("cancelled event should be emitted");
        assert_eq!(cancelled_event.status, HostedEventStatus::Cancelled);
        assert!(cancelled_event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("detail"))
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("live executor was not attached"));

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn cancel_rejects_completed_task_before_mutating_transport_state() {
        let state = Arc::new(ServerState::default());
        let router = app(state.clone());

        let create_response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "prompt": "Complete before cancel",
                            "repository": "repo-complete",
                            "source": "api"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
        let task_id = created["task_id"].as_str().unwrap().to_string();

        let complete_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/complete"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "finish_reason": "stop",
                            "tokens_output": 32,
                            "result": "done"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(complete_response.status(), StatusCode::OK);

        let cancel_response = super::app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/tasks/{task_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(cancel_response.status(), StatusCode::BAD_REQUEST);
    }
}

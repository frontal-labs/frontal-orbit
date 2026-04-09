use std::fmt::{Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use orbit_telemetry::SessionTracer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentryConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traces_sample_rate: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles_sample_rate: Option<f32>,
}

impl SentryConfig {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            dsn: None,
            environment: None,
            release: None,
            server_name: None,
            traces_sample_rate: None,
            profiles_sample_rate: None,
        }
    }

    #[must_use]
    pub fn enabled(dsn: impl Into<String>) -> Self {
        Self {
            enabled: true,
            dsn: Some(dsn.into()),
            ..Self::disabled()
        }
    }

    #[must_use]
    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self
    }

    #[must_use]
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    #[must_use]
    pub fn with_server_name(mut self, server_name: impl Into<String>) -> Self {
        self.server_name = Some(server_name.into());
        self
    }

    #[must_use]
    pub fn with_traces_sample_rate(mut self, sample_rate: f32) -> Self {
        self.traces_sample_rate = Some(sample_rate);
        self
    }

    #[must_use]
    pub fn with_profiles_sample_rate(mut self, sample_rate: f32) -> Self {
        self.profiles_sample_rate = Some(sample_rate);
        self
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.dsn.is_some()
    }
}

impl Default for SentryConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SentryLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentryEvent {
    pub message: String,
    pub level: SentryLevel,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fingerprint: Vec<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub tags: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
}

impl SentryEvent {
    #[must_use]
    pub fn new(message: impl Into<String>, level: SentryLevel) -> Self {
        Self {
            message: message.into(),
            level,
            timestamp_ms: current_timestamp_ms(),
            fingerprint: Vec::new(),
            tags: Map::new(),
            extra: Map::new(),
            environment: None,
            release: None,
            server_name: None,
        }
    }

    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, SentryLevel::Error)
    }

    #[must_use]
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn with_fingerprint(mut self, value: impl Into<String>) -> Self {
        self.fingerprint.push(value.into());
        self
    }
}

pub trait SentrySink: Send + Sync {
    fn capture(&self, event: SentryEvent);
}

#[derive(Default)]
pub struct NoopSentrySink;

impl SentrySink for NoopSentrySink {
    fn capture(&self, _event: SentryEvent) {}
}

#[derive(Default)]
pub struct MemorySentrySink {
    events: Mutex<Vec<SentryEvent>>,
}

impl MemorySentrySink {
    #[must_use]
    pub fn events(&self) -> Vec<SentryEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl SentrySink for MemorySentrySink {
    fn capture(&self, event: SentryEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

#[derive(Clone)]
pub struct SentryClient {
    config: SentryConfig,
    sink: Arc<dyn SentrySink>,
}

impl Debug for SentryClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl SentryClient {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            config: SentryConfig::disabled(),
            sink: Arc::new(NoopSentrySink),
        }
    }

    #[must_use]
    pub fn new(config: SentryConfig, sink: Arc<dyn SentrySink>) -> Self {
        Self { config, sink }
    }

    #[must_use]
    pub fn config(&self) -> &SentryConfig {
        &self.config
    }

    pub fn capture(&self, mut event: SentryEvent) {
        if !self.config.is_enabled() {
            return;
        }

        if event.environment.is_none() {
            event.environment = self.config.environment.clone();
        }
        if event.release.is_none() {
            event.release = self.config.release.clone();
        }
        if event.server_name.is_none() {
            event.server_name = self.config.server_name.clone();
        }
        self.sink.capture(event);
    }
}

impl Default for SentryClient {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRunContext {
    pub agent_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub tags: Map<String, Value>,
}

impl AgentRunContext {
    #[must_use]
    pub fn new(agent_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            run_id: run_id.into(),
            session_id: None,
            workflow: None,
            provider: None,
            model: None,
            tags: Map::new(),
        }
    }

    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    #[must_use]
    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflow = Some(workflow.into());
        self
    }

    #[must_use]
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentObservationKind {
    RunStarted,
    RunCompleted,
    ErrorCaptured,
    SpanStarted,
    SpanEvent,
    SpanFinished,
}

impl AgentObservationKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RunStarted => "run_started",
            Self::RunCompleted => "run_completed",
            Self::ErrorCaptured => "error_captured",
            Self::SpanStarted => "span_started",
            Self::SpanEvent => "span_event",
            Self::SpanFinished => "span_finished",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSpanKind {
    Workflow,
    Turn,
    Tool,
    Model,
    Custom,
}

impl AgentSpanKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Turn => "turn",
            Self::Tool => "tool",
            Self::Model => "model",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSpanStatus {
    Ok,
    Error,
    Cancelled,
}

impl AgentSpanStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentObservation {
    pub agent_id: String,
    pub run_id: String,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub kind: AgentObservationKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attributes: Map<String, Value>,
}

pub trait AgentObservationSink: Send + Sync {
    fn record(&self, observation: AgentObservation);
}

#[derive(Default)]
pub struct NoopAgentObservationSink;

impl AgentObservationSink for NoopAgentObservationSink {
    fn record(&self, _observation: AgentObservation) {}
}

#[derive(Default)]
pub struct MemoryAgentObservationSink {
    observations: Mutex<Vec<AgentObservation>>,
}

impl MemoryAgentObservationSink {
    #[must_use]
    pub fn observations(&self) -> Vec<AgentObservation> {
        self.observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl AgentObservationSink for MemoryAgentObservationSink {
    fn record(&self, observation: AgentObservation) {
        self.observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(observation);
    }
}

#[derive(Clone)]
pub struct SessionTracerAgentObservationSink {
    tracer: SessionTracer,
}

impl Debug for SessionTracerAgentObservationSink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTracerAgentObservationSink")
            .field("session_id", &self.tracer.session_id())
            .finish_non_exhaustive()
    }
}

impl SessionTracerAgentObservationSink {
    #[must_use]
    pub fn new(tracer: SessionTracer) -> Self {
        Self { tracer }
    }
}

impl AgentObservationSink for SessionTracerAgentObservationSink {
    fn record(&self, observation: AgentObservation) {
        let mut attributes = observation.attributes;
        attributes.insert(
            "agent_id".to_string(),
            Value::String(observation.agent_id.clone()),
        );
        attributes.insert(
            "run_id".to_string(),
            Value::String(observation.run_id.clone()),
        );
        attributes.insert("sequence".to_string(), Value::from(observation.sequence));
        attributes.insert(
            "kind".to_string(),
            Value::String(observation.kind.as_str().to_string()),
        );
        if let Some(span_id) = observation.span_id {
            attributes.insert("span_id".to_string(), Value::String(span_id));
        }
        if let Some(parent_span_id) = observation.parent_span_id {
            attributes.insert("parent_span_id".to_string(), Value::String(parent_span_id));
        }
        self.tracer.record(observation.name, attributes);
    }
}

#[derive(Clone)]
pub struct Observability {
    sentry: SentryClient,
    agent_sink: Arc<dyn AgentObservationSink>,
}

impl Debug for Observability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observability")
            .field("sentry", &self.sentry)
            .finish_non_exhaustive()
    }
}

impl Observability {
    #[must_use]
    pub fn new(sentry: SentryClient, agent_sink: Arc<dyn AgentObservationSink>) -> Self {
        Self { sentry, agent_sink }
    }

    #[must_use]
    pub fn builder() -> ObservabilityBuilder {
        ObservabilityBuilder::default()
    }

    #[must_use]
    pub fn sentry(&self) -> &SentryClient {
        &self.sentry
    }

    #[must_use]
    pub fn start_agent_run(&self, context: AgentRunContext) -> AgentRunObserver {
        AgentRunObserver::new(context, self.agent_sink.clone(), self.sentry.clone())
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self {
            sentry: SentryClient::default(),
            agent_sink: Arc::new(NoopAgentObservationSink),
        }
    }
}

#[derive(Default)]
pub struct ObservabilityBuilder {
    sentry: Option<SentryClient>,
    agent_sink: Option<Arc<dyn AgentObservationSink>>,
}

impl ObservabilityBuilder {
    #[must_use]
    pub fn with_sentry_client(mut self, sentry: SentryClient) -> Self {
        self.sentry = Some(sentry);
        self
    }

    #[must_use]
    pub fn with_agent_sink(mut self, agent_sink: Arc<dyn AgentObservationSink>) -> Self {
        self.agent_sink = Some(agent_sink);
        self
    }

    #[must_use]
    pub fn build(self) -> Observability {
        Observability {
            sentry: self.sentry.unwrap_or_default(),
            agent_sink: self
                .agent_sink
                .unwrap_or_else(|| Arc::new(NoopAgentObservationSink)),
        }
    }
}

#[derive(Clone)]
pub struct AgentRunObserver {
    context: AgentRunContext,
    sequence: Arc<AtomicU64>,
    span_sequence: Arc<AtomicU64>,
    agent_sink: Arc<dyn AgentObservationSink>,
    sentry: SentryClient,
}

impl Debug for AgentRunObserver {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRunObserver")
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

impl AgentRunObserver {
    #[must_use]
    pub fn new(
        context: AgentRunContext,
        agent_sink: Arc<dyn AgentObservationSink>,
        sentry: SentryClient,
    ) -> Self {
        Self {
            context,
            sequence: Arc::new(AtomicU64::new(0)),
            span_sequence: Arc::new(AtomicU64::new(0)),
            agent_sink,
            sentry,
        }
    }

    #[must_use]
    pub fn context(&self) -> &AgentRunContext {
        &self.context
    }

    pub fn record_run_started(&self) {
        let mut attributes = Map::new();
        if let Some(workflow) = &self.context.workflow {
            attributes.insert("workflow".to_string(), Value::String(workflow.clone()));
        }
        if let Some(provider) = &self.context.provider {
            attributes.insert("provider".to_string(), Value::String(provider.clone()));
        }
        if let Some(model) = &self.context.model {
            attributes.insert("model".to_string(), Value::String(model.clone()));
        }
        for (key, value) in &self.context.tags {
            attributes.insert(key.clone(), value.clone());
        }
        self.record_observation(
            AgentObservationKind::RunStarted,
            "agent_run_started",
            None,
            None,
            attributes,
        );
    }

    pub fn record_run_completed(&self, attributes: Map<String, Value>) {
        self.record_observation(
            AgentObservationKind::RunCompleted,
            "agent_run_completed",
            None,
            None,
            attributes,
        );
    }

    #[must_use]
    pub fn start_span(&self, kind: AgentSpanKind, name: impl Into<String>) -> AgentSpan {
        self.start_span_with_parent(kind, name, None, Map::new())
    }

    #[must_use]
    pub fn start_span_with_attributes(
        &self,
        kind: AgentSpanKind,
        name: impl Into<String>,
        attributes: Map<String, Value>,
    ) -> AgentSpan {
        self.start_span_with_parent(kind, name, None, attributes)
    }

    pub fn capture_error(
        &self,
        component: impl Into<String>,
        error: impl Into<String>,
        mut attributes: Map<String, Value>,
    ) {
        let component = component.into();
        let error = error.into();
        attributes.insert("component".to_string(), Value::String(component.clone()));
        attributes.insert("error".to_string(), Value::String(error.clone()));
        self.record_observation(
            AgentObservationKind::ErrorCaptured,
            "agent_error",
            None,
            None,
            attributes.clone(),
        );

        let mut event = SentryEvent::error(error)
            .with_tag("agent_id", self.context.agent_id.clone())
            .with_tag("run_id", self.context.run_id.clone())
            .with_tag("component", component);
        if let Some(session_id) = &self.context.session_id {
            event = event.with_tag("session_id", session_id.clone());
        }
        if let Some(provider) = &self.context.provider {
            event = event.with_tag("provider", provider.clone());
        }
        if let Some(model) = &self.context.model {
            event = event.with_tag("model", model.clone());
        }
        for (key, value) in attributes {
            event = event.with_extra(key, value);
        }
        self.sentry.capture(event);
    }

    fn start_span_with_parent(
        &self,
        kind: AgentSpanKind,
        name: impl Into<String>,
        parent_span_id: Option<String>,
        mut attributes: Map<String, Value>,
    ) -> AgentSpan {
        let span_id = format!(
            "span-{}",
            self.span_sequence.fetch_add(1, Ordering::Relaxed)
        );
        let name = name.into();
        attributes.insert(
            "span_kind".to_string(),
            Value::String(kind.as_str().to_string()),
        );
        self.record_observation(
            AgentObservationKind::SpanStarted,
            name.clone(),
            Some(span_id.clone()),
            parent_span_id.clone(),
            attributes,
        );

        AgentSpan {
            observer: self.clone(),
            span_id,
            parent_span_id,
            kind,
            name,
        }
    }

    fn record_observation(
        &self,
        kind: AgentObservationKind,
        name: impl Into<String>,
        span_id: Option<String>,
        parent_span_id: Option<String>,
        attributes: Map<String, Value>,
    ) {
        let observation = AgentObservation {
            agent_id: self.context.agent_id.clone(),
            run_id: self.context.run_id.clone(),
            session_id: self.context.session_id.clone(),
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            timestamp_ms: current_timestamp_ms(),
            kind,
            name: name.into(),
            span_id,
            parent_span_id,
            attributes,
        };
        self.agent_sink.record(observation);
    }
}

#[derive(Clone)]
pub struct AgentSpan {
    observer: AgentRunObserver,
    span_id: String,
    parent_span_id: Option<String>,
    kind: AgentSpanKind,
    name: String,
}

impl Debug for AgentSpan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentSpan")
            .field("span_id", &self.span_id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl AgentSpan {
    #[must_use]
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn start_child_span(&self, kind: AgentSpanKind, name: impl Into<String>) -> AgentSpan {
        self.observer
            .start_span_with_parent(kind, name, Some(self.span_id.clone()), Map::new())
    }

    pub fn add_event(&self, name: impl Into<String>, mut attributes: Map<String, Value>) {
        attributes.insert(
            "span_kind".to_string(),
            Value::String(self.kind.as_str().to_string()),
        );
        self.observer.record_observation(
            AgentObservationKind::SpanEvent,
            name,
            Some(self.span_id.clone()),
            self.parent_span_id.clone(),
            attributes,
        );
    }

    pub fn finish(&self, status: AgentSpanStatus, mut attributes: Map<String, Value>) {
        attributes.insert(
            "span_kind".to_string(),
            Value::String(self.kind.as_str().to_string()),
        );
        attributes.insert(
            "status".to_string(),
            Value::String(status.as_str().to_string()),
        );
        self.observer.record_observation(
            AgentObservationKind::SpanFinished,
            self.name.clone(),
            Some(self.span_id.clone()),
            self.parent_span_id.clone(),
            attributes,
        );
    }

    pub fn fail(&self, error: impl Into<String>, mut attributes: Map<String, Value>) {
        let error = error.into();
        attributes.insert("span_name".to_string(), Value::String(self.name.clone()));
        attributes.insert("error".to_string(), Value::String(error.clone()));
        self.observer
            .capture_error(self.kind.as_str(), error, attributes.clone());
        self.finish(AgentSpanStatus::Error, attributes);
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_telemetry::{MemoryTelemetrySink, TelemetryEvent};

    #[test]
    fn sentry_client_applies_config_defaults_when_enabled() {
        let sink = Arc::new(MemorySentrySink::default());
        let sentry = SentryClient::new(
            SentryConfig::enabled("https://public@example.invalid/1")
                .with_environment("test")
                .with_release("orbit@1.2.3")
                .with_server_name("devbox"),
            sink.clone(),
        );

        sentry.capture(SentryEvent::error("provider timeout").with_tag("component", "provider"));

        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].environment.as_deref(), Some("test"));
        assert_eq!(events[0].release.as_deref(), Some("orbit@1.2.3"));
        assert_eq!(events[0].server_name.as_deref(), Some("devbox"));
        assert_eq!(
            events[0].tags["component"],
            Value::String("provider".to_string())
        );
    }

    #[test]
    fn disabled_sentry_client_drops_events() {
        let sink = Arc::new(MemorySentrySink::default());
        let sentry = SentryClient::new(SentryConfig::disabled(), sink.clone());

        sentry.capture(SentryEvent::error("should not be captured"));

        assert!(sink.events().is_empty());
    }

    #[test]
    fn agent_run_observer_records_spans_and_forwards_failures_to_sentry() {
        let sentry_sink = Arc::new(MemorySentrySink::default());
        let agent_sink = Arc::new(MemoryAgentObservationSink::default());
        let observability = Observability::builder()
            .with_sentry_client(SentryClient::new(
                SentryConfig::enabled("https://public@example.invalid/1"),
                sentry_sink.clone(),
            ))
            .with_agent_sink(agent_sink.clone())
            .build();

        let run = observability.start_agent_run(
            AgentRunContext::new("planner", "run-42")
                .with_session_id("session-99")
                .with_workflow("interactive_turn")
                .with_provider("anthropic")
                .with_model("claude-sonnet"),
        );

        run.record_run_started();
        let turn = run.start_span(AgentSpanKind::Turn, "turn");
        let tool = turn.start_child_span(AgentSpanKind::Tool, "bash");
        tool.add_event("tool_input_ready", Map::new());
        tool.fail("permission denied", Map::new());
        run.record_run_completed(Map::new());

        let observations = agent_sink.observations();
        assert!(matches!(
            &observations[0],
            AgentObservation {
                kind: AgentObservationKind::RunStarted,
                name,
                ..
            } if name == "agent_run_started"
        ));
        assert!(observations.iter().any(|event| {
            event.kind == AgentObservationKind::SpanStarted && event.name == "turn"
        }));
        assert!(observations.iter().any(|event| {
            event.kind == AgentObservationKind::SpanEvent && event.name == "tool_input_ready"
        }));
        assert!(observations.iter().any(|event| {
            event.kind == AgentObservationKind::ErrorCaptured && event.name == "agent_error"
        }));
        assert!(observations.iter().any(|event| {
            event.kind == AgentObservationKind::SpanFinished
                && event.attributes["status"] == Value::String("error".to_string())
        }));

        let sentry_events = sentry_sink.events();
        assert_eq!(sentry_events.len(), 1);
        assert_eq!(
            sentry_events[0].tags["agent_id"],
            Value::String("planner".to_string())
        );
        assert_eq!(
            sentry_events[0].tags["run_id"],
            Value::String("run-42".to_string())
        );
    }

    #[test]
    fn session_tracer_adapter_forwards_agent_observations() {
        let telemetry_sink = Arc::new(MemoryTelemetrySink::default());
        let tracer = SessionTracer::new("session-telemetry", telemetry_sink.clone());
        let agent_sink = Arc::new(SessionTracerAgentObservationSink::new(tracer));
        let observability = Observability::builder().with_agent_sink(agent_sink).build();

        let run = observability.start_agent_run(
            AgentRunContext::new("writer", "run-7").with_session_id("session-telemetry"),
        );
        run.record_run_started();
        run.record_run_completed(Map::new());

        let events = telemetry_sink.events();
        assert!(matches!(
            &events[0],
            TelemetryEvent::SessionTrace(trace) if trace.name == "agent_run_started"
        ));
        assert!(matches!(
            &events[1],
            TelemetryEvent::SessionTrace(trace) if trace.name == "agent_run_completed"
        ));
    }
}

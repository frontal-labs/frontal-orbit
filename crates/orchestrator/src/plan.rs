use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub work_item_id: String,
    pub prompt: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub source: WorkItemSource,
    pub priority: WorkItemPriority,
    pub context: WorkItemContext,
}

impl WorkItem {
    #[must_use]
    pub fn new(
        prompt: impl Into<String>,
        source: WorkItemSource,
        repository: Option<String>,
        branch: Option<String>,
        priority: WorkItemPriority,
        context: WorkItemContext,
    ) -> Self {
        Self {
            work_item_id: Uuid::new_v4().to_string(),
            prompt: prompt.into(),
            repository,
            branch,
            source,
            priority,
            context,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkItemContext {
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WorkItemPriority {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkItemSource {
    Slack,
    Linear,
    Github,
    Cron,
    Webhook,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub work_item: WorkItem,
    pub lanes: Vec<LaneAssignment>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAssignment {
    pub lane_id: String,
    pub role: LaneRole,
    pub description: String,
    pub priority: WorkItemPriority,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LaneRole {
    Triager,
    Planner,
    Implementer,
    Reviewer,
    Verifier,
    Release,
    Maintenance,
}

#[must_use]
pub fn plan_work_item(work_item: WorkItem) -> ExecutionPlan {
    let primary_role = match work_item.source {
        WorkItemSource::Cron => LaneRole::Maintenance,
        WorkItemSource::Github | WorkItemSource::Webhook => {
            if work_item.prompt.to_lowercase().contains("deploy") {
                LaneRole::Release
            } else {
                LaneRole::Implementer
            }
        }
        WorkItemSource::Linear => LaneRole::Planner,
        WorkItemSource::Slack => {
            if work_item.prompt.to_lowercase().contains("review") {
                LaneRole::Reviewer
            } else {
                LaneRole::Implementer
            }
        }
        WorkItemSource::Unknown => LaneRole::Triager,
    };

    let mut lanes = vec![LaneAssignment {
        lane_id: Uuid::new_v4().to_string(),
        role: primary_role,
        description: format!(
            "{} lane for {}",
            role_description(primary_role),
            work_item.prompt
        ),
        priority: work_item.priority,
    }];

    if work_item.priority == WorkItemPriority::High
        && !lanes.iter().any(|lane| lane.role == LaneRole::Verifier)
    {
        lanes.push(LaneAssignment {
            lane_id: Uuid::new_v4().to_string(),
            role: LaneRole::Verifier,
            description: "Verifier ensures high-priority changes stay safe".to_string(),
            priority: WorkItemPriority::High,
        });
    }

    ExecutionPlan {
        plan_id: Uuid::new_v4().to_string(),
        work_item,
        lanes,
        created_at: Utc::now(),
    }
}

fn role_description(role: LaneRole) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_prompt_assigns_implementer() {
        let work_item = WorkItem::new(
            "Fix UI bug",
            WorkItemSource::Slack,
            Some("ui".into()),
            None,
            WorkItemPriority::Medium,
            WorkItemContext::default(),
        );
        let plan = plan_work_item(work_item.clone());
        assert_eq!(plan.work_item.work_item_id, work_item.work_item_id);
        assert_eq!(plan.lanes[0].role, LaneRole::Implementer);
    }

    #[test]
    fn cron_prompt_creates_maintenance_lane() {
        let work_item = WorkItem::new(
            "kernel upkeep",
            WorkItemSource::Cron,
            None,
            None,
            WorkItemPriority::Low,
            WorkItemContext::default(),
        );
        let plan = plan_work_item(work_item);
        assert_eq!(plan.lanes.len(), 1);
        assert_eq!(plan.lanes[0].role, LaneRole::Maintenance);
    }

    #[test]
    fn high_priority_adds_verifier() {
        let work_item = WorkItem::new(
            "Deploy release",
            WorkItemSource::Github,
            Some("core".into()),
            Some("main".into()),
            WorkItemPriority::High,
            WorkItemContext::default(),
        );
        let plan = plan_work_item(work_item);
        assert!(plan.lanes.iter().any(|lane| lane.role == LaneRole::Release));
        assert!(plan
            .lanes
            .iter()
            .any(|lane| lane.role == LaneRole::Verifier));
    }

    #[test]
    fn prompt_review_route() {
        let work_item = WorkItem::new(
            "Please review the config",
            WorkItemSource::Slack,
            None,
            None,
            WorkItemPriority::Low,
            WorkItemContext::default(),
        );
        let plan = plan_work_item(work_item);
        assert_eq!(plan.lanes[0].role, LaneRole::Reviewer);
    }
}

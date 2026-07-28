use orbit_orchestrator::{
    plan_work_item, LaneRole, WorkItem, WorkItemContext, WorkItemPriority, WorkItemSource,
};

#[test]
fn creates_work_item_with_uuid() {
    let item = WorkItem::new(
        "Fix bug",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    assert!(!item.work_item_id.is_empty());
    assert_eq!(item.prompt, "Fix bug");
    assert_eq!(item.source, WorkItemSource::Slack);
    assert_eq!(item.priority, WorkItemPriority::Medium);
}

#[test]
fn work_item_stores_repository_and_branch() {
    let item = WorkItem::new(
        "Deploy",
        WorkItemSource::Github,
        Some("acme/web".to_string()),
        Some("main".to_string()),
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    assert_eq!(item.repository.as_deref(), Some("acme/web"));
    assert_eq!(item.branch.as_deref(), Some("main"));
}

#[test]
fn slack_source_routes_to_implementer() {
    let item = WorkItem::new(
        "Fix UI bug",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Implementer);
}

#[test]
fn slack_source_with_review_keyword_routes_to_reviewer() {
    let item = WorkItem::new(
        "Please review the config changes",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Reviewer);
}

#[test]
fn github_source_routes_to_implementer() {
    let item = WorkItem::new(
        "Add new feature",
        WorkItemSource::Github,
        Some("repo".to_string()),
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Implementer);
}

#[test]
fn github_source_with_deploy_keyword_routes_to_release() {
    let item = WorkItem::new(
        "Deploy release v2.0 to production",
        WorkItemSource::Github,
        Some("repo".to_string()),
        Some("main".to_string()),
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Release);
}

#[test]
fn webhook_source_with_deploy_keyword_routes_to_release() {
    let item = WorkItem::new(
        "Deploy to staging",
        WorkItemSource::Webhook,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Release);
}

#[test]
fn webhook_source_without_deploy_routes_to_implementer() {
    let item = WorkItem::new(
        "Update configuration",
        WorkItemSource::Webhook,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Implementer);
}

#[test]
fn linear_source_routes_to_planner() {
    let item = WorkItem::new(
        "Implement new API endpoint",
        WorkItemSource::Linear,
        None,
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Planner);
}

#[test]
fn cron_source_routes_to_maintenance() {
    let item = WorkItem::new(
        "Weekly dependency updates",
        WorkItemSource::Cron,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Maintenance);
}

#[test]
fn unknown_source_routes_to_triager() {
    let item = WorkItem::new(
        "Investigate issue",
        WorkItemSource::Unknown,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes[0].role, LaneRole::Triager);
}

#[test]
fn low_priority_has_single_lane() {
    let item = WorkItem::new(
        "Minor cleanup",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 1);
}

#[test]
fn medium_priority_has_single_lane() {
    let item = WorkItem::new(
        "Feature work",
        WorkItemSource::Github,
        None,
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 1);
}

#[test]
fn work_item_priority_default() {
    assert_eq!(WorkItemPriority::default(), WorkItemPriority::Medium);
}

#[test]
fn work_item_context_default() {
    let context = WorkItemContext::default();
    assert!(context.metadata.is_empty());
}

#[test]
fn work_item_context_with_metadata() {
    let context = WorkItemContext {
        metadata: std::collections::HashMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]),
    };
    assert_eq!(context.metadata.len(), 2);
    assert_eq!(context.metadata.get("key1").unwrap(), "value1");
}

#[test]
fn plan_preserves_original_work_item() {
    let item = WorkItem::new(
        "Refactor auth module",
        WorkItemSource::Github,
        Some("acme/app".to_string()),
        Some("develop".to_string()),
        WorkItemPriority::High,
        WorkItemContext {
            metadata: std::collections::HashMap::from([("author".to_string(), "bot".to_string())]),
        },
    );
    let plan = plan_work_item(item.clone());
    assert_eq!(plan.work_item.work_item_id, item.work_item_id);
    assert_eq!(plan.work_item.prompt, "Refactor auth module");
    assert_eq!(plan.work_item.repository, Some("acme/app".to_string()));
    assert_eq!(
        plan.work_item.context.metadata.get("author").unwrap(),
        "bot"
    );
}

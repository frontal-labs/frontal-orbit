use orbit_orchestrator::{
    plan_work_item, ExecutionPlan, LaneAssignment, LaneRole, WorkItem, WorkItemContext,
    WorkItemPriority, WorkItemSource,
};

#[test]
fn plan_has_a_unique_plan_id() {
    let item = WorkItem::new(
        "Test plan",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::Low,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert!(!plan.plan_id.is_empty());
}

#[test]
fn plan_has_a_creation_timestamp() {
    let item = WorkItem::new(
        "Timestamp test",
        WorkItemSource::Github,
        None,
        None,
        WorkItemPriority::Medium,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    let json = serde_json::to_value(&plan).unwrap();
    let created = json["created_at"].as_str().unwrap();
    assert!(
        created.contains('T'),
        "expected ISO timestamp, got {created}"
    );
    assert!(created.ends_with('Z'), "expected UTC timestamp");
}

#[test]
fn high_priority_adds_verifier_lane() {
    let item = WorkItem::new(
        "Critical fix",
        WorkItemSource::Github,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 2);
    assert!(plan
        .lanes
        .iter()
        .any(|lane| lane.role == LaneRole::Verifier));
}

#[test]
fn high_priority_from_slack_adds_verifier() {
    let item = WorkItem::new(
        "Review and deploy hotfix",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 2);
    let roles: Vec<LaneRole> = plan.lanes.iter().map(|l| l.role).collect();
    assert!(roles.contains(&LaneRole::Verifier));
}

#[test]
fn high_priority_from_cron_adds_verifier() {
    let item = WorkItem::new(
        "Emergency maintenance",
        WorkItemSource::Cron,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 2);
    let roles: Vec<LaneRole> = plan.lanes.iter().map(|l| l.role).collect();
    assert!(roles.contains(&LaneRole::Verifier));
    assert!(roles.contains(&LaneRole::Maintenance));
}

#[test]
fn lane_assignments_have_unique_ids() {
    let item = WorkItem::new(
        "Multi-lane test",
        WorkItemSource::Github,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert!(!plan.lanes.is_empty());
    for lane in &plan.lanes {
        assert!(!lane.lane_id.is_empty());
    }
    if plan.lanes.len() > 1 {
        assert_ne!(plan.lanes[0].lane_id, plan.lanes[1].lane_id);
    }
}

#[test]
fn lane_assignments_have_descriptions() {
    let item = WorkItem::new(
        "Refactor database layer",
        WorkItemSource::Linear,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    for lane in &plan.lanes {
        assert!(!lane.description.is_empty());
    }
}

#[test]
fn lane_assignments_carry_priority() {
    let item = WorkItem::new(
        "High priority item",
        WorkItemSource::Slack,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    for lane in &plan.lanes {
        assert_eq!(lane.priority, WorkItemPriority::High);
    }
}

#[test]
fn execution_plan_serde_roundtrip() {
    let item = WorkItem::new(
        "Serialize test",
        WorkItemSource::Github,
        Some("repo".to_string()),
        Some("branch".to_string()),
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["work_item"]["prompt"], "Serialize test");
    assert_eq!(json["lanes"].as_array().unwrap().len(), 2);

    let deserialized: ExecutionPlan = serde_json::from_value(json).unwrap();
    assert_eq!(plan.plan_id, deserialized.plan_id);
    assert_eq!(plan.lanes.len(), deserialized.lanes.len());
}

#[test]
fn lane_assignment_serde_roundtrip() {
    let lane = LaneAssignment {
        lane_id: "lane-1".to_string(),
        role: LaneRole::Implementer,
        description: "Implementation lane".to_string(),
        priority: WorkItemPriority::High,
    };
    let json = serde_json::to_value(&lane).unwrap();
    assert_eq!(json["lane_id"], "lane-1");
    assert_eq!(json["description"], "Implementation lane");

    let deserialized: LaneAssignment = serde_json::from_value(json).unwrap();
    assert_eq!(lane.lane_id, deserialized.lane_id);
    assert_eq!(lane.role, deserialized.role);
    assert_eq!(lane.description, deserialized.description);
    assert_eq!(lane.priority, deserialized.priority);
}

#[test]
fn all_lane_roles_serialize_to_snake_case() {
    for (role, expected) in [
        (LaneRole::Triager, "Triager"),
        (LaneRole::Planner, "Planner"),
        (LaneRole::Implementer, "Implementer"),
        (LaneRole::Reviewer, "Reviewer"),
        (LaneRole::Verifier, "Verifier"),
        (LaneRole::Release, "Release"),
        (LaneRole::Maintenance, "Maintenance"),
    ] {
        assert_eq!(serde_json::to_value(role).unwrap(), expected);
    }
}

#[test]
fn high_priority_github_deploy_has_verifier_and_release() {
    let item = WorkItem::new(
        "Deploy release v3.0",
        WorkItemSource::Github,
        None,
        None,
        WorkItemPriority::High,
        WorkItemContext::default(),
    );
    let plan = plan_work_item(item);
    assert_eq!(plan.lanes.len(), 2);
    let roles: Vec<LaneRole> = plan.lanes.iter().map(|l| l.role).collect();
    assert!(roles.contains(&LaneRole::Release));
    assert!(roles.contains(&LaneRole::Verifier));
}

#[test]
fn plan_generates_unique_ids_on_each_call() {
    let make_item = || {
        WorkItem::new(
            "Unique test",
            WorkItemSource::Unknown,
            None,
            None,
            WorkItemPriority::Low,
            WorkItemContext::default(),
        )
    };
    let plan1 = plan_work_item(make_item());
    let plan2 = plan_work_item(make_item());
    assert_ne!(plan1.plan_id, plan2.plan_id);
}

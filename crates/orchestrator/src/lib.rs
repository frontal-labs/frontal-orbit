//! # Orbit Orchestrator
//! Provides routing, execution plan creation, and lane assignment for hosted work items.

mod plan;

pub use plan::{
    plan_work_item, ExecutionPlan, LaneAssignment, LaneRole, WorkItem, WorkItemContext,
    WorkItemPriority, WorkItemSource,
};

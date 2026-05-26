use crate::graphite_client::{GraphiteClient, GraphiteStackCommentRequest};
use crate::linear_client::{LinearClient, LinearIssueCommentRequest};
use crate::HostedTaskContext;

#[derive(Debug)]
pub enum TrackerEvent<'a> {
    Completed { result: Option<&'a str> },
    Failed { error: Option<&'a str> },
    ApprovalRequested { reason: Option<&'a str> },
}

pub fn maybe_spawn_tracker_report(
    task_id: String,
    context: HostedTaskContext,
    event: TrackerEvent<'_>,
) {
    let graphite_client = GraphiteClient::from_env();
    let linear_issue_id = context.linear_issue_id.clone();
    let graphite_stack_id = context.graphite_stack_id.clone();
    let message = render_message(&task_id, &event);

    if linear_issue_id.is_some() {
        let body = message.clone();
        tokio::spawn(async move {
            let client = LinearClient::from_oauth_or_env().await;
            if let (Some(client), Some(issue_id)) = (client, linear_issue_id) {
                let req = LinearIssueCommentRequest { issue_id, body };
                if let Err(err) = client.create_issue_comment(req).await {
                    eprintln!("linear status post failed: {err}");
                }
            }
        });
    }

    if let (Some(client), Some(stack_id)) = (graphite_client, graphite_stack_id) {
        let body = message.clone();
        tokio::spawn(async move {
            let req = GraphiteStackCommentRequest { stack_id, body };
            if let Err(err) = client.create_stack_comment(req).await {
                eprintln!("graphite status post failed: {err}");
            }
        });
    }
}

fn render_message(task_id: &str, event: &TrackerEvent<'_>) -> String {
    match event {
        TrackerEvent::Completed { result } => match result {
            Some(result) if !result.is_empty() => {
                format!("Orbit task {task_id} completed successfully.\n\nResult:\n{result}")
            }
            _ => format!("Orbit task {task_id} completed successfully."),
        },
        TrackerEvent::Failed { error } => match error {
            Some(error) if !error.is_empty() => {
                format!("Orbit task {task_id} failed.\n\nError:\n{error}")
            }
            _ => format!("Orbit task {task_id} failed."),
        },
        TrackerEvent::ApprovalRequested { reason } => match reason {
            Some(reason) if !reason.is_empty() => {
                format!("Orbit task {task_id} requires approval/follow-up.\n\nReason:\n{reason}")
            }
            _ => format!("Orbit task {task_id} requires approval/follow-up."),
        },
    }
}

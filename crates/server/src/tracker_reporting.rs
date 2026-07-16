use crate::HostedTaskContext;
use orbit_integrations::mcp::integration::global_integration_registry;

#[derive(Debug)]
pub enum TrackerEvent<'a> {
    Completed { result: Option<&'a str> },
    Failed { error: Option<&'a str> },
}

#[allow(clippy::needless_pass_by_value)]
pub fn maybe_spawn_tracker_report(
    task_id: String,
    context: HostedTaskContext,
    event: TrackerEvent<'_>,
) {
    let linear_issue_id = context.linear_issue_id.clone();
    let graphite_stack_id = context.graphite_stack_id.clone();
    let message = render_message(&task_id, &event);

    if let Some(issue_id) = linear_issue_id {
        let body = message.clone();
        tokio::spawn(async move {
            let result =
                global_integration_registry().call_linear_create_issue_comment(&issue_id, &body);
            if let Err(err) = result {
                eprintln!("linear status post failed: {err}");
            }
        });
    }

    if let Some(stack_id) = graphite_stack_id {
        let body = message.clone();
        tokio::spawn(async move {
            let result =
                global_integration_registry().call_graphite_create_stack_comment(&stack_id, &body);
            if let Err(err) = result {
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
    }
}

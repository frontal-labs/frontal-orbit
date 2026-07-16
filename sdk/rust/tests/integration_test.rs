mod common;

use std::path::PathBuf;

use orbit_sdk::{
    InputEntry, Orbit, OrbitError, OrbitEvent, OrbitOptions, ThreadInput, ThreadOptions,
    ThreadRunOptions,
};

use common::MockOrbit;

const SINGLE_TURN: &str = concat!(
    "{\"type\":\"turn.started\"}\n",
    "{\"type\":\"item.completed\",\"item\":{\"type\":\"text\",\"text\":\"thinking...\"}}\n",
    "{\"type\":\"item.completed\",\"item\":{\"type\":\"tool_use\",\"name\":\"edit\",\"input\":\"{}\"}}\n",
    "{\"type\":\"turn.completed\",\"finalResponse\":\"done\",\"usage\":{\"input_tokens\":3,\"output_tokens\":5,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":1},\"sessionId\":\"sess-1\"}",
);

fn orbit_for(mock: &MockOrbit) -> Orbit {
    Orbit::new(OrbitOptions {
        command: Some(mock.bin_path.to_string_lossy().into_owned()),
        env: Some(mock.env()),
        ..Default::default()
    })
}

#[tokio::test]
async fn run_returns_buffered_turn() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let turn = orbit
        .start_thread(ThreadOptions::default())
        .run(
            &ThreadInput::Text("diagnose the failure".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("run");

    assert_eq!(turn.final_response, "done");
    assert_eq!(
        turn.usage,
        Some(orbit_sdk::Usage {
            input_tokens: 3,
            output_tokens: 5,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 1,
        })
    );
    assert_eq!(
        turn.items,
        vec![
            orbit_sdk::ThreadItem::Text {
                text: "thinking...".into()
            },
            orbit_sdk::ThreadItem::ToolUse {
                name: "edit".into(),
                input: "{}".into()
            },
        ]
    );
}

#[tokio::test]
async fn run_stores_session_id() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let thread = orbit.start_thread(ThreadOptions::default());
    thread
        .run(
            &ThreadInput::Text("first".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("run");
    assert_eq!(thread.id().await.as_deref(), Some("sess-1"));
}

#[tokio::test]
async fn run_requests_json_streaming_output() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    orbit
        .start_thread(ThreadOptions::default())
        .run(
            &ThreadInput::Text("my prompt".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("run");
    let args = mock.captured_args();
    assert!(args.contains(&"prompt".to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"my prompt".to_string()));
    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"json".to_string()));
    assert!(args.contains(&"--stream".to_string()));
}

#[tokio::test]
async fn first_turn_has_no_resume() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    orbit
        .start_thread(ThreadOptions::default())
        .run(
            &ThreadInput::Text("first".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("run");
    assert!(!mock.captured_args().contains(&"--resume".to_string()));
}

#[tokio::test]
async fn subsequent_turns_pass_resume() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let thread = orbit.start_thread(ThreadOptions::default());
    thread
        .run(
            &ThreadInput::Text("first".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("first");
    thread
        .run(
            &ThreadInput::Text("second".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("second");
    let args = mock.captured_args();
    let idx = args
        .iter()
        .position(|a| a == "--resume")
        .expect("resume flag");
    assert_eq!(args[idx + 1], "sess-1");
}

#[tokio::test]
async fn images_emit_image_flags() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let entries = vec![
        InputEntry::Text {
            text: "describe these".into(),
        },
        InputEntry::LocalImage {
            path: "./ui.png".into(),
        },
        InputEntry::LocalImage {
            path: "./diagram.jpg".into(),
        },
    ];
    orbit
        .start_thread(ThreadOptions::default())
        .run(&ThreadInput::Entries(entries), &ThreadRunOptions::default())
        .await
        .expect("run");
    let args = mock.captured_args();
    let image_indexes: Vec<usize> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| *a == "--image")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(image_indexes.len(), 2);
    assert_eq!(args[image_indexes[0] + 1], "./ui.png");
    assert_eq!(args[image_indexes[1] + 1], "./diagram.jpg");
    assert!(args.contains(&"describe these".to_string()));
}

#[tokio::test]
async fn multiple_text_entries_concatenated() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let entries = vec![
        InputEntry::Text {
            text: "line one".into(),
        },
        InputEntry::Text {
            text: "line two".into(),
        },
    ];
    orbit
        .start_thread(ThreadOptions::default())
        .run(&ThreadInput::Entries(entries), &ThreadRunOptions::default())
        .await
        .expect("run");
    let args = mock.captured_args();
    assert!(args.iter().any(|a| a == "line one\nline two"));
}

#[tokio::test]
async fn provider_model_permission_mode_passed() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    orbit
        .start_thread(ThreadOptions {
            provider: Some("frontal".into()),
            model: Some("opus".into()),
            permission_mode: Some("safe-mode".into()),
            ..Default::default()
        })
        .run(&ThreadInput::Text("x".into()), &ThreadRunOptions::default())
        .await
        .expect("run");
    let args = mock.captured_args();
    assert_eq!(
        args[args.iter().position(|a| a == "--provider").unwrap() + 1],
        "frontal"
    );
    assert_eq!(
        args[args.iter().position(|a| a == "--model").unwrap() + 1],
        "opus"
    );
    assert_eq!(
        args[args.iter().position(|a| a == "--permission-mode").unwrap() + 1],
        "safe-mode"
    );
}

#[tokio::test]
async fn output_schema_passed_as_config() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "summary": { "type": "string" } },
        "required": ["summary"],
    });
    orbit
        .start_thread(ThreadOptions::default())
        .run(
            &ThreadInput::Text("summarize".into()),
            &ThreadRunOptions {
                output_schema: Some(schema),
                ..Default::default()
            },
        )
        .await
        .expect("run");
    let args = mock.captured_args();
    let idx = args.iter().position(|a| a == "--config").unwrap();
    assert!(args[idx + 1].contains("output_schema="));
}

#[tokio::test]
async fn runs_in_configured_working_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let wd = dir.path().to_string_lossy().into_owned();
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    orbit
        .start_thread(ThreadOptions {
            working_directory: Some(PathBuf::from(&wd)),
            ..Default::default()
        })
        .run(&ThreadInput::Text("x".into()), &ThreadRunOptions::default())
        .await
        .expect("run");
    assert_eq!(mock.captured_cwd(), wd);
}

#[tokio::test]
async fn skip_git_repo_check_passed_as_config() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    orbit
        .start_thread(ThreadOptions {
            skip_git_repo_check: true,
            ..Default::default()
        })
        .run(&ThreadInput::Text("x".into()), &ThreadRunOptions::default())
        .await
        .expect("run");
    let args = mock.captured_args();
    let overrides: Vec<&String> = args
        .iter()
        .filter(|a| a.starts_with("skip_git_repo_check"))
        .collect();
    assert!(!overrides.is_empty());
    assert!(overrides[0].contains("true"));
}

#[tokio::test]
async fn global_config_and_base_url_flattened() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = Orbit::new(OrbitOptions {
        command: Some(mock.bin_path.to_string_lossy().into_owned()),
        env: Some(mock.env()),
        base_url: Some("https://frontal.example".into()),
        config: Some(serde_json::json!({ "show_raw_agent_reasoning": true })),
    });
    orbit
        .start_thread(ThreadOptions::default())
        .run(&ThreadInput::Text("x".into()), &ThreadRunOptions::default())
        .await
        .expect("run");
    let args = mock.captured_args();
    assert!(args.contains(&"--config".to_string()));
    assert!(args.iter().any(|a| a == "show_raw_agent_reasoning=true"));
    let idx = args.iter().position(|a| a == "--config").unwrap();
    assert!(args[idx + 1].contains("frontal_base_url="));
}

#[tokio::test]
async fn non_zero_exit_is_an_error() {
    let mock = MockOrbit::new("", 2);
    let orbit = orbit_for(&mock);
    let result = orbit
        .start_thread(ThreadOptions::default())
        .run(
            &ThreadInput::Text("boom".into()),
            &ThreadRunOptions::default(),
        )
        .await;
    assert!(matches!(
        result,
        Err(OrbitError::Exit { code: Some(2), .. })
    ));
}

#[tokio::test]
async fn unparseable_lines_ignored() {
    let response = format!("{}\n{}", "not json", SINGLE_TURN.lines().last().unwrap());
    let mock = MockOrbit::new(&response, 0);
    let orbit = orbit_for(&mock);
    let turn = orbit
        .start_thread(ThreadOptions::default())
        .run(&ThreadInput::Text("x".into()), &ThreadRunOptions::default())
        .await
        .expect("run");
    assert_eq!(turn.final_response, "done");
}

#[tokio::test]
async fn run_streamed_yields_ordered_events() {
    let mock = MockOrbit::new(SINGLE_TURN, 0);
    let orbit = orbit_for(&mock);
    let thread = orbit.start_thread(ThreadOptions::default());
    let mut streamed = thread
        .run_streamed(
            &ThreadInput::Text("diagnose".into()),
            &ThreadRunOptions::default(),
        )
        .await
        .expect("streamed");

    let mut types = Vec::new();
    while let Some(event) = streamed.events.recv().await {
        let name = match event {
            OrbitEvent::TurnStarted => "turn.started",
            OrbitEvent::ItemCompleted { .. } => "item.completed",
            OrbitEvent::TurnCompleted { .. } => "turn.completed",
            OrbitEvent::TurnFailed { .. } => "turn.failed",
        };
        types.push(name.to_string());
    }
    streamed.finish().await.expect("finish");

    assert_eq!(
        types,
        vec![
            "turn.started".to_string(),
            "item.completed".to_string(),
            "item.completed".to_string(),
            "turn.completed".to_string(),
        ]
    );
    assert_eq!(thread.id().await.as_deref(), Some("sess-1"));
}

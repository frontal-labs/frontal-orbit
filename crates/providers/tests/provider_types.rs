use orbit_providers::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageDelta, MessageDeltaEvent, MessageRequest,
    MessageResponse, MessageStartEvent, MessageStopEvent, OutputContentBlock, StreamEvent,
    ToolChoice, ToolDefinition, ToolResultContentBlock, Usage,
};

#[test]
fn message_request_can_be_streaming() {
    let req = MessageRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 4096,
        messages: vec![],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
    };
    let streaming = req.with_streaming();
    assert!(streaming.stream);
}

#[test]
fn message_request_default_not_streaming() {
    let req = MessageRequest {
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        messages: vec![],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
    };
    assert!(!req.stream);
}

#[test]
fn input_message_user_text() {
    let msg = InputMessage::user_text("hello world");
    assert_eq!(msg.role, "user");
    assert!(matches!(&msg.content[0], InputContentBlock::Text { text } if text == "hello world"));
}

#[test]
fn input_message_user_tool_result() {
    let msg = InputMessage::user_tool_result("tool-1", "result data", false);
    assert_eq!(msg.role, "user");
    assert!(
        matches!(&msg.content[0], InputContentBlock::ToolResult { tool_use_id, .. } if tool_use_id == "tool-1")
    );
}

#[test]
fn input_message_user_tool_result_error() {
    let msg = InputMessage::user_tool_result("tool-2", "error message", true);
    assert!(matches!(
        &msg.content[0],
        InputContentBlock::ToolResult { is_error: true, .. }
    ));
}

#[test]
fn tool_definition_construction() {
    let def = ToolDefinition {
        name: "test_tool".to_string(),
        description: Some("A test tool".to_string()),
        input_schema: serde_json::json!({"type": "object", "properties": {}}),
    };
    assert_eq!(def.name, "test_tool");
    assert_eq!(def.description.as_deref(), Some("A test tool"));
}

#[test]
fn tool_choice_variants() {
    let auto = ToolChoice::Auto;
    assert!(matches!(auto, ToolChoice::Auto));

    let any = ToolChoice::Any;
    assert!(matches!(any, ToolChoice::Any));

    let tool = ToolChoice::Tool {
        name: "test".to_string(),
    };
    assert!(matches!(tool, ToolChoice::Tool { name } if name == "test"));
}

#[test]
fn usage_total_tokens() {
    let usage = Usage {
        input_tokens: 100,
        cache_creation_input_tokens: 10,
        cache_read_input_tokens: 20,
        output_tokens: 50,
    };
    assert_eq!(usage.total_tokens(), 180);
}

#[test]
fn usage_zero_tokens() {
    let usage = Usage {
        input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        output_tokens: 0,
    };
    assert_eq!(usage.total_tokens(), 0);
}

#[test]
fn output_content_block_text() {
    let block = OutputContentBlock::Text {
        text: "hello".to_string(),
    };
    assert!(matches!(block, OutputContentBlock::Text { .. }));
}

#[test]
fn output_content_block_tool_use() {
    let block = OutputContentBlock::ToolUse {
        id: "tu_1".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({"cmd": "ls"}),
    };
    assert!(matches!(block, OutputContentBlock::ToolUse { name, .. } if name == "bash"));
}

#[test]
fn output_content_block_thinking() {
    let block = OutputContentBlock::Thinking {
        thinking: "thinking text".to_string(),
        signature: Some("sig".to_string()),
    };
    assert!(matches!(block, OutputContentBlock::Thinking { .. }));
}

#[test]
fn output_content_block_redacted_thinking() {
    let block = OutputContentBlock::RedactedThinking {
        data: serde_json::json!({"type": "redacted"}),
    };
    assert!(matches!(block, OutputContentBlock::RedactedThinking { .. }));
}

#[test]
fn content_block_delta_variants() {
    let text_delta = ContentBlockDelta::TextDelta {
        text: "hello".to_string(),
    };
    assert!(matches!(text_delta, ContentBlockDelta::TextDelta { .. }));

    let json_delta = ContentBlockDelta::InputJsonDelta {
        partial_json: "{}".to_string(),
    };
    assert!(matches!(
        json_delta,
        ContentBlockDelta::InputJsonDelta { .. }
    ));

    let thinking_delta = ContentBlockDelta::ThinkingDelta {
        thinking: "...".to_string(),
    };
    assert!(matches!(
        thinking_delta,
        ContentBlockDelta::ThinkingDelta { .. }
    ));

    let sig_delta = ContentBlockDelta::SignatureDelta {
        signature: "abc".to_string(),
    };
    assert!(matches!(
        sig_delta,
        ContentBlockDelta::SignatureDelta { .. }
    ));
}

#[test]
fn stream_event_variants() {
    let msg = MessageResponse {
        id: "msg_1".to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![],
        model: "claude-3".to_string(),
        stop_reason: None,
        stop_sequence: None,
        usage: Usage {
            input_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 0,
        },
        request_id: None,
    };

    let start = StreamEvent::MessageStart(MessageStartEvent {
        message: msg.clone(),
    });
    assert!(matches!(start, StreamEvent::MessageStart(_)));

    let delta = StreamEvent::MessageDelta(MessageDeltaEvent {
        delta: MessageDelta {
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
        },
        usage: Usage {
            input_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 10,
        },
    });
    assert!(matches!(delta, StreamEvent::MessageDelta(_)));

    let cb_start = StreamEvent::ContentBlockStart(ContentBlockStartEvent {
        index: 0,
        content_block: OutputContentBlock::Text {
            text: String::new(),
        },
    });
    assert!(matches!(cb_start, StreamEvent::ContentBlockStart(_)));

    let cb_delta = StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
        index: 0,
        delta: ContentBlockDelta::TextDelta {
            text: "hi".to_string(),
        },
    });
    assert!(matches!(cb_delta, StreamEvent::ContentBlockDelta(_)));

    let cb_stop = StreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 0 });
    assert!(matches!(cb_stop, StreamEvent::ContentBlockStop(_)));

    let msg_stop = StreamEvent::MessageStop(MessageStopEvent {});
    assert!(matches!(msg_stop, StreamEvent::MessageStop(_)));
}

#[test]
fn message_response_total_tokens() {
    let response = MessageResponse {
        id: "msg_1".to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![],
        model: "claude-3".to_string(),
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            cache_creation_input_tokens: 2,
            cache_read_input_tokens: 3,
            output_tokens: 5,
        },
        request_id: None,
    };
    assert_eq!(response.total_tokens(), 20);
}

#[test]
fn tool_result_content_block_variants() {
    let text = ToolResultContentBlock::Text {
        text: "result".to_string(),
    };
    assert!(matches!(text, ToolResultContentBlock::Text { .. }));

    let json_block = ToolResultContentBlock::Json {
        value: serde_json::json!({"key": "val"}),
    };
    assert!(matches!(json_block, ToolResultContentBlock::Json { .. }));
}

#[test]
fn input_content_block_variants() {
    let text = InputContentBlock::Text {
        text: "hi".to_string(),
    };
    assert!(matches!(text, InputContentBlock::Text { .. }));

    let tool_use = InputContentBlock::ToolUse {
        id: "tu_1".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({}),
    };
    assert!(matches!(tool_use, InputContentBlock::ToolUse { .. }));

    let tool_result = InputContentBlock::ToolResult {
        tool_use_id: "tu_1".to_string(),
        content: vec![],
        is_error: false,
    };
    assert!(matches!(tool_result, InputContentBlock::ToolResult { .. }));
}

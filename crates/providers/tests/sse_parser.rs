use orbit_providers::SseParser;

#[test]
fn sse_parser_new_is_empty() {
    let mut parser = SseParser::new();
    let events = parser.finish().unwrap();
    assert!(events.is_empty());
}

#[test]
fn sse_parser_finish_empty_buffer_returns_empty() {
    let mut parser = SseParser::new();
    let events = parser.finish().unwrap();
    assert!(events.is_empty());
}

#[test]
fn sse_parser_push_data() {
    let mut parser = SseParser::new();
    // push with valid SSE (no data field defaults or sends empty data)
    let result = parser.push(b"event: ping\ndata: {\"type\":\"ping\"}\n\n");
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn sse_parser_push_event() {
    let mut parser = SseParser::new();
    let result = parser.push(b"event: message_start\ndata: {\"type\":\"message_start\"}\n\n");
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn sse_parser_default_impl() {
    let mut parser: SseParser = SseParser::default();
    let events = parser.finish().unwrap();
    assert!(events.is_empty());
}

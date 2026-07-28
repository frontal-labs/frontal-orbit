# Rust SDK Testing Guide

## Test Architecture

Tests are fully hermetic - they do **not** require:
- Network access
- API keys (`CODEX_API_KEY`)
- Real `orbit` CLI binary

Instead, tests use a mock CLI script (`tests/mock_orbit.sh`) that emits
predefined JSONL events over stdout.

## Running Tests

```bash
# Run all tests
cargo test --package orbit-sdk

# Run with output
cargo test --package orbit-sdk -- --nocapture

# Run specific test
cargo test --package orbit-sdk test_thread_run -- --nocapture
```

## Test Structure

```
tests/
├── mock_orbit.sh          # Mock CLI - emits JSONL events
├── test_basic.rs          # Basic thread run tests
├── test_streaming.rs      # Stream event tests
├── test_structured.rs     # Structured output / JSON Schema tests
├── test_resume.rs         # Thread resume tests
├── test_config.rs         # Config override tests
└── test_errors.rs         # Error handling tests
```

## Mock CLI Protocol

The mock CLI (`tests/mock_orbit.sh`) reads stdin for the request payload
and writes JSONL events to stdout. It understands these request fields:

```json
{
  "input": "user message",
  "session_id": "optional-resume-id",
  "config": { "key": "value" },
  "output_schema": { ... }
}
```

The mock responds with events matching the real CLI format:

```jsonl
{"type":"item_completed","item":{"id":"msg-1","type":"message","content":"Hello!"}}
{"type":"turn_completed","result":{"final_response":"Hello!","items":[],"usage":{}}}
```

## Writing New Tests

### 1. Add test events to `mock_orbit.sh`

```bash
case "$REQUEST_TYPE" in
  "my_new_scenario")
    cat <<'EOF'
    {"type":"item_completed","item":{"id":"msg-1","type":"message","content":"Custom response"}}
    {"type":"turn_completed","result":{"final_response":"Custom response","items":[],"usage":{}}}
EOF
    ;;
esac
```

### 2. Create test case

```rust
#[tokio::test]
async fn test_my_new_scenario() {
    let orbit = Orbit::new(OrbitConfig {
        command: Some("tests/mock_orbit.sh".into()),
        env: Some([("REQUEST_TYPE".into(), "my_new_scenario".into())].into()),
        ..Default::default()
    });

    let thread = orbit.start_thread(ThreadOptions::default()).unwrap();
    let result = thread.run(&ThreadInput::Text("test".into()), &ThreadRunOptions::default()).unwrap();

    assert_eq!(result.final_response, "Custom response");
}
```

## Test Utilities

### `TestOrbitBuilder` (in `tests/common.rs`)

```rust
let orbit = TestOrbitBuilder::new()
    .with_mock_response("my_scenario")
    .with_env("CUSTOM_VAR", "value")
    .build();
```

### Event Assertions

```rust
use orbit_sdk::test_utils::{assert_event_seq, assert_final_response};

let mut stream = thread.run_streamed(&input, &options).unwrap();
assert_event_seq!(&mut stream, ItemCompleted, TurnCompleted);
let result = assert_final_response(stream).await;
```

## Continuous Integration

Tests run in GitHub Actions on every PR:

```yaml
# .github/workflows/rust-sdk.yml
- uses: actions-rs/cargo@v1
  with:
    command: test
    args: --package orbit-sdk --workspace
```

## Coverage

```bash
cargo llvm-cov --package orbit-sdk --html
```
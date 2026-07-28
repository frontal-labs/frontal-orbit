# Rust SDK API Reference

## Module: `orbit_sdk`

### `Orbit`

#### `Orbit::new(config: OrbitConfig) -> Orbit`
Create a new Orbit client.

**Parameters:**
- `config.command` - Path to `orbit` binary (default: `"orbit"`)
- `config.base_url` - Override API base URL (maps to `--config frontal_base_url`)
- `config.env` - Environment variables to pass to CLI process
- `config.config_overrides` - Global `--config` key/value pairs

#### `Orbit::start_thread(&self, options: ThreadOptions) -> Result<Thread, OrbitError>`
Start a new conversation thread.

#### `Orbit::resume_thread(&self, thread_id: &str, options: ThreadOptions) -> Result<Thread, OrbitError>`
Resume an existing thread by session ID.

---

### `Thread`

#### `Thread::id(&self) -> &str`
Get the session ID.

#### `Thread::run(&self, input: &ThreadInput, options: &ThreadRunOptions) -> Result<TurnResult, OrbitError>`
Run a turn and buffer all events until completion. Returns final result.

#### `Thread::run_streamed(&self, input: &ThreadInput, options: &ThreadRunOptions) -> Result<StreamedTurn, OrbitError>`
Run a turn and return a stream of events for real-time processing.

---

### `ThreadOptions`

```rust
pub struct ThreadOptions {
    pub working_directory: Option<PathBuf>,
    pub skip_git_repo_check: bool,
    pub config_overrides: HashMap<String, Value>,
}
```

---

### `ThreadRunOptions`

```rust
pub struct ThreadRunOptions {
    pub output_schema: Option<Value>,           // JSON Schema for structured output
    pub config_overrides: HashMap<String, Value>, // Per-turn --config overrides
}
```

---

### `ThreadInput`

```rust
pub enum ThreadInput {
    Text(String),
    LocalImage { path: PathBuf },
}
```

---

### `TurnResult`

```rust
pub struct TurnResult {
    pub final_response: String,
    pub items: Vec<Item>,
    pub usage: Option<Usage>,
}
```

---

### `StreamedTurn`

```rust
pub struct StreamedTurn {
    pub events: Pin<Box<dyn Stream<Item = Result<StreamEvent, OrbitError>> + Send>>,
}
```

---

### `StreamEvent`

```rust
pub enum StreamEvent {
    ItemCompleted(Item),
    TurnCompleted(TurnResult),
    Error(ErrorEvent),
}
```

---

### `Item`

```rust
pub struct Item {
    pub id: String,
    pub item_type: ItemType,
    pub content: Option<String>,
    pub tool_call: Option<ToolCall>,
    pub tool_result: Option<ToolResult>,
}
```

---

### `OrbitConfig`

```rust
pub struct OrbitConfig {
    pub command: Option<String>,           // default: "orbit"
    pub base_url: Option<String>,          // maps to frontal_base_url
    pub env: HashMap<String, String>,      // additional env vars
    pub config_overrides: HashMap<String, Value>, // global --config
}
```

---

### `OrbitError`

```rust
pub enum OrbitError {
    Io(std::io::Error),
    Json(serde_json::Error),
    CliNotFound(String),
    ProcessExited(std::process::ExitStatus),
    InvalidEvent(String),
    ThreadNotFound(String),
}
```

Implements `std::error::Error`, `Debug`, `Display`.

---

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `stream` | ✓ | Enables `run_streamed()` and `StreamEvent` |

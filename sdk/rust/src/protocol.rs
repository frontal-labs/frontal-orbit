//! Shared protocol types for the Orbit SDK.
//!
//! The SDK wraps the `orbit` CLI. The event names (`item.completed`,
//! `turn.completed`) and the [`TurnResult`] shape follow the SDK README and are
//! mapped from the CLI's `--output-format json --stream` JSONL output.

/// A single item produced during a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadItem {
  Text {
    text: String,
  },
  ToolUse {
    name: String,
    input: String,
  },
  ToolResult {
    content: String,
  },
  Image {
    path: Option<String>,
    url: Option<String>,
  },
}

/// Token usage reported by the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
  pub input_tokens: u64,
  pub output_tokens: u64,
  pub cache_creation_input_tokens: u64,
  pub cache_read_input_tokens: u64,
}

/// The buffered result of a completed turn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnResult {
  pub final_response: String,
  pub items: Vec<ThreadItem>,
  pub usage: Option<Usage>,
}

/// Structured events emitted while a turn runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrbitEvent {
  TurnStarted,
  ItemCompleted {
    item: ThreadItem,
  },
  TurnCompleted {
    final_response: String,
    usage: Option<Usage>,
    session_id: Option<String>,
  },
  TurnFailed {
    error: String,
  },
}

/// Structured input entry accepted by [`Thread::run`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEntry {
  Text { text: String },
  LocalImage { path: String },
}

/// A turn can be started with a plain prompt string or structured entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadInput {
  Text(String),
  Entries(Vec<InputEntry>),
}

/// Options for constructing the [`Orbit`] client.
#[derive(Debug, Clone, Default)]
pub struct OrbitOptions {
  /// Environment passed to the spawned CLI. Merged over the ambient env.
  pub env: Option<std::collections::HashMap<String, String>>,
  /// Base URL the CLI should use; passed as a `frontal_base_url` config
  /// override.
  pub base_url: Option<String>,
  /// Global CLI `--config` overrides.
  pub config: Option<serde_json::Value>,
  /// Path to the `orbit` CLI binary. Defaults to `"orbit"`.
  pub command: Option<String>,
}

/// Options for starting or resuming a [`Thread`].
#[derive(Debug, Clone, Default)]
pub struct ThreadOptions {
  /// Run the CLI with this working directory.
  pub working_directory: Option<std::path::PathBuf>,
  /// Skip the CLI's Git repository check.
  pub skip_git_repo_check: bool,
  /// Provider override (e.g. `anthropic`, `frontal`).
  pub provider: Option<String>,
  /// Model override (e.g. `opus`, `claude-opus-4-6`).
  pub model: Option<String>,
  /// Permission mode override.
  pub permission_mode: Option<String>,
  /// Thread-level CLI `--config` overrides (precedence over global).
  pub config: Option<serde_json::Value>,
}

/// Per-turn options for [`Thread::run`].
#[derive(Debug, Clone, Default)]
pub struct ThreadRunOptions {
  /// JSON schema the agent should conform its response to.
  pub output_schema: Option<serde_json::Value>,
  /// Run-level CLI `--config` overrides (highest precedence).
  pub config: Option<serde_json::Value>,
}

impl ThreadInput {
  /// Build the CLI prompt string and the list of image paths.
  pub fn to_prompt_and_images(&self) -> (String, Vec<String>) {
    match self {
      ThreadInput::Text(text) => (text.clone(), Vec::new()),
      ThreadInput::Entries(entries) => {
        let mut texts = Vec::new();
        let mut images = Vec::new();
        for entry in entries {
          match entry {
            InputEntry::Text { text } => texts.push(text.clone()),
            InputEntry::LocalImage { path } => images.push(path.clone()),
          }
        }
        (texts.join("\n"), images)
      }
    }
  }
}

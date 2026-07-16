//! The top-level [`Orbit`] client.

use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::{OrbitOptions, ThreadOptions};
use crate::thread::Thread;

const REQUIRED_ENV_VARS: &[&str] = &["CODEX_API_KEY"];

/// The top-level Orbit client. Wraps the `orbit` CLI and is used to start and
/// resume conversation [`Thread`]s.
#[derive(Debug, Clone)]
pub struct Orbit {
  pub options: OrbitOptions,
  pub command: String,
}

impl Orbit {
  /// Create a client from the given options.
  pub fn new(options: OrbitOptions) -> Self {
    let command = options
      .command
      .clone()
      .unwrap_or_else(|| "orbit".to_string());
    Self { options, command }
  }

  /// Start a fresh conversation thread.
  pub fn start_thread(&self, options: ThreadOptions) -> Thread {
    Thread::new(Arc::new(self.clone()), options, None)
  }

  /// Reconstruct a thread from a previously persisted session id.
  pub fn resume_thread(&self, session_id: String, options: ThreadOptions) -> Thread {
    Thread::new(Arc::new(self.clone()), options, Some(session_id))
  }

  /// Build the environment for a spawned CLI. Starts from the ambient
  /// environment, applies the user-provided `env`, then injects any required
  /// variables (such as `CODEX_API_KEY`) if still missing.
  pub fn build_env(&self) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    if let Some(user_env) = &self.options.env {
      for (key, value) in user_env {
        env.insert(key.clone(), value.clone());
      }
    }
    for var in REQUIRED_ENV_VARS {
      if !env.contains_key(*var) {
        if let Ok(value) = std::env::var(var) {
          env.insert((*var).to_string(), value);
        }
      }
    }
    env
  }
}

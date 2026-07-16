//! Orbit SDK — embed the Orbit agent in your Rust workflows and apps.
//!
//! The SDK wraps the `orbit` CLI. Construct an [`Orbit`] client, start a
//! [`Thread`], and call [`Thread::run`] (buffered) or [`Thread::run_streamed`]
//! (event stream) to drive turns.

#![allow(missing_docs)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::type_complexity)]

mod config;
mod orbit;
mod protocol;
mod spawn;
mod thread;

pub use config::{config_to_args, flatten_config, merge_config, to_toml_literal};
pub use orbit::Orbit;
pub use protocol::{
    InputEntry, OrbitEvent, OrbitOptions, ThreadInput, ThreadItem, ThreadOptions, ThreadRunOptions,
    TurnResult, Usage,
};
pub use spawn::{OrbitError, StreamedTurn};
pub use thread::Thread;

# Orbit SDK (Rust)

Embed the Orbit agent in your Rust workflows and apps.

The SDK wraps the `orbit` CLI (`@frontal-labs/orbit`). It spawns the CLI per
turn and continues conversations via the CLI's `--resume <sessionId>` flag,
parsing the `--output-format json --stream` JSONL event stream.

## Installation

```toml
[dependencies]
orbit-sdk = "0.1"
```

Requires the `orbit` CLI on `PATH` (or provide a custom `command`).

## Quickstart

```rust
use orbit_sdk::{Orbit, ThreadInput, ThreadOptions, ThreadRunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orbit = Orbit::new(Default::default());
    let thread = orbit.start_thread(ThreadOptions::default());
    let turn = thread
        .run(&ThreadInput::Text("Diagnose the test failure".into()), &ThreadRunOptions::default())
        .await?;
    println!("{}", turn.final_response);
    Ok(())
}
```

## Features

- `Orbit::start_thread` / `Orbit::resume_thread`
- `Thread::run` (buffered `TurnResult`) and `Thread::run_streamed` (event stream)
- Structured input entries (`text`, `local_image`)
- `--config` overrides flattened to dotted TOML literals (incl. `baseUrl` →
  `frontal_base_url`)
- Multi-turn continuation via `--resume`

## Testing

The SDK's tests are hermetic: they spawn a small mock `orbit` script instead of
the real binary, so no API key or network is required.

```bash
cargo test -p orbit-sdk
```

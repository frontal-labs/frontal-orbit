# Rust SDK Examples

## Installation

```toml
[dependencies]
orbit-sdk = "0.1"
```

Requires the `orbit` CLI on `PATH` (or provide custom `command`).

## Basic Usage

### Simple Text Turn

```rust
use orbit_sdk::{Orbit, ThreadInput, ThreadOptions, ThreadRunOptions};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let orbit = Orbit::new(Default::default());
    let thread = orbit.start_thread(ThreadOptions::default());
    
    let turn = thread.run(
        &ThreadInput::Text("Explain async/await in Rust".into()),
        &ThreadRunOptions::default()
    ).await?;
    
    println!("{}", turn.final_response);
    Ok(())
}
```

### Multi-Turn Conversation

```rust
let thread = orbit.start_thread(ThreadOptions::default());

// Turn 1
let turn1 = thread.run(
    &ThreadInput::Text("Create a CLI argument parser in Rust".into()),
    &ThreadRunOptions::default()
).await?;

// Turn 2 - continues same conversation
let turn2 = thread.run(
    &ThreadInput::Text("Add subcommand support".into()),
    &ThreadRunOptions::default()
).await?;

for item in &turn2.items {
    println!("{:?}: {}", item.item_type, item.content.as_deref().unwrap_or(""));
}
```

## Streaming Events

```rust
use futures_util::StreamExt;

let streamed = thread.run_streamed(
    &ThreadInput::Text("Refactor this function".into()),
    &ThreadRunOptions::default()
).await?;

let mut events = streamed.events;
while let Some(event) = events.next().await {
    match event? {
        StreamEvent::ItemCompleted(item) => {
            println!("✓ {:?}: {:?}", item.item_type, item.content.as_deref().unwrap_or(""));
        }
        StreamEvent::TurnCompleted(result) => {
            println!("\nFinal: {}", result.final_response);
        }
        StreamEvent::Error(err) => {
            eprintln!("Error: {:?}", err);
        }
    }
}
```

## Structured Output (JSON Schema)

```rust
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "summary": { "type": "string" },
        "issues": { 
            "type": "array", 
            "items": { "type": "string" } 
        },
        "severity": { 
            "type": "string", 
            "enum": ["low", "medium", "high", "critical"] 
        }
    },
    "required": ["summary", "severity"],
    "additionalProperties": false
});

let result = thread.run(
    &ThreadInput::Text("Analyze this code for security issues".into()),
    &ThreadRunOptions {
        output_schema: Some(schema),
        ..Default::default()
    }
).await?;

// result.final_response is guaranteed valid JSON matching schema
let analysis: serde_json::Value = serde_json::from_str(&result.final_response)?;
println!("Severity: {}", analysis["severity"]);
```

## Image Input

```rust
use std::path::PathBuf;

let result = thread.run(
    &ThreadInput::LocalImage { path: PathBuf::from("./screenshot.png") },
    &ThreadRunOptions::default()
).await?;
```

## Configuration

### Custom CLI Path

```rust
let orbit = Orbit::new(OrbitConfig {
    command: Some("/custom/path/to/orbit".into()),
    ..Default::default()
});
```

### Custom API Base URL

```rust
let orbit = Orbit::new(OrbitConfig {
    base_url: Some("https://api.example.com".into()),
    ..Default::default()
});
```

### Environment Variables

```rust
use std::collections::HashMap;

let mut env = HashMap::new();
env.insert("CUSTOM_VAR".into(), "value".into());
env.insert("DEBUG".into(), "orbit:*".into());

let orbit = Orbit::new(OrbitConfig {
    env,
    ..Default::default()
});
```

### Global Config Overrides

```rust
use std::collections::HashMap;
use serde_json::json;

let mut config = HashMap::new();
config.insert("show_raw_agent_reasoning".into(), json!(true));
config.insert("sandbox_workspace_write".into(), json!({ "network_access": true }));

let orbit = Orbit::new(OrbitConfig {
    config_overrides: config,
    ..Default::default()
});
```

### Per-Turn Config Overrides

```rust
let result = thread.run(
    &ThreadInput::Text("Be concise".into()),
    &ThreadRunOptions {
        config_overrides: {
            let mut m = HashMap::new();
            m.insert("temperature".into(), json!(0.1));
            m.insert("model".into(), json!("gpt-4"));
            m
        },
        ..Default::default()
    }
).await?;
```

## Thread Options

### Working Directory

```rust
let thread = orbit.start_thread(ThreadOptions {
    working_directory: Some("/path/to/project".into()),
    ..Default::default()
});
```

### Skip Git Repo Check

```rust
let thread = orbit.start_thread(ThreadOptions {
    working_directory: Some("/non/git/dir".into()),
    skip_git_repo_check: true,
    ..Default::default()
});
```

## Resume Existing Thread

```rust
// First run - persist the thread ID
let thread = orbit.start_thread(ThreadOptions::default());
let thread_id = thread.id().to_string();
save_thread_id(&thread_id); // Your persistence logic

// Later - resume the conversation
let resumed = orbit.resume_thread(&thread_id, ThreadOptions::default())?;
let turn = resumed.run(
    &ThreadInput::Text("Continue from where we left off".into()),
    &ThreadRunOptions::default()
).await?;
```

## Error Handling

```rust
use orbit_sdk::OrbitError;

match thread.run(&input, &options).await {
    Ok(result) => println!("{}", result.final_response),
    Err(OrbitError::Io(e)) => eprintln!("IO error: {}", e),
    Err(OrbitError::Json(e)) => eprintln!("JSON parse error: {}", e),
    Err(OrbitError::CliNotFound(cmd)) => eprintln!("CLI not found: {}", cmd),
    Err(OrbitError::ProcessExited(status)) => eprintln!("CLI exited: {}", status),
    Err(OrbitError::InvalidEvent(msg)) => eprintln!("Invalid event: {}", msg),
    Err(OrbitError::ThreadNotFound(id)) => eprintln!("Thread not found: {}", id),
}
```

## Testing with Mock CLI

```rust
#[cfg(test)]
mod tests {
    use orbit_sdk::{Orbit, OrbitConfig, ThreadInput, ThreadOptions, ThreadRunOptions};

    #[tokio::test]
    async fn test_basic_thread() {
        let orbit = Orbit::new(OrbitConfig {
            command: Some("tests/mock_orbit.sh".into()),
            env: Some([("REQUEST_TYPE".into(), "basic".into())].into()),
            ..Default::default()
        });

        let thread = orbit.start_thread(ThreadOptions::default());
        let result = thread.run(
            &ThreadInput::Text("test".into()),
            &ThreadRunOptions::default()
        ).await.unwrap();

        assert_eq!(result.final_response, "Mock response");
    }
}
```

## Feature Flags

```toml
# Cargo.toml
[dependencies]
orbit-sdk = { version = "0.1", features = ["stream"] }  # default

# Disable streaming to reduce dependencies
orbit-sdk = { version = "0.1", default-features = false }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `stream` | ✓ | Enables `run_streamed()` and `StreamEvent` |

## TypeScript Schema Integration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SecurityAnalysis {
    summary: String,
    issues: Vec<String>,
    severity: Severity,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

// Convert to JSON Schema using schemars
use schemars::JsonSchema;

fn security_schema() -> serde_json::Value {
    serde_json::to_value(<SecurityAnalysis as JsonSchema>::schema()).unwrap()
}

let result = thread.run(
    &ThreadInput::Text("Check for vulnerabilities".into()),
    &ThreadRunOptions {
        output_schema: Some(security_schema()),
        ..Default::default()
    }
).await?;

let analysis: SecurityAnalysis = serde_json::from_str(&result.final_response)?;
```
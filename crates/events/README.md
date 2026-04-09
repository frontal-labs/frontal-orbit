# Orbit Events

Event system and messaging infrastructure for the Orbit ecosystem.

## Overview

The `orbit-events` crate provides the foundational event system that enables communication and coordination between different components of the Orbit ecosystem. It defines common event types, event handling patterns, and messaging primitives used throughout the workspace.

## Features

- **Event Types**: Common event structures and payloads
- **Event Handlers**: Traits and utilities for event processing
- **Event Bus**: Central event distribution and routing
- **Event Serialization**: Efficient event encoding/decoding
- **Event Filtering**: Event routing and subscription management
- **Event History**: Event persistence and replay capabilities

## Key Components

### Event Types
- System events (startup, shutdown, errors)
- User interaction events (commands, requests)
- Tool execution events (start, progress, completion)
- Provider events (requests, responses, errors)
- Session events (create, update, delete)

### Event Handling
- Event listener traits and implementations
- Event processor interfaces
- Async event handling utilities
- Event error handling and recovery

### Event Distribution
- Event bus implementation
- Topic-based routing
- Event subscription management
- Event filtering and transformation

## Current Status

This crate is included in workspace build/test gates and currently exposes a minimal baseline surface for event evolution. The event system is designed to grow with the needs of the Orbit ecosystem.

## Usage

```rust
use orbit_events::{Event, EventHandler, EventBus};

// Define custom events
#[derive(Event)]
struct MyEvent {
    data: String,
}

// Implement event handlers
struct MyHandler;
impl EventHandler<MyEvent> for MyHandler {
    fn handle(&self, event: &MyEvent) -> Result<(), EventError> {
        // Handle event
        Ok(())
    }
}

// Use the event bus
let bus = EventBus::new();
bus.subscribe(MyHandler);
bus.publish(MyEvent { data: "test".to_string() });
```

## Event Patterns

The crate supports several common event patterns:

### Request/Response
- Request events with correlation IDs
- Response events matching requests
- Timeout and error handling

### Publish/Subscribe
- One-to-many event distribution
- Topic-based subscriptions
- Event filtering and routing

### Event Sourcing
- Immutable event streams
- Event replay capabilities
- State reconstruction from events

## Integration

The event system integrates with:
- `orbit-runtime` for session and tool events
- `orbit-telemetry` for event tracking and analytics
- `orbit-server` for distributed event handling
- `orbit-providers` for AI provider events

## Performance Considerations

- Efficient event serialization
- Minimal allocation for hot paths
- Async event processing support
- Event batching for high-frequency events

## Testing

Event system tests include:
- Event serialization/deserialization
- Event handler correctness
- Event bus routing
- Performance benchmarks

Run tests with:
```bash
cargo test -p orbit-events
```

## Future Development

Planned enhancements:
- Event schema validation
- Event versioning support
- Distributed event handling
- Event analytics and monitoring

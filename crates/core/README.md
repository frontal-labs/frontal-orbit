# Orbit Core

Shared core capabilities and foundational types for the Orbit ecosystem.

## Overview

The `orbit-core` crate provides essential shared types, traits, and utilities that form the foundation for all Orbit components. It serves as the central collection of common functionality used across the entire workspace, ensuring consistency and reducing code duplication.

## Features

- **Shared Types**: Common data structures used across Orbit components
- **Foundation Traits**: Core traits for component integration
- **Utility Functions**: Essential helper functions and macros
- **Error Types**: Base error handling and common error variants
- **Constants**: Shared constants and configuration values
- **Extensions**: Common extensions to standard library types

## Key Components

### Shared Types
- Common request/response structures
- Configuration types and builders
- Identifier types and validation
- Serialization/deserialization helpers

### Foundation Traits
- Component lifecycle traits
- Plugin interface definitions
- Provider abstractions
- Tool execution contracts

### Utilities
- String manipulation helpers
- Path and file system utilities
- Validation and parsing functions
- Time and duration helpers

## Current Status

This crate is included in workspace build/test gates and provides a minimal baseline surface with room for additional shared primitives. As the Orbit ecosystem grows, this crate will expand to include more common functionality.

## Usage

```rust
use orbit_core::{SharedType, CoreTrait, core_utility};

// Use shared types across components
let data = SharedType::new();

// Implement core traits for custom components
struct MyComponent;
impl CoreTrait for MyComponent {
    // implementation
}

// Use utility functions
let result = core_utility(&input)?;
```

## Design Principles

The core crate follows these principles:
- **Minimalism**: Only includes truly shared functionality
- **Stability**: Provides stable APIs for other crates to depend on
- **Performance**: Optimized for common use cases
- **Extensibility**: Designed to grow with the ecosystem

## Dependencies

The core crate has minimal external dependencies to keep it lightweight and fast to compile. It primarily depends on:
- Standard library types
- Common serialization libraries (serde)
- Essential utility crates

## Testing

Run core tests with:
```bash
cargo test -p orbit-core
```

## Future Expansion

Planned additions include:
- More shared data structures
- Additional utility functions
- Performance-optimized collections
- Enhanced error handling types

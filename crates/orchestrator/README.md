# Orbit Orchestrator

Orchestration and workflow management system for the Orbit ecosystem, providing intelligent routing, execution planning, and resource allocation for AI agent workflows.

## Overview

The `orbit-orchestrator` crate provides the core orchestration capabilities that enable complex AI agent workflows to be executed efficiently and reliably. It handles work item routing, execution planning, lane assignment, and resource management across the Orbit ecosystem.

## Features

- **Work Item Management**: Typed work items with metadata and priority handling
- **Execution Planning**: Intelligent planning and scheduling of complex workflows
- **Lane Assignment**: Resource allocation and execution lane management
- **Routing Logic**: Smart routing of work items to appropriate handlers
- **Priority Queuing**: Priority-based work item processing
- **Resource Management**: Efficient allocation and tracking of system resources

## Key Components

### Work Items
- Typed work item definitions with metadata
- Priority levels and execution requirements
- Source tracking and provenance

### Planning Engine
- Execution plan generation
- Dependency resolution
- Resource requirement analysis

### Routing System
- Intelligent work item routing
- Lane assignment algorithms
- Load balancing and optimization

### Resource Management
- Resource allocation and tracking
- Capacity planning
- Performance monitoring

## Current Status

This crate now exposes the minimal planning API used by the hosted server: typed work items, sources, priorities, and lane assignments with routing tests. The implementation focuses on providing a solid foundation for workflow orchestration while maintaining flexibility for future expansion.

## Usage

```rust
use orbit_orchestrator::{WorkItem, WorkSource, Priority, LaneAssignment};

let work_item = WorkItem {
    id: "task_123".to_string(),
    source: WorkSource::UserRequest,
    priority: Priority::High,
    // ... other fields
};

let lane_assignment = orchestrator.assign_lane(&work_item)?;
let execution_plan = orchestrator.create_plan(&work_item)?;
```

## Architecture

The orchestrator follows a modular architecture with clear separation of concerns:

1. **Work Item Layer**: Defines work items and their metadata
2. **Planning Layer**: Handles execution planning and scheduling
3. **Routing Layer**: Manages work item routing and lane assignment
4. **Resource Layer**: Handles resource allocation and management

## Testing

The crate includes comprehensive routing tests and planning validation:

```bash
cargo test -p orbit-orchestrator
```

## Integration

The orchestrator integrates with:
- `orbit-runtime` for execution coordination
- `orbit-telemetry` for performance monitoring
- `orbit-server` for hosted workflow management

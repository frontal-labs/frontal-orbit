# Orbit Repo

Repository lifecycle management and source tree preparation for hosted execution environments.

## Overview

The `orbit-repo` crate provides comprehensive repository management capabilities for hosted execution scenarios. It serves as the boundary between connector/control-plane code and source-tree preparation, handling all aspects of repository lifecycle management while remaining platform-agnostic.

## Features

- **Repository Cloning**: Secure and efficient repository checkout
- **Source Tree Management**: Fetch, update, and branch operations
- **Base Reference Resolution**: Automatic detection and resolution of base branches
- **Branch Management**: Creation, switching, and reset operations
- **Platform Agnostic**: Works with any Git hosting service
- **Clean Environment**: Isolated repository preparation for each execution

## Key Components

### Repository Operations
- `RepoManager` - Main repository lifecycle management
- `CheckoutManager` - Local checkout preparation and cleanup
- `BranchManager` - Branch creation, switching, and management
- `ReferenceResolver` - Base reference and commit resolution

### Source Tree Preparation
- Clone operations with proper authentication
- Fetch operations with selective depth
- Branch creation from specific commits
- Working tree preparation and cleanup

## Architecture

The crate is designed as a clear boundary layer:
1. **Control Plane Interface**: High-level repository operations
2. **Git Operations**: Low-level Git command execution
3. **File System Management**: Working directory handling
4. **Authentication**: Secure access to repositories

## Current Capabilities

The crate handles local checkout preparation concerns including:
- Clone operations with proper authentication
- Fetch operations for updating repositories
- Base-ref resolution for determining target branches
- Branch creation and reset operations
- Working directory isolation and cleanup

## Usage

```rust
use orbit_repo::{RepoManager, CheckoutConfig};

let manager = RepoManager::new();
let config = CheckoutConfig {
    repo_url: "https://github.com/owner/repo.git",
    target_dir: "/tmp/workspace",
    base_ref: "main",
};

let checkout = manager.prepare_checkout(config)?;
let repo = checkout.repository();
```

## Platform Agnostic Design

The crate is intentionally GitHub-agnostic and works with any Git hosting service:
- GitHub, GitLab, Bitbucket, and self-hosted Git servers
- SSH and HTTPS authentication methods
- Custom Git server configurations
- Enterprise Git environments

## Security Considerations

- Isolated checkout directories for each execution
- Secure credential handling
- Clean environment preparation
- Proper cleanup of temporary files
- Access control and permission management

## Integration

The repo crate integrates with:
- `orbit-server` for hosted execution workflows
- `orbit-orchestrator` for task preparation
- `orbit-runtime` for execution environment setup

## Error Handling

Comprehensive error handling for:
- Network connectivity issues
- Authentication failures
- Repository access problems
- File system permissions
- Git operation failures

## Testing

Extensive test coverage includes:
- Repository cloning and checkout
- Branch management operations
- Reference resolution accuracy
- Error handling scenarios
- Integration with hosted workflows

Run tests with:
```bash
cargo test -p orbit-repo
```

## Performance Optimizations

- Shallow clone support for faster checkouts
- Incremental fetch operations
- Parallel repository operations
- Efficient reference resolution
- Optimized working directory management

## Future Development

Planned enhancements:
- Additional Git hosting service integrations
- Advanced caching strategies
- Performance monitoring
- Enhanced security features
- Multi-repository workflows

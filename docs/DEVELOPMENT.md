# Development Guide

This guide covers how to set up a development environment, contribute to the project, and understand the development workflow.

## Prerequisites

- Rust 1.70+ (recommended: latest stable)
- Git
- Docker (optional, for containerized development)

## Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd claw-code-main
   ```

2. **Install Rust dependencies**
   ```bash
   cargo build --workspace
   ```

3. **Set up environment variables**
   ```bash
   # Copy the example environment file
   cp .env.example .env
   
   # Edit .env with your API keys
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

## Development Workflow

### Building

```bash
# Build the entire workspace
cargo build --workspace

# Build with optimizations
cargo build --release

# Build specific crate
cargo build -p orbit-cli
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test -p orbit-cli test_name
```

### Running the CLI

```bash
# Development build
cargo run -p orbit-cli -- --help

# With specific model
cargo run -p orbit-cli -- --model claude-sonnet-4-6

# Interactive REPL
cargo run -p orbit-cli -- repl
```

## Code Organization

### Workspace Structure

The project uses a Cargo workspace with multiple crates:

- **`crates/cli`** - Main CLI binary and argument parsing
- **`crates/runtime`** - Core runtime, session management, configuration
- **`crates/providers`** - AI provider implementations (Anthropic, OpenAI, xAI)
- **`crates/tools`** - Built-in tools and skill system
- **`crates/commands`** - Slash command definitions and parsing
- **`crates/plugins`** - Plugin system and management
- **`crates/api`** - Public API facade
- **`crates/mock-anthropic-service`** - Testing mock service

### Adding New Features

1. **New Tools**: Add to `crates/tools/src/`
2. **New Commands**: Add to `crates/commands/src/`
3. **New Providers**: Add to `crates/providers/src/`
4. **New Plugins**: Add to `crates/plugins/src/`

## Testing Strategy

### Unit Tests

Each crate contains unit tests for its core functionality:

```bash
cargo test -p crate-name
```

### Integration Tests

The CLI crate contains integration tests that test the full command flow:

```bash
cargo test -p orbit-cli --test integration
```

### Mock Parity Tests

The project includes a comprehensive mock testing harness:

```bash
# Run the mock parity harness
./scripts/run_mock_parity_harness.sh

# Run mock service manually
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Linting and Formatting

The project uses strict linting rules:

```bash
# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

## Documentation

### API Documentation

Generate and view API documentation:

```bash
# Generate documentation
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

### CLI Documentation

CLI help is automatically generated from argument definitions:

```bash
cargo run -p orbit-cli -- --help
cargo run -p orbit-cli -- help <command>
```

## Debugging

### Logging

Enable debug logging:

```bash
RUST_LOG=debug cargo run -p orbit-cli -- <command>
```

### Common Issues

1. **Permission denied errors**: Check file permissions and API key configuration
2. **Build failures**: Ensure Rust version is up to date and dependencies are current
3. **Test failures**: Verify environment variables are set correctly

## Contributing

### Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make changes and add tests
4. Run the test suite: `cargo test --workspace`
5. Run linting: `cargo clippy --workspace`
6. Submit a pull request

### Code Style

- Follow Rust idioms and conventions
- Use `cargo fmt` for formatting
- Write comprehensive tests for new features
- Update documentation for API changes

## Release Process

Releases are automated through GitHub Actions:

1. Update version numbers in `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create a release tag
4. GitHub Actions will build and publish releases

## Performance Considerations

- Use `cargo build --release` for performance testing
- Profile with `cargo flamegraph` for CPU-bound issues
- Monitor memory usage with `valgrind` or similar tools
- Use the mock service for consistent performance testing

## Troubleshooting

### Build Issues

- Clear cargo cache: `cargo clean`
- Update dependencies: `cargo update`
- Check Rust version: `rustc --version`

### Runtime Issues

- Check environment variables
- Verify API keys and permissions
- Enable debug logging for detailed error information
- Use the `doctor` command for system diagnostics

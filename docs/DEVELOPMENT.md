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
   export ORBIT_API_KEY="sk-ant-..."
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
- **`crates/orbit-mock-gateway`** - Testing mock service

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
cargo run -p orbit-mock-gateway -- --bind 127.0.0.1:0
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

## Bazel Roadmap

This tracks the planned maturation of the Bazel monorepo foundation. The
scaffold is intentionally conservative: aspects, extensions, transitions, and
per-language `macros` are **placeholders** until wired to real tooling.

### Done

- [x] Bzlmod-only root: `.bazelversion`, `.bazelrc`, `.bazelrc.project`,
      `.bazelignore`, root `BUILD`.
- [x] `MODULE.bazel` with `bazel_skylib`, `rules_shell`, `buildifier_prebuilt`, and the
      `third_party` `non_module_deps` extension.
- [x] `bazel/` infrastructure library (defs, toolchains, platforms,
      constraints, config, aspects, transitions, extensions, bzlmod, ci).
- [x] `third_party/` conventions (`repos.bzl`, `README`, `patches/`,
      `overrides/`, `archives/`, `manifests/`, `libraries/`, `tools/`).
- [x] Dev container, CI (`ci.yml`, `lint.yml`), pre-commit, Makefile, scripts.
- [x] Per-language `*_app()` macros and demo trees (`rust/`, `typescript/`).
- [x] Vendored `typescript_binary` rule under `third_party/bazel_rules/rules_typescript`.
- [x] `lint_aspect` wired to buildifier (real), with documented extension points
      for clippy and eslint.
- [x] `coverage_aspect` wired to `coverage_common.instrumented_files()` (real),
      with documented extension points for llvm-cov and istanbul.
- [x] Remote cache configuration documented in `.bazelrc.project`.
- [x] CI hardened: pinned `setup-bazel`, added `make ci` step.

### Next

- [ ] Implement clippy wiring in `lint_aspect` (requires `rust_clippy` target).
- [ ] Implement eslint wiring in `lint_aspect` (requires `aspect_rules_js`).
- [ ] Implement llvm-cov wiring in `coverage_aspect` for Rust.
- [ ] Implement istanbul wiring in `coverage_aspect` for TypeScript.
- [ ] Validate `bazel coverage //... --combined_report=lcov` end-to-end.
- [ ] Add per-language `*_app()` macros and demo trees for Go and Python
      (requires `rules_go` and `rules_python` in `MODULE.bazel`).
- [ ] Add a remote cache / RBE configuration behind `.bazelrc.project`.

## Troubleshooting

### Build Issues

- Clear cargo cache: `cargo clean`
- Update dependencies: `cargo update`
- Check Rust version: `rustc --version`

### Bazel Issues

- Clear Bazel state: `make clean` (or `EXPUNGE=1 make clean` for full expunge)
- Verify the Bazel version is pinned: `cat .bazelversion`
- Run `make doctor` to check the toolchain

### Runtime Issues

- Check environment variables
- Verify API keys and permissions
- Enable debug logging for detailed error information
- Use the `doctor` command for system diagnostics

# Contributing to Frontal Orbit Code

Thank you for your interest in contributing to Frontal Orbit Code! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70.0 or later
- Git
- A GitHub account

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/your-username/frontal-orbit.git
   cd frontal-orbit
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/frontal-labs/frontal-orbit.git
   ```
4. Install dependencies and build:
   ```bash
   cargo build --workspace
   ```
5. Run tests to ensure everything works:
   ```bash
   cargo test --workspace
   ```

## Development Workflow

### Branch Naming

Use the following branch naming conventions:
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test-related changes

### Making Changes

1. Create a new branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the coding standards below

3. Test your changes:
   ```bash
   cargo test --workspace
   cargo fmt --all --check
   cargo clippy --workspace
   ```

4. Commit your changes with a clear message:
   ```bash
   git commit -m "feat: add new feature description"
   ```

5. Push to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

6. Create a Pull Request on GitHub

## Coding Standards

### Rust Code Style

- Follow the official Rust style guide
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Write comprehensive tests for new functionality
- Document public APIs with `///` doc comments

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

### Documentation

- Update README.md if adding new user-facing features
- Update USAGE.md for CLI changes
- Add inline documentation for new APIs
- Consider adding examples for complex features

## Testing

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html

# Run specific test
cargo test --workspace test_name
```

### Writing Tests

- Write unit tests for individual modules
- Write integration tests for cross-module functionality
- Use `#[cfg(test)]` for test-only code
- Mock external dependencies when appropriate
- Test both success and failure cases

## Pull Request Process

### Before Submitting

1. Ensure your code passes all tests
2. Format your code with `cargo fmt`
3. Run `cargo clippy` and fix any warnings
4. Update documentation if needed
5. Rebase your branch on the latest main branch

### Pull Request Template

Use the provided pull request template when creating PRs. Include:

- Clear description of changes
- Type of change (bug fix, feature, etc.)
- Testing performed
- Any breaking changes

### Review Process

1. Automated checks must pass
2. At least one maintainer approval required
3. Address all review feedback
4. Maintain a clean commit history

## Release Process

Releases are handled by maintainers:

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Create a release tag
4. GitHub Actions will automatically build and release binaries

## Community

### Code of Conduct

Please be respectful and inclusive. Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

### Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for general questions
- Join our Discord community (link in README)

### Security Issues

For security vulnerabilities, please email security@frontal.dev instead of opening a public issue.

## Maintainers

Current maintainers:
- @frontal-labs
- @gabrielvfonseca

Maintainer responsibilities:
- Review and merge pull requests
- Manage releases
- Handle security issues
- Guide project direction

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)

Thank you for contributing to Frontal Orbit Code!

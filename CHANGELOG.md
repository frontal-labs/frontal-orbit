# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Complete development infrastructure setup
- Nix flake for reproducible development environment
- Pre-commit hooks configuration
- Docker and Docker Compose setup
- VS Code workspace configuration
- PostgreSQL database schema for structured memory
- Development documentation
- Homebrew formula for installing the `orbit` CLI from source

### Changed
- Repository structure moved to root level
- Updated documentation references
- Improved development workflow
- Release assets now include Homebrew-friendly tarballs and SHA-256 checksum files

### Fixed
- Corrected file path references in documentation
- Fixed Docker container permissions
- Resolved build configuration issues

## [0.1.0] - 2026-04-03

### Added
- Initial Rust implementation of Orbit
- Core CLI functionality
- Basic agent runtime
- Tool system implementation
- Session management
- Permission system
- MCP server lifecycle management
- Mock parity harness for testing
- Comprehensive test suite

### Features
- Interactive REPL with slash commands
- One-shot prompt execution
- Session persistence and resume
- Multi-provider support (Anthropic, OpenAI, xAI, Frontal, Bedrock, Azure)
- Comprehensive tool ecosystem
- Plugin management
- Configuration management
- Git integration
- Markdown terminal rendering

### Infrastructure
- 9-lane parity checkpoint achieved
- All 9 requested features merged to main
- Mock service for deterministic testing
- CI/CD pipeline configuration
- Container deployment support

### Documentation
- Complete usage documentation
- Development setup guide
- Architecture philosophy
- Roadmap and parity status
- API documentation

## [0.0.1] - 2026-03-31

### Added
- Project initialization
- Basic repository structure
- Initial documentation
- Development workflow setup

---

## Release Notes

### Version 0.1.0
This marks the first stable release of Orbit with complete Rust implementation. The project has achieved full parity with the original design specifications and includes all core functionality for both local development and server deployment.

Key achievements:
- Complete 9-lane parity implementation
- Production-ready infrastructure
- Comprehensive development tooling
- Full documentation and guides

### Upcoming Releases
Future releases will focus on:
- Enhanced TUI experience
- Multi-agent coordination
- Autonomous server features
- Advanced memory systems
- Event-driven automation

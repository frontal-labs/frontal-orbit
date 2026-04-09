# Orbit CLI Documentation

Welcome to the Orbit CLI documentation hub. This comprehensive guide covers everything you need to know about using Orbit, the high-performance Rust-based AI agent harness.

## 📚 Documentation Overview

### Getting Started
- **[Quick Start](../README.md#quick-start)** - Get up and running in minutes
- **[Installation](../DEVELOPMENT.md#setup)** - Installation and setup instructions
- **[Configuration](./CONFIGURATION.md)** - Complete configuration guide
- **[Examples](./EXAMPLES.md)** - Practical usage examples

### Core Features
- **[CLI Reference](./CLI_REFERENCES.md)** - Complete command-line interface reference
- **[API Reference](./API_REFERENCES.md)** - API documentation and integration guide
- **[Architecture](./ARCHITECTURE.md)** - System architecture and design
- **[Plugins](./PLUGINS.md)** - Plugin system and development guide

### Advanced Topics
- **[MCP Integration](./MCP.md)** - Model Context Protocol guide
- **[Performance](./PERFORMANCE.md)** - Performance optimization and monitoring
- **[Security](./SECURITY.md)** - Security features and best practices
- **[Containers](./CONTAINERS.md)** - Docker and containerization
- **[Hosted Engineering Plan](./HOSTED_ENGINEERING_IMPLEMENTATION_PLAN.md)** - Implementation plan for the hosted server, worker containers, multi-agent execution, and GitHub automation

### Support
- **[Troubleshooting](./TROUBLESHOOTING.md)** - Common issues and solutions
- **[FAQ](./FAQ.md)** - Frequently asked questions
- **[Development](../DEVELOPMENT.md)** - Contributing and development guide

## 🚀 Quick Start

### Installation

```bash
# Install with Homebrew from this repo
git clone <repository-url>
cd claw-code-main
brew install --HEAD ./homebrew/orbit.rb
```

For local development, build from source with `cargo build --workspace`.

### Basic Usage

```bash
# Interactive session
orbit repl

# One-shot prompt
orbit prompt "What files are in the current directory?"

# Read and analyze
orbit prompt "Read the README.md file and summarize it"

# Use specific model
orbit --model claude-sonnet-4-6 prompt "Explain this codebase"
```

### Configuration

```bash
# Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Configure default model
orbit config set runtime.default_model "claude-sonnet-4-6"

# Show configuration
orbit config show
```

## 🏗️ Architecture Overview

Orbit is built as a modular Rust workspace with the following key components:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Layer     │    │   Runtime       │    │   Providers    │
│                 │    │                 │    │                 │
│ • REPL          │◄──►│ • Session Mgmt  │◄──►│ • Anthropic     │
│ • Commands      │    │ • Permissions   │    │ • OpenAI       │
│ • UI           │    │ • MCP           │    │ • xAI          │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   Tools Layer   │
                    │                 │
                    │ • File Ops      │
                    │ • Shell         │
                    │ • Web           │
                    │ • Plugins       │
                    └─────────────────┘
```

### Key Features

- **🔧 Tool System**: Comprehensive built-in tools for file operations, shell commands, web access
- **🔌 Plugin Architecture**: Extensible plugin system for custom tools and providers
- **🛡️ Security**: Advanced permission system with sandboxing
- **⚡ Performance**: Rust-based implementation with async I/O and caching
- **🔄 MCP Support**: Model Context Protocol integration for external tools
- **💾 Session Management**: Persistent sessions with resume capability
- **📊 Monitoring**: Built-in performance monitoring and diagnostics

## 📖 Documentation Structure

### By User Role

#### For Developers
- **[Development Guide](../DEVELOPMENT.md)** - Contributing to Orbit
- **[API Reference](./API_REFERENCES.md)** - Programmatic usage
- **[Plugin Development](./PLUGINS.md#developing-plugins)** - Creating custom plugins
- **[Architecture](./ARCHITECTURE.md)** - System design and internals

#### For System Administrators
- **[Configuration](./CONFIGURATION.md)** - Deployment and configuration
- **[Security](./SECURITY.md)** - Security best practices
- **[Performance](./PERFORMANCE.md)** - Optimization and monitoring
- **[Containers](./CONTAINERS.md)** - Container deployment

#### For End Users
- **[Examples](./EXAMPLES.md)** - Practical usage examples
- **[CLI Reference](./CLI_REFERENCES.md)** - Command reference
- **[Troubleshooting](./TROUBLESHOOTING.md)** - Common issues
- **[FAQ](./FAQ.md)** - Frequently asked questions

### By Topic

#### Configuration and Setup
- Environment variables and config files
- API key management
- Permission modes
- Provider configuration

#### Usage and Features
- Interactive REPL usage
- Command-line interface
- Tool system
- Session management
- Plugin system

#### Integration and Extension
- MCP integration
- Custom plugins
- API usage
- External tool integration

#### Operations and Maintenance
- Performance optimization
- Security hardening
- Troubleshooting
- Monitoring and logging

## 🛠️ Common Workflows

### Development Workflow

```bash
# 1. Set up development environment
cargo build --workspace

# 2. Run tests
cargo test --workspace

# 3. Start development REPL
cargo run -p orbit-cli -- repl

# 4. Test changes
orbit prompt "test new feature"
```

### Automation Workflow

```bash
# 1. Configure for automation
orbit config set permission-mode danger-full-access
orbit config set output-format json

# 2. Create automation script
orbit prompt "analyze codebase" > analysis.json

# 3. Process results
jq '.result' analysis.json
```

### Plugin Development Workflow

```bash
# 1. Create plugin
orbit plugin init --type tool --name my-plugin

# 2. Implement plugin
cd my-plugin
# ... implement plugin ...

# 3. Test plugin
orbit plugin install .
orbit plugin test my-plugin

# 4. Publish plugin
orbit plugin publish
```

## 🔍 Navigation Tips

### Finding Information

1. **Start with FAQ** - Check [FAQ.md](./FAQ.md) for common questions
2. **Use Examples** - See [Examples.md](./EXAMPLES.md) for practical usage
3. **Reference Docs** - Use [CLI Reference](./CLI_REFERENCES.md) for command details
4. **Troubleshoot** - Check [Troubleshooting.md](./TROUBLESHOOTING.md) for issues

### Documentation Conventions

- **Code blocks** show command examples and configuration
- **Tables** provide reference information and comparisons
- **Notes** highlight important information and tips
- **Warnings** indicate potential issues or security concerns

### Interactive Learning

```bash
# Built-in help system
orbit --help
orbit help <command>
/help          # In REPL

# Interactive tutorials
orbit tutorial start
orbit tutorial list
```

## 🤝 Contributing to Documentation

### How to Contribute

1. **Fork the repository**
2. **Create a documentation branch**
3. **Make your changes**
4. **Test your changes**
5. **Submit a pull request**

### Documentation Guidelines

- **Be accurate** - Verify all commands and examples
- **Be comprehensive** - Cover edge cases and common scenarios
- **Be consistent** - Follow established formatting and style
- **Be helpful** - Focus on user needs and practical information

### Areas Needing Contributions

- **More examples** - Real-world usage scenarios
- **Translations** - Documentation in other languages
- **Videos** - Tutorial videos and screencasts
- **Templates** - Configuration and workflow templates

## 📞 Getting Help

### Built-in Help

```bash
# General help
orbit --help

# Command-specific help
orbit help prompt
orbit help repl

# In-REPL help
/help
/doctor
```

### Community Support

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Community discussions and Q&A
- **Documentation Issues** - Report documentation problems

### Professional Support

For enterprise support and consulting, visit the Orbit website or contact the team directly.

## 🗺️ Roadmap

### Documentation Roadmap

- **Interactive tutorials** - Step-by-step guided learning
- **Video content** - Screencasts and tutorials
- **API explorer** - Interactive API documentation
- **Template gallery** - Configuration and workflow templates
- **Community examples** - User-contributed examples and use cases

### Feature Documentation

- **New providers** - Documentation for upcoming AI providers
- **Advanced MCP** - Deep dive into MCP integration
- **Enterprise features** - Documentation for enterprise deployments
- **Performance tuning** - Advanced optimization guides

## 📄 License

This documentation is licensed under the same MIT license as the Orbit project. See the [LICENSE](../LICENSE.md) file for details.

## 🙏 Acknowledgments

Thanks to all contributors who have helped build and improve this documentation. Special thanks to the community for feedback, examples, and contributions.

---

**Ready to get started?** Jump to the [Quick Start](../README.md#quick-start) guide or explore the [Examples](./EXAMPLES.md) to see Orbit in action!

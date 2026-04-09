# Orbit CLI

Main command-line interface for the Orbit ecosystem.

## Overview

The Orbit CLI is the primary user interface for interacting with the Orbit AI assistant system. It provides interactive REPL mode, one-shot command execution, and comprehensive tool integration for AI-powered development workflows.

## Features

- Interactive REPL with readline support
- One-shot command execution
- Markdown rendering and syntax highlighting
- Plugin system integration
- Compatibility harness for testing
- Rich terminal interface with crossterm
- Command registry and execution

## Key Components

- **Interactive Mode**: Full-featured REPL with command history and completion
- **Command Processing**: Unified command parsing and execution
- **Output Rendering**: Rich formatting for AI responses and code
- **Plugin Integration**: Extensible architecture via plugins
- **Compatibility Testing**: Built-in testing and validation tools

## Dependencies

- `orbit-api` for AI provider integration
- `orbit-commands` for command system
- `orbit-compat-harness` for testing
- `orbit-runtime` for core functionality
- `orbit-plugins` for extensibility
- `orbit-tools` for tool integration
- `crossterm` for terminal handling
- `rustyline` for readline functionality
- `pulldown-cmark` for markdown parsing
- `syntect` for syntax highlighting

## Usage

The main binary is named `orbit` and can be used in multiple ways:

```bash
# Interactive mode
orbit

# One-shot execution
orbit prompt "your question here"

# With specific provider
orbit --provider anthropic prompt "your question"
orbit --provider frontal prompt "your question"
```

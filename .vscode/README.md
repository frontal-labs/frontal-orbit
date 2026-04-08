# VS Code Configuration for Orbit

This directory contains VS Code configuration files optimized for developing Orbit.

## Files Overview

### **launch.json** - Debug Configurations
- **Debug Orbit CLI**: Main CLI debugging with environment variables
- **Debug Orbit REPL**: Interactive REPL debugging
- **Debug Orbit Prompt**: One-shot prompt debugging
- **Debug Mock Parity Harness**: Testing with mock service
- **Debug Mock Service**: Standalone mock service debugging

### **tasks.json** - Build & Test Tasks
- **cargo build debug**: Debug build
- **cargo build release**: Release build
- **cargo build mock service**: Mock service build
- **cargo test**: Run all tests
- **cargo test workspace**: Tests with output
- **cargo check**: Quick syntax check
- **cargo clippy**: Linting
- **cargo fmt**: Code formatting
- **cargo run orbit**: Run the CLI
- **orbit doctor**: Health check
- **mock parity harness**: Run parity tests
- **docker-compose up/down/logs**: Container management

### **settings.json** - Editor Configuration
- **Rust Analyzer**: Enhanced IDE support with clippy integration
- **File Exclusions**: Hide build artifacts and temporary files
- **Environment Variables**: Debug logging in terminals
- **Code Formatting**: Auto-format on save
- **Spell Check**: Project-specific word list
- **LLDB**: Native debugging configuration

### **extensions.json** - Recommended Extensions
- **rust-lang.rust-analyzer**: Rust language server
- **vadimcn.vscode-lldb**: Native debugging
- **ms-vscode.vscode-docker**: Docker integration
- **streetsidesoftware.code-spell-checker**: Spell checking
- **ms-vscode.vscode-precommit-hook**: Pre-commit integration

## Quick Start

1. **Install Extensions**: VS Code will prompt to install recommended extensions
2. **Build Project**: `Ctrl+Shift+P` > `Tasks: Run Task` > `cargo build debug`
3. **Debug**: `F5` or use `Debug Orbit CLI` configuration
4. **Test**: `Ctrl+Shift+P` > `Tasks: Run Task` > `cargo test`

## Key Features

### **Debugging**
- Native debugging with LLDB
- Environment variables automatically set
- Breakpoints in Rust code
- Stack traces and variable inspection

### **Code Intelligence**
- Full Rust Analyzer integration
- Real-time error checking
- Auto-completion and go-to-definition
- Inlay hints for types and parameters

### **Build Integration**
- One-key building (`Ctrl+Shift+B`)
- Test runner integration
- Problem matching for compiler errors
- Build task dependencies

### **Docker Support**
- Container management tasks
- Docker Compose integration
- Container debugging support

### **Code Quality**
- Clippy integration on save
- Auto-formatting with rustfmt
- Spell checking for documentation
- Pre-commit hook integration

## Troubleshooting

### **Debug Issues**
- Install CodeLLDB extension
- Ensure Rust toolchain is up to date
- Check that debug symbols are enabled

### **Performance**
- Exclude target directory from search
- Disable unused extensions
- Use workspace-specific settings

### **Docker**
- Install Docker extension
- Ensure Docker daemon is running
- Check docker-compose.yml syntax

## Customization

You can customize these configurations for your specific needs:

- **Environment Variables**: Add to `launch.json` environments
- **Build Flags**: Modify cargo commands in `tasks.json`
- **Editor Settings**: Adjust in `settings.json`
- **Extensions**: Add to `extensions.json`

## Notes

- Debug configurations assume LLDB extension is installed
- Some linting warnings may appear until all extensions are installed
- Docker tasks require Docker and Docker Compose
- Environment variables are set automatically for debugging

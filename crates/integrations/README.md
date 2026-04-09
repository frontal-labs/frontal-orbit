# Orbit Integrations

Integration modules, including MCP interoperability surfaces and Orbit-native IDE integration used by runtime and CLI.

## Current Status

This crate is active in the workspace and provides:
- MCP-related configuration, lifecycle, stdio management, and tool bridge functionality.
- IDE integration primitives for `/ide` target parsing, workspace-local config persistence, per-editor config wiring (`.vscode/orbit.json` / `.cursor/orbit.json` / `.antigravity/orbit.json` / `.windsurf/orbit.json`), `.vsix` packaging from `extensions/orbit-ide`, extension install via editor CLI, and editor launching.

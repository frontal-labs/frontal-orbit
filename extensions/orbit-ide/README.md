# Orbit IDE Extension

Local Orbit integration for VS Code and Cursor.

## Commands

- `Orbit: Start REPL`
- `Orbit: Ask Orbit`
- `Orbit: Ask About Selection`

## Settings

- `orbit.cliPath` (default: `orbit`)
- `orbit.defaultModel` (default: empty)

## `/ide` Integration

`/ide vscode`, `/ide cursor`, `/ide antigravity`, and `/ide windsurf` now:

- package this extension into `.orbit/extensions/orbit-ide-<version>.vsix`
- install it into the target editor via editor CLI (`--install-extension ... --force`)
- write editor integration config (`.vscode/orbit.json` or `.cursor/orbit.json`)
- launch the editor at the current workspace root

If install fails, `/ide` reports the exact install error while still attempting editor launch.

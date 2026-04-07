# CLI Reference

Reference for the `orbit` binary built from `crates/cli`.

## Build and run

```bash
cargo build --workspace
./target/debug/orbit --help
```

## Top-level usage patterns

```text
orbit [--model MODEL] [--allowedTools TOOL[,TOOL...]]
orbit [--model MODEL] [--output-format text|json] prompt TEXT
orbit [--model MODEL] [--output-format text|json] TEXT
orbit --resume [SESSION.jsonl|session-id|latest] [/status] [/compact] [...]
```

## Common commands

```bash
./target/debug/orbit status
./target/debug/orbit sandbox
./target/debug/orbit doctor
./target/debug/orbit agents
./target/debug/orbit mcp
./target/debug/orbit skills
./target/debug/orbit login
./target/debug/orbit logout
./target/debug/orbit init
```

## Key flags

- `--model MODEL` - override the active model.
- `--provider PROVIDER` - force provider (`anthropic`, `openai`, `xai`, `ollama`).
- `--output-format text|json` - non-interactive output shape.
- `--permission-mode MODE` - `read-only`, `workspace-write`, or `danger-full-access`.
- `--dangerously-skip-permissions` - bypass permission checks.
- `--allowedTools TOOLS` - restrict enabled tools.
- `--version` / `-V` - print version/build information.

## Session and resume

REPL sessions are persisted under `.orbit/sessions/`.

```bash
./target/debug/orbit --resume latest
./target/debug/orbit --resume latest /status /diff /export notes.txt
```

## High-frequency slash commands

- `/help`, `/status`, `/sandbox`, `/cost`, `/usage`, `/stats`
- `/config`, `/memory`, `/diff`, `/files`, `/context`
- `/agents`, `/mcp`, `/skills`, `/doctor`
- `/plugin`, `/hooks`, `/subagent`, `/review`

Use `./target/debug/orbit --help` for the canonical full command list.

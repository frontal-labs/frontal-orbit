# Orbit API

HTTP API service crate for Orbit, plus the existing provider API re-exports.

## What It Provides

- Library surface: re-exports `orbit-providers` (`pub use orbit_providers::*;`)
- Binary: `orbit-api` HTTP server for CLI-compatible operations

## Run

```bash
cargo run -p orbit-api --bin orbit-api
```

By default it binds to `127.0.0.1:8787`.

## Environment Variables

- `ORBIT_API_HOST` (default: `127.0.0.1`)
- `ORBIT_API_PORT` (default: `8787`)
- `ORBIT_CLI_BIN` (optional path to `orbit` binary)
- `ORBIT_API_WORKDIR` (optional working directory for executed CLI commands)
- `ORBIT_API_KEY` (optional API key; accepts `x-api-key` or `Authorization: Bearer ...`)
- `ORBIT_API_ALLOWED_COMMANDS` (optional comma-separated allowlist for `/v1/cli/run`)
- `ORBIT_API_COMMAND_TIMEOUT_MS` (default: `120000`)

## REST Endpoints

- `GET /health`
- `POST /v1/cli/run` - generic CLI execution (`args` array)
- `POST /v1/prompt` - prompt request with model/provider/options
- `GET /v1/status`
- `GET /v1/sandbox`
- `GET /v1/version`

All command endpoints run the CLI with JSON output by default and return:
- command args
- exit code and success flag
- stdout/stderr
- parsed `json` payload when stdout is valid JSON

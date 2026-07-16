# Frontal Provider

The Frontal provider is the **default AI provider** for Orbit. It is an
OpenAI-compatible API served at `https://ai.frontal.dev/v1`.

## Status

- **Current Status**: Implemented and available
- **Availability**: Available at `https://ai.frontal.dev/v1`
- **Provider Name**: `frontal`

## Configuration

The Frontal provider is enabled by default (`runtime.default_provider = "frontal"`).
To use it, set your credentials via environment variables:

```bash
export FRONTAL_API_KEY="frontal-..."
export FRONTAL_BASE_URL="https://ai.frontal.dev/v1"
```

If `FRONTAL_BASE_URL` is not set, Orbit defaults to `https://ai.frontal.dev/v1`.

## Usage

When no `--provider` flag is given, Orbit routes requests through the Frontal
gateway. The model name configured in `runtime.providers.frontal.default_model`
(or `DEFAULT_MODEL` when invoked without `--model`) is passed to the gateway,
which maps it to the underlying model.

Explicit selection still works:

```bash
orbit --provider frontal -p "hello"
orbit --model frontal/gpt-4.1 -p "hello"
```

To use a different provider instead, pass `--provider <name>`
(`anthropic`, `openai`, `xai`, `bedrock`, `azure`, `ollama`).

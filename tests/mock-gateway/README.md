# Orbit Mock Gateway

A provider-agnostic mock of the **Frontal AI Gateway** (OpenAI-compatible Chat
Completions API) used for local development and CLI parity tests.

## Why it replaced the mock Anthropic service

The previous `mock-anthropic-service` crate spoke the Anthropic `/v1/messages`
wire format. Orbit now defaults to the **Frontal AI Gateway**, which exposes an
**OpenAI-compatible** `/v1/chat/completions` contract. This crate is the
redesign:

- Speaks OpenAI Chat Completions (JSON + SSE `data:` streaming) so it works for
  **any provider** routed through the gateway (`anthropic`, `openai`, `xai`,
  `frontal`, ...).
- Reflects the `model` field from each request, so the same harness exercises
  multiple providers by changing the model/provider selection.
- Default model is `claude-4-8` (see `DEFAULT_MODEL`).

## Usage

```bash
cargo run -p orbit-mock-gateway -- --bind 127.0.0.1:0
```

The server prints `MOCK_GATEWAY_BASE_URL=http://127.0.0.1:<port>`, which you
point at the gateway via `FRONTAL_BASE_URL` (or `OPENAI_BASE_URL`, etc.).

## Protocol

- `POST /v1/chat/completions`
- Body: `{ "model", "messages", "tools", "tool_choice", "stream" }`
- Streaming: SSE frames `data: {chat.completion.chunk}` terminating with
  `data: [DONE]`.
- Non-streaming: `{ "id", "object": "chat.completion", "model", "choices", "usage" }`.

Scenarios are selected by embedding a `PARITY_SCENARIO:<name>` token in the
user prompt (see `crates/cli/tests/mock_parity_harness.rs`).

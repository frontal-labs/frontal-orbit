# Orbit Embeddings

Embedding primitives used by semantic memory and retrieval.

## Current Capabilities

- Local embedding provider (`LocalMlEmbeddingProvider`)
- Embedding model configuration (`EmbeddingModelConfig`)
- Embedding model metadata (`EmbeddingModelInfo`)
- Fallible provider APIs with explicit errors (`EmbeddingError`)
- Cosine similarity and top-k ranking helpers
- Unit tests for vector shape and similarity behavior

## API Notes

- Compatibility path:
  - Existing callers can continue using `EmbeddingProvider::embed` and
    `EmbeddingProvider::embed_batch`.
- Error-aware path:
  - New callers can use `try_embed`, `try_embed_batch`, `embed_checked`, and
    `embed_batch_checked` to surface provider failures and dimension mismatches.

## Usage

This crate is consumed by `orbit-memory` and tool integrations for semantic search.

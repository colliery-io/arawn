---
id: arawn-embed-crate-embedder-trait
level: task
title: "arawn-embed crate — Embedder trait, config, LocalEmbedder (ONNX), ApiEmbedder"
short_code: "ARAWN-T-0093"
created_at: 2026-04-05T14:40:50.957819+00:00
updated_at: 2026-04-05T15:06:23.389753+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# arawn-embed crate — Embedder trait, config, LocalEmbedder (ONNX), ApiEmbedder

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective

New standalone `arawn-embed` crate providing a configurable embedding provider system. Reusable by memory, search, and any future subsystem needing semantic similarity.

### Type: Feature | Priority: P1 | Effort: M

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Embedder` async trait: `embed(text) -> Vec<f32>`, `embed_batch(texts) -> Vec<Vec<f32>>`, `dimensions() -> usize`
- [ ] `LocalEmbedder` using ONNX runtime (port from backup's vendored `orp-vendored` crate or use `ort` crate directly)
- [ ] `ApiEmbedder` calling OpenAI-compatible embedding endpoints (reqwest)
- [ ] `EmbeddingConfig` parsed from `arawn.toml` `[embeddings]` section: provider, model, dimensions, api_key_env
- [ ] Default config: `provider = "local"`, `model = "all-MiniLM-L6-v2"`, `dimensions = 384`
- [ ] Factory function: `create_embedder(config) -> Arc<dyn Embedder>`
- [ ] Crate added to workspace, usable as dependency from other crates
- [ ] Unit tests: config parsing, local embedder produces correct dimension vectors, batch embedding

## Implementation Notes

- Port or vendor the ONNX model loading from backup's `orp-vendored` crate, or use the `ort` crate (ORT = ONNX Runtime) directly which is more maintained
- Model file (`all-MiniLM-L6-v2.onnx`) needs to be bundled or downloaded on first use
- `ApiEmbedder` follows OpenAI's `/v1/embeddings` API shape — works with OpenAI, Azure, and compatible providers
- The trait must be `Send + Sync` for use in async contexts across the engine

## Status Updates **[REQUIRED]**

### 2026-04-05
- Created `arawn-embed` crate with Embedder trait, EmbeddingConfig, ApiEmbedder, LocalEmbedder
- LocalEmbedder uses ort 2.0.0-rc.12 with Tensor::from_array tuple form (proven pattern from backup)
- Downloads model files from HuggingFace on first use (~/.arawn/models/)
- ApiEmbedder calls OpenAI-compatible /v1/embeddings endpoint
- Config parsed from [embeddings] section with serde defaults
- 7 tests passing, workspace compiles clean
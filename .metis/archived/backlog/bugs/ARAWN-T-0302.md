---
id: config-endpoint-memory-enabled
level: task
title: "Config endpoint memory_enabled checks indexer instead of memory_store"
short_code: "ARAWN-T-0302"
created_at: 2026-03-09T13:27:19.797946+00:00
updated_at: 2026-03-09T14:13:43.229388+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Config endpoint memory_enabled checks indexer instead of memory_store

## Objective

The `GET /api/v1/config` endpoint reports `memory_enabled: false` even when the server has a working memory store, because it checks `state.indexer().is_some()` (embedding model) instead of `state.memory_store().is_some()`. This is misleading — API consumers see memory as "disabled" when the store/search/notes endpoints all work fine, just without semantic embedding search.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: All REST API consumers checking feature availability via `/api/v1/config`
- **Reproduction Steps**:
  1. Start server with memory store but no embedding model configured
  2. `GET /api/v1/config`
  3. Observe `features.memory_enabled` is `false`
  4. `POST /api/v1/memory` with content — succeeds (201)
  5. `GET /api/v1/memory/search?q=test` — succeeds (200)
- **Expected vs Actual**: Expected `memory_enabled: true` when memory store is available. Actual: `memory_enabled: false` because the check is `state.indexer().is_some()` which requires an embedding model.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `memory_enabled` returns `true` when the memory store is available (regardless of embedding/indexer)
- [ ] Consider adding a separate `semantic_search_enabled` or `embeddings_enabled` flag if the indexer distinction matters to API consumers
- [ ] Update `scenario_config_reflects_server_state` E2E test to verify correct behavior

## Implementation Notes

### Technical Approach

In `crates/arawn-server/src/routes/config.rs` line 82, change:
```rust
memory_enabled: state.indexer().is_some(),
```
to:
```rust
memory_enabled: state.services.memory_store.is_some(),
```

Optionally add a new field:
```rust
embeddings_enabled: state.indexer().is_some(),
```

### Key Files
- `crates/arawn-server/src/routes/config.rs:82` — the incorrect check
- `crates/arawn-server/tests/e2e_scenarios.rs` — update `scenario_config_reflects_server_state` test

## Status Updates

### Fix Applied
- **`config.rs:82`**: Changed `memory_enabled` from `state.indexer().is_some()` to `state.memory_store().is_some()`
- **New field**: Added `embeddings_enabled: bool` to `ConfigFeatures` struct, backed by `state.indexer().is_some()`
- **Unit test**: Updated `test_get_config` to assert `embeddings_enabled` is `false`
- **E2E test**: Updated `scenario_config_reflects_server_state` — `memory_enabled` now correctly `true` (test servers have memory store by default), `embeddings_enabled` correctly `false` (no embedding model in tests)
- **All 70 E2E tests pass**, 20 unit tests pass, clippy clean
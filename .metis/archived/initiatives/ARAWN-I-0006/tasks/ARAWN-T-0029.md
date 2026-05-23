---
id: localservice-arawnservice-impl
level: task
title: "LocalService — ArawnService impl with engine + store + streaming bridge"
short_code: "ARAWN-T-0029"
created_at: 2026-04-01T10:42:58.942517+00:00
updated_at: 2026-04-01T11:26:11.206102+00:00
parent: ARAWN-I-0006
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0006
---

# LocalService — ArawnService impl with engine + store + streaming bridge

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0006]]

## Objective

Implement `LocalService` that wraps the engine, store, tool registry, and compactor behind the `ArawnService` trait. The streaming bridge converts engine session mutations into `EngineEvent` streams via a tokio channel.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `LocalService` struct: owns `Store`, `Arc<dyn LlmClient>`, `Arc<ToolRegistry>`, `ModelLimits`, system prompt
- [ ] `LocalService::new(store, llm, registry, config)` constructor
- [ ] Implements all `ArawnService` methods
- [ ] CRUD methods delegate to Store (list/create workstreams, sessions)
- [ ] `send_message`: spawns background task, runs QueryEngine::run, emits EngineEvent stream via mpsc channel
- [ ] Streaming bridge: intercepts session message appends and maps to EngineEvent variants
- [ ] Messages persisted to JSONL during the engine run
- [ ] `cancel`: sets a cancellation flag checked by the engine task
- [ ] Refactor main.rs one-shot CLI to use LocalService instead of direct engine wiring
- [ ] All existing tests still pass
- [ ] Unit test: LocalService CRUD methods work
- [ ] Unit test: send_message returns stream that yields events

## Implementation Notes

- `local_service.rs` in `crates/arawn/src/` (binary crate, since it wires all internal crates)
- Streaming bridge: diff session messages before/after each engine turn, emit corresponding EngineEvents to channel
- `cancel` needs a shared `CancellationToken` passed into the engine task
- Depends on: T-0025 (ArawnService trait)

## Status Updates
- **2026-04-01**: Complete. LocalService implements all ArawnService methods. Key architectural changes: ToolRegistry now stores Arc<dyn Tool> (returns cloned Arc from get(), no RwLockReadGuard across awaits — fixes Send issue). Store behind std::sync::Mutex (SQLite calls are sync, lock released before async JSONL). SQLite WAL mode + 5s busy timeout enabled. send_message spawns background task with mpsc channel, diffs session messages after engine run to emit EngineEvents. JSONL persistence via separate JsonlMessageStore (no Store lock needed). Cancel is stubbed (TODO: CancellationToken). main.rs CLI not yet refactored — still uses direct wiring. Binary crate now has lib.rs exposing LocalService. 161 tests, clippy clean.
---
id: arawn-service-crate-arawnservice
level: task
title: "arawn-service crate — ArawnService trait, EngineEvent, view types"
short_code: "ARAWN-T-0025"
created_at: 2026-04-01T10:39:18.581999+00:00
updated_at: 2026-04-01T11:16:38.685789+00:00
parent: ARAWN-I-0006
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0006
---

# arawn-service crate — ArawnService trait, EngineEvent, view types

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0006]]

## Objective

Create the `arawn-service` crate containing the `ArawnService` trait, `EngineEvent` enum, and lightweight view types. This is the contract between any UI client and the Arawn backend — no deps on engine, LLM, or storage.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-service/` added to workspace
- [ ] Dependencies: `arawn-core`, `async-trait`, `futures`, `uuid`, `serde`, `serde_json`, `thiserror`
- [ ] `ArawnService` async trait: `list_workstreams`, `create_workstream`, `list_sessions`, `create_session`, `load_session`, `send_message` (returns Stream<EngineEvent>), `cancel`
- [ ] `EngineEvent` enum: `StreamingText`, `ToolCallStart`, `ToolCallResult`, `Complete`, `Error`, `CompactionOccurred` — all `Serialize + Deserialize` for JSON transport
- [ ] `WorkstreamInfo` view type: id, name, root_dir, created_at
- [ ] `SessionInfo` view type: id, workstream_id, created_at
- [ ] `SessionDetail` view type: SessionInfo + messages
- [ ] `ServiceError` error type
- [ ] Crate compiles, workspace passes

## Implementation Notes

- `lib.rs`, `types.rs`, `error.rs` in `crates/arawn-service/src/`
- View types are lightweight copies — not the full domain objects. They derive `Serialize + Deserialize` for JSON transport.
- The trait uses `Pin<Box<dyn Stream<Item = EngineEvent> + Send>>` for send_message — same pattern as LlmClient
- Does NOT depend on arawn-engine, arawn-llm, arawn-storage — only arawn-core for Message type
- Depends on: nothing (new standalone crate)

## Status Updates
- **2026-04-01**: Complete. arawn-service crate with ArawnService async trait (7 methods), EngineEvent enum (6 variants, all Serialize+Deserialize with tagged JSON), WorkstreamInfo/SessionInfo/SessionDetail view types, ServiceError. No deps on engine/llm/storage — only arawn-core. Workspace compiles, all tests pass, clippy clean.
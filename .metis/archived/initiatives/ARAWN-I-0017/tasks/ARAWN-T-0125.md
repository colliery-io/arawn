---
id: session-resumption-metadata-on
level: task
title: "Session resumption metadata on MaxIterations — include session ID and turn count in error for caller restart"
short_code: "ARAWN-T-0125"
created_at: 2026-04-09T16:03:01.652503+00:00
updated_at: 2026-04-09T16:13:14.895151+00:00
parent: ARAWN-I-0017
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0017
---

# Session resumption metadata on MaxIterations — include session ID and turn count in error for caller restart

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0017]]

## Objective

When the engine hits `max_iterations`, ensure the session is fully persisted and the error includes resumption metadata (session ID, turn count) so a co-ordinating caller can restart the agent from where it left off.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `EngineError::MaxIterations` variant includes session_id (Uuid) and iteration count
- [ ] The error's user-facing message includes the session ID and a hint like "Resume with session {id}"
- [ ] Session JSONL is confirmed flushed before the error is returned (verify existing persistence is synchronous per-message)
- [ ] `arawn.toml` default for `max_iterations` documented as 200
- [ ] Integration test: engine hits max_iterations, session is loadable, caller can send a new message to the same session

### Key files
- `crates/arawn-engine/src/error.rs` — `MaxIterations` variant
- `crates/arawn-engine/src/query_engine.rs` — error return site
- `crates/arawn/src/ws_server.rs` — surface resumption info in the WS error response

## Status Updates

*To be added during implementation*
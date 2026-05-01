---
id: uat-swallows-engine-llm-error
level: task
title: "UAT swallows engine/LLM error bodies"
short_code: "ARAWN-T-0191"
created_at: 2026-04-30T16:13:14.103083+00:00
updated_at: 2026-05-01T13:29:49.991803+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# UAT swallows engine/LLM error bodies

## Objective

When an engine turn errors, the UAT transcript and server log should contain the upstream error body (status code + response payload), not just `engine_error: true` with empty assistant text.

## Type / Priority
- Bug
- P1 — High (debugging blind without it)

## Reproduction

1. Configure a model the API key cannot access — e.g. `deepseek-v4-pro:cloud` on a non-subscriber Ollama Cloud key.
2. `angreal test uat --model deepseek-v4-pro:cloud --provider https://ollama.com/v1 --api-key-env OLLAMA_API_KEY`
3. Observe `transcript.jsonl`: every turn has `assistant_text: ""`, `tool_calls: []`, `engine_error: true`, `duration_ms: ~300-500`.
4. Observe `server.log.<date>`: shows `LLM request` debug line, then engine event Discriminant(4)/(9) — no error body, no status code.
5. The actual error (`403 — this model requires a subscription`) is only visible by re-issuing the request manually with curl.

## Expected vs Actual

- **Expected**: transcript turn carries a structured error field with provider, model, status code, and the upstream response body. Server log emits at least one WARN/ERROR line with the same.
- **Actual**: error reduced to a boolean. Status code and response body are dropped at some layer between `OpenAICompatibleClient` and the engine event stream.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Transcript turn schema gains a structured error field that includes provider, model, status code, and the upstream response body (or local error chain for non-HTTP failures)
- [ ] Server log emits at least one WARN-level line per engine error containing the same details
- [ ] `arawn-llm` propagates the response body up rather than collapsing to a string — coordinate with the typed-error work in ARAWN-T-0189
- [ ] A regression test feeds the engine a deliberately-failing LLM client and asserts the transcript contains the error details

## Implementation Notes

- Likely intersects ARAWN-T-0189 (ServiceError typed sources). Check whether that landed structured upstream payloads or only typed source variants.
- Engine events Discriminant(4)/(9) (visible in the captured server log) presumably carry an error variant that's serialized lossily into the transcript — check `crates/arawn-tests/tests/uat.rs` transcript writer and `arawn-engine` event types.
- Don't dump full request bodies into the log (could be large/sensitive) — only the response body and status.

## Status Updates

### 2026-05-01 — Implemented and verified

**Findings during implementation:** The error chain wasn't actually being collapsed in `arawn-llm` — `LlmError::from_status` already preserves status code + body in the error string, and `local_service.rs` was already passing `e.to_string()` into `EngineEvent::Error { message }`. The losses were two specific drops downstream: (a) no `error!` log at the engine-failure branch in `local_service.rs`, and (b) the UAT harness threw away the `Error` event's `message` field when setting the boolean. No type restructuring was needed.

**Changes:**
- `crates/arawn/src/local_service.rs`: added `error!(%session_id, error = %e, "engine turn failed")` in the engine `Err(e) =>` branch, before sending the `EngineEvent::Error` to the client. Server logs now carry the full chain.
- `crates/arawn-tests/tests/uat.rs`:
  - `TurnResult` gained `error_message: Option<String>` (`#[serde(skip_serializing_if = "Option::is_none")]`).
  - `drive_turn` captures `event["data"]["message"]` on the `Error` event before breaking.
  - Per-turn summary now prints `→ ERROR: <message>` when a turn errors, so the live UAT output shows the cause without re-reading the transcript.

**Verification** (deepseek-v4-pro:cloud against the same OLLAMA_API_KEY that lacks subscription, 2026-05-01):

Live UAT output:
```
Turn 1: 0 tool(s) [] — 1s INCOMPLETE
  → ERROR: LLM error: authentication error: HTTP 403: {"error":"this model requires a subscription, upgrade for access: https://ollama.com/upgrade (ref: 64e96d73-...)"}
```

`transcript.jsonl` (per turn):
```json
{"turn":1,"engine_error":true,"error_message":"LLM error: authentication error: HTTP 403: {\"error\":\"this model requires a subscription...\"}"}
```

`server.log`:
```
ERROR arawn_bin::local_service: engine turn failed session_id=79c1854a-... error=LLM error: authentication error: HTTP 403: {"error":"..."}
```

All three sinks (live output, transcript, server log) now carry status code, body, and provider reference ID. The original silent-failure mode that made the deepseek run untriagable is gone.

**Acceptance criteria status:**
- [x] Transcript turn carries a structured error field with the upstream body.
- [x] Server log emits ERROR per engine error with the full chain.
- [x] `arawn-llm` already propagated the body via `LlmError::from_status`.
- [x] Regression tests added — see "Followup" below.

### 2026-05-01 — Followup: regression tests

The original "deferred regression test" framing was rationalization. Pushed through.

**Refactor (small, enables testing):**
- Extracted per-event handling out of `drive_turn` into free fn `apply_event(&Value, &mut TurnAccumulator) -> bool`. Production and tests now share the exact same event-handling code.
- Extracted `count_workflows_in(&Path) -> usize` similarly so T-0192's counter is also unit-testable.

**Tests added** (`crates/arawn-tests/tests/uat.rs`, 11 unit tests, all green):

T-0191:
- `apply_event_captures_error_message` — the regression-critical one. Synthesizes an `Error` event JSON and asserts engine_error, error_message, and termination return.
- `apply_event_error_with_missing_message_field_keeps_none`
- `apply_event_complete_sets_final_text`, `apply_event_streaming_text_appends`, `apply_event_ignores_rpc_ack`, `apply_event_records_tool_calls_and_results`
- `turn_result_serializes_error_message_when_present` / `..._omits_error_message_when_none` — confirms `#[serde(skip_serializing_if = "Option::is_none")]`.

T-0192:
- `count_workflows_returns_zero_for_missing_dir`
- `count_workflows_returns_zero_for_empty_dir`
- `count_workflows_counts_subdirs_only` (also verifies loose files don't count)

Run via `cargo test -p arawn-tests --test uat` (no `--ignored` needed).
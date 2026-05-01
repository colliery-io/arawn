---
id: extend-mockllmclient-with-request
level: task
title: "Extend MockLlmClient with request capture and TestHarness with plan mode and progress channel"
short_code: "ARAWN-T-0134"
created_at: 2026-04-09T16:57:04.426200+00:00
updated_at: 2026-04-09T17:16:33.843542+00:00
parent: ARAWN-I-0020
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0020
---

# Extend MockLlmClient with request capture and TestHarness with plan mode and progress channel

## Parent Initiative

[[ARAWN-I-0020]]

## Objective

Extend the test infrastructure so that tests can: (1) inspect what `ChatRequest` was sent to the LLM (request capture), (2) test plan mode enforcement on the engine, and (3) verify progress events are emitted correctly. These are prerequisites for several gap tests in ARAWN-T-0135, ARAWN-T-0136, and ARAWN-T-0138.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MockLlmClient` gains a `captured_requests: Arc<Mutex<Vec<ChatRequest>>>` field that records every request passed to `stream()`
- [ ] `MockLlmClient::captured_requests()` accessor returns the captured requests for assertion
- [ ] `TestHarnessBuilder::with_plan_state(active: bool)` method added — wires a `PlanModeState` into the engine
- [ ] `TestHarness` exposes `mock_llm` accessor so tests can inspect `call_count()` and `captured_requests()`
- [ ] `TestHarnessBuilder::with_progress_channel()` method added — returns a `tokio::sync::mpsc::Receiver<ProgressEvent>` for tests to consume
- [ ] Unit tests for request capture (verify captured request contains expected tools/messages)
- [ ] Existing tests still pass

## Implementation Notes

### Files to Modify
- `crates/arawn-llm/src/mock.rs` — add `captured_requests` field and accessor
- `crates/arawn-engine/src/testing.rs` — add builder methods, expose `mock_llm`

### Technical Approach
Request capture: clone the `ChatRequest` into the `captured_requests` vec at the top of `stream()`. The `TestHarness` already holds `mock_llm: Arc<MockLlmClient>`, just needs a public accessor.

Plan mode: `QueryEngine` already has `with_plan_state()`. Add corresponding builder method on `TestHarnessBuilder` that stores an `Option<Arc<PlanModeState>>` and wires it in `build()`.

Progress channel: `QueryEngine` already has `with_progress_tx()`. Create the channel in `build()` when requested, store the sender in the engine and return the receiver to the test.

### Dependencies
None — can be done in parallel with ARAWN-T-0133.

## Status Updates

- Added `captured_requests` field + `captured_requests()` accessor to `MockLlmClient` — captures every `ChatRequest` passed to `stream()`
- Added `mock_llm()` accessor to `TestHarness`
- Added `with_plan_active()` builder method — enters plan mode with temp dir as working directory
- Added `with_progress_channel()` builder method + `take_progress_rx()` on `TestHarness`
- Updated `build_engine()` to wire plan state and progress sender into `QueryEngine`
- All 11 existing testing.rs tests pass, no regressions
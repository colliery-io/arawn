---
id: llm-error-handling-and-retry-tests
level: task
title: "LLM error handling and retry tests: fatal errors, transient retries, mid-stream failures"
short_code: "ARAWN-T-0137"
created_at: 2026-04-09T16:57:08.690115+00:00
updated_at: 2026-04-09T17:22:24.714826+00:00
parent: ARAWN-I-0020
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0020
---

# LLM error handling and retry tests: fatal errors, transient retries, mid-stream failures

## Parent Initiative

[[ARAWN-I-0020]]

## Objective

Using the new `MockResponse::Error` and `MockResponse::StreamError` variants (from ARAWN-T-0133), add integration tests covering the engine's LLM error handling and retry logic. In production, LLM API failures happen regularly and the retry logic must work correctly.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Fatal LLM error (auth)**: `MockResponse::Error(LlmError::Auth(...))` — verify immediate `EngineError::Llm` propagation, no retry
- [ ] **Fatal LLM error (model not found)**: verify no retry, immediate propagation
- [ ] **Transient error + successful retry**: `[Error(RateLimited), Text("ok")]` — verify engine retries and returns "ok"
- [ ] **Transient error exhausting retries**: 3x `Error(RateLimited)` — verify gives up after max retries, propagates last error
- [ ] **Mid-stream error (text)**: `StreamError { chunks: [TextDelta("partial")], error }` — verify `EngineError::Llm`, partial text discarded
- [ ] **Mid-stream error (tool call)**: `StreamError { chunks: [ToolUseStart(...)], error }` — verify no partial tool executed
- [ ] **Transient pattern matching**: Verify errors with "rate", "overloaded", "529", "500" are transient; "auth" is not
- [ ] **Retry count verification**: Use `mock_llm.call_count()` to verify exact number of LLM calls
- [ ] All new tests pass

## Implementation Notes

### Files to Modify
- `crates/arawn-engine/src/testing.rs` — inline tests

### Key Engine Behavior
The retry logic in `stream_with_retry` attempts up to `MAX_LLM_RETRIES` (2) times. It checks `is_transient(&error_string)` for known transient patterns. Non-transient errors propagate immediately.

### Dependencies
**Blocked by ARAWN-T-0133** — requires `MockResponse::Error` and `MockResponse::StreamError`.

## Status Updates

- Added 7 new tests in `testing.rs`:
  - `harness_fatal_llm_error_no_retry` — Auth error, call_count=1, no retry
  - `harness_model_not_found_is_not_transient` — ModelNotFound, call_count=1
  - `harness_transient_error_then_success` — RateLimited then Text, call_count=2
  - `harness_transient_error_exhausts_retries` — 3x RateLimited, call_count=3, propagates last error
  - `harness_mid_stream_error_during_text` — StreamError with TextDelta then Api error, no partial tool
  - `harness_mid_stream_error_during_tool_call` — StreamError with ToolUseStart then Api error, no partial execution
  - `harness_server_error_is_transient` — ServerError(500) retried, recovers on second attempt
- Discovery: "connection" is a transient keyword, so `LlmError::Stream("connection reset")` triggers retries. Used non-transient `LlmError::Api` for mid-stream tests to avoid retry confusion.
- All 31 tests pass
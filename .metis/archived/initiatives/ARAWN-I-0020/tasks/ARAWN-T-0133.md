---
id: extend-mockllmclient-with-error
level: task
title: "Extend MockLlmClient with Error and StreamError response variants"
short_code: "ARAWN-T-0133"
created_at: 2026-04-09T16:57:02.724230+00:00
updated_at: 2026-04-09T17:13:38.659898+00:00
parent: ARAWN-I-0020
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0020
---

# Extend MockLlmClient with Error and StreamError response variants

## Parent Initiative

[[ARAWN-I-0020]]

## Objective

Add two new `MockResponse` variants to `crates/arawn-llm/src/mock.rs` that allow tests to simulate LLM-level failures — both immediate errors (auth, rate limit) and mid-stream errors (connection drops partway through a response). This is the prerequisite for all error handling and retry tests in ARAWN-T-0137.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MockResponse::Error(LlmError)` variant added — makes `stream()` return `Err(error)` instead of a stream
- [ ] `MockResponse::StreamError { chunks_before_error: Vec<ChatChunk>, error: LlmError }` variant added — yields Ok chunks then a final Err item in the stream
- [ ] `into_chunks()` refactored or `stream()` updated to handle the new variants correctly
- [ ] Unit tests for both new variants in `mock.rs` inline tests
- [ ] `MockResponse::Error` test: verify `stream()` returns `Err` with correct error type
- [ ] `MockResponse::StreamError` test: verify stream yields N Ok chunks then Err
- [ ] Existing tests still pass — no regressions

## Implementation Notes

### Files to Modify
- `crates/arawn-llm/src/mock.rs` — add variants, update `MockLlmClient::stream()`

### Technical Approach
The `Error` variant is straightforward: match on it in `stream()` and return `Err(error)` before creating the stream. The `StreamError` variant needs to produce a stream that yields `Ok(chunk)` for each chunk in `chunks_before_error`, then yields `Err(error)` as the final item. Use `futures::stream::iter` with a chain of Ok items followed by one Err item.

### Dependencies
None — this is a leaf task with no blockers.

## Status Updates

- Added `MockResponse::Error(LlmError)` and `MockResponse::StreamError { chunks_before_error, error }` variants
- Added `error()` and `stream_error()` constructors
- Updated `stream()` to handle new variants before delegating to `into_chunks()`
- Added 3 unit tests: immediate error, stream error with chunks then err, error-then-success retry simulation
- All 7 mock tests pass, no regressions
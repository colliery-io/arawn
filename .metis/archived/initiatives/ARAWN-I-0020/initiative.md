---
id: mockllm-test-coverage-happy-path
level: initiative
title: "MockLLM Test Coverage: Happy Path and Sad Path Completeness"
short_code: "ARAWN-I-0020"
created_at: 2026-04-09T16:55:56.559978+00:00
updated_at: 2026-04-16T12:32:16.084644+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: mockllm-test-coverage-happy-path
---

# MockLLM Test Coverage: Happy Path and Sad Path Completeness Initiative

## Context

A thorough audit of arawn's integration/e2e test suite revealed significant gaps in test coverage for LLM interaction patterns. While basic happy paths (text replies, single tool calls, multi-step chains) are well-covered, many critical engine behaviors have zero test coverage — particularly around error handling, edge cases in stream processing, and subsystem interactions.

The MockLLM infrastructure (`MockLlmClient` + `TestHarness`) is solid but needs two additions to unlock the highest-impact gap cluster: `MockResponse::Error` and `MockResponse::StreamError` variants for testing LLM-level failures and retry logic.

### Current Coverage Summary

**Well-covered:** text-only responses, single tool calls, multi-step chains, max iterations, basic permissions (allow/deny/ask), hook block/allow, skill invocation, session persistence, compaction triggers, memory stack rendering.

**Not covered:** parallel tool calls, mixed text+tool responses, malformed LLM output, retry logic, mid-stream errors, plan mode, repeated failure circuit breaker, progress events, most WebSocket RPC methods, cancel flow, multi-turn conversations through the service layer.

## Goals & Non-Goals

**Goals:**
- Achieve comprehensive happy-path coverage for all engine loop behaviors (parallel tools, mixed responses, streaming edge cases)
- Achieve comprehensive sad-path coverage for all error/recovery paths (malformed JSON, LLM errors, retries, circuit breakers)
- Extend MockLLM infrastructure to support error injection (`MockResponse::Error`, `MockResponse::StreamError`)
- Extend TestHarness to support plan mode, progress channels, and request capture
- Cover critical subsystem interactions (permissions+hooks, skills+permissions, denial→recovery loops)
- Fill WebSocket/service layer gaps for untested RPC methods

**Non-Goals:**
- Performance/load testing
- Testing against real LLM APIs
- UI/TUI rendering tests
- Plugin dylib loading tests (external dependency)

## Detailed Design

### Phase 1: Mock Infrastructure Extensions
Extend `MockLlmClient` and `MockResponse` in `crates/arawn-llm/src/mock.rs`:
- `MockResponse::Error(LlmError)` — makes `stream()` return `Err(...)` 
- `MockResponse::StreamError { chunks_before_error, error }` — yields Ok chunks then Err
- Request capture (`Arc<Mutex<Vec<ChatRequest>>>`) for inspecting what was sent to the LLM

Extend `TestHarness` in `crates/arawn-engine/src/testing.rs`:
- `with_plan_state()` for plan mode testing
- `with_progress_channel()` for progress event verification

### Phase 2: Engine Loop Gap Tests
Add tests in `crates/arawn-engine/src/testing.rs` (inline) and `crates/arawn-tests/tests/`:
- Parallel tool calls (multiple ToolUseStart before Done)
- Mixed text + tool call in same turn
- Malformed JSON arguments (fallback to `{}`)
- Non-object JSON arguments (rejected)
- Empty text/stream responses
- Stream without Done chunk (flush path)
- Repeated failing tool call circuit breaker
- Usage/token stats accumulation
- Progress event emission

### Phase 3: Error Handling & Retry Tests
Using new mock error variants:
- Fatal LLM error (auth, model not found) — no retry, immediate propagation
- Transient error + successful retry (rate limit → success)
- Transient error exhausting retries (3x rate limit → give up)
- Mid-stream error (partial text then connection failure)
- Mid-stream error during tool call assembly

### Phase 4: Subsystem Interaction Tests
- Permission denial → LLM recovery (denied tool → tries alternative → succeeds)
- Hook + permission interaction (ordering, conflicts)
- Plan mode blocks write tools, allows read-only
- Compaction failure circuit breaker (3 failures → skip)
- Hook stderr propagation to ToolResult

### Phase 5: Service Layer Gap Tests
- WebSocket: untested RPC methods (cancel, list_sessions, promote_session, etc.)
- LocalService: multi-turn conversation, cancel, engine error propagation
- Invalid/missing parameters for WS RPCs

## Implementation Plan

1. **Mock infrastructure** (Phase 1) — prerequisite for Phase 3
2. **Engine loop tests** (Phase 2) — highest value, mostly testable now
3. **Error/retry tests** (Phase 3) — depends on Phase 1
4. **Subsystem interaction tests** (Phase 4) — testable now
5. **Service layer tests** (Phase 5) — independent, can parallelize
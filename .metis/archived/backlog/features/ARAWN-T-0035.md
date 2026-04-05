---
id: error-classification-retryable-vs
level: task
title: "Error classification — retryable vs terminal, user-facing error messages"
short_code: "ARAWN-T-0035"
created_at: 2026-04-01T11:01:58.549660+00:00
updated_at: 2026-04-02T13:49:41.277761+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Error classification — user-facing error messages with actionable guidance

## Objective

Translate raw technical errors into user-facing messages with actionable guidance. Currently errors surface as `"Error: LLM error: API error: HTTP 401: {json blob}"` — users need to see `"Invalid API key — check that GROQ_API_KEY is set correctly"` instead.

`LlmError` already has `is_retryable()` and `from_status()` (T-0032). This task adds `user_message()` methods across the error stack and improves classification with HTTP-status-aware error construction.

### Priority
- P1 — critical for UX, especially for new users hitting config issues

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `LlmError::user_message()` returns actionable guidance for each variant (auth, rate limit, model not found, network, etc.)
- [ ] `LlmError::from_status()` detects 401 (auth), 403 (forbidden), 404 (model not found), 429 (rate limit), 5xx (server) with appropriate messages
- [ ] `EngineError::user_message()` wraps LLM errors and adds engine-specific guidance (max iterations, tool errors)
- [ ] CLI uses `user_message()` for display instead of raw Debug/Display
- [ ] ServiceError propagates user messages to TUI/WS clients
- [ ] Tests: each HTTP status maps to correct variant and user message

## Implementation Notes

### Current State
- `LlmError`: Api, RateLimited, ServerError, Stream, Config, Request, Json — `is_retryable()` exists
- `EngineError`: Tool, ToolNotFound, Llm, MaxIterations, Other — no user messages
- `ServiceError`: NotFound, InvalidOperation, Engine, Storage, Internal — string wrappers
- CLI: `eprintln!("Error: {e}")` — shows raw thiserror Display

### Approach
1. Add `user_message() -> String` to `LlmError` — match on variant, return guidance
2. Improve `from_status()` to parse common API error bodies (Groq returns `{"error": {"message": ...}}`)
3. Add new `LlmError` variants: `AuthError`, `ModelNotFound` for 401/403/404
4. Add `user_message()` to `EngineError`
5. CLI: `eprintln!("{}", e.user_message())` instead of `eprintln!("Error: {e}")`

## Status Updates

*To be added during implementation*
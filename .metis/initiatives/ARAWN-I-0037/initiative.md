---
id: deduplicate-llm-backend-code
level: initiative
title: "Deduplicate LLM Backend Code"
short_code: "ARAWN-I-0037"
created_at: 2026-03-22T23:50:10.547262+00:00
updated_at: 2026-03-22T23:50:10.547262+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: deduplicate-llm-backend-code
---

# Deduplicate LLM Backend Code

## Context

`arawn-llm/src/openai.rs` (1,594 lines) and `arawn-llm/src/anthropic.rs` (1,199 lines) contain parallel implementations of streaming, retry logic, rate limit handling, tool call marshalling, and error mapping. While the API protocols differ (OpenAI uses SSE with `data:` lines; Anthropic uses its own event stream format), significant logic is duplicated: HTTP client setup, retry with exponential backoff, token counting, response aggregation, and interaction logging.

Promoted from blocked task ARAWN-T-0379.

## Goals & Non-Goals

**Goals:**
- Extract ~240 lines of truly shared logic into a `common.rs` module (~15-17% of production code)
- Improve consistency of retry/rate-limit behavior across providers
- Make it easier to add new backends by providing shared primitives

**Non-Goals:**
- Unifying streaming protocols (fundamentally different: OpenAI SSE `data: [DONE]` vs Anthropic typed `event:` + `data:` pairs)
- Unifying request building (Anthropic sends `CompletionRequest` directly; OpenAI needs a 155-line translation layer)
- Reducing LOC by 30%+ (discovery showed overlap is smaller than initially estimated)

## Discovery Findings (from ARAWN-S-0002)

**Line breakdown** (production code only):
- `openai.rs`: ~833 prod + 761 test = 1,594 total
- `anthropic.rs`: ~604 prod + 595 test = 1,199 total

**Fundamental asymmetry**: Anthropic's `CompletionRequest` matches wire format via `Serialize` — sent directly. OpenAI requires `to_openai_request()` translation (155 lines). Request building CANNOT be unified.

## Detailed Design

### 5 clear extraction candidates (low risk):
1. **`build_http_client()`** — identical HTTP client construction in both files
2. **`ProviderErrorResponse` / `ProviderErrorDetail`** — identical error JSON shape
3. **`map_stop_reason()`** — near-duplicate match arms with different string constants (parameterize)
4. **`SseBuffer` with `next_line()`** — identical buffer/byte-stream reading loop
5. **`handle_error_response()`** skeleton — identical retry-after extraction + status code mapping

### 3 things to leave alone (high risk):
1. Request building (architecturally different)
2. SSE event parsing (different protocols)
3. `add_headers()` (different auth schemes)

### Target: Create `common.rs` with:
```
pub fn build_http_client(timeout: Duration) -> Result<Client>
pub struct ProviderErrorResponse { ... }
pub fn map_stop_reason(reason: &str, provider: Provider) -> StopReason
pub struct SseBuffer { ... }  // with next_line()
pub fn extract_retry_after(headers: &HeaderMap) -> Option<Duration>
pub fn map_status_to_error(status: StatusCode, body: &str) -> LlmError
```

## Implementation Plan

- Task 1: Create `common.rs` with `build_http_client` + `ProviderErrorResponse` (safest, most isolated)
- Task 2: Extract `SseBuffer` + `next_line()` into common
- Task 3: Extract `map_stop_reason` + `extract_retry_after` + `map_status_to_error`
- Task 4: Refactor both backends to use shared code, verify all LLM tests pass

Each task in a worktree with full test suite verification.
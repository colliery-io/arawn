---
id: deduplication-analysis-openai-rs
level: specification
title: "Deduplication Analysis: openai.rs vs anthropic.rs"
short_code: "ARAWN-S-0002"
created_at: 2026-03-23T02:17:52.200606+00:00
updated_at: 2026-03-23T02:17:52.200606+00:00
parent: ARAWN-I-0037
blocked_by: []
archived: true

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Deduplication Analysis: openai.rs vs anthropic.rs

## Overview

This document provides a detailed structural analysis of `crates/arawn-llm/src/openai.rs` (1,594 lines) and `crates/arawn-llm/src/anthropic.rs` (1,199 lines) to identify shared, similar, and unique code for deduplication under initiative ARAWN-I-0037.

---

## 1. File Structure Summary

### openai.rs (1,594 lines)

| Section | Lines | Range |
|---------|-------|-------|
| Imports + constants | 31 | 1-31 |
| `OpenAiConfig` struct + builder methods | 137 | 37-168 |
| `OpenAiBackend` struct + inherent methods | 264 | 174-432 |
| `LlmBackend` trait impl | 59 | 434-493 |
| `create_shared_backend()` | 3 | 496-498 |
| Wire types (request/response/error structs) | 165 | 504-668 |
| `From<OpenAiChatResponse> for CompletionResponse` | 63 | 576-639 |
| SSE streaming (`parse_openai_sse_stream`) | 128 | 673-833 |
| Tests | 761 | 839-1594 |

### anthropic.rs (1,199 lines)

| Section | Lines | Range |
|---------|-------|-------|
| Imports + constants | 31 | 1-31 |
| `AnthropicConfig` struct + builder methods | 85 | 51-116 |
| `AnthropicBackend` struct + inherent methods | 91 | 122-208 |
| `LlmBackend` trait impl | 55 | 210-259 |
| `create_shared_backend()` | 3 | 262-264 |
| Wire types (response/error/content structs) | 92 | 270-356 |
| `From<ApiResponse> for CompletionResponse` | 42 | 282-324 |
| SSE streaming (`parse_sse_stream` + helpers) | 242 | 362-597 |
| Tests | 595 | 603-1199 |

---

## 2. Function-by-Function Analysis

### 2.1 Config Structs

| Aspect | OpenAiConfig | AnthropicConfig | Category |
|--------|-------------|----------------|----------|
| Fields: `api_key`, `base_url`, `timeout`, `max_retries`, `retry_backoff` | Yes | Yes | **Shared** |
| Field: `model: Option<String>` | Yes | No | **Unique** (OpenAI) |
| Field: `name: String` | Yes | No | **Unique** (OpenAI) |
| Field: `api_version: String` | No | Yes | **Unique** (Anthropic) |
| `with_base_url()` | Yes | Yes | **Shared** (identical) |
| `with_timeout()` | Yes | Yes | **Shared** (identical) |
| `with_max_retries()` | Yes | Yes | **Shared** (identical) |
| `with_retry_backoff()` | Yes | Yes | **Shared** (identical) |
| `with_model()` | Yes | No | **Unique** (OpenAI) |
| `with_name()` | Yes | No | **Unique** (OpenAI) |
| `openai()` / `groq()` / `ollama()` | Yes | No | **Unique** (OpenAI) |
| `new()` / `from_env()` | N/A | Yes | **Unique** (Anthropic) |

### 2.2 Backend Structs

| Function | OpenAiBackend | AnthropicBackend | Category |
|----------|--------------|-----------------|----------|
| `new(config) -> Result<Self>` | `Client::builder().timeout().build()` + store | Identical logic | **Shared** |
| `from_env()` factory methods | `openai_from_env()`, `groq_from_env()`, `ollama()` | `from_env()` | **Similar** (pattern same, details differ) |
| Endpoint URL builder | `completions_url()` -> `{base}/chat/completions` | `messages_url()` -> `{base}/v1/messages` | **Similar** (pattern same, URL differs) |
| `add_headers()` | Returns `RequestBuilder`, adds `Bearer` auth, no error return | Returns `Result<RequestBuilder>`, adds `x-api-key` + `anthropic-version`, errors on missing key | **Similar** (same pattern, different auth scheme) |
| `handle_response()` | Check status, parse body to `OpenAiChatResponse`, convert | Check status, parse body to `ApiResponse`, convert | **Similar** (identical pattern, different wire type) |
| `handle_error_response()` | Extract `retry-after` header, parse error JSON, match on status code (401/429/500+) | Identical pattern | **Shared** (nearly identical, ~90% same) |
| `to_openai_request()` | Converts `CompletionRequest` to OpenAI wire format | N/A (Anthropic sends `CompletionRequest` directly via serde) | **Unique** (OpenAI) |

### 2.3 LlmBackend Trait Implementation

| Method | OpenAiBackend | AnthropicBackend | Category |
|--------|--------------|-----------------|----------|
| `complete()` | Set `stream=false`, build request, `with_retry(|| send + handle_response)` | Identical pattern | **Similar** (identical skeleton, different request building) |
| `complete_stream()` | Set `stream=true`, send, check status, call `parse_openai_sse_stream()` | Identical pattern calling `parse_sse_stream()` | **Similar** (identical skeleton) |
| `name()` | Returns `&self.config.name` | Returns `"anthropic"` hardcoded | **Similar** |
| `supports_native_tools()` | Returns `true` | Returns `true` | **Shared** (identical) |

### 2.4 Wire Types

| Type | OpenAI | Anthropic | Category |
|------|--------|-----------|----------|
| Request struct | `OpenAiChatRequest` (8 fields) | None (uses `CompletionRequest` directly) | **Unique** (OpenAI) |
| Message struct | `OpenAiMessage`, `OpenAiContent` | None | **Unique** (OpenAI) |
| Tool types | `OpenAiTool`, `OpenAiFunction`, `OpenAiToolCall`, `OpenAiFunctionCall` | None | **Unique** (OpenAI) |
| Response struct | `OpenAiChatResponse` | `ApiResponse` | **Unique** (different shapes) |
| Response content | `OpenAiChoice`, `OpenAiResponseMessage` | `ApiContentBlock` (tagged enum) | **Unique** |
| Usage | `OpenAiUsage` (2 fields) | `ApiUsage` (4 fields, with cache tokens) | **Unique** |
| Error types | `OpenAiErrorResponse`, `OpenAiError` | `ApiError`, `ApiErrorDetail` | **Similar** (same shape: `{error: {message}}`) |
| Response -> CompletionResponse | `From<OpenAiChatResponse>` | `From<ApiResponse>` | **Similar** (same goal, different mapping) |

### 2.5 SSE Streaming

| Component | OpenAI | Anthropic | Category |
|-----------|--------|-----------|----------|
| SSE state struct | `OpenAiSseState` (6 fields) | `SseState` (4 fields) | **Similar** |
| Buffer reading loop | Line-by-line from byte stream, `find('\n')` | Identical pattern | **Shared** (identical loop structure) |
| Byte stream reading | `state.byte_stream.next().await`, UTF-8 lossy conversion | Identical | **Shared** |
| Error handling | Network error -> `LlmError::Network`, set `done=true` | Identical | **Shared** |
| SSE line parsing | Inline `strip_prefix("data: ")` | Separate `parse_sse_line()` function | **Similar** |
| Event dispatch | Handles `data: [DONE]`, parses chunks inline | Handles `event:` + `data:` pairs, uses `parse_stream_event()` dispatcher | **Unique** (fundamentally different SSE protocols) |
| Stream chunk types | `OpenAiStreamChunk`, `OpenAiStreamChoice`, `OpenAiStreamDelta`, etc. | `MessageStartEvent`, `ContentBlockStartEvent`, etc. | **Unique** |
| Stop reason mapping | `"stop"` -> EndTurn, `"tool_calls"` -> ToolUse, `"length"` -> MaxTokens | `"end_turn"` -> EndTurn, `"tool_use"` -> ToolUse, `"max_tokens"` -> MaxTokens, `"stop_sequence"` -> StopSequence | **Similar** (same concept, different strings) |

---

## 3. Quantitative Analysis

### Line Counts by Category (Production Code Only, Excluding Tests)

| Category | openai.rs | anthropic.rs | Notes |
|----------|-----------|-------------|-------|
| **Shared** (identical or near-identical logic) | ~85 | ~85 | `new()`, buffer loop, error handling, `create_shared_backend` |
| **Similar** (same pattern, provider-specific details) | ~195 | ~165 | Config builders, trait impl, handle_response, handle_error, SSE skeleton |
| **Unique** (provider-specific, must stay separate) | ~553 | ~352 | Wire types, request conversion (OpenAI), SSE event parsing |
| **Tests** | 761 | 595 | |
| **Total** | 1,594 | 1,199 | |

### Estimated Extractable Lines

If a `common.rs` module is introduced:

| What | Estimated Lines Saved | Source |
|------|----------------------|--------|
| Shared config base trait/struct (5 common fields + 4 builder methods) | ~40 lines per file (~80 total) | Config structs |
| Shared `Backend::new()` (HTTP client construction) | ~10 lines per file (~20 total) | Backend structs |
| Shared `handle_error_response()` skeleton | ~25 lines per file (~50 total) | Error handling |
| Shared SSE buffer/byte-stream loop | ~30 lines per file (~60 total) | Streaming |
| Stop reason mapping helper | ~8 lines per file (~16 total) | Both files |
| Error response types | ~8 lines per file (~16 total) | Wire types |
| **Total extractable** | **~121 lines per file (~242 total)** | |

This represents roughly **15% of openai.rs** and **17% of anthropic.rs** production code (excluding tests).

---

## 4. The `LlmBackend` Trait

Defined in `crates/arawn-llm/src/backend.rs` (lines 236-304):

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(&self, request: CompletionRequest) -> Result<ResponseStream>;
    fn name(&self) -> &str;
    fn supports_native_tools(&self) -> bool { false }
    fn tool_calling_instructions(&self) -> Option<&str> { None }
    fn format_tool_definitions(&self, tools: &[ToolDefinition]) -> String { ... }
    fn format_tool_result(&self, tool_use_id: &str, content: &str, is_error: bool) -> String { ... }
    fn parse_tool_calls(&self, text: &str) -> (String, Vec<ParsedToolCall>) { ... }
}
```

### Actual Differences in Trait Implementation

| Method | OpenAI | Anthropic | Diff |
|--------|--------|-----------|------|
| `complete()` | Builds `OpenAiChatRequest`, logs with `tracing`, calls `with_retry` | Directly serializes `CompletionRequest`, calls `with_retry` | OpenAI needs request transformation; Anthropic sends native types |
| `complete_stream()` | Calls `parse_openai_sse_stream()` | Calls `parse_sse_stream()` | Different SSE parsers |
| `name()` | Dynamic from `self.config.name` | Hardcoded `"anthropic"` | Minor |
| `supports_native_tools()` | `true` | `true` | Identical |

**Key architectural difference**: Anthropic's `CompletionRequest` is already in Anthropic wire format (the `types.rs` types derive `Serialize` and match Anthropic's API). OpenAI requires a full translation layer (`to_openai_request()`, 155 lines) because its wire format differs substantially.

---

## 5. Proposed `common.rs` Module Structure

```
crates/arawn-llm/src/common.rs
```

### 5.1 Shared Config Base

```rust
/// Common configuration fields shared by all HTTP-based LLM backends.
pub struct BackendConfigBase {
    pub api_key: ApiKeyProvider,
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_backoff: Duration,
}

/// Trait for backend configs that share common fields.
pub trait BackendConfig {
    fn base(&self) -> &BackendConfigBase;
    fn base_mut(&mut self) -> &mut BackendConfigBase;

    fn with_base_url(mut self, url: impl Into<String>) -> Self where Self: Sized { ... }
    fn with_timeout(mut self, timeout: Duration) -> Self where Self: Sized { ... }
    fn with_max_retries(mut self, retries: u32) -> Self where Self: Sized { ... }
    fn with_retry_backoff(mut self, backoff: Duration) -> Self where Self: Sized { ... }
}
```

### 5.2 Shared HTTP Client Construction

```rust
/// Create a reqwest::Client with the given timeout.
pub fn build_http_client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| LlmError::Internal(format!("Failed to create HTTP client: {}", e)))
}
```

### 5.3 Shared Error Response Handling

```rust
/// Common structure for provider error responses: `{"error": {"message": "..."}}`
#[derive(Debug, serde::Deserialize)]
pub struct ProviderErrorResponse {
    pub error: ProviderErrorDetail,
}

#[derive(Debug, serde::Deserialize)]
pub struct ProviderErrorDetail {
    pub message: String,
}

/// Map an HTTP error status + parsed error body to LlmError.
pub fn map_error_status(status: u16, message: &str, retry_after: Option<&str>) -> LlmError {
    match status {
        401 => LlmError::Auth(format!("Authentication failed: {}", message)),
        429 => {
            let info = RateLimitInfo::parse_openai(message, retry_after);
            LlmError::RateLimit(info)
        }
        500..=599 => LlmError::Backend(format!("Server error: {}", message)),
        _ => LlmError::Backend(message.to_string()),
    }
}

/// Extract retry-after header, read body, parse error, and return LlmError.
pub async fn handle_error_response<E: serde::de::DeserializeOwned + HasMessage>(
    response: Response,
) -> LlmError { ... }
```

### 5.4 Shared SSE Buffering

```rust
/// Buffered SSE reader that handles byte-stream to line conversion.
pub struct SseBuffer {
    byte_stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
    done: bool,
}

impl SseBuffer {
    /// Read the next non-empty line from the SSE stream.
    pub async fn next_line(&mut self) -> Option<Result<String>> { ... }
}
```

### 5.5 Stop Reason Mapping Helper

```rust
/// Map a provider stop reason string to StopReason.
/// OpenAI: "stop" | "tool_calls" | "length"
/// Anthropic: "end_turn" | "tool_use" | "max_tokens" | "stop_sequence"
pub fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" | "end_turn" => StopReason::EndTurn,
        "tool_calls" | "tool_use" => StopReason::ToolUse,
        "length" | "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        _ => StopReason::EndTurn,
    }
}
```

### 5.6 Shared `create_shared_backend` Pattern

Both files have an identical pattern:

```rust
pub fn create_shared_backend(config: XxxConfig) -> Result<Arc<dyn LlmBackend>> {
    Ok(Arc::new(XxxBackend::new(config)?))
}
```

This is only 3 lines each and generic extraction adds complexity without clear value. **Recommend leaving as-is.**

---

## 6. Risk Assessment

### Low Risk

| Change | Risk | Mitigation |
|--------|------|------------|
| Extract `build_http_client()` | Trivial refactor | Mechanical replacement |
| Extract `ProviderErrorResponse` / `ProviderErrorDetail` | Both use identical `{"error":{"message":""}}` shape | Verify with provider docs |
| Extract `map_stop_reason()` | Pure function, easy to test | Unit test covers all branches |
| Extract `SseBuffer` | Encapsulates buffer management only | Keep protocol-specific parsing in each module |

### Medium Risk

| Change | Risk | Mitigation |
|--------|------|------------|
| Extract `BackendConfig` trait | Builder pattern changes could break downstream; Anthropic has `api_version` while OpenAI has `model`/`name` | Use composition (embed `BackendConfigBase`) not inheritance |
| Unify `handle_error_response()` | OpenAI has Groq-specific rate limit parsing (`parse_groq`) that Anthropic does not | Allow provider-specific override via closure or match on provider name |

### High Risk

| Change | Risk | Mitigation |
|--------|------|------------|
| Attempting to unify SSE streaming | The protocols are fundamentally different (OpenAI: `data: [DONE]` with inline chunks; Anthropic: typed `event:` + `data:` pairs with `message_start`, `content_block_start`, etc.) | Only extract the buffer/byte-stream reading; keep all event parsing provider-specific |
| Attempting to unify request building | Anthropic serializes `CompletionRequest` directly; OpenAI requires a 155-line translation. Forcing a common request builder would add an unnecessary abstraction layer for Anthropic | Leave request construction entirely provider-specific |
| Changing `types.rs` Serialize format | `CompletionRequest` currently matches Anthropic's wire format. Any changes break Anthropic's direct serialization | Do not modify `types.rs` as part of this initiative |

---

## 7. Recommendations

### Do Extract (clear value, low risk)

1. `build_http_client(timeout) -> Result<Client>` -- saves ~10 lines per backend, eliminates duplicated error message string
2. `ProviderErrorResponse` + `ProviderErrorDetail` shared types -- both providers use the exact same error JSON shape
3. `map_stop_reason(reason: &str) -> StopReason` -- consolidates 4 near-duplicate match arms
4. `SseBuffer` struct with `next_line()` method -- extracts the identical buffer/byte-stream loop (~30 lines per backend)
5. `handle_error_response()` shared skeleton -- extracts retry-after header extraction and status code mapping

### Consider Extracting (moderate value, some complexity)

6. `BackendConfigBase` struct embedded in each config -- eliminates 4 duplicated builder methods, but requires each config to delegate

### Do Not Extract (high risk, low value)

7. Request building -- architecturally different (translation vs direct serialization)
8. SSE event parsing -- fundamentally different protocols
9. `create_shared_backend()` -- only 3 lines, not worth abstracting
10. `add_headers()` -- different auth schemes (Bearer vs x-api-key), different error handling (infallible vs Result)

### Estimated Impact

Implementing items 1-5 would:
- Create a `common.rs` of approximately 80-100 lines
- Remove approximately 100-120 duplicated lines from each backend file
- Reduce `openai.rs` production code by ~15%
- Reduce `anthropic.rs` production code by ~17%
- Establish patterns for any future backend (e.g., Google Gemini) to reuse
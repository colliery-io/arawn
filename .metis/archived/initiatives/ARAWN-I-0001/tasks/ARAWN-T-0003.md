---
id: arawn-llm-llmclient-trait-provider
level: task
title: "arawn-llm — LlmClient trait, provider-neutral types, Groq implementation"
short_code: "ARAWN-T-0003"
created_at: 2026-03-31T17:37:37.365013+00:00
updated_at: 2026-03-31T19:00:37.351892+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# arawn-llm — LlmClient trait, provider-neutral types, Groq implementation

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Build the backend-agnostic LLM client layer. Define the `LlmClient` trait and provider-neutral types (`ChatRequest`, `ChatChunk`, etc.). Implement the first concrete provider: Groq. The engine will code against the trait, never a concrete provider.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `LlmClient` async trait: `stream(ChatRequest) → Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>>>>>`
- [ ] `ChatRequest` struct: `model`, `system_prompt`, `messages: Vec<Message>`, `tools: Vec<ToolDefinition>`, `max_tokens`
- [ ] `ChatChunk` enum: `TextDelta { text }`, `ToolUseStart { id, name }`, `ToolUseInputDelta { json }`, `Done { usage }`
- [ ] `ToolDefinition` struct: `name`, `description`, `parameters: serde_json::Value` (JSON Schema)
- [ ] `Usage` struct: `input_tokens`, `output_tokens`
- [ ] `GroqClient` implementing `LlmClient` — connects to Groq's OpenAI-compatible chat completions API with streaming
- [ ] API key resolution from `GROQ_API_KEY` env var
- [ ] SSE stream parsing for Groq's response format
- [ ] Maps Groq's tool_call response format to provider-neutral `ChatChunk` types
- [ ] Unit tests for type serialization and chunk parsing
- [ ] Integration test (gated behind feature flag or `#[ignore]`) that hits real Groq API

## Implementation Notes
- `client.rs` (trait), `types.rs` (neutral types), `groq.rs` (provider), `error.rs` in `crates/arawn-llm/src/`
- Groq uses OpenAI-compatible API — `reqwest` + `reqwest-eventsource` or manual SSE parsing
- Tool use in Groq follows OpenAI format (`tool_calls` array in choices), not Anthropic format (`tool_use` content blocks) — the provider module maps between formats
- Depends on: ARAWN-T-0001 (workspace scaffolding)

## Status Updates
- **2026-03-31**: Complete. LlmClient trait, all provider-neutral types, GroqClient with streaming SSE parser implemented. 8 unit tests passing (message building, tool formatting, chunk parsing for text/tool_use_start/tool_use_delta/usage). Custom SSE parser handles Groq's OpenAI-compatible streaming format. Integration test deferred to T-0008.
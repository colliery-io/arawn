---
id: multi-provider-llm-support
level: task
title: "Multi-provider LLM support — Anthropic Claude as second backend"
short_code: "ARAWN-T-0036"
created_at: 2026-04-01T11:01:59.474241+00:00
updated_at: 2026-04-04T00:14:46.820958+00:00
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

# Multi-provider LLM support — Anthropic Claude as second backend

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Add Anthropic Claude as a second LLM provider behind the existing `LlmClient` trait. The trait is already provider-agnostic — this task implements `AnthropicClient` alongside `GroqClient`. Configurable via env var or config file. Proves the multi-provider design works and gives access to Claude's stronger reasoning for complex tasks.

### Priority
- P2 — Groq works, but Claude is better for complex reasoning. Also validates the LlmClient abstraction.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AnthropicClient` struct implementing `LlmClient` in `crates/arawn-llm/src/anthropic.rs`
- [ ] Connects to Anthropic Messages API with streaming (`stream: true`)
- [ ] API key from `ANTHROPIC_API_KEY` env var
- [ ] Maps Anthropic's SSE event format to provider-neutral `ChatChunk` types
- [ ] Anthropic uses content blocks (`type: "text"`, `type: "tool_use"`) not OpenAI's `tool_calls` array — parser handles both
- [ ] Anthropic tool_use has `input` as a JSON object (not stringified) — handle the difference
- [ ] System prompt sent via the `system` parameter (not as a message) — Anthropic's format
- [ ] Tool definitions mapped to Anthropic's `tools` format (`input_schema` not `parameters`)
- [ ] Provider selection: `ARAWN_PROVIDER=anthropic` or `ARAWN_PROVIDER=groq` (default: groq)
- [ ] `ModelLimits::for_model` handles `claude-*` models (200k context)
- [ ] Unit tests for Anthropic SSE parsing (same pattern as Groq tests)
- [ ] Integration test (gated `#[ignore]`) that hits real Anthropic API

## Implementation Notes

- `anthropic.rs` in `crates/arawn-llm/src/` — follows same structure as `groq.rs`
- Anthropic SSE format uses `event:` + `data:` lines with event types: `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`
- Key difference from OpenAI/Groq: Anthropic streams content blocks with indices, tool_use input arrives as `input_json_delta` not `function.arguments`
- The `ChatChunk` enum already handles this — just need to map Anthropic events to the right variants
- Binary needs a provider factory: `match provider { "anthropic" => AnthropicClient, _ => GroqClient }`
- Reference: previous Arawn implementation in `backup/crates/arawn-llm/src/anthropic.rs` (1,172 lines)

## Status Updates

- Pivoted from "Anthropic only" to generic OpenAI-compatible client
- `OpenAICompatibleClient` with configurable base_url, api_key, provider_name
- Convenience constructors: groq(), ollama(), openai(), from_config()
- Known providers with default URLs: groq, ollama, openai, lmstudio, together, fireworks
- No API key required for local providers (Ollama, LM Studio)
- Config: `provider` + optional `base_url` + `api_key_env` in arawn.toml
- SSE parser generalized from groq.rs, provider-aware error messages
- main.rs updated for both serve and CLI modes
- Anthropic client (different API format) deferred to separate task
- 39 LLM tests passing
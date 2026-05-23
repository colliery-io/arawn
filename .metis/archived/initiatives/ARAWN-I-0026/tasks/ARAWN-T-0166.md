---
id: ollama-cloud-provider-integration
level: task
title: "Ollama Cloud provider integration and UAT config templates"
short_code: "ARAWN-T-0166"
created_at: 2026-04-12T13:48:05.813235+00:00
updated_at: 2026-04-12T13:48:05.813235+00:00
parent: ARAWN-I-0026
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0026
---

# Ollama Cloud provider integration and UAT config templates

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0026]]

## Objective
Verify Ollama Cloud works as an arawn LLM provider and create reusable config templates for UAT runs. Ensure the OpenAI-compatible client handles Ollama Cloud's API correctly (auth, streaming, tool_use format).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Manual smoke test: `arawn --data-dir /tmp/arawn-smoke serve` with Ollama Cloud config, send a message via WS, get a tool-using response
- [ ] Verify streaming works (EngineEvent::StreamingText arrives incrementally)
- [ ] Verify tool_use works (agent calls at least think + one other tool)
- [ ] Config templates for each model in the matrix: `uat-configs/gemma4.toml`, `uat-configs/llama-3.3-70b.toml`, `uat-configs/qwen3-32b.toml`
- [ ] Each config template has correct: provider, model, base_url, api_key_env, context_window, max_tokens
- [ ] Document any Ollama Cloud quirks (rate limits, model availability, tool_use format differences)
- [ ] If Ollama Cloud doesn't support tool_use for a model, document which models work and update the matrix

## Implementation Notes
Ollama Cloud uses the OpenAI-compatible API format. Our `OpenAICompatibleClient` should work out of the box, but verify:
- Auth header format (Bearer token vs API key)
- Tool call response format (some providers omit `tool_calls` or format them differently)
- Streaming SSE format matches what our client expects
- Model names match what Ollama Cloud expects (e.g., `gemma4` vs `gemma:4` vs `google/gemma-4`)

Config template:
```toml
[llm]
provider = "ollama"
model = "gemma4"
base_url = "https://api.ollama.com/v1"
api_key_env = "OLLAMA_API_KEY"
context_window = 128000
max_tokens = 8192

[engine]
max_iterations = 30

[compactor]
compaction_threshold = 0.85
```

This is the first task to execute — it unblocks all others by confirming the provider works.

## Status Updates
*To be added during implementation*
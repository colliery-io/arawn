---
id: token-estimator-modellimits-config
level: task
title: "Token estimator + ModelLimits config"
short_code: "ARAWN-T-0020"
created_at: 2026-04-01T03:28:11.030640+00:00
updated_at: 2026-04-01T03:42:35.526430+00:00
parent: ARAWN-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0004
---

# Token estimator + ModelLimits config

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0004]]

## Objective

Implement fast token estimation for messages, sessions, and tool definitions. Add `ModelLimits` to `QueryEngineConfig` so the engine knows how much context it has to work with.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TokenEstimator` struct with static methods: `estimate_message(&Message) → u32`, `estimate_session(&Session) → u32`, `estimate_tools(&[ToolDefinition]) → u32`, `estimate_system_prompt(&str) → u32`
- [ ] Heuristic: `tokens ≈ chars / 4` (simple, fast, good enough for threshold decisions)
- [ ] `estimate_message` counts chars in content + tool_use inputs + tool_result content
- [ ] `ModelLimits` struct: `context_window: u32`, `compaction_threshold: f32` (default 0.85)
- [ ] `ModelLimits` added to `QueryEngineConfig`
- [ ] Known model limits: `ModelLimits::for_model(name)` returns defaults for known models (llama-3.3 = 128k, gpt-oss-20b = 128k, etc.), falls back to 128k for unknown
- [ ] Unit tests: estimate_message returns reasonable values, estimate_session sums correctly, threshold math works

## Implementation Notes

- `token_estimator.rs` in `crates/arawn-engine/src/`
- Keep it simple — no tokenizer deps. The 85% threshold with chars/4 gives ~20% safety margin which is plenty.
- `ModelLimits::should_compact(session_tokens, tool_tokens, system_tokens) → bool` — checks if total exceeds `context_window * threshold`
- Depends on: nothing (new module in arawn-engine)

## Status Updates
- **2026-04-01**: Complete. TokenEstimator with chars/4 heuristic for messages, sessions, tools, system prompts. ModelLimits with context_window, compaction_threshold, for_model() lookup, should_compact(), available_for_messages(). Added to QueryEngineConfig with Default. Also added Message::Summary variant to arawn-core (needed for estimate_message match arm). 9 new unit tests. Binary updated with ModelLimits::for_model(). 139 total workspace tests, clippy clean.
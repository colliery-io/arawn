---
id: t-e-model-context-aware-bypass
level: task
title: "T-E: Model-context-aware filter bypass (≥100K context)"
short_code: "ARAWN-T-0409"
created_at: 2026-05-22T16:36:04.000000+00:00
updated_at: 2026-05-22T16:36:04.000000+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-E: Model-context-aware filter bypass (≥100K context)

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

When the configured model's context window is ≥100K tokens, bypass `filter_tools_for_context` entirely and ship the full catalog. The filter is only justified for small models; for Claude/GPT-4 the brittleness is not worth the catalog savings.

## Acceptance Criteria

- [ ] `filter_tools_for_context` has a top-of-function guard:
  ```rust
  const FILTER_BYPASS_CONTEXT_THRESHOLD: u32 = 100_000;
  if model_limits.context_window >= FILTER_BYPASS_CONTEXT_THRESHOLD {
      return all_tools.to_vec();
  }
  ```
- [ ] The function signature accepts `model_limits: &ModelLimits` (currently it takes `&[ToolDefinition], &Session, &ToolRegistry`). Plumb `model_limits` through from the caller in `query_engine.rs:634`.
- [ ] `FILTER_BYPASS_CONTEXT_THRESHOLD` is a `const` in `query_engine.rs` — single source of truth for the threshold.
- [ ] Unit tests:
  - [ ] `filter_bypasses_for_large_context_model`: build a filter call with `model_limits.context_window = 200_000` and a session/user message that would normally drop all integration tools. Assert all tools (including integrations) are in the filtered list.
  - [ ] `filter_active_for_small_context_model`: same call with `context_window = 32_000`. Assert filter behaves as before (integration tools dropped without capability).
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- The 100K threshold is conservative — even at 100K, 25K tokens of catalog leaves 75K for conversation. The cutoff is "what's the smallest context window for which we definitely don't need to filter."
- Models that currently fall into the bypass: claude-opus-4-7, claude-sonnet-4-6, claude-haiku-4-5 (all 200K), gpt-4 (128K). Models that stay filtered: gemma4:31b-cloud (32K), llama-3.3-70b (8K), etc.
- This is independent of T-D (always-on promotion); both can land in either order after T-B.

## Status Updates

*To be added during implementation*

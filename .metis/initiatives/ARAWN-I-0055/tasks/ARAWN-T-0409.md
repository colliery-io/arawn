---
id: t-e-model-context-aware-filter
level: task
title: "T-E: Model-context-aware filter bypass (≥100K context)"
short_code: "ARAWN-T-0409"
created_at: 2026-05-22T16:36:04+00:00
updated_at: 2026-05-22T18:25:02.456032+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-E: Model-context-aware filter bypass (≥100K context)

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

When the configured model's context window is ≥100K tokens, bypass `filter_tools_for_context` entirely and ship the full catalog. The filter is only justified for small models; for Claude/GPT-4 the brittleness is not worth the catalog savings.

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-05-22 — landed

**Signature change:** `filter_tools_for_context` now takes `model_limits: &ModelLimits` (5th arg). Call site in `QueryEngine::run` passes `&self.config.model_limits`.

**Top-of-function guard:**
```rust
const FILTER_BYPASS_CONTEXT_THRESHOLD: u32 = 100_000;
if model_limits.context_window >= FILTER_BYPASS_CONTEXT_THRESHOLD {
    return all_tools.to_vec();
}
```

**Bypass coverage:** models with `context_window ≥ 100_000` get the full catalog every turn. Concretely:
- claude-opus-4-7, claude-sonnet-4-6, claude-haiku-4-5 (all 200K) → bypass.
- gpt-4 (128K) → bypass.
- llama-3.3-70b-versatile (128K per `ModelLimits::for_model`) → bypass.
- gemma4:31b-cloud (32K) → filter active.
- qwen* (32K) → filter active.

**Test sweep:** every existing filter test had to be updated to pass a `&ModelLimits` arg. Did this by introducing a `small_model_limits()` helper (32K) at the top of the I-0055 test block, then `sed`-replacing all 6 existing call sites in the file. Two new T-E tests use a `large_model_limits()` helper (200K) and assert the bypass path.

**Unit tests added (2):**
- `filter_bypasses_for_large_context_model` — capability absent, 200K context, message has no calendar keywords; assert `calendar_upcoming` is visible.
- `filter_active_for_small_context_model` — companion negative; same setup with 32K context; assert tool dropped (confirms the filter still runs when bypass doesn't fire).

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 00s).
- `cargo test -p arawn-engine --lib query_engine`: ✅ **34 tests pass** (was 32 pre-T-E).
- `cargo test --workspace --lib`: ✅ **1,783 tests pass**, 0 fail.

Side note: hit two test-isolation flakes during the workspace run (`reshelve::tests::merge_picks_most_reinforced_survivor` and the earlier `testing::harness::tests::harness_shell_tool_receives_arguments`). Both pass cleanly on re-run; not introduced by T-E. Pre-existing flakes worth flagging.
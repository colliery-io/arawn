---
id: hint-style-model-routing-taxonomy
level: task
title: "Hint-style model routing taxonomy — accept `hint:*` strings, map to providers"
short_code: "ARAWN-T-0272"
created_at: 2026-05-15T14:12:42.792143+00:00
updated_at: 2026-05-15T17:13:07.305019+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Hint-style model routing taxonomy

## Tier
Tier 1 — taxonomy only. The actual policy (`T-0278`) lands later in tier 1 and depends on this.

## Reference
`/tmp/openhuman/src/openhuman/routing/policy.rs::classify` — maps `hint:reaction`, `hint:classify`, `hint:format`, `hint:sentiment`, `hint:lightweight` → Lightweight; `hint:summarize`, `hint:medium`, `hint:tool_lite` → Medium; `hint:chat`, `hint:reasoning`, exact model names → Heavy.

## Goal
`arawn-llm` accepts `hint:*` model strings everywhere it accepts a model name. Each hint resolves to a concrete provider+model via config. Today every hint can resolve to the same provider — the value is the *call-site decoupling*, not the actual routing decision (that lands in T-0278).

## Acceptance
- New enum `ModelHint { Lightweight, Medium, Heavy }` + `classify(&str) -> Option<ModelHint>` mirroring openhuman shape.
- `arawn.toml` gains `[llm.hints]` section: each tier maps to an existing `[llm.<profile>]` key. Defaults: all three map to `engine.llm`.
- LLM client recognises `hint:*` at the model-string boundary, resolves via config, then dispatches normally.
- Three real callers updated to emit hints instead of hard model names: the compactor (Medium), classification points in workstream_router (Lightweight), the main engine loop (Heavy).
- Tests cover hint parsing, config resolution, and "unknown hint falls through to remote heavy" fallback.

## Out of scope
Local/remote routing, health checks, telemetry — all T-0278.

## Status Updates

**2026-05-15 — implementation landed.**

Acceptance check, with three documented deviations from the task spec:

1. **Config namespace.** Task said `[llm.hints]`; that would clash with the existing `[llm.NAME]` HashMap. Landed `[routing.hints]` instead — also sets up T-0278's `[routing.*]` namespace for privacy / latency / usage-pressure config.
2. **"Workstream-router classification" caller.** No such caller exists in arawn — `workstream_router` is per-workstream *memory* routing, not LLM classification. Substituted the **steward subroutines** (`reshelve`, `map`, `doorwatch`) which are focused summarisation tasks that match Medium semantically. All three updated.
3. **Lightweight tier.** Wired through the type / parse / config / resolve path but no caller emits it yet. All current arawn LLM callers are Medium or Heavy. The hint string still resolves correctly; the first Lightweight caller will likely appear with T-0278 telemetry or the triage drop tier (ARAWN-S-0004 §E).

Files:

- New `crates/arawn-llm/src/hints.rs`: `ModelHint { Lightweight, Medium, Heavy }`, `classify(&str)`, `is_hint_shape(&str)`, `as_hint()`/`as_str()`. Re-exported from `arawn-llm::lib.rs` (`HINT_PREFIX`, `ModelHint`, `classify_hint`, `is_hint_shape`). 6 unit tests.
- `crates/arawn/src/config.rs`: new `RoutingConfig { hints: HintRoutingConfig }` with `lightweight: Option<String>` / `medium` / `heavy`. Added to `ArawnConfig` + default impl.
- `crates/arawn/src/llm_pool.rs`: new `LlmClientPool::resolve_hint(model_str: &str) -> (Arc<dyn LlmClient>, String)`. Hint → mapped profile (engine fallback). Concrete model → engine client + model unchanged. Unknown hint → engine + tracing warn. New `resolve_hint_names` helper drops unresolvable entries at construction with a warning. `hint_names: HashMap<ModelHint, String>` field on the pool. 6 unit tests covering hint match / unconfigured fallback / unknown-hint fallback / concrete passthrough / missing-profile silent fallback.
- Call-site updates:
  - `main.rs::build_engine_config`: `QueryEngineConfig.model` now set to `ModelHint::Heavy.as_hint()` (resolved later).
  - `local_service.rs::engine_for_send`: resolves both engine (`self.config.model`) and compactor (`hint:medium`) through `pool.resolve_hint(...)` before constructing `QueryEngine` / `Compactor`. The engine's `ChatRequest.model` is the resolved concrete name, not the hint.
  - `main.rs`: reshelve / map / doorwatch steward subroutines built via `pool.resolve_hint(&ModelHint::Medium.as_hint())`.

Tests: 6 in `arawn-llm::hints`, 6 in `arawn::llm_pool` covering the resolve_hint matrix. Workspace test suite: 1100 passed / 0 failed.

Resolution lives at the call-site boundary today (caller asks the pool, pool returns client + concrete model). The hint string never reaches the LLM provider. T-0278 will replace this with a per-request routing provider that wraps `LlmClient` and folds in health checks + privacy / latency / usage-pressure inputs.
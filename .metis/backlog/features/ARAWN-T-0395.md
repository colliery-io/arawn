---
id: dynamic-usagepressure-computation
level: task
title: "Dynamic `UsagePressure` computation — wire usage tracker into RoutingHints"
short_code: "ARAWN-T-0395"
created_at: 2026-05-22T00:11:25.708104+00:00
updated_at: 2026-05-23T02:36:05.263262+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Dynamic `UsagePressure` computation — wire usage tracker into RoutingHints

## Backlog Item Details

### Type
- [ ] Bug
- [x] Feature — finishes the deferred portion of T-0393 / T-0278
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — capability exists, dynamic biasing would polish it
- [ ] P3

### Business Justification
- **User Value**: When remote token usage spikes (e.g., heavy day) and a local model is available, routing should bias toward Local to spread the load and avoid burning the remote quota. Today every call passes `UsagePressure::Low` (the default), so this knob is unused.
- **Effort Estimate**: M — requires reading usage rollups, plumbing a threshold config, and computing pressure at hint-build time.

## Objective

T-0393 wired `IntelligentRoutingProvider` into the engine/compactor/steward call sites but only with `RoutingHints::default()` (or `LatencyBudget::Low` for the engine). The third hint dimension — `usage_pressure: UsagePressure::Low|High` — is always `Low`. This task makes it dynamic by reading the `arawn-llm::usage::UsageTracker` rollups and comparing against a configurable threshold.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Add a `[routing.usage_pressure]` block to `crates/arawn/src/config.rs`:
  - `threshold_period`: `"24h"` (string parsed to `Duration` or `chrono::TimeDelta`).
  - `threshold_tokens`: `u64`, default `1_000_000`.
  - Both with serde defaults so the block is optional.
- [ ] Plumb the threshold into `LlmClientPool` (constructor or a field on the pool struct), so the helper `routed_or_fallback` can consult it.
- [ ] Add `LlmClientPool::compute_usage_pressure(&self, model: &str) -> UsagePressure`:
  - Reads recent token usage for the named remote model from `arawn_llm::usage::UsageTracker`.
  - Returns `High` when the rolling-window sum exceeds `threshold_tokens`; `Low` otherwise.
  - Cache the result for `threshold_period / 60` seconds so we don't hammer the tracker per call.
- [ ] Update `routed_or_fallback` (or add a sibling helper that takes a flag) so callers can request "merge in the pool's computed usage_pressure" — keeps the helper composable.
- [ ] Update the engine/compactor/steward call sites to pass the computed pressure.
- [ ] Add unit tests for `compute_usage_pressure`: below-threshold returns Low, above returns High, missing tracker returns Low.
- [ ] Update `docs/src/reference/config-schema.md` to document the new `[routing.usage_pressure]` block.

## Implementation Notes

### Technical Approach

1. Add the config struct with serde defaults.
2. Pass the threshold into `LlmClientPool::from_config`.
3. Implement `compute_usage_pressure` with a small in-memory cache (LRU or just a single `(timestamp, value)` per model since we only have one remote profile at a time).
4. Make the helper variants ergonomic: probably `routed_or_fallback_with_pressure(hint, mut hints, ...)` that fills in `hints.usage_pressure = self.compute_usage_pressure(...)` before delegating.
5. Wire the engine + compactor + steward callsites.
6. Tests.

### Dependencies

- Originating task: T-0393 (ARAWN-I-0053).
- Implementation needs the existing `arawn_llm::usage::UsageTracker` to be queryable for rollups by model name + time window. Verify the API surface before starting.

### Risk Considerations

- The `IntelligentRoutingProvider` policy treats `usage_pressure: High` as a strong Local bias when health permits. Get the threshold wrong (too low) and the system pins everything to a possibly-overloaded local model. Default the threshold high; document tuning in the config schema.
- Caching: 1-minute cache is fine for production but stale-able. Make sure tests don't rely on the cache duration.

## Status Updates

### 2026-05-23 — closed as wontfix

The whole `arawn-llm::routing` layer that this task depended on was ripped out. The decision: capabilities are assigned to specific models via `[routing.hints]` config (lightweight/medium/heavy → `[llm.NAME]`), not via runtime per-call dispatch policy. With no `RoutingHints` / `IntelligentRoutingProvider` / `UsagePressure` types, there's nothing to wire `usage_pressure` into.

If hybrid local+remote dispatch becomes a real user need later, rebuilding the routing layer from scratch is cheaper than maintaining premature scaffolding through every refactor. This task should be re-filed with fresh requirements at that point, not resurrected.

Commit: see the routing-layer removal in the current session.
---
id: wire-intelligentroutingprovider
level: task
title: "Wire `IntelligentRoutingProvider` into engine agent loop — finish T-0278 deferred wiring"
short_code: "ARAWN-T-0393"
created_at: 2026-05-21T14:53:38.031637+00:00
updated_at: 2026-05-22T00:06:10.240031+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# Wire `IntelligentRoutingProvider` into engine agent loop — finish T-0278 deferred wiring

## Backlog Item Details

### Type
- [ ] Bug
- [x] Feature — completes deferred wiring from a previously-shipped task
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — capability exists but isn't producing the value it could
- [ ] P3

### Business Justification
- **User Value**: With the routing provider wired in, the engine can transparently route lightweight calls to a local model and heavy calls to the remote model based on health and policy hints. Today the agent loop still uses `resolve_hint()` which doesn't apply the routing policy — so all calls go to whichever model the hint resolves to without considering local health or usage pressure.
- **Effort Estimate**: M — requires per-callsite decisions about `RoutingHints` (privacy? latency? usage pressure?) plus the actual wiring swap.

## Objective

T-0278 (archived) shipped the `IntelligentRoutingProvider` and its policy
matrix under `crates/arawn-llm/src/routing/`. The provider is exposed via
`LlmClientPool::routing_provider(hints)`, but the engine agent loop in
`LocalService::send_message` and friends still calls `resolve_hint(...)`
instead. The comment at `crates/arawn-engine/src/query_engine.rs:750` notes
the pending wiring: "T-0278 will switch the agent loop to RemotePermit when
the [wiring lands]."

Per ARAWN-I-0053 discovery (Tier 3 candidate 3.9 audit): this is a real
pending-wiring follow-up.

## Acceptance Criteria

## Acceptance Criteria

- [ ] For each engine call site that currently uses `resolve_hint()` (agent loop, compactor, summarizer, etc.), decide the right `RoutingHints`:
  - `privacy_required`: should defaults be opt-in privacy? Almost certainly the assistant's main loop is NOT privacy-required by default. Specific tool flows might be.
  - `latency_budget`: probably `Low` for the agent loop (user is watching), `Normal` for background work.
  - `usage_pressure`: read from the token usage tracker rollups; threshold should land in `[routing.usage_pressure]` config.
- [ ] Add a `[routing.usage_pressure]` config block with `threshold_period` (e.g., `"24h"`) and `threshold_tokens` (e.g., `1_000_000`). Compute pressure at hint-build time.
- [ ] Switch each chosen call site from `pool.resolve_hint(...)` to `pool.routing_provider(hints).stream(...)`. Verify `RemotePermit` is acquired where the policy decides Remote.
- [ ] Emit `RoutingRecord` telemetry on every routed call (already done by `IntelligentRoutingProvider` per T-0278).
- [ ] Update integration tests to exercise the routed path with a configured local + remote pair.
- [ ] `angreal test uat` passes — including any tests that exercise routing behavior.

## Implementation Notes

### Technical Approach

1. Audit current `resolve_hint` call sites in arawn-engine, arawn-ceremonies, arawn-extractor.
2. For each, design `RoutingHints` defaults.
3. Wire the routing provider, falling back gracefully when no local profile is configured (the provider already handles this — returns `None`, and the call site uses the remote engine directly).
4. Add the `[routing.usage_pressure]` config.
5. Add integration tests with a fake local + fake remote provider.
6. Run UAT.

### Dependencies

None blocking — T-0278's routing layer is already shipped and exposed via the pool.

### Risk Considerations

- Routing decisions are observable in latency and cost; getting hints wrong (e.g., always-Remote) is a regression in resource usage. Carefully test the policy matrix in integration tests.
- Privacy-required call sites are critical to get right. If the operator ever flags a tool as privacy-sensitive, routing it to Remote is a behavior regression. Default conservatively when in doubt.

## Status Updates

### 2026-05-21 — landed (main wiring) + 2 sub-tasks deferred

**Main wiring landed:**
- New helper `LlmClientPool::routed_or_fallback(hint, RoutingHints) -> (Arc<dyn LlmClient>, String)` in `crates/arawn/src/llm_pool.rs:262-281`. When a local profile is configured it wraps the hint's client in an `IntelligentRoutingProvider` (so the policy can route Lightweight/Medium to Local when healthy); when no local is configured it falls back to plain `resolve_hint`. The returned model string is always the hint's canonical model so telemetry continues to look right.
- Re-exported `LatencyBudget` and `UsagePressure` from `llm_pool.rs` so call sites can name the policy enums.
- Updated **5 call sites** to use the helper:
  - `crates/arawn/src/local_service.rs:466-486` — compactor uses default hints (Normal latency, non-private). Engine uses `LatencyBudget::Low` because the user is actively waiting.
  - `crates/arawn/src/main.rs:670-704` — three steward subroutines (reshelve, map, doorwatch) all use default `RoutingHints` (Normal latency, non-private; background work cadence is hourly).
- Added 2 unit tests in `llm_pool.rs`:
  - `routed_or_fallback_wraps_in_router_when_local_configured` — confirms the helper takes the routed path when both local + remote profiles are configured.
  - `routed_or_fallback_uses_resolve_hint_when_no_local` — confirms the fallback path returns the hint's resolved engine client + model.

**What this gets us:** every production LLM call from the agent loop, compactor, and steward subroutines now goes through `IntelligentRoutingProvider` when a local profile is configured. The provider applies the T-0278 policy matrix (privacy → Local; Heavy → Remote; Medium → hint-driven; Lightweight + Healthy → Local; etc.) and emits `RoutingRecord` telemetry per call. Without a local profile, behavior is identical to before (passthrough).

**Deferred to backlog:**
Two acceptance-criteria items proved to be substantial sub-tasks of their own. Filed as backlog:

- **[[ARAWN-T-0395]]** — Dynamic `UsagePressure` computation. Current call sites always pass `usage_pressure: Low` because there is no integration with the `arawn_llm::usage::UsageTracker` rollups yet. Adding the `[routing.usage_pressure]` config block + threshold lookup + caching is its own task. Independent of T-0393.
- **[[ARAWN-T-0396]]** — `IntelligentRoutingProvider` should call `acquire_remote()` to acquire a `RemotePermit` when the policy picks Remote (so the gate module's counters reflect remote traffic). The comment at `query_engine.rs:750` originally tied this to T-0278; it's a small one-file change.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (58.84s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass** (was 1,756; added 2 routed_or_fallback tests), 0 fail.
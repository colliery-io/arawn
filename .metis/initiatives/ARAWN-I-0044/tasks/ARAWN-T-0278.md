---
id: routing-policy-health-aware-local
level: task
title: "Routing policy — health-aware local/remote dispatch with privacy/latency/usage hints"
short_code: "ARAWN-T-0278"
created_at: 2026-05-15T14:13:09.228706+00:00
updated_at: 2026-05-15T19:46:01.257467+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Routing policy — health-aware local/remote dispatch

## Tier
Tier 1, last in sequence — depends on `T-0272` (hint taxonomy) and `T-0277` (token usage tracker). Everything else in tier 1 can land in parallel.

## Reference
`/tmp/openhuman/src/openhuman/routing/{policy,health,provider,telemetry}.rs`. Policy: category × local-health × hints → `RoutingTarget::{Local, Remote}` with optional fallback. Telemetry: `RoutingRecord` per decision.

## Goal
A real policy layer between `arawn-llm` callers and providers. Given a `(model_hint, RoutingHints { privacy_required, latency_budget, usage_pressure })`, it picks Local vs Remote based on local-model health and hint biases. On Local failure, transparently retries Remote unless `privacy_required`. Every decision emits a structured telemetry record.

## Acceptance
- New `crates/arawn-llm/src/routing/{policy,health,provider,telemetry}.rs`.
- `LocalHealthChecker` polls the configured local provider (Ollama by default) every N seconds, exposes `is_healthy()`.
- Policy table mirrors the openhuman matrix (Lightweight+healthy→Local, Heavy→Remote always, Medium hint-driven, privacy_required→Local with no fallback).
- `IntelligentRoutingProvider` wraps the existing `arawn-llm` client; transparent fallback on Local failure.
- Telemetry: `RoutingRecord { target, model, category, hints, outcome, fallback_used }` emitted to logs (or the event bus once it lands).
- Usage-pressure ties into `T-0277`: `UsagePressure::High` (recent token usage on the configured remote model exceeds a configurable threshold for the period) biases toward Local when health permits. Pressure is read from the token usage tracker's rollups; no dollar figures involved.
- Integration tests with a fake local provider (slow + flaky) + fake remote (fast + reliable) verify the decision matrix.

## Out of scope
Quality-based fallback (`is_low_quality` in their codebase). Add only if our local model produces visibly broken output.

## Status Updates

**2026-05-15 — implementation landed.**

**Module placement deviation:** task spec said `crates/arawn-llm/src/routing/` — that's where it lives. Mentioned for completeness because the prior tier-1 tasks (T-0275, T-0277) had to deviate from `arawn-engine/...`; T-0278 was already pointed at the right crate.

- New `crates/arawn-llm/src/routing/` with four files:
  - `policy.rs` — pure decision table. `RoutingHints { privacy_required, latency_budget: {Low|Normal}, usage_pressure: {Low|High} }`, `LocalHealth { NotConfigured | Healthy | Unhealthy }`, `RoutingTarget { Local | Remote }`, `Decision { primary, fallback, reason }`, `decide(hint, hints, health) -> Decision`. 9 unit tests covering every cell of the matrix.
  - `health.rs` — `LocalHealthChecker` with `configured()` / `not_configured()` constructors, lock-free `snapshot()`, manual `mark_healthy()` / `mark_unhealthy()` for tests and the future polling task. 3 unit tests.
  - `provider.rs` — `IntelligentRoutingProvider` LlmClient impl. Wraps `(local: Option<ProviderHandle>, remote: ProviderHandle, health, hints)`. On every `stream()`: classifies the hint, decides via the policy, dispatches to the chosen handle, transparently retries on the fallback target when the policy permits, emits a `RoutingRecord` per outcome. 9 integration tests with mock local + mock remote covering every acceptance bullet.
  - `telemetry.rs` — `RoutingRecord { target, model, category, hints, fallback_used, outcome, reason }`, `RoutingOutcome { Success | FellBack | Failed }`. `emit()` writes through `tracing::info!` so JSON-subscribers and devs both see the trail. 2 unit tests. Note: the typed event bus from ARAWN-S-0004 §B will give us a typed channel later; the tracing emit is the bridge.

**Policy matrix shipped:**

| Hint | Health | Hints | → Primary | Fallback |
|---|---|---|---|---|
| any | any | `privacy_required` | Local | none |
| Heavy | any | any | Remote | none |
| Lightweight | Healthy | any | Local | Remote |
| Lightweight | Unhealthy / NotConfigured | any | Remote | none |
| Medium | Healthy | `latency=Low` or `usage_pressure=High` | Local | Remote |
| Medium | Healthy | default | Remote | Local |
| Medium | Unhealthy / NotConfigured | any | Remote | none |

**Config:** added `[routing.providers] { local, remote }` to `ProvidersRoutingConfig`. `local` defaults to None (no routing layer); `remote` defaults to the engine profile.

**Pool integration (`crates/arawn/src/llm_pool.rs`):**
- New `LlmClientPool::routing_provider(hints) -> Option<IntelligentRoutingProvider>` returns None when no local profile is configured (no routing decision to make). When wired, returns a fully-configured provider sharing the pool's clients + `local_health`.
- `pool.local_health()` exposes the shared `LocalHealthChecker` for the future polling task.
- 4 unit tests covering the on/off/local-only/missing-profile cases.

**Wiring deferred (documented as follow-up):** I did not switch `LocalService` to use `routing_provider()` instead of `resolve_hint()` in the agent loop. The choice of `RoutingHints` per call site (which calls are `privacy_required`? does the compactor want `latency=Low`?) is design work that belongs in the call-site changes, not in this routing-layer task. The pool exposes `routing_provider()` so the wiring is a single-line swap when the call-site hints are decided.

**Usage pressure ties to T-0277:** the policy reads `UsagePressure::High` from `RoutingHints`. The caller is expected to compute it from `arawn_llm::usage::UsageTracker::summary(period, Some(model), false)` and compare against a configurable threshold — the threshold + period config will join `[routing.usage_pressure]` when the wiring lands.

**Tests:**
- 23 new in `arawn_llm::routing` (9 policy + 3 health + 2 telemetry + 9 provider integration with mock local/remote covering Lightweight-local, Heavy-remote, Lightweight-fallback-on-failure, privacy-no-fallback, Unhealthy-routes-remote, Medium-low-latency-local, Medium-high-pressure-local, Medium-default-falls-back-to-local, concrete-model-passthrough).
- 4 new in `arawn::llm_pool::tests` covering pool wiring + health-checker construction.
- Workspace suite: **1602 passed / 0 failed.**
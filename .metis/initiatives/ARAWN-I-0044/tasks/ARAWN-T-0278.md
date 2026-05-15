---
id: routing-policy-health-aware-local
level: task
title: "Routing policy — health-aware local/remote dispatch with privacy/latency/usage hints"
short_code: "ARAWN-T-0278"
created_at: 2026-05-15T14:13:09.228706+00:00
updated_at: 2026-05-15T14:13:09.228706+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

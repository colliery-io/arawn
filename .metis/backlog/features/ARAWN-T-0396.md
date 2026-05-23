---
id: intelligentroutingprovider-should
level: task
title: "`IntelligentRoutingProvider` should acquire `RemotePermit` on Remote routing decisions"
short_code: "ARAWN-T-0396"
created_at: 2026-05-22T00:11:26.674623+00:00
updated_at: 2026-05-23T02:36:24.146878+00:00
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

# `IntelligentRoutingProvider` should acquire `RemotePermit` on Remote routing decisions

## Backlog Item Details

### Type
- [ ] Bug
- [x] Feature — finishes the deferred portion of T-0393 / T-0278
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2
- [ ] P3

### Business Justification
- **User Value**: The `arawn-llm::gate` module exposes `acquire_remote() -> RemotePermit` — a cheap, always-succeeds, never-blocks permit type designed to attribute remote calls for telemetry. Today the `IntelligentRoutingProvider` doesn't call `acquire_remote()` when the policy picks Remote, so the permit count never reflects actual remote traffic.
- **Effort Estimate**: S — one file, a handful of lines.

## Objective

When `IntelligentRoutingProvider::stream` resolves to a Remote target (whether primary or via fallback), it should `acquire_remote()` and hold the permit for the duration of the stream so the gate module can count and attribute the call. The comment at `crates/arawn-engine/src/query_engine.rs:750` originally noted this as part of T-0278's deferred wiring.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Modify `crates/arawn-llm/src/routing/provider.rs` so each Remote dispatch path acquires a `RemotePermit` via `acquire_remote()` and holds it through the stream's lifetime.
- [ ] When the routing policy picks `Local` as primary and `Remote` as fallback, acquire the permit only when the fallback fires.
- [ ] Per-call cost is negligible (`acquire_remote` is sync + zero-allocation per its docstring); add a small unit test that verifies the permit-count increments on a Remote decision via a mock policy.
- [ ] Update the comment at `crates/arawn-engine/src/query_engine.rs:750` (or remove the comment if it's now satisfied).
- [ ] `cargo test -p arawn-llm` passes.
- [ ] `cargo test --workspace --no-run` clean.

## Implementation Notes

### Technical Approach

1. Read `arawn-llm/src/gate/mod.rs` to confirm the `RemotePermit` API.
2. In `IntelligentRoutingProvider::stream`, after the policy decides, if `target == Remote`, acquire a permit before dispatching.
3. Hold the permit alongside the stream until the stream is consumed (probably stuff it into the same wrapper stream struct that emits `RoutingRecord`).
4. Same treatment for the fallback path when Local fails and we retry Remote.

### Dependencies

- Originating task: T-0393 (ARAWN-I-0053).
- Independent of T-0395 (dynamic usage pressure).

### Risk Considerations

- The gate module is currently a marker / no-op (per `acquire_remote` docstring). If the gate ever becomes a real semaphore (capacity-limited), this wiring would suddenly start blocking — which would be a correctness improvement, but worth flagging when gate is enhanced.

## Status Updates

### 2026-05-23 — closed as wontfix

The `arawn-llm::routing` layer was ripped out (premature scaffolding — no real hybrid local+remote users). `RemotePermit` + `acquire_remote()` were also removed from `arawn-llm::gate`. The agent loop continues to use `acquire_local()` for laptop-RAM safety on the 1-slot semaphore; cloud-bound calls flow through the same gate today, which is fine because nobody runs arawn in a configuration where Remote-call serialization matters.

If hybrid dispatch comes back, rebuild from scratch with concrete requirements; don't resurrect this task.

Commit: see the routing-layer removal in the current session.
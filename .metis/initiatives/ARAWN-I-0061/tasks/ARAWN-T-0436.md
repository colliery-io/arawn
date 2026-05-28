---
id: t-a-memory-goes-global-remember
level: task
title: "T-A: Memory goes global — /remember and memory_store write to the global store"
short_code: "ARAWN-T-0436"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T01:05:42.169529+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-A: Memory goes global — /remember and memory_store write to the global store

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
A **memory** is a global statement of fact / behavioral tuning (e.g. "Pat Collins
is someone I manage") — always known, never lens-filed. Route `/remember` and the
`memory_store` tool to the **global** store instead of "the active lens."

## Technical Approach
- `crates/arawn-engine/src/tools/memory_store.rs:134` ("Route to appropriate store
  for the active lens") — write to the global store, not the SessionLens/active
  lens. Memory is global by definition.
- Trace the `/remember` slash path (`arawn/src/local_service/` memory write) to the
  same global destination.
- Keep the storage layout otherwise unchanged (global `memory.db` already exists).
- This is the first of the entangled core trio (T-A/T-B/T-C around the SessionLens
  write path); land them together if cleaner.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `/remember <fact>` and `memory_store` write to the global store regardless of
      any lens state.
- [ ] No memory-write path consults SessionLens / active lens for its destination.
- [ ] `memory_store` tool description says memories are global facts/tunings.
- [ ] `angreal check workspace` + affected unit tests green.

## Dependencies
Pairs with [[ARAWN-T-0437]] (removing the write-target) and [[ARAWN-T-0438]]
(un-smearing memory_search). Land the trio together.

## Status Updates

**2026-05-27 — Done.** Made memory global at the production layer:
- `arawn-memory/src/types.rs` — `EntityType::default_scope()` now returns
  `Scope::Global` for every type (was preference/person→global, others→lens).
  Drives both `/remember` (`store_fact_embedded(entity, None)`) and the
  `memory_store` tool. Updated the `default_scopes` test.
- `arawn-engine/src/tools/memory_store.rs` — removed the `scope` param (memory has
  no lens); `execute` forces `Scope::Global`. Reworded the description; updated
  tests (`store_decision_goes_global` renamed, asserts `mgr.global`); dropped
  `store_with_explicit_scope_override`.
- Per-lens stores stay for extractor-written **signals** (`cot.rs` unchanged).
- Tests green: `arawn-memory::default_scopes`, `arawn-engine` `memory_store` 5/5.
  Destination no longer depends on lens; full SessionLens removal lands in T-B.
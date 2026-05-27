---
id: t-a-memory-goes-global-remember
level: task
title: "T-A: Memory goes global — /remember and memory_store write to the global store"
short_code: "ARAWN-T-0436"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-27T20:18:49.413618+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- [ ] `/remember <fact>` and `memory_store` write to the global store regardless of
      any lens state.
- [ ] No memory-write path consults SessionLens / active lens for its destination.
- [ ] `memory_store` tool description says memories are global facts/tunings.
- [ ] `angreal check workspace` + affected unit tests green.

## Dependencies
Pairs with [[ARAWN-T-0437]] (removing the write-target) and [[ARAWN-T-0438]]
(un-smearing memory_search). Land the trio together.

## Status Updates
*To be added during implementation*

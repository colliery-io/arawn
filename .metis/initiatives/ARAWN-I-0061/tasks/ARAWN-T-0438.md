---
id: t-c-un-smear-memory-search-global
level: task
title: "T-C: Un-smear memory_search — global memory only, signals stay cross-lens"
short_code: "ARAWN-T-0438"
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

# T-C: Un-smear memory_search — global memory only, signals stay cross-lens

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
I-0060 made `memory_search` read across all lenses + global, smearing global
**memory** (facts) into lens **signals** (extracted activity). Revert
`memory_search` to **global memory only**. The cross-lens read stays where it
belongs — on the `signal_*` tools.

## Technical Approach
- `crates/arawn-engine/src/tools/memory_search.rs` — remove the `router` /
  all-lens enumeration (`memory_search.rs:147-185`); search the global memory
  store only. Drop the per-hit `lens` label / `scope=` cross-lens selector that
  no longer applies (a memory has no lens).
- Keep the cross-lens machinery (`arawn_memory::search_labeled_stores`,
  `all_lens_managers`) for the `signal_*` tools — do not delete it.
- Update the `memory_search` description: searches global facts/tunings.
- `/memory` summary (`local_service/memory.rs:80-102`) reduces to **global**
  memory facts (the lens-scoped count was the write-target lens, removed in T-B).

## Acceptance Criteria
- [ ] `memory_search` queries only the global memory store; no lens enumeration.
- [ ] `signal_search` / `signal_query` / `signal_timeline` remain cross-lens.
- [ ] `/memory` summarizes global memory facts only.
- [ ] `memory_search` description reflects global-facts scope.
- [ ] Unit tests updated (the I-0060 cross-lens memory_search test reverted); build
      + `angreal check workspace` green.

## Dependencies
Pairs with [[ARAWN-T-0436]] / [[ARAWN-T-0437]]. Partial revert of I-0060 T-B.

## Status Updates
*To be added during implementation*

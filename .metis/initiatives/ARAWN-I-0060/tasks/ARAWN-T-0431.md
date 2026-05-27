---
id: t-b-retarget-signal-search-memory
level: task
title: "T-B: Retarget signal_search / memory_search to cross-lens default"
short_code: "ARAWN-T-0431"
created_at: 2026-05-27T02:35:54.636734+00:00
updated_at: 2026-05-27T02:35:54.636734+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-B: Retarget signal_search / memory_search to cross-lens default

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

Make the read tools roam by default: `signal_search` and `memory_search` query
**across all lenses + global** (via [[ARAWN-T-0430]]) instead of the active lens,
with an optional `lens=<name>` to narrow to one. Drop the active-lens read
dependency.

## Type
Feature — `arawn-engine/src/tools/{signal,memory_search}.rs`.

## Technical Approach

- Default scope → cross-lens (`MemoryManager::search_all`). Add an optional
  `lens` arg that narrows to a single store (today's behavior, now opt-in).
- Remove the read-path call into `SessionLens`/`LensMemoryRouter.current()`;
  these tools no longer consult the active lens for reads.
- Results include each hit's source lens so the agent (and UI) can attribute it.
- Reconcile `memory_search`'s existing `scope` (global vs lens) with the new
  `lens=` selector (initiative Open Question) — likely: `scope=global|all|<lens>`.
- Update tool descriptions: "searches across all your lenses" not "the active
  lens".

## Acceptance Criteria

- [ ] With no args, both tools return hits spanning all lenses + global, labeled
      by source lens.
- [ ] `lens=<name>` narrows to that one store; `scope` semantics reconciled +
      documented in the tool schema.
- [ ] Neither tool reads the active-lens shim on the query path.
- [ ] Tests: a session with ≥2 lenses gets cross-lens hits by default; `lens=`
      narrows correctly.
- [ ] `angreal check workspace` + engine tests pass.

## Dependencies
Depends on [[ARAWN-T-0430]]. Pairs with [[ARAWN-T-0432]] (shim demotion).

## Status Updates

*To be added during implementation*

---
id: t-a-cross-lens-merged-read-in
level: task
title: "T-A: Cross-lens merged read in arawn-memory"
short_code: "ARAWN-T-0430"
created_at: 2026-05-27T02:35:53.211234+00:00
updated_at: 2026-05-27T02:35:53.211234+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-A: Cross-lens merged read in arawn-memory

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

Add the one new capability the roam model needs: a query in `arawn-memory` that
searches **all lens KBs + global** and returns one fused, ranked result set with
each hit labeled by its source lens. This is the foundation; T-B wires the tools
to it.

## Type
Feature — `arawn-memory` (+ lens enumeration from `arawn-storage`).

## Technical Approach

- Enumerate lenses from the registry (`LensStore::list`), resolve each KB path
  (`lenses/<dir>/memory.db`), open lazily and **cache the handles** (a user has a
  handful of lenses). Include the `global` store.
- Per store: run the existing FTS + vector search, then **RRF-merge** across
  stores into a single ranked list. Tag each hit with its originating lens
  (`global` for the global tier).
- Expose `MemoryManager::search_all(query, opts)` (or similar) alongside the
  current single-store path; keep the single-lens path for the `lens=` narrow
  case (T-B).
- Watch fusion fairness across stores of different sizes/embedders (see initiative
  Open Questions) — validate with the seeded multi-lens test.

## Acceptance Criteria

- [ ] A cross-lens search API returns fused, ranked hits across ≥2 lens stores +
      global, each carrying its source-lens label.
- [ ] Lens KB handles are opened lazily and cached (no re-open per query).
- [ ] The existing single-store query path still works (used by `lens=` narrow).
- [ ] Unit test seeds two lens stores with distinct entities and asserts a query
      surfaces hits from both, correctly labeled.
- [ ] `angreal check workspace` + `arawn-memory` tests pass.

## Dependencies
Root of the initiative; blocks [[ARAWN-T-0431]].

## Risk Considerations
- RRF normalization across heterogeneous stores — resolve in the multi-lens test.
- Opening many DBs: bounded by lens count; lazy + cached keeps it cheap.

## Status Updates

*To be added during implementation*

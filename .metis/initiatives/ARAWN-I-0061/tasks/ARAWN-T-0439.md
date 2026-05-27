---
id: t-d-lenses-read-from-global-memory
level: task
title: "T-D: Lenses read from global memory"
short_code: "ARAWN-T-0439"
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

# T-D: Lenses read from global memory

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
A lens reads from global **memory** to inform its behavior — known facts/tunings
should be available to the extractor's in-scope/ontology decisions (e.g. the
"people I manage" lens benefits from knowing who is managed). Make global memory
generally available to lens extraction.

## Technical Approach
- Extraction chain: `crates/arawn-extractor/src/cot.rs` Stage 1 (classify
  in-scope) and Stage 2 (ontology extract) should have access to relevant global
  memory facts as context.
- Decide the mechanism: inject a compact set of global memory facts into the
  classify/extract prompts, vs. a lookup the chain can call. Prefer the simplest
  that demonstrably influences in-scope decisions (avoid over-engineering).
- Keep it bounded — a handful of relevant facts, not the whole store.

## Acceptance Criteria
- [ ] The extraction chain can read global memory facts during classify/extract.
- [ ] A test shows a global memory fact influencing an in-scope/extraction outcome.
- [ ] No coupling that re-introduces per-lens memory writes (memory stays global).
- [ ] `angreal check workspace` + extractor tests green.

## Dependencies
Depends on [[ARAWN-T-0436]] (memory is global). Independent of the TUI tasks.

## Status Updates
*To be added during implementation*

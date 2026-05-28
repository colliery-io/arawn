---
id: t-d-lenses-read-from-global-memory
level: task
title: "T-D: Lenses read from global memory"
short_code: "ARAWN-T-0439"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T14:25:49.448651+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria
- [ ] The extraction chain can read global memory facts during classify/extract.
- [ ] A test shows a global memory fact influencing an in-scope/extraction outcome.
- [ ] No coupling that re-introduces per-lens memory writes (memory stays global).
- [ ] `angreal check workspace` + extractor tests green.

## Dependencies
Depends on [[ARAWN-T-0436]] (memory is global). Independent of the TUI tasks.

## Status Updates

**2026-05-28 — Done.** The CoT extraction chain now consults global memory.

Wiring:
- New `relevant_global_facts(kb, query, limit)` helper does FTS over
  `kb.global`. Sanitizes the query to alphanumeric tokens of length ≥3, ORs
  them, caps at 40 terms — FTS5 defaults to AND-across-terms and chokes on
  hyphens, so a raw `row.title + body` query (e.g. `subj-m1 …`) silently missed
  everything until this fix.
- `CotChain::run` builds the query from `row.title + row.body_text` (truncated),
  looks up up to 5 facts, passes them through to `classify` and `extract`.
- Both stages render a `Relevant known facts (from global memory):` block via
  `format_known_facts`; system prompts updated to acknowledge it.

Tier-routing correction (cleanup from T-A): the extractor wrote signals via
`default_scope()`, which T-A flipped to `Global` — so post-T-A signals were
landing in global memory and the lens-KB integration tests (`happy_path…`,
`two_lenses_each_get_the_entity`, `entity_dates_inherit_source_ts…`) panicked.
Stage 4 now hard-codes `Scope::Lens` for extracted entities — extractor signals
*are* the lens by definition, separate from the memory-write default.

Tests: two new integration tests prove the read path is live —
`global_memory_fact_reaches_classify_prompt` seeds a global fact whose content
phrase ("Dylan manages …") does not appear in the row body, and a
`ScopeGatedByPrompt` mock returns `in_scope: true` only when that phrase is in
the classify prompt. Companion `classify_without_global_facts_is_out_of_scope`
confirms the gate falls through to `false` without the seeded fact. Both green.
Refactored `runner_with` to a generic so multiple mock types share it.

Verification: `arawn-extractor` 30/0; `angreal check workspace` exit 0.
---
id: t-f-truth-up-tool-descriptions-and
level: task
title: "T-F: Truth-up tool descriptions and docs to the memory/signal/lens model"
short_code: "ARAWN-T-0441"
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

# T-F: Truth-up tool descriptions and docs to the memory/signal/lens model

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
Make every user/LLM-facing surface describe the corrected model — **memory =
global facts; signals = cross-lens extracted stream; lens = memory-aware standing
extractor, not a place you switch into.** Remove all "active lens scopes your
view" / write-target / "reads-all-writes-one" framing.

## Scope
- **Tool descriptions** (`crates/arawn-engine/src/tools/`):
  - `signal_query` (`signal.rs:256`), `signal_timeline` (`signal.rs:424`), module
    doc (`signal.rs:1-9`) — say signals are cross-lens, labeled by source lens
    (not "active lens").
  - `memory_search` / `memory_store` — global facts/tunings (aligns with T-A/T-C).
  - Remove `lens_switch` references; `lens_promote` reframed or removed if it no
    longer fits (it filed scratch into a lens — revisit under global memory).
  - steward / `lens_tag` / `lens_show` — "write-target lens" wording dropped.
- **Docs** (`docs/src`): rewrite `explanation/lenses.md` to the memory/signal/lens
  model (drop "reads-all/writes-one"); fix `explanation/memory-design.md:22`,
  `explanation/the-agent-loop.md:14-15` (persona/lens metadata),
  `reference/lens-tools.md:5`, `reference/agent-tools.md:75,99`,
  `explanation/identity-by-lens.md` body, `tutorials/first-lens.md:36`.
- Add/refresh a short "memory vs signals vs lenses" explainer so the distinction
  is documented once, canonically.

## Acceptance Criteria
- [ ] `rg -i "active (lens|workstream)|scoped to|write-target|reads-all" docs/src crates`
      returns only correct/intentional usages.
- [ ] `signal_*` descriptions say cross-lens + source-labeled; no "active lens."
- [ ] `lenses.md` leads with the memory/signal/lens model; no switching/write-target.
- [ ] `angreal docs build` + `angreal check workspace` clean.

## Dependencies
Follows the behavior tasks ([[ARAWN-T-0436]], [[ARAWN-T-0437]], [[ARAWN-T-0438]])
so docs describe shipped behavior.

## Status Updates
*To be added during implementation*

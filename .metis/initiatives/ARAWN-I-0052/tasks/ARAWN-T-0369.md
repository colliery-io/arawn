---
id: ceremonies-docs-rewrite
level: task
title: "Docs rewrite — ceremonies.md + ceremonies-tools.md cover back-fill, cadence, recovered flag"
short_code: "ARAWN-T-0369"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T18:55:47.326211+00:00
parent: ARAWN-I-0052
blocked_by: ["ARAWN-T-0366", "ARAWN-T-0367", "ARAWN-T-0368"]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#ceremonies"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0052
---

# Docs rewrite — ceremonies.md + ceremonies-tools.md

## Objective

Bring the ceremony docs in line with the code once T-0364–T-0368
have landed. Closes the back-fill doc-vs-code gap that the
I-0051 triple-check exposed (the docs were corrected to say
"back-fill is not implemented"; this task replaces that note
with the actual behaviour).

Blocks on the implementation tasks (T-0366 for back-fill,
T-0367 for cadence, T-0368 for the current-time header — T-0364
and T-0365 are intermediate and don't need user-facing docs of
their own).

## Scope

Files to rewrite or extend:

- `docs/src/explanation/ceremonies.md` — add a section on the
  "watch / catch up" model: pinned date windows, the 14-day
  recovery cap and its UX rationale, why retro is excluded
  from back-fill, the `recovered` flag and what to do with it.
- `docs/src/reference/ceremonies-tools.md` — document
  `retro_set_cadence`, the `[ceremonies.retro] cadence` knob,
  the `[ceremonies] backfill_lookback_days` knob, and the
  `recovered` field on tablet payloads.
- Remove or rewrite any leftover "back-fill is not
  implemented" notes.

## Acceptance criteria

- [ ] `ceremonies.md` explains pinned date windows, back-fill
  (incl. the 14-day rationale), and retro cadence in plain
  prose.
- [ ] `ceremonies-tools.md` documents every new knob and tool
  introduced by T-0366 / T-0367 / T-0368.
- [ ] No "not implemented" caveats remain for behaviour that
  this initiative ships.
- [ ] Cross-references to `ARAWN-I-0052` in the docs commit
  message; no Metis links in the docs body.
- [ ] `mdbook build` (via `angreal docs build`) succeeds.

Parent: [[ARAWN-I-0052]].

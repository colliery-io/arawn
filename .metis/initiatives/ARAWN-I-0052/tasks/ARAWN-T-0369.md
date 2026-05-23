---
id: docs-rewrite-ceremonies-md
level: task
title: "Docs rewrite — ceremonies.md + ceremonies-tools.md cover back-fill, cadence, recovered flag"
short_code: "ARAWN-T-0369"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T22:49:16.938191+00:00
parent: ARAWN-I-0052
blocked_by: [ARAWN-T-0366, ARAWN-T-0367, ARAWN-T-0368]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#ceremonies"
  - "#phase/completed"


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

- [x] `ceremonies.md` rewritten: replaced the old "back-fill
  not implemented" section with three new sections — Pinned
  date windows, Boot-time back-fill (incl. the 14-day
  rationale), and Retro cadence — plus a current-time header
  section.
- [x] `ceremonies-tools.md` documents `retro_set_cadence`, the
  `[ceremonies.retro] cadence` knob, the new `[backfill]
  ceremony_lookback_days` knob, the `recovered` flag, pinned
  windows, and the back-fill loop.
- [x] `config-schema.md` adds the `cadence` field to the
  ceremonies table and a new `[backfill]` section.
- [x] No "not implemented" / "on the roadmap" caveats remain
  for behaviour this initiative ships.
- [x] `angreal docs build` succeeds.

## Status Updates — 2026-05-19

Landed.

- `docs/src/explanation/ceremonies.md`: deleted the
  "What the recovery loop actually does today" paragraph
  with its roadmap caveat. Added new sections — *Pinned date
  windows*, *Boot-time back-fill* (with the why-14-days
  rationale), *What the nightly maintenance loop still does*,
  *Retro cadence*, *Current-time header*.
- `docs/src/reference/ceremonies-tools.md`: added
  `retro_set_cadence` row to the retro tools table; replaced
  the "Recovery" section with three sections — *Retro
  cadence*, *Back-fill* (incl. boot log line example), and
  *Pinned date windows*; the *Nightly maintenance* section
  now disambiguates against back-fill.
- `docs/src/reference/config-schema.md`: added `cadence` row
  to the ceremonies table and a new `[backfill]` section.
- `angreal docs build` → clean, no warnings.

Ready for review. Once accepted, I-0052 itself can transition
to completed (all 6 child tasks done).

Parent: [[ARAWN-I-0052]].
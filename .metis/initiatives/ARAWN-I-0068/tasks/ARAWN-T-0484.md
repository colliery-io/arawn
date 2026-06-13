---
id: extraction-operator-tooling-signal
level: task
title: "Extraction operator tooling — signal_explain / extract_rerun / signal_dismiss + per-row reasoning store"
short_code: "ARAWN-T-0484"
created_at: 2026-06-13T12:41:44.053214+00:00
updated_at: 2026-06-13T12:41:44.053214+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# Extraction operator tooling — signal_explain / extract_rerun / signal_dismiss + per-row reasoning store

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — spun out of [[ARAWN-T-0482]] (P2-7). T-0482 shipped the extraction *determinism* core (confidence floor, token filter, idempotent re-runs). This task is the operator *tooling* half, which needs new persisted state and is a clean separable sub-project.

## Objective **[REQUIRED]**

Give the user three agent tools to inspect and correct extraction, backed by a per-row extraction-reasoning store:
- **`signal_explain`** — why a given projection row was classified in/out of scope (the LLM's classify verdict + reason), and what entities/links it produced.
- **`extract_rerun <row|lens>`** — re-run extraction for a specific row (or reset a lens's cursor) so improved scope/prompts re-evaluate old rows. (`store_fact` dedup already makes re-runs idempotent — T-0481.)
- **`signal_dismiss <entity>`** — mark an extracted signal as garbage so it's removed AND not re-created on the next extraction pass.

**Why deferred from T-0482:** `signal_explain` requires persisting the per-row classify verdict + reason (not stored today), and `signal_dismiss` requires a per-row "dismissed" marker so re-extraction skips it. That's a new `extraction_log` table (per lens+row: run_id, outcome ok/empty/skipped, reason, dismissed) — a foundational piece the determinism fixes didn't need. Building it + three LLM tools + wiring is its own unit.

### Type
- [x] Feature — extraction observability/correction tooling

### Priority
- [x] P3 - Low (quality-of-life; the determinism core that prevents silent corruption already shipped in T-0482)

## Acceptance Criteria **[REQUIRED]**

- [ ] An `extraction_log` table records, per (lens, projection row): the run id, outcome (`ok` | `empty` | `skipped`), the classify reason, and a `dismissed` flag. Distinguishes "extracted empty" from "skipped" (the T-0482 acceptance bullet deferred here).
- [ ] The extractor chain writes a log row on every classify (in-scope / out-of-scope / empty).
- [ ] `signal_explain <row>` tool returns the stored verdict + reason + produced entities/links.
- [ ] `extract_rerun <row|lens>` resets the relevant cursor/log so the next pass re-evaluates; idempotent via existing `store_fact` dedup.
- [ ] `signal_dismiss <entity>` removes the entity and marks its source row dismissed so re-extraction won't recreate it.
- [ ] EXTRACTED_FROM provenance optionally stamped with the run id (entities already dedup via `store_fact`, so this is annotation, not dedup).
- [ ] Tests + `angreal check all` + `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Add the `extraction_log` table (arawn-storage migration or the projections DB). Have `CotChain::run` write a row per classify with the verdict/reason. Add the three tools in `arawn-engine/src/tools/` (mirror `signal.rs`), wire into the registry + `/` command surface. `extract_rerun` reuses `ExtractorCursorStore`; `signal_dismiss` reuses memory forget/supersede + sets the log's `dismissed` flag (checked by the chain to skip re-creation).

### Dependencies
Builds on [[ARAWN-T-0482]] (determinism core, done) and [[ARAWN-T-0476]] (extraction cursors already surface in `/status`).

### Risk Considerations
The `dismissed` flag must be consulted by the chain BEFORE re-extraction, else a dismissed signal reappears. Keep the log writes best-effort (a logging failure must not fail extraction).

## Status Updates **[REQUIRED]**

*To be added during implementation*
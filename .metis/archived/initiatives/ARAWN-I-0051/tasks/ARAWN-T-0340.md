---
id: phase-c-8-reference-data-model
level: task
title: "Phase C-8: Reference — data model (feeds-overview, templates, projections, palaces, memory, workflows)"
short_code: "ARAWN-T-0340"
created_at: 2026-05-19T01:39:44.912894+00:00
updated_at: 2026-05-19T02:45:52.172342+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-8: Reference — data model (feeds-overview, templates, projections, palaces, memory, workflows)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Reference pages for the data model: feed mechanics + on-disk layout, the (now 17, not 12) feed template catalog including four new GitHub templates, projection schemas, palace type catalog, memory model tables, and workflow tools catalog. Split the existing `palaces/*` and `feeds/index.md` content cleanly — reference here, explanation goes to C-10.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `reference/feeds-overview.md` — on-disk layout, `meta.json`, 5 status states, cadence table, backfill mechanics, `/feeds` subcommands, disk-usage estimates. ~95 lines.
- [x] `reference/feed-templates.md` — UPDATED to "Sixteen templates ship today" (was "Twelve"). Added GitHub section: notifications, issues-and-prs, review-queue, repo-mirror — each with params, cadence, on-disk shape. ~290 lines now (was 275).
- [x] `reference/projection-tables.md` — shared columns + per-table metadata (9 existing tables + 4 GitHub tables: notifications/issues/prs/reviews) + ProjectionRow struct + embedding pass mechanics + when-to-read-which-layer table. ~110 lines.
- [x] `reference/palace-types.md` — 6 entity types (3 scope-locked global vs 3 workstream) + 8 relation types (incl. special `extracted_from` / `summarizes`) + tag ontology + entity fields + confidence levels. ~85 lines.
- [x] `reference/memory-model.md` — two-store split, scope-locking, FTS-vs-vector retrieval, embedder install location, agent surface (`memory_store`/`memory_search`), slash commands. ~75 lines.
- [x] `reference/workflow-tools.md` — 4 `workflow_*` tools + JSON spec + 3 task types + cron syntax + storage layout + caveats (task bodies run unsandboxed). ~115 lines.
- [x] `docs/src/memory.md` deleted (content → memory-model.md + explanation/memory-design.md in C-10).
- [x] `docs/src/workflows.md` deleted (content → workflow-tools.md + explanation/workflows.md in C-10 + how-to/author-a-workflow-by-hand.md in C-4).
- [x] `docs/src/feeds/index.md` deleted (content → feeds-overview.md + explanation/feeds.md in C-10). `feeds/` directory removed.
- [x] `docs/src/palaces/index.md` deleted (content → palace-types.md + explanation/palaces.md in C-10).
- [x] `docs/src/palaces/projections.md` deleted (content → projection-tables.md + explanation/projections.md in C-10).
- [x] `SUMMARY.md` updated — references the new flat reference pages, no longer points at any of the deleted source files.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

`palaces/extraction.md` intentionally left in place — its content is overwhelmingly explanation (4-stage CoT, two-tag rationale, UAT war story). C-10 picks it up and moves to `explanation/extraction.md`. It's currently orphaned from SUMMARY (mdbook doesn't include unlinked files in the rendered book), which is acceptable mid-flight.

Headline drift fixes:
- "Twelve templates ship today" → "Sixteen templates ship today" with the 4 GitHub entries (notifications, issues-and-prs, review-queue, repo-mirror) and 4 new projection table types.
- The `memory.md` "Direct access (work-in-progress)" disclaimer is gone — `/remember`, `/memory`, `/forget` listed as live in `memory-model.md`.
- The `extracted_from` and `summarizes` relations called out as "never removed" / "DETACH DELETE on rollback" — both were buried prose in palaces/index.md / palaces/steward.md.
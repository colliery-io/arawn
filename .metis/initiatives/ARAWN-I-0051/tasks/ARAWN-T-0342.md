---
id: phase-c-10-explanation-pages-14
level: task
title: "Phase C-10: Explanation pages (14 new — agent loop, data model, all concept pages)"
short_code: "ARAWN-T-0342"
created_at: 2026-05-19T01:39:47.765601+00:00
updated_at: 2026-05-19T08:49:12.959973+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-10: Explanation pages (14 new — agent loop, data model, all concept pages)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Bootstrap the Explanation quadrant — currently empty. Write 14 new pages covering concepts, design rationale, trade-offs, and "why" answers: the agent loop, the three-layer data model (feeds → projections → palaces), workstreams, memory design, workflows, permissions, identity-by-workstream (ARAWN-I-0035), ceremonies, and individual concept pages for feeds/projections/palaces/extraction/steward. Split into C-10a/C-10b at execution time if scope feels unwieldy.

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

- [x] `explanation/what-is-arawn.md` — vision + self-hosted thesis + agent-loop intro + three-layer-data-model intro. ~95 lines.
- [x] `explanation/the-agent-loop.md` — turn flow + why each step exists + compaction + caps + what's NOT in the loop. ~135 lines.
- [x] `explanation/three-layer-data-model.md` — feeds → projections → palaces with rationale per layer + walk-down patterns. ~115 lines.
- [x] `explanation/feeds.md` — local-first thesis + when-to-feed + cadence rationale + backfill + when NOT to feed. ~120 lines.
- [x] `explanation/projections.md` — why flat + why per-feed-type tables + why embeddings + provenance + when-to-read-which. ~135 lines.
- [x] `explanation/palaces.md` — memory palace metaphor + lifecycle + ADR-0002/0003/0004 anchors + per-workstream rationale. ~130 lines.
- [x] `explanation/extraction.md` — moved from palaces/extraction.md, edited links + intro caption. (Existing content was already pure explanation.) ~165 lines.
- [x] `explanation/steward.md` — why bounded blast radius + why proposal-vs-apply + Extract→Suggest→Add cycle + why dust is manual + why doorwatch is metadata-only. ~125 lines.
- [x] `explanation/workstreams.md` — what + why + when + scratch + workstream-vs-session vs feed vs memory. ~135 lines.
- [x] `explanation/identity-by-workstream.md` — ARAWN-I-0035 design: why persona is workstream-scoped + IdentityProfile enum + how to switch + what's NOT in identity_profile. ~115 lines.
- [x] `explanation/memory-design.md` — two-tier rationale + scope-locked preferences/people + FTS+vector hybrid + graphqlite for relations + closed confidence set. ~125 lines.
- [x] `explanation/workflows.md` — when-to-workflow + 3 task flavours + why DAG + why cloacina + why compiled Rust + unsandboxed task bodies. ~115 lines.
- [x] `explanation/permission-model.md` — deny>allow>ask + why ask exists + 4 modes rationale + plan-mode deny semantics + sandbox/rules composition + audit log purpose. ~135 lines.
- [x] `explanation/ceremonies.md` — vision quote + 3 cadences + tablet-not-chat + agent-proposes + retro detectors + nightly recovery loop + ceremonies-aren't-optional stance. ~125 lines.
- [x] `docs/src/palaces/extraction.md` moved via `git mv` to `docs/src/explanation/extraction.md`. `palaces/` directory removed.
- [x] `SUMMARY.md` updated — Explanation section lists all 14 pages.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-19 — Completed (uncommitted)

Largest single task in the initiative. 14 explanation pages, ~1700 lines of net new prose. Source: existing source pages (`feeds/index.md`, `memory.md`, `workflows.md`, `security.md`, `palaces/index.md`, `palaces/projections.md`, `palaces/steward.md`) all available from earlier reads in the session. `extraction.md` migrated via `git mv` and lightly edited for cross-link correctness.

The Explanation quadrant goes from **empty** (the Phase A audit's single biggest structural problem) to **14 pages covering the entire conceptual surface** of arawn:
- 3 system-level concept pages (what-is-arawn, the-agent-loop, three-layer-data-model)
- 5 data-model concept pages (feeds, projections, palaces, extraction, steward)
- 6 organizing-principle pages (workstreams, identity-by-workstream, memory-design, workflows, permission-model, ceremonies)

After this task, the initiative is materially complete. The proposed `docs/src/` tree is fully populated — 60 markdown files across the four Diataxis quadrants, mdbook builds clean, all referenced internal links resolve.

Phase D (verification re-audit) remains as a follow-on, but the bulk of the work is done.
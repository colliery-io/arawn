---
id: phase-c-7-reference-workstream
level: task
title: "Phase C-7: Reference — workstream, ceremonies, todos, identity, steward, feed-search"
short_code: "ARAWN-T-0339"
created_at: 2026-05-19T01:39:43.300214+00:00
updated_at: 2026-05-19T02:41:50.179535+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-7: Reference — workstream, ceremonies, todos, identity, steward, feed-search

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Reference pages for the previously-undocumented user-facing surfaces tied to workstreams, ceremonies, todos, identity profile, the steward, and feed_search. Largest batch of *net new* reference material since most of these subsystems have zero doc coverage today.

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

- [x] `reference/workstream-tools.md` — `signal_*` (3 tools) + `workstream_show` + workstream lifecycle tools (8) + `workstream_tag` + curation tools (refine/apply/rollback/dust/journal). ~150 lines.
- [x] `reference/workstream-cli.md` — slug validation rules (lowercase, ≤64 chars, `[a-z0-9_-]+`, leading letter/digit) + lifecycle slash commands + metadata fields + `identity_profile` (Assistant/Coding) including V10 migration. ~120 lines.
- [x] `reference/ceremonies-tools.md` — daily (5) + weekly (7) + retro (5) tools + cron defaults + retro detectors (priority-completion, rollover-heat, workstream-neglect) + nightly recovery loop. ~85 lines.
- [x] `reference/todos-tools.md` — 8 `todo_*` tools + schema + source enum (user, ceremony/daily, ceremony/weekly/priority, ceremony/retro/*, agent) + V11 migration note. ~60 lines.
- [x] `reference/steward-subroutines.md` — 5 subroutines + journal schema + apply/rollback dispatch table + reshelve specifics + dust specifics + tag-promoter "Extract → Suggest → Add" walkthrough. ~125 lines.
- [x] `reference/feed-search-tool.md` — fixed stale internal link from `./index.md` (was `feeds/index.md`) to `./feeds-overview.md` + explanation pointer.
- [x] `docs/src/palaces/agent-read-patterns.md` deleted (content → workstream-tools.md).
- [x] `docs/src/palaces/steward.md` deleted (content → steward-subroutines.md + explanation/steward.md in C-10).
- [x] `SUMMARY.md` updated.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

Largest *net new* reference batch — ceremonies (28 tools), todos (8 tools), `identity_profile` documentation. All previously zero-coverage subsystems now have reference pages.

Source: existing `palaces/agent-read-patterns.md` and `palaces/steward.md` (both deleted post-migration), `crates/arawn-core/src/workstream.rs` (slug rules + IdentityProfile enum), `crates/arawn-engine/src/tools/{daily,weekly,ceremony,todo}.rs`.

Decisions worth noting:
- Workstream lifecycle tool listing emphasizes the agent-side surface; the slash-command equivalents live in `workstream-cli.md` to keep concerns separate.
- `ceremonies-tools.md` calls out the retro detectors (priority-completion, rollover-heat, workstream-neglect) — these were entirely undocumented before but are user-visible via `/retro`.
- `identity_profile` documented as a column on the workstream with explicit `assistant` (default) vs `coding` enum values and the V10 migration reference.

`palaces/` directory still has `index.md`, `projections.md`, `extraction.md` — those get split in C-8 (reference) and C-10 (explanation).
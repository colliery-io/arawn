---
id: phase-c-1-doc-tree-scaffold
level: task
title: "Phase C-1: Doc tree scaffold + SUMMARY rewrite + landing pages"
short_code: "ARAWN-T-0333"
created_at: 2026-05-19T01:39:33.806962+00:00
updated_at: 2026-05-19T02:12:16.223496+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-1: Doc tree scaffold + SUMMARY rewrite + landing pages

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Create the new four-quadrant directory structure under `docs/src/` (`tutorials/`, `how-to/`, `reference/`, `explanation/`), move pages whose new home doesn't require splitting, write the new `SUMMARY.md`, and add a landing `index.md` for each quadrant. Split-required pages stay in place — C-2…C-10 will read from them and delete the originals once content is migrated.

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

- [x] Directories created: `docs/src/{tutorials,how-to,reference,explanation}/`
- [x] Landing pages exist with quadrant orientation: `tutorials/index.md`, `how-to/index.md`, `reference/index.md`, `explanation/index.md`
- [x] Three clean moves done via `git mv` (rename tracked, not delete+add):
  - `feeds/template-catalog.md` → `reference/feed-templates.md`
  - `feeds/feed-search.md` → `reference/feed-search-tool.md`
  - `feeds/agent-read-patterns.md` → `how-to/read-feeds-with-the-agent.md`
- [x] `SUMMARY.md` rewritten with four top-level groups (Tutorials, How-to guides, Reference, Explanation) plus an interim "Integrations" group; references moved pages in their new locations and existing-but-unsplit pages in current locations
- [x] `angreal docs build` succeeds — clean mdbook build, no broken links
- [ ] Single commit (deferred — operator's call)

## Status Updates

### 2026-05-18 — Completed (uncommitted)

Scaffold built end-to-end:

- Created 4 quadrant directories.
- Wrote 4 landing pages (~30-60 lines each) explaining each Diataxis quadrant and cross-linking to siblings.
- Moved 3 pages via `git mv` (renames tracked correctly in `git status`).
- Rewrote `SUMMARY.md` with the new 4-section structure. Existing unsplit pages (`security.md`, `memory.md`, `workflows.md`, `feeds/index.md`, `palaces/*.md`, `integrations/*.md`, `getting-started.md`) remain in their current paths under the appropriate quadrant headers; later C-tasks (C-2 → C-10) read from them, write new pages, and delete the originals.
- `angreal docs build` exits 0 with no warnings.

`docs/src/getting-started.md` is currently parked under the **Tutorials** header even though it's mostly how-to material — expected. C-2 splits it: tutorial core moves to `tutorials/first-chat.md`; OAuth recipes move to `how-to/connect-*.md` in C-3; troubleshooting moves to `reference/troubleshooting.md` in C-5.

Commit deferred to operator. Suggested message: `docs(I-0051): Phase C-1 — scaffold Diataxis tree + SUMMARY rewrite`.
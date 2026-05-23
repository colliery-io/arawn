---
id: phase-c-4-how-tos-usage-recipes-8
level: task
title: "Phase C-4: How-tos — usage recipes (8 pages)"
short_code: "ARAWN-T-0336"
created_at: 2026-05-19T01:39:38.295613+00:00
updated_at: 2026-05-19T02:29:04.703488+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-4: How-tos — usage recipes (8 pages)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Write 8 task-oriented how-to recipes covering the most common user goals: create a feed, bind a workstream to a feed, curate a workstream (refine/apply/rollback), read feeds with the agent, lock down permissions, hand-author a workflow, debug OAuth failures, and recover from LLM warmup failure. Each page: single concrete goal, prerequisites, steps, verification.

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

- [x] `how-to/create-a-feed.md` — auto-create on `/connect` + `/watch` recipe + `since=` backfill + `/feeds` subcommands (rm, pause, resume, run). Drift fix: `/feeds rm` not `/unwatch`. ~85 lines.
- [x] `how-to/bind-a-workstream-to-a-feed.md` — direct feed_id bind + `github:repo:` + `github:org:` URI schemes + org-supersedes-repo semantics + hot-register. ~80 lines.
- [x] `how-to/curate-a-workstream.md` — refine → apply → rollback flow + the three common subroutines (tag-promoter, relation-suggester, dust-summarizer) + journal review. ~80 lines.
- [x] `how-to/read-feeds-with-the-agent.md` — H1 + intro reworded from "Agent Read Patterns" reference framing to how-to framing. Body content (the 10 prompt-recipes) preserved from the C-1 move.
- [x] `how-to/lock-down-permissions.md` — three preset setups (paranoid / hands-off CI / strict review) + `/accept` runtime mode switch. ~75 lines.
- [x] `how-to/author-a-workflow-by-hand.md` — JSON spec walkthrough + task flavours + `workflow_create` lifecycle + caveats. ~85 lines.
- [x] `how-to/debug-oauth-failures.md` — 7 common errors expanded (redirect_uri_mismatch, access_denied, insufficient_scope, invalid_grant, Connection error, scope mismatch, port 8080 conflict) + verbose-logging escape hatch. ~110 lines.
- [x] `how-to/recover-from-llm-warmup-failure.md` — match-your-error-body table + "no API key set" + "embedding model unavailable" + TUI connection-refused. ~70 lines.
- [x] `SUMMARY.md` updated — all 8 pages listed under How-to guides.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

All 8 how-tos written. Source content pulled from:
- `docs/src/security.md` (Limiting blast radius → lock-down-permissions.md)
- `docs/src/workflows.md` (JSON walkthrough → author-a-workflow-by-hand.md)
- `docs/src/feeds/index.md` (create + backfill mechanics → create-a-feed.md)
- `HEAD~3:docs/src/getting-started.md` (Common integration errors → debug-oauth-failures.md; Troubleshooting LLM section → recover-from-llm-warmup-failure.md)
- New prose for bind-a-workstream-to-a-feed.md and curate-a-workstream.md (built from the completeness audit's enumeration of workstream operations + steward subroutines)

Drift fixes applied: `/feeds rm` not `/unwatch`; `workstream_refine/apply/rollback` named correctly; the four GitHub templates and the org-supersedes-repo semantics from T-0322/T-0327 reflected in bind-a-workstream-to-a-feed.

Source files (`security.md`, `workflows.md`, `feeds/index.md`) are NOT deleted by this task — they still contain reference content that C-5 (troubleshooting/permissions), C-7 (steward/feed-search), and C-8 (feeds-overview/workflow-tools) split out. Final deletion happens in those later tasks.
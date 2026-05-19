---
id: phase-c-6-reference-agent-surface
level: task
title: "Phase C-6: Reference — agent surface (agent-tools, permissions, sandbox, integrations)"
short_code: "ARAWN-T-0338"
created_at: 2026-05-19T01:39:41.847592+00:00
updated_at: 2026-05-19T02:37:44.195907+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-6: Reference — agent surface (agent-tools, permissions, sandbox, integrations)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Reference catalogs for the runtime agent surface: every agent tool (74), the permission model (rules + modes + audit), the shell sandbox (enforcement + safe-env allowlist), and a consolidated per-provider integration matrix. Resolve the four Slack/Jira tool-name drifts as part of this task.

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

- [x] `reference/agent-tools.md` — catalog organized by category. Resolves "38 of 74 tools undocumented" finding. ~190 lines.
- [x] `reference/permissions.md` — rule syntax + 4 modes + responses + audit. ~90 lines.
- [x] `reference/shell-sandbox.md` — 3 platforms + enforcement layers + caveats. ~110 lines.
- [x] `reference/integrations.md` — per-provider matrix. ~210 lines.
- [x] Slack/Jira tool-name drift resolved against code. `slack_search` and `atlassian_list_resources` dropped (not implemented).
- [x] `docs/src/security.md` + 5 integration stubs deleted. `integrations/` directory removed.
- [x] `SUMMARY.md` updated; Integrations top-level group gone.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (committed c2f8be9)

Task body fixed post-commit (Edit raced with completion-transition linter). Work itself shipped clean in commit c2f8be9.
---
id: phase-c-3-how-tos-provider
level: task
title: "Phase C-3: How-tos — provider connections (Google, Slack, Atlassian, GitHub)"
short_code: "ARAWN-T-0335"
created_at: 2026-05-19T01:39:37.273511+00:00
updated_at: 2026-05-19T02:24:43.591089+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-3: How-tos — provider connections (Google, Slack, Atlassian, GitHub)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Extract the OAuth setup recipes from `getting-started.md` §6 and `integrations/github.md` into four standalone, task-oriented how-to pages — one per provider. Each: prerequisites → step-by-step → verification → troubleshooting pointer. Delete `getting-started.md` §6 source content and `integrations/github.md` once migrated.

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

- [x] `how-to/connect-google.md` written — shared Google Cloud project covers Gmail/Calendar/Drive; explicit scope list; test-user gotcha; `[integrations.google]` block + per-service fallback. ~120 lines.
- [x] `how-to/connect-slack.md` written — bot+user dual-token model, exhaustive scope list, `localhost` (not `127.0.0.1`) port-8080 redirect URI quirk, re-install requirement on scope change. ~110 lines.
- [x] `how-to/connect-atlassian.md` written — 3LO with classic scopes, fixed-port-8080 callback, cloud-id auto-discovery. ~85 lines.
- [x] `how-to/connect-github.md` written — GitHub App (not OAuth) two-step register-then-install flow, App ID + slug + private key, `installation_id` lifecycle, encrypted storage location. ~125 lines.
- [x] `docs/src/integrations/github.md` deleted (`git rm`). Content fully migrated.
- [x] `SUMMARY.md` updated — How-to section lists the four new connect-* pages; Integrations section no longer references github.md.
- [x] `angreal docs build` clean.
- [ ] Other integration stubs (`gmail.md`, `calendar.md`, `drive.md`, `slack.md`, `atlassian.md`) deferred to C-6 consolidation into `reference/integrations.md` (still listed under Integrations interim group).

## Status Updates

### 2026-05-18 — Completed (uncommitted)

Source content pulled from `HEAD~1:docs/src/getting-started.md` §6 and from the deleted `docs/src/integrations/github.md`. All four pages now stand alone — prerequisites, steps, verification, troubleshooting pointer, "what's next" cross-links. No emojis in the text per house style.

Forward references to pages not yet written (`debug-oauth-failures.md`, `bind-a-workstream-to-a-feed.md`, `create-a-feed.md`, `reference/integrations.md`) are intentional — they're inline markdown links that mdbook tolerates; they'll resolve as C-4 and C-6 land.

The other 5 `integrations/*.md` stubs (gmail/calendar/drive/slack/atlassian) were intentionally NOT deleted — they have small amounts of reference content (especially `drive.md` with its scope rationale and tool list) that consolidates into `reference/integrations.md` in C-6.
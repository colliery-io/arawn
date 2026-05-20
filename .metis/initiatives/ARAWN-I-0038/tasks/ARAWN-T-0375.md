---
id: integration-docs-phase-4-atlassian
level: task
title: "Integration docs Phase 4 — Atlassian how-to rewrite"
short_code: "ARAWN-T-0375"
created_at: 2026-05-20T16:00:00+00:00
updated_at: 2026-05-20T16:00:00+00:00
parent: ARAWN-I-0038
blocked_by: ["ARAWN-T-0372"]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0038
---

# Phase 4 — Atlassian how-to rewrite

## Objective

Rewrite `docs/src/how-to/connect-atlassian.md` covering Jira +
Confluence access via Atlassian's 3LO OAuth flow. The
`accessible-resources` / `cloud_id` discovery dance is the
biggest gotcha beyond standard OAuth.

## Scope

REWRITE `docs/src/how-to/connect-atlassian.md` following the
Phase 2 template:

1. **What you get** — Jira tools (`jira_search`, `jira_get_issue`,
   etc.) + Confluence tools (`confluence_search`, etc.). From
   `crates/arawn-integrations/src/atlassian/`.
2. **Prerequisites** — Atlassian developer.atlassian.com
   account; access to the target Jira/Confluence cloud site.
3. **Setup**:
   - Create OAuth 2.0 (3LO) app at developer.atlassian.com.
   - Add scopes (`read:jira-work`, `write:jira-work`, etc. —
     verify exact strings from code).
   - Set callback URL: confirm against the auth crate.
   - Set distribution to private — public-distribution review
     not needed for self-use.
4. **Configure arawn** — TOML snippet (Jira + Confluence
   share the same OAuth app), env-var alternative.
5. **Connect** — `/connect atlassian` triggers the consent
   flow, then arawn calls `accessible-resources` to discover
   the user's cloud_id and persists it alongside the token.
6. **Verification** — minimal exercise prompt for both Jira
   and Confluence.
7. **Troubleshooting** — link to Phase 5; Atlassian-specific
   notes (insufficient_scope after scope change requires
   re-connect, expired token error vs revoked token).

## Correctness mandate

- 3LO scope strings: grep `atlassian/` for the exact scope
  list requested.
- `accessible-resources` API call: verify it's actually made
  during connect — cite the file:line in a comment if useful.
- cloud_id storage: where + how (`<data>/integrations/atlassian/`
  or similar) — verify against the integration crate.
- Tool name list: from `Tool::name()` impls.

## Acceptance criteria

- [ ] `connect-atlassian.md` rewritten following Phase 2
  template.
- [ ] Scope list grep-able in the Atlassian integration crate.
- [ ] `accessible-resources` / cloud_id flow explained
  accurately, anchored in code.
- [ ] Both Jira and Confluence tool lists are present.
- [ ] `angreal docs build` clean.
- [ ] User docs-UAT: follow on a fresh Atlassian site to a
  working `/connect atlassian`.

Parent: [[ARAWN-I-0038]].

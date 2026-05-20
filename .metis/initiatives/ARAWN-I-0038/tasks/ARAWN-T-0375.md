---
id: integration-docs-phase-4-atlassian
level: task
title: "Integration docs Phase 4 — Atlassian how-to rewrite"
short_code: "ARAWN-T-0375"
created_at: 2026-05-20T16:00:00+00:00
updated_at: 2026-05-20T20:40:41.512736+00:00
parent: ARAWN-I-0038
blocked_by: [ARAWN-T-0372]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/active"


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

- [x] `connect-atlassian.md` rewritten with the full scope set
  and the cloud_id discovery flow accurately documented.
- [x] All 13 scopes grep-verified against
  `crates/arawn-integrations/src/atlassian/integration.rs:29-47`
  (`ATLASSIAN_OAUTH_SCOPES`):
  - 3 Jira (`read:jira-work`, `write:jira-work`, `read:jira-user`)
  - 5 Confluence classic (incl. `search:confluence`)
  - 4 Confluence granular v2 (`read:space:confluence`,
    `read:page:confluence`, `write:page:confluence`,
    `read:content-details:confluence`)
  - 1 `offline_access` for refresh tokens
- [x] `accessible-resources` discovery flow explained in step 7
  with the right URL (`https://api.atlassian.com/oauth/token/accessible-resources`)
  and what gets persisted (cloud_id list in the encrypted token
  store's `extras` field).
- [x] Tool counts noted (6 Jira + 5 Confluence = 11) matching the
  per-tool `*_SCOPES` constants in `jira.rs` / `confluence.rs`.
- [x] Redirect URI `http://localhost:8080/oauth/callback` —
  `localhost` not `127.0.0.1`, code-anchored.
- [x] Classic-vs-granular Confluence scope distinction explained
  prominently — this is the correctness fix that matters most for
  the docs-UAT.
- [x] `audience=api.atlassian.com` extra param + `offline_access`
  refresh-token requirement called out so users understand why so
  many scopes.
- [x] Five `VERIFY` markers on developer.atlassian.com console UI
  navigation paths.
- [x] `angreal docs build` clean.
- [ ] User docs-UAT pending (tonight).

## Correctness fix worth flagging in commit

The previous `connect-atlassian.md` listed **7 scopes**; arawn
actually requests **13**. Users following the old doc were
silently missing:
- 4 granular Confluence v2 scopes → v2 page tools 401.
- `search:confluence` → Confluence search broken.
- `offline_access` → no refresh token issued → connection breaks
  after ~1 hour.

That's the single biggest doc-vs-code drift the I-0038 audit
caught.

Parent: [[ARAWN-I-0038]].
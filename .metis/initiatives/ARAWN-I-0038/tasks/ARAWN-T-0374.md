---
id: integration-docs-phase-3-slack-how
level: task
title: "Integration docs Phase 3 — Slack how-to rewrite"
short_code: "ARAWN-T-0374"
created_at: 2026-05-20T16:00:00+00:00
updated_at: 2026-05-20T21:02:33.908364+00:00
parent: ARAWN-I-0038
blocked_by: [ARAWN-T-0372]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0038
---

# Phase 3 — Slack how-to rewrite

## Objective

Rewrite `docs/src/how-to/connect-slack.md` covering the full
single-workspace setup. Multi-workspace is out of scope
(I-0034).

## Scope

REWRITE `docs/src/how-to/connect-slack.md` following the
Diátaxis how-to template established by Phase 2:

1. **What you get** — Slack tools (channel list, search,
   post, react, etc.). Names from
   `crates/arawn-integrations/src/slack/`.
2. **Prerequisites** — Slack workspace + workspace-admin role
   note (required to install apps).
3. **Setup**:
   - Create the Slack app at api.slack.com/apps.
   - Add bot + user scopes (separately).
   - Add redirect URL: `http://localhost:<port>/callback` —
     **note the host quirk**: Slack rejects `127.0.0.1`,
     accepts `localhost`. Confirm against
     `crates/arawn-auth/src/oauth.rs`.
   - Install to workspace.
4. **Configure arawn** — TOML snippet, env-var alternative,
   fixed-port mode note (Slack requires the redirect port to
   match exactly).
5. **Connect** — `/connect slack`.
6. **Verification** — minimal exercise prompt.
7. **Troubleshooting** — link to Phase 5; Slack-specific
   notes: workspace-admin caveat (non-admins can't install),
   bot-vs-user scope distinction.

## Correctness mandate

- Bot scopes vs user scopes vs both — verify against
  `slack/tools.rs` and the OAuth client config which scopes we
  actually request. Don't list scopes we don't need.
- Redirect URI: `localhost` (not `127.0.0.1`) — confirm via
  code before stating as a hard rule.
- Fixed-port mode: confirm the callback server uses a fixed
  port for Slack vs ephemeral port for Google.
- Tool name list: from `Tool::name()` impls in the Slack
  integration crate.
- Workspace-admin caveat: come from Slack's docs, mark
  `VERIFY` if not in our code.

## Acceptance criteria

- [x] `connect-slack.md` rewritten, structurally aligned with
  Phase 2's template (cross-links, env-var alt, VERIFY markers).
- [x] All 16 bot scopes + 10 user scopes grep-verified against
  `crates/arawn-integrations/src/slack/integration.rs` (`SLACK_OAUTH_SCOPES`
  + `SLACK_OAUTH_USER_SCOPES`). Counts match.
- [x] Redirect URI `http://localhost:8080/oauth/callback`
  code-verified — `arawn-auth/src/server.rs:69-73` emits the
  `localhost` host string deliberately; `slack/integration.rs:107`
  pins port 8080 via `DEFAULT_SLACK_REDIRECT_PORT`. Reconciled
  the previously-conflicting code comment in
  `slack/integration.rs:98,104` (says `127.0.0.1` but
  `localhost` is what the server actually presents).
- [x] Bot-vs-user dual-token model explained at the scope
  list, with a callout box.
- [x] Workspace-admin caveat in Prerequisites + Step 4 —
  not arawn-logged (it's a Slack-side error from the install
  flow); flagged as the "Workspace admin needed" note.
- [x] Env-var alternative (`ARAWN_SLACK_CLIENT_*`) added with
  cross-link to integrations-config.md.
- [x] Five VERIFY markers on Slack admin UI nav (app creation
  / Scopes / Redirect URLs / Install / Credentials section
  paths).
- [x] `angreal docs build` clean.
- [ ] User docs-UAT pending (tonight).

Parent: [[ARAWN-I-0038]].
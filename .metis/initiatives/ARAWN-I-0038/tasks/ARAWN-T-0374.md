---
id: integration-docs-phase-3-slack
level: task
title: "Integration docs Phase 3 — Slack how-to rewrite"
short_code: "ARAWN-T-0374"
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

- [ ] `connect-slack.md` rewritten following the Phase 2
  template shape.
- [ ] Scope list grep-able in the Slack integration crate.
- [ ] Redirect URI host (`localhost`) and fixed-port behaviour
  documented and code-verified.
- [ ] Bot-vs-user scope split explained.
- [ ] Workspace-admin caveat noted (with `VERIFY` if not in
  arawn's logs).
- [ ] `angreal docs build` clean.
- [ ] User docs-UAT: follow on a fresh Slack workspace to a
  working `/connect slack`.

Parent: [[ARAWN-I-0038]].

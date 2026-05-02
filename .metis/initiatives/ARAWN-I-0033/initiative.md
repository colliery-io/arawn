---
id: external-integrations-layer-gmail
level: initiative
title: "External integrations layer — Gmail, Calendar, notifications"
short_code: "ARAWN-I-0033"
created_at: 2026-05-02T04:13:45.230690+00:00
updated_at: 2026-05-02T04:13:45.230690+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: external-integrations-layer-gmail
---

# External integrations layer — Gmail, Calendar, notifications

## Vision Alignment

A general-purpose AI assistant that lives in your terminal needs to reach the systems you actually run your day from — email, calendar, chat. Today arawn can read files and run shells; it can't see what's in your inbox or post a message to your team. Closing this gap is the difference between "smart REPL" and "useful assistant."

## Problem Statement

Arawn has the plumbing for OAuth (`crates/arawn-auth/` ships with token store, OAuth2, server). No tool uses it. The agent has no way to read mail, check calendar, or send notifications. Users wanting any of these capabilities have to fall back to shell + curl + manually managing tokens — which defeats the purpose of having an assistant.

## Goals

- A general integration trait/pattern in `arawn-tool` (or similar) that future external services can plug into without re-inventing OAuth, token storage, and TUI consent flow each time.
- Three concrete integrations shipped: Gmail (read + search + send), Google Calendar (read upcoming + create event), and one notification channel (Slack DM via webhook OR proper API — pick during design).
- A first-run OAuth UX in the TUI: "/connect gmail" prompts the user, opens a browser, captures the callback, stores the refresh token, surfaces success/failure clearly.
- Integrations respect the existing permissions/sandbox story (per T-0196): user can deny per-tool, see audit of what was accessed.

## Non-Goals

- Building a generic "any-OAuth-provider" framework. Three integrations cover the common shape; over-generalizing now is a trap.
- Mailbox sync / local caching of email or calendar data. Read-through is fine for MVP.
- Multi-account support (one Gmail account per arawn install at first).

## Approach

Phased decomposition (tasks to be created during decompose phase):

1. **Discovery / ADR**: integration trait pattern, where credentials live (filesystem? OS keychain? both?), how OAuth tokens get refreshed, how the TUI brokers the consent flow. Lands as an ADR.
2. **Auth UX foundations**: extend `arawn-auth` to expose a per-service OAuth flow callable from the engine, plus a TUI "connect / disconnect / status" surface.
3. **Gmail tool**: read inbox, search, send, mark read. Built against the trait from step 1.
4. **Google Calendar tool**: list upcoming, create event, find conflicts.
5. **Notification tool**: pick Slack as the first channel; design the abstraction so a second (Discord? Email-as-notify?) is a small follow-up.
6. **Integration tests** against recorded fixtures (don't hit real APIs in CI).

## Success Criteria

- [ ] User can run `/connect gmail` in the TUI, complete the OAuth flow, and have the agent successfully read their inbox in the next message.
- [ ] User can ask "what's on my calendar tomorrow" and get an answer from real data.
- [ ] User can ask "send a slack message to #foo saying X" and have it delivered.
- [ ] All three integrations honor the permission system (rule-based deny works).
- [ ] Tokens stored securely (not in `arawn.toml`), survive restart, refresh transparently.
- [ ] Documentation for adding a fourth integration is clear enough that it's a 1–2 day task.

## Open Questions

- **Credential storage**: filesystem-only (encrypted at rest like the existing `secrets.age`?), or use OS keychain via `keyring` crate? Decide in ADR.
- **Slack auth**: webhook (simpler, channel-scoped) vs. proper OAuth app (more capable, multi-channel). User preference?
- **Sandbox interaction**: do integration tools bypass the shell sandbox entirely (since they're not shell commands), or do they participate in the same audit/permission flow?

## Dependencies

- T-0196 (sandbox/permissions UX) — integrations should plug into the same permission grammar; ideally land that first or in parallel.
- T-0194 (config UX) — `arawn init` should know about and offer to set up integrations during onboarding (as a stretch).

## Risks

- OAuth UX in a TUI is awkward (browser handoff). The existing `arawn-auth::server` already does callback capture for one provider; reuse it.
- Token refresh failures are the kind of thing that fails silently 90 days after setup. Design observability in from day one.
- Scope creep into "integrate with everything" — the trait pattern is supposed to enable that, not require it.

## Status Updates

*Discovery phase. Awaiting decomposition into tasks.*

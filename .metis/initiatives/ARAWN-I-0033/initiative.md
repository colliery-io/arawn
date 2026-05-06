---
id: external-integrations-layer-gmail
level: initiative
title: "External integrations layer — Gmail, Calendar, notifications"
short_code: "ARAWN-I-0033"
created_at: 2026-05-02T04:13:45.230690+00:00
updated_at: 2026-05-04T20:12:47.639918+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


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

## Resolved (was Open Questions)

- **Credential storage** → filesystem, encrypted with the existing `~/.arawn/identity.age`. Per-service paths under `~/.arawn/integrations/<service>/credentials.age`. See [ARAWN-A-0001](../../adrs/ARAWN-A-0001.md) for full rationale.
- **OAuth UX** → in-TUI `/connect <service>` slash-command brokered through the server, with localhost callback capture. See ARAWN-A-0001.
- **Slack auth** → **incoming webhook for v1.** Simpler, channel-scoped, no OAuth dance. The Notification tool is built against an abstraction so a future Slack-OAuth path is a swap, not a rewrite.
- **Sandbox interaction** → integration tools are normal `arawn-engine::Tool` impls. They don't go through the shell sandbox (different code path entirely) but they DO participate in the permission system — every call gets evaluated by `PermissionChecker::check_explained` (T-0196), so rules like `deny gmail_send(*)` work. Tools declare their `permission_category` so the mode-default fallback applies (e.g. `gmail_inbox_read` → ReadOnly, `gmail_send` → Other).

## Dependencies

- T-0196 (sandbox/permissions UX) — integrations should plug into the same permission grammar; ideally land that first or in parallel.
- T-0194 (config UX) — `arawn init` should know about and offer to set up integrations during onboarding (as a stretch).

## Risks

- OAuth UX in a TUI is awkward (browser handoff). The existing `arawn-auth::server` already does callback capture for one provider; reuse it.
- Token refresh failures are the kind of thing that fails silently 90 days after setup. Design observability in from day one.
- Scope creep into "integrate with everything" — the trait pattern is supposed to enable that, not require it.

## Status Updates

### 2026-05-03 — Decomposed (4 tasks)

Open questions answered (see Resolved section above), [ARAWN-A-0001](../../adrs/ARAWN-A-0001.md) landed with the credential-storage + OAuth-UX decisions. Initiative moved discovery → design → ready → decompose. Tasks created:

| Task | Title | Deps |
|---|---|---|
| [ARAWN-T-0200](./tasks/ARAWN-T-0200.md) | arawn-integrations crate scaffolding + Integration trait + connect/disconnect RPCs | (foundation) |
| [ARAWN-T-0201](./tasks/ARAWN-T-0201.md) | TUI /connect, /disconnect, /integrations commands + OAuth UX flow | T-0200 |
| [ARAWN-T-0202](./tasks/ARAWN-T-0202.md) | Gmail integration: read inbox, search, send, mark read | T-0200, T-0201 |
| [ARAWN-T-0203](./tasks/ARAWN-T-0203.md) | Google Calendar integration: list upcoming, create event, find conflicts | T-0200, T-0201 |
| [ARAWN-T-0204](./tasks/ARAWN-T-0204.md) | Notification integration: Slack incoming-webhook channel for v1 | T-0200, T-0201 |

**Suggested execution order:** T-0200 → T-0201 → T-0202 (proves the framework end-to-end with a real consumer) → T-0203 + T-0204 in parallel.

T-0202 is the MVP — once Gmail works end-to-end, the framework is proven. T-0203 and T-0204 follow the same shape with different wire formats.

### 2026-05-05 — Follow-ups discovered during smoke test

Surfaced while manually exercising `/connect slack` end-to-end. None blocked the integration work; recording here so they're not lost. Promote to their own initiative when someone owns TUI/auth-flow polish.

**Auth flow / callback server:**
- Failed `/connect <service>` leaves the local callback listener bound for its full 5-minute timeout. Retries fail with `Address already in use` until either the timeout expires or the user restarts `arawn serve`. Fix: cancellable callback server + drop the listener on auth-flow error path.

**TUI rendering — resolved during smoke:**
- Chat area was clipping at the bottom because `Paragraph::wrap` + `scroll((y, 0))` was driven by an external wrap estimate that disagreed with ratatui's actual wrap. Replaced with a pre-wrap pipeline (`crates/arawn-tui/src/wrap.rs`) that owns wrap and slices the visible window. Resolved.
- Streaming-token throughput was overrunning the render path: every event triggered its own `terminal.draw`. Moved WS reads onto a dedicated reader task (`WsClient::events_take`) and added a 33ms minimum frame interval (`MIN_FRAME_INTERVAL`). Resolved.

**TUI rendering — open:**
- Chat doesn't redraw at end of turn unless the next event arrives or extra lines get appended. Currently masked by the `───` end-of-turn rule we emit after each completed turn — adding lines bumps the buffer enough to flush. Removing the marker brings the bug back. Right fix unclear; likely the `force_draw` after `Complete` happens before the final `Assistant` message is visible to the renderer (timing or `last_draw` budget interaction). Investigate: instrument render call counts, confirm `Complete` triggers a real frame, decide whether `Complete` should bypass the frame budget unconditionally.

**Workflow scaffolding for agent-authored schedules — open:**
- `workflow_create` is registered as an agent tool but the `task.body` parameter is a Rust async function body. Tasks compile into a Rust crate, so they don't have access to the agent's tool surface (`slack_list_channels`, `slack_history`, `memory_store`, etc.) as native APIs. To do agent-driven work in a scheduled task, the body needs HTTP-callback boilerplate against `arawn-workflow::DecisionService` — not something the LLM produces reliably cold.
- The agent (gemma4 via Ollama Cloud, in the smoke test that surfaced this) correctly identified the gap and offered cron+shell as a workaround, which cuts arawn out of the loop and defeats the purpose.
- Right fix: a higher-level `workflow_create_decision_task(name, cron, prompt, [tool_allowlist])` tool that takes a prompt + cron and auto-generates a workflow whose task body just HTTP-calls the DecisionService to run the agent with that prompt. The agent composes a scheduled "intent" without writing Rust. ~60–100 LOC, plus a small docs update on the workflow authoring surface. Belongs alongside the broader "default workflow templates" initiative if/when that's filed.
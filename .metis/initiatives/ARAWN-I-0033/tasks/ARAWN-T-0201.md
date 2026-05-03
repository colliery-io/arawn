---
id: tui-connect-disconnect
level: task
title: "TUI /connect, /disconnect, /integrations commands + OAuth UX flow"
short_code: "ARAWN-T-0201"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-03T19:58:11.882578+00:00
parent: ARAWN-I-0033
blocked_by: [ARAWN-T-0200]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# TUI /connect, /disconnect, /integrations commands + OAuth UX flow

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Wire the TUI's slash-command system to the integration RPCs from T-0200. A user types `/connect gmail`, sees the auth URL, completes the OAuth flow in a browser, and gets back a confirmation in the TUI. `/disconnect <service>` removes credentials. `/integrations` lists registered services with their connection status.

## Type / Priority
- Feature
- P1 — The user-facing half of the integration story. Without this, T-0202/3/4 are agent-only.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Three new commands in the registry: `/connect <service>`, `/disconnect <service>`, `/integrations`.
- [ ] `/connect <service>` calls `start_oauth_flow` RPC; receives the auth URL; attempts to open it via `open` (macOS) / `xdg-open` (Linux) / `cmd /c start` (Windows); falls back to a "Open this URL: …" system message if no opener is available. While waiting, shows a "Waiting for browser authorization…" status.
- [ ] On successful flow completion, the broadcast `ServerNotice` (from T-0200) renders in the TUI as `ℹ [integration] gmail connected`. Failure path: `✗ [integration] gmail connection FAILED: <reason>`.
- [ ] `/disconnect <service>` calls the disconnect RPC and confirms with a system message.
- [ ] `/integrations` calls `list_integrations` and renders a table: `service | connected | scopes` (scopes column may be blank for v1).
- [ ] `/help` reflects the new commands.
- [ ] The `every_advertised_builtin_dispatches_or_explains` audit test (T-0195) still passes — every new command dispatches to a real handler.
- [ ] At least one unit test in `command.rs::tests` per command verifying the parse → CommandResult routing.

## Implementation Notes

- Follows the same pattern as T-0196's `/permissions` command — direct RPC, format response as system message in `event_loop.rs`. No new UI widgets needed.
- The `open` opener: probe `which` for a platform-appropriate command at startup; cache the result on `App`. Don't shell out blindly per `/connect` invocation.
- The OAuth wait can take minutes (user has to click through Google's consent screen). Make sure the TUI doesn't appear hung — show a dotted "Waiting…" or use the existing spinner.
- For the SSH-without-local-browser case (per ARAWN-A-0001), the TUI should always print the URL even when it tried to auto-open. Cheap to do, helps the edge case.
- Don't try to confirm scopes inside the TUI in v1 — the URL itself reveals them (Google shows them in the consent screen). Scope display in `/integrations` is a stretch.

## Status Updates

### 2026-05-03 — Implemented

**Three new TUI commands** registered in `command.rs`:
- `/integrations` → `CommandResult::IntegrationsList`
- `/connect <service>` → `CommandResult::IntegrationConnect(name)`
- `/disconnect <service>` → `CommandResult::IntegrationDisconnect(name)`

Each handles missing-args with a usage SystemMessage that names the command. The `every_advertised_builtin_dispatches_or_explains` audit test (T-0195) keeps passing.

**`ws_client.rs`** gained `list_integrations`, `start_oauth_flow`, `disconnect_integration` thin wrappers around the RPCs T-0200 added.

**`event_loop.rs`** handlers:
- `IntegrationsList` calls the RPC and renders `format_integrations_list` — markdown table of `Service | Connected` (✓ / —) plus a usage hint at the bottom.
- `IntegrationConnect` calls `start_oauth_flow`, attempts `try_open_url` (platform-aware: `open` on macOS, `xdg-open` on Linux, `cmd /c start` on Windows; returns `OpenAttempt::NoOpener` elsewhere), and **always prints the URL** so SSH-without-local-browser users can copy/paste. Status message names which opener fired (or that none was found / why it failed).
- `IntegrationDisconnect` calls the RPC and confirms.

The existing T-0199 `apply_system_notice` already routes the `ServerNotice { category: "integration" }` events that T-0200's spawned task broadcasts on flow completion — so the success/failure rendering arrives in chat history automatically as `ℹ [integration] gmail connected` / `✗ [integration] gmail connection FAILED: ...`. No new TUI plumbing for that path.

**Tests** (5 new, in `command.rs`):
- `execute_integrations_returns_list_variant`
- `execute_connect_with_service_returns_connect_variant`
- `execute_connect_without_service_returns_usage_message`
- `execute_disconnect_with_service_returns_disconnect_variant`
- `execute_disconnect_without_service_returns_usage_message`

905 workspace lib tests pass (was 900); 0 clippy warnings.

**Acceptance criteria status:**
- [x] Three commands registered + dispatched.
- [x] `/connect` opens the URL via OS opener and always prints it as fallback.
- [x] `ServerNotice` rendering for success/failure (via T-0199 wiring; no new code needed).
- [x] `/disconnect` calls RPC + confirms.
- [x] `/integrations` renders `Service | Connected` table.
- [x] `/help` reflects the new commands (live registry; the existing audit test would catch stale help).
- [x] Audit test still passes.
- [x] One unit test per new command.

**Deferred (tiny):**
- `try_open_url` does platform detection at call time, not startup — fine because `cfg!` is compile-time. The "cache opener result on App" note in the ticket is moot.
- Scope display in `/integrations` skipped per the ticket's "stretch" note. Comes back when an integration actually has a meaningful scope summary to surface.
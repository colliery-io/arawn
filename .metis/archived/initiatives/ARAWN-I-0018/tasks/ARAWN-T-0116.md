---
id: slash-commands-for-session
level: task
title: "Slash commands for session management — /session new|list and /promote"
short_code: "ARAWN-T-0116"
created_at: 2026-04-06T19:29:52.623812+00:00
updated_at: 2026-04-07T12:09:59.347504+00:00
parent: ARAWN-I-0018
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0018
---

# Slash commands for session management — /session new|list and /promote

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0018]]

## Objective

Add `/session` and `/promote` slash commands for managing sessions within workstreams. `/session new` creates a new session in the current workstream, `/session list` shows sessions, and `/promote <workstream>` moves the current scratch session into a named workstream.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/session new` creates a new session in the current workstream, switches to it, clears chat
- [ ] `/session list` displays sessions for the current workstream with IDs and dates as a system message
- [ ] `/promote <name>` moves the current session from scratch into the named workstream (calls `promote_session` RPC), updates sidebar and status bar
- [ ] `/promote` with no arg shows usage help
- [ ] `/promote` errors gracefully if the current session isn't in scratch or the target workstream doesn't exist
- [ ] All commands appear in `/help` output and autocomplete

## Implementation Notes

### Key files
- `crates/arawn-tui/src/command.rs` — register "session" and "promote" as BuiltIn commands
- `crates/arawn-tui/src/event_loop.rs` — handle `SessionNew`, `SessionList`, `PromoteSession` command results via WS

### Approach
`/session new` reuses the existing `NewSession` action logic but triggered from a command instead of the `n` key. `/session list` is a client-side format of `app.sessions`. `/promote` needs the `promote_session` RPC from T-0117.

New `CommandResult` variants:
- `SessionNew`
- `SessionList`
- `PromoteSession(String)` — target workstream name

### Dependencies
- T-0117 (server RPC) for `promote_session` endpoint

## Status Updates

*To be added during implementation*
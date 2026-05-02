---
id: wire-stubbed-tui-slash-commands
level: task
title: "Wire stubbed TUI slash-commands"
short_code: "ARAWN-T-0195"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Wire stubbed TUI slash-commands

## Objective

`crates/arawn-tui/src/command.rs` defines slash-commands that show up in `/help` but the event loop and backend don't act on them. `/remember`, `/forget`, `/memory` resolve to `CommandResult` variants but nothing handles those variants. `/help`, `/plan`, `/accept`, `/workstream`, `/session`, `/promote` are partially implemented. Users see a command in the help list, type it, and nothing happens. Wire the stubbed commands so what `/help` advertises actually works.

## Type / Priority
- Feature
- P1 — Blocker for "general assistant" perception. Broken-on-purpose UI undermines trust.

## Acceptance Criteria

- [ ] Audit `crates/arawn-tui/src/command.rs` — for every command listed in `/help`, either (a) wire it to a working handler, or (b) remove it from `/help` until it works. No "advertised but broken" state.
- [ ] `/remember <text>` → invokes the `memory_store` tool with the captured text. (Coordinates with T-0197.)
- [ ] `/forget <id-or-title>` → invokes a memory-forget pathway (engine has the `MemoryForgetTool`? — verify; if not, this acceptance criterion is the smaller one of "scope this and either remove from /help or expose a stub that explains how").
- [ ] `/memory` → opens an inspection view (modal or new pane) listing recent memory entries with type and confidence. (Coordinates with T-0197.)
- [ ] `/clear` → resets the conversation transcript in the TUI (does NOT delete server-side history; just clears the rendered view).
- [ ] `/help` text matches actual command set after this work.
- [ ] At least one integration-style test driving a `/remember` round-trip end-to-end.

## Implementation Notes

- The action-routing path is `event.rs` → `Action` → `event_loop.rs::handle_action`. Trace which `Action` variants the command commands dispatch to.
- Server-side, `memory_store` is registered as a tool; `/remember` should construct a `send_message` with content shaped to invoke it, OR fire a direct RPC method. Pick one and document the choice — direct RPC is cleaner for non-LLM UI affordances.
- Don't try to implement everything in one shot — the audit step (criterion 1) should produce a small follow-up ticket for any command too big to land here.

## Status Updates

*To be added during implementation*

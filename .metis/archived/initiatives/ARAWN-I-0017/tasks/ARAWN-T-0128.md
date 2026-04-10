---
id: tui-status-bar-display-current
level: task
title: "TUI status bar — display current permission mode with color-coded indicator"
short_code: "ARAWN-T-0128"
created_at: 2026-04-09T16:03:05.119069+00:00
updated_at: 2026-04-09T16:13:17.307764+00:00
parent: ARAWN-I-0017
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0017
---

# TUI status bar — display current permission mode with color-coded indicator

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0017]]

## Objective

Add a permission mode indicator to the TUI status bar so the user always knows the current autonomy level. The mode is fetched via `get_permission_mode` RPC on connect and updated when `/accept` commands change it.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Status bar shows mode label between workstream name and session ID
- [ ] `DEFAULT` shown in normal color (no extra attention needed)
- [ ] `BYPASS` shown in red/highlighted — this is the "dangerous" mode, must be visible
- [ ] `ACCEPT EDITS` shown in yellow
- [ ] `PLAN` shown in blue/cyan
- [ ] Mode indicator updates immediately when `/accept` or `/plan` command is used
- [ ] Mode fetched from server via `get_permission_mode` on TUI connect
- [ ] `App` struct gets a `permission_mode: String` field for render access
- [ ] Render test: status bar contains mode label

### Key files
- `crates/arawn-tui/src/render.rs` — `render_status_bar` function
- `crates/arawn-tui/src/app.rs` — add `permission_mode` field
- `crates/arawn-tui/src/event_loop.rs` — fetch mode on connect, update on `/accept`

### Dependencies
- T-0126 (RPC methods)
- T-0127 (commands that change mode)

## Status Updates

*To be added during implementation*
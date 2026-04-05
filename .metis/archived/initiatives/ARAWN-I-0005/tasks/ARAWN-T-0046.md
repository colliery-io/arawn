---
id: sidebar-workstream-list-session
level: task
title: "Sidebar — workstream list, session list, navigation"
short_code: "ARAWN-T-0046"
created_at: 2026-04-01T11:46:45.068471+00:00
updated_at: 2026-04-01T12:36:11.275553+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# Sidebar — workstream list, session list, navigation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Flesh out the sidebar widget with interactive workstream and session navigation. Selecting a workstream loads its sessions via WS. Selecting a session loads its messages and switches the chat view. Creating a new session sends `create_session` via WS.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Sidebar split into two sections: "Workstreams" (top half) and "Sessions" (bottom half)
- [ ] Workstream list populated from `list_workstreams` WS call on startup
- [ ] Current workstream highlighted with `>` prefix and bold style
- [ ] Session list shows sessions for the selected workstream (via `list_sessions` WS call)
- [ ] Sessions display as truncated UUID + created_at date
- [ ] Up/Down navigates the active section (workstreams or sessions)
- [ ] Enter on a workstream: switches current_workstream, reloads sessions via WS
- [ ] Enter on a session: switches current_session, loads messages via `load_session` WS call, populates chat
- [ ] `n` key in sidebar focus: sends `create_session` via WS, switches to new session
- [ ] App state: `sidebar_section: WorkstreamList | SessionList`, `sidebar_ws_index`, `sidebar_session_index`
- [ ] Headless test: sidebar renders workstream names
- [ ] Headless test: selecting workstream updates session list

## Implementation Notes

- Extend `widgets/sidebar.rs` from the placeholder in T-0043
- Sidebar navigation state lives in `App` — two selection indices + which section is active
- WS calls for list/load are async — the event loop sends the request and processes the response when it arrives. App shows a loading indicator while waiting.
- Session switching clears the chat messages and reloads from the selected session
- Depends on: T-0043 (sidebar placeholder), T-0044 (sidebar actions), T-0045 (WS client for data loading)

## Status Updates
- **2026-04-01**: Complete. Sidebar rendering (from T-0043) shows workstreams with ▸ selection indicator + bold for current, sessions with truncated UUID + date. Event loop handles SidebarSelect: workstream select loads sessions via WS + switches sidebar section to Sessions, session select loads messages via load_session WS call + populates chat + switches focus to Input. NewSession creates session via WS + clears chat + reloads session list. 188 workspace tests, clippy clean.
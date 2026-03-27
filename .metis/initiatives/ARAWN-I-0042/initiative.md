---
id: tui-rewrite-clean-room
level: initiative
title: "TUI Rewrite — clean room implementation of workstream/session chat interface"
short_code: "ARAWN-I-0042"
created_at: 2026-03-27T01:28:39.660320+00:00
updated_at: 2026-03-27T02:37:58.939263+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: tui-rewrite-clean-room
---

# TUI Rewrite — clean room implementation of workstream/session chat interface Initiative

## Context

The current TUI was built incrementally without a clear spec. The event loop blocks on HTTP calls, starving WebSocket messages. Focus management is inconsistent. Every fix reveals another bug. Headless tests prove the backend works but the real terminal experience keeps breaking. Time for a clean room rewrite.

## Spec

"Create and interact with workstreams via sessions."

Think Claude Code's interface but with all your projects available in the left panel.

### Layout

```
┌─────────────────┬──────────────────────────────────────────┐
│ Workstreams      │                                          │
│ ─────────────── │  Chat                                    │
│ > my-blog       │                                          │
│   code-review   │  You: How does the auth module work?     │
│   research      │                                          │
│                 │  Arawn: The auth module uses...           │
│ Sessions        │                                          │
│ ─────────────── │                                          │
│ + New Session   │                                          │
│   Mar 26 14:00  │                                          │
│   Mar 25 09:30  │                                          │
│                 │                                          │
│                 ├──────────────────────────────────────────┤
│                 │ > Type a message...                      │
└─────────────────┴──────────────────────────────────────────┘
```

**Left panel**: Workstreams list (top) + sessions for selected workstream (bottom). Select a workstream → shows its sessions. Select a session → loads its chat. Select "+ New Session" → starts fresh chat.

**Right panel**: Chat messages (scrollable) + input box at bottom.

That's it. No tool pane, no logs panel, no command palette, no overlays. Those can come later as opt-in features.

## Goals

- Clean room rewrite of `arawn-tui` — delete and start fresh
- Non-blocking event loop — WS messages never starved by HTTP calls
- Two panels: sidebar (workstreams + sessions) and chat
- Focus is simple: sidebar or input. No overlay stack.
- Every user flow has a headless test before the code is written (test-first)
- Working against a real server from day 1

## Non-Goals

- Tool execution pane (future)
- Log viewer (future)
- Command palette (future)
- Mouse interaction (keyboard only for v1)
- Streaming indicator / typing animation (future)
- Context usage display (future)

## Architecture

### Event Loop — Non-Blocking

```rust
loop {
    terminal.draw(|f| render(&app, f))?;
    
    tokio::select! {
        // Terminal events (crossterm)
        event = events.next() => handle_event(&mut app, event),
        
        // WebSocket messages from server
        msg = ws.recv() => handle_ws_message(&mut app, msg),
        
        // Background HTTP task results
        result = http_results.recv() => apply_http_result(&mut app, result),
    }
}
```

HTTP calls (fetch workstreams, fetch sessions) are spawned as background tasks. Results arrive via a channel. The event loop NEVER awaits an HTTP call directly.

### State — Simple Struct

```rust
struct App {
    // Connection
    ws: WsClient,
    
    // Data
    workstreams: Vec<Workstream>,
    sessions: Vec<Session>,          // for selected workstream
    messages: Vec<ChatMessage>,      // for selected session
    
    // Selection
    selected_workstream: Option<usize>,
    selected_session: Option<usize>,
    current_session_id: Option<String>,
    current_workstream_id: Option<String>,
    
    // UI
    focus: Focus,                    // Sidebar or Input
    input: String,
    chat_scroll: usize,
    sidebar_scroll: usize,
    waiting: bool,
    status: Option<String>,
}

enum Focus { Sidebar, Input }
```

No BoundedVec, no PendingAction queue, no overlay stack, no PanelAreas. State is flat and obvious.

### Focus — Two States

- `Focus::Input` — keystrokes go to chat input. Tab switches to sidebar.
- `Focus::Sidebar` — keystrokes navigate workstreams/sessions. Tab switches to input. Enter selects.

No overlays. No nested focus stack. Tab toggles between the two panels.

### Testing — Test First

Every feature gets a headless test before implementation:
1. Write test: create app, inject events, assert on rendered buffer
2. Watch it fail
3. Implement
4. Watch it pass

Reuse the `TestBackend` + `run_headless()` infrastructure from I-0040.

## Implementation Plan

Build incrementally, each phase produces a working (if minimal) TUI:

### Phase 1: Bare bones — connect and chat
- Non-blocking event loop with WS + crossterm
- Single chat panel, no sidebar
- Type message → send → see response
- Headless test: send message, verify response renders

### Phase 2: Sidebar — workstreams
- Left panel lists workstreams (fetched via background HTTP)
- Arrow keys navigate, Enter selects
- Tab toggles focus between sidebar and input
- Selecting workstream loads its sessions
- Headless test: select workstream, verify sessions load

### Phase 3: Sessions
- Sessions list under workstreams in sidebar
- "+ New Session" entry at top
- Select session → load its messages
- Select "+ New Session" → clear chat, start fresh
- Headless test: switch sessions, verify chat clears and reloads

### Phase 4: Polish
- Scroll in chat (PageUp/PageDown)
- Status bar (connection status, current workstream/session)
- Ctrl+Q to quit
- Create new workstream from sidebar
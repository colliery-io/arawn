---
id: websocket-client-streaming-chat
level: task
title: "WebSocket client + streaming chat display"
short_code: "ARAWN-T-0045"
created_at: 2026-04-01T11:46:45.028620+00:00
updated_at: 2026-04-01T12:33:08.170918+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# WebSocket client + streaming chat display

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Connect the TUI to the Arawn WebSocket server. Implement the async event loop that concurrently polls terminal events (crossterm) and WebSocket messages (EngineEvents). When the user submits a message, send it via WS; when streaming events arrive, update App state and re-render. This is where the TUI becomes alive.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ws_client.rs`: connects to `ws://localhost:3100/ws` via `tokio-tungstenite`
- [ ] `run_tui(url: &str)` public function: initializes terminal, connects WS, runs event loop, restores terminal on exit
- [ ] Event loop: `tokio::select!` between crossterm `EventStream` and WS message stream
- [ ] Terminal events → map to Action → `app.handle_action()` → re-render
- [ ] On `Action::Submit`: send `{"id": N, "method": "send_message", "params": {...}}` over WS
- [ ] WS `EngineEvent::StreamingText` → append to `app.streaming_text`, re-render (live typing effect)
- [ ] WS `EngineEvent::ToolCallStart` → add tool call placeholder to chat, re-render
- [ ] WS `EngineEvent::ToolCallResult` → update tool call with result content
- [ ] WS `EngineEvent::Complete` → finalize streaming_text into chat message, set `is_generating = false`, auto-scroll
- [ ] WS `EngineEvent::Error` → display error in chat, set `is_generating = false`
- [ ] On startup: send `list_workstreams` + `create_session` to populate App state
- [ ] Ctrl-C gracefully restores terminal before exit
- [ ] Connection failure shows error and exits cleanly
- [ ] Re-render at ~30fps during streaming (throttle if needed)

## Implementation Notes

- `ws_client.rs` + `event_loop.rs` in `crates/arawn-tui/src/`
- Use `crossterm::event::EventStream` (async) + `tokio-tungstenite` WS stream in `tokio::select!`
- The event loop owns the `Terminal<CrosstermBackend>` and calls `terminal.draw(|f| render(&app, f))` after each state change
- JSON protocol matches what the server expects (same as websocket.rs tests)
- `app.streaming_text` accumulates `StreamingText` deltas. On `Complete`, it becomes a final `ChatMessage` and streaming_text clears.
- Render throttling: only re-render if state actually changed (dirty flag on App)
- Terminal setup/teardown: `crossterm::terminal::{enable_raw_mode, EnterAlternateScreen}` on start, reverse on exit (including panic handler)
- Depends on: T-0042 (App), T-0043 (render), T-0044 (input handling)

## Status Updates
- **2026-04-01**: Complete. ws_client.rs with WsClient (connect, list_workstreams, list_sessions, create_session, send_message, read_response). parse_engine_event for WS→EngineEvent conversion. EventUpdate enum bridging EngineEvent→App state mutations. event_loop.rs with run_tui(): terminal setup/teardown with panic hook, tokio::select! between EventStream + WS read, all EngineEvent variants handled (streaming text accumulation, tool calls, complete, error, compaction). Startup loads workstreams + creates session. 188 workspace tests, clippy clean.
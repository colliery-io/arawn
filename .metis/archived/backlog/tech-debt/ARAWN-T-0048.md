---
id: fill-integration-test-gaps
level: task
title: "Fill integration test gaps — LocalService, WS streaming, TUI integration"
short_code: "ARAWN-T-0048"
created_at: 2026-04-01T12:42:47.408726+00:00
updated_at: 2026-04-02T12:35:22.748984+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fill integration test gaps — LocalService, WS streaming, TUI integration

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Fill three integration test gaps identified in the test audit: (1) LocalService direct tests, (2) WebSocket streaming end-to-end, (3) TUI App driven through a full conversation with mock data.

### Priority
- P1 — these are the main integration boundaries that could break silently

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### LocalService tests (in arawn-tests)
- [ ] Test: list_workstreams returns scratch workstream
- [ ] Test: create_session + load_session roundtrip
- [ ] Test: send_message returns stream with Complete event (MockLLM text-only)
- [ ] Test: send_message with tool call returns ToolCallStart + ToolCallResult + Complete events
- [ ] Test: send_message persists messages to JSONL

### WebSocket streaming tests (in arawn-tests/websocket.rs)
- [ ] Test: send_message over WS → receive streaming_text events → Complete
- [ ] Test: send_message with tool calls → ToolCallStart + ToolCallResult events arrive in order
- [ ] Test: send_message error → Error event
- [ ] Test: concurrent connections don't interfere

### TUI integration tests (in arawn-tui)
- [ ] Test: App driven through full conversation: type → submit → streaming events → complete → message in chat
- [ ] Test: App receives ToolCallStart → adds tool placeholder, ToolCallResult → adds result
- [ ] Test: App receives Error → shows error in chat, clears generating state
- [ ] Test: sidebar selection updates app state correctly

## Implementation Notes

- LocalService tests need a test helper that creates LocalService with MockLLM + tempdir Store — similar to the Fixture pattern in engine_persistence.rs
- WS streaming tests extend the existing websocket.rs test file — use the same server spinup helper but with MockLLM scripted to return tool calls
- TUI integration tests are purely in-memory: create App, simulate EventUpdates, verify state. No network needed — test the App state machine with the EventUpdate→App mutation path
- All tests use MockLlmClient — no real LLM calls

## Status Updates
- **2026-04-01**: Complete. 11 new integration tests added:
  - **LocalService (5)**: list_workstreams, create/load session roundtrip, send_message text-only → Complete, send_message with tool call → ToolCallStart+ToolCallResult+Complete, send_message persists to JSONL
  - **WS streaming (2)**: send_message → streaming Complete over WS, send_message with tool call → events arrive in order (ToolCallStart before ToolCallResult before Complete)
  - **TUI integration (4)**: full conversation flow (type→submit→streaming→complete), tool call flow (start→result→complete), error event clears generating state, sidebar navigation clamping
  - Also added `App::apply_engine_event()` method for testable event application without the network event loop
  - Deferred: WS error event test, concurrent connections test (would need more server infrastructure)
  - 199 total workspace tests, 0 failures, clippy clean.
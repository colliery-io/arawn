---
id: agent-never-emits-streamchunk
level: task
title: "Agent never emits StreamChunk::ToolOutput during tool execution"
short_code: "ARAWN-T-0301"
created_at: 2026-03-09T13:27:19.047965+00:00
updated_at: 2026-03-09T14:13:42.618918+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Agent never emits StreamChunk::ToolOutput during tool execution

## Objective

The `StreamChunk::ToolOutput` variant and its constructor `StreamChunk::tool_output()` exist in `crates/arawn-agent/src/stream.rs` but are never emitted anywhere in the agent crate. The WebSocket handler in `crates/arawn-server/src/routes/ws/handlers.rs` has code to receive and forward `StreamChunk::ToolOutput` messages (lines 458-467), but they never arrive because the agent's streaming loop doesn't emit them during tool execution.

This means WebSocket clients never see incremental tool output — they only get `ToolStart` and `ToolEnd` with no intermediate progress.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: All WebSocket API consumers expecting tool output streaming
- **Reproduction Steps**:
  1. Connect via WebSocket, authenticate
  2. Send a chat message that triggers tool execution
  3. Observe messages received
- **Expected vs Actual**: Expected `ToolStart → ToolOutput → ToolEnd` sequence. Actual: `ToolStart → ToolEnd` with no `ToolOutput` in between.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Agent streaming loop emits `StreamChunk::ToolOutput` with tool execution results during tool execution
- [ ] WebSocket clients receive `ToolOutput` messages between `ToolStart` and `ToolEnd`
- [ ] E2E test verifies the `ToolOutput` message is received (re-enable the assertion removed in `e2e_websocket.rs`)

## Implementation Notes

### Technical Approach

In `crates/arawn-agent/src/stream.rs`, after tool execution completes (around line 315-321 where `ToolEnd` is yielded), emit a `StreamChunk::tool_output(id, content)` with the tool's result content before yielding the `ToolEnd` chunk.

### Key Files
- `crates/arawn-agent/src/stream.rs` — emit `ToolOutput` during tool execution
- `crates/arawn-server/src/routes/ws/handlers.rs` — already handles `ToolOutput` (no change needed)
- `crates/arawn-server/tests/e2e_websocket.rs` — re-enable `ToolOutput` assertion

## Status Updates

### Fix Applied
- **Root cause**: `StreamChunk::tool_output()` constructor existed (stream.rs:102-108) but was never called during tool execution
- **Fix**: Added `yield StreamChunk::tool_output(&tool_use.id, &content);` in `stream.rs` between tool execution result and `tool_end` yield (line 327)
- **SSE handler** (`chat.rs:310`) and **WS handler** (`handlers.rs:458-467`) already handled `ToolOutput` chunks — no changes needed there
- **Tests**: Re-enabled `ToolOutput` assertion in `e2e_websocket.rs` (scenario_ws_chat_with_tool_execution), added `tool_output` SSE event assertion in `e2e_scenarios.rs` (scenario_streaming_chat_with_tool)
- **All 70 E2E tests pass**, clippy clean
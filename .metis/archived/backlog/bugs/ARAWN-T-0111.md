---
id: tui-rendering-stalls-during-long
level: task
title: "TUI rendering stalls during long tool execution — messages batch-dump instead of streaming"
short_code: "ARAWN-T-0111"
created_at: 2026-04-05T19:08:44.630126+00:00
updated_at: 2026-04-05T21:25:14.464872+00:00
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

# TUI rendering stalls during long tool execution — messages batch-dump instead of streaming

## Objective

During long agentic tool execution, the TUI appears frozen — no visual feedback that work is happening. Then when tool calls complete, a burst of messages dumps all at once. Two related sub-problems:

1. **No progress feedback during tool execution** — the agent makes 20+ consecutive silent tool calls (`content: ""`) with no narration text between them. The user sees nothing happening.
2. **WebSocket event batching** — the TUI receives streaming events (ToolCallStart, ToolCallResult, text deltas) via WebSocket, but rendering appears to batch/stall during rapid-fire tool rounds rather than updating incrementally.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Observed in session `79668a6f`
- 37 assistant messages, **32 are silent tool-only** (empty content)
- Only 5 messages have any visible text for the user
- First text response doesn't appear until message 53 (after ~20 tool calls)
- Long-running shell commands (curl+unzip, 120s timeout) produce no feedback during execution
- When results arrive, TUI dumps multiple messages at once instead of streaming incrementally

### Expected vs Actual
- **Expected**: User sees tool call indicators as they happen ("Searching files...", "Reading source..."). TUI updates incrementally per event.
- **Actual**: TUI appears stuck. Then a batch of tool calls and results appear all at once.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] TUI shows live tool call indicators as they fire (tool name, elapsed time, spinner/animation)
- [ ] Tool results render incrementally, not batched
- [ ] Long-running tools (shell with high timeout) show a progress indicator ("Running `git clone`... 5s")
- [ ] System prompt encourages the agent to narrate progress between tool call rounds

## Implementation Notes

### Two separate problems

**Problem 1: Agent narration (system prompt / model behavior)**
The model generates `content: ""` with every tool call — it never says "Let me check..." or "Now scanning the source." This is a system prompt issue. Add guidance like:
- "When starting a multi-step task, briefly describe your plan before the first tool call"
- "Between tool call rounds, provide a brief progress update if the task involves more than 3 consecutive tool calls"

**Problem 2: TUI rendering pipeline (WebSocket → ratatui)**
The TUI receives events via WebSocket JSON-RPC. Need to investigate:
- Are `ToolCallStart` events emitted before tool execution begins? Or only after?
- Is the TUI event loop draining the WebSocket fast enough?
- Is `terminal.draw()` being called after each event or only at certain checkpoints?
- Does microcompact (`[Previous result cleared — N chars]`) cause visual confusion?

### Key files
- `crates/arawn-tui/src/event_loop.rs` — main TUI event loop
- `crates/arawn-tui/src/ws_client.rs` — WebSocket client receiving server events
- `crates/arawn/src/ws_server.rs` — server-side event emission
- `crates/arawn-service/src/types.rs` — event types (ToolCallStart, ToolCallResult, Complete, etc.)
- `crates/arawn-engine/src/system_prompt.rs` — agent narration guidance

## Status Updates

### 2026-04-05 — Complete
**Root cause found and fixed**: `engine.run()` ran the entire agentic loop as one async call. Events were only emitted AFTER the loop completed by iterating `session.messages()[msgs_before..]`. The TUI received nothing during execution.

**Fix (3 parts):**
1. **Live progress events** — Added `ProgressEvent` enum and optional `mpsc::Sender<ProgressEvent>` to `QueryEngine`. Engine now emits `ToolCallStart` before execution and `ToolCallResult` after each tool completes, in real time.
2. **Service layer forwarding** — `local_service.rs` creates a progress channel, wires it into the engine, and spawns a task that forwards progress events to the WebSocket channel with `Flush` after each. Post-completion event emission simplified to only handle compaction events (tool events already streamed live).
3. **System prompt narration** — Added "Progress narration" section guiding the agent to state its plan before first tool call and provide periodic checkpoints every 3-5 tool calls.

Also fixed: `web_fetch` schema test updated for optional `prompt` parameter. System prompt snapshot updated.

All 763+ tests pass.
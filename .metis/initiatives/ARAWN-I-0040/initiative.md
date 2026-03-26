---
id: tui-end-to-end-testing-headless
level: initiative
title: "TUI End-to-End Testing: Headless Mode, Render Tests, and Live Server Validation"
short_code: "ARAWN-I-0040"
created_at: 2026-03-26T15:23:09.108288+00:00
updated_at: 2026-03-26T15:26:01.701352+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: tui-end-to-end-testing-headless
---

# TUI End-to-End Testing: Headless Mode, Render Tests, and Live Server Validation Initiative

## Context

The TUI has ~1,047 lines of unit tests but they all test handlers in isolation by injecting mock messages directly. There are zero tests that verify what the user actually sees on screen. We confirmed via integration testing that the WS backend plumbing works (messages flow through channels), but the real TUI hangs when a user chats — and we can't reproduce it in tests because we have no way to exercise the actual render loop against a real server.

We need three things:
1. A headless mode that runs the full `app.run()` event loop without a terminal
2. Render tests using `ratatui::backend::TestBackend` that verify visible output
3. End-to-end tests that start a real server, run the headless TUI against it, send a message, and verify the response appears in the rendered buffer

## Goals

- Build a headless `app.run()` mode that uses `TestBackend` instead of `CrosstermBackend`
- Write render tests that verify chat messages, tool executions, status bar, sidebar, and error states are visible in the terminal buffer
- Write e2e tests that exercise the full flow: start TestServer → create App → type message → verify response renders
- Find and fix the bug where the real TUI hangs after sending a chat message
- All tests runnable in CI without a terminal

## Non-Goals

- Visual regression testing (screenshot comparison)
- Performance benchmarking of render loop
- Mouse interaction testing (keyboard-only for now)

## Architecture

### Headless Mode

`App` needs a `run_headless()` method that:
- Accepts a `ratatui::Terminal<TestBackend>` instead of creating its own terminal
- Runs the same `tokio::select!` event loop as `run()`
- Can be driven by injected events (key presses, ticks) instead of crossterm
- Exits after a configurable number of iterations or a timeout
- Returns the final `TestBackend` buffer for assertion

Alternatively, factor out the event loop body into a `process_event()` method that both `run()` and tests can call, keeping the terminal/render concern separate.

### Test Infrastructure

```
TestServer (arawn-test-utils)
    ↕ HTTP + WS
App (headless, TestBackend)
    ↕ channels
WsClient (real, connects to TestServer)
```

Tests inject key events to simulate typing, call `process_event()` or `run_headless()`, then inspect the `TestBackend` buffer for expected text.

### Render Assertions

Use `TestBackend::buffer()` to get the rendered cell grid. Helper functions:
- `buffer_contains_text(buffer, "expected text") -> bool`
- `assert_rendered(terminal, "expected text")`
- `assert_not_rendered(terminal, "unexpected text")`

## Detailed Design

### Phase 1: Refactor App for testability
- Extract the event loop body from `app.run()` into `app.process_tick()` / `app.process_key()` / `app.process_ws_message()`
- Make rendering work with any `ratatui::backend::Backend` (currently hardcoded to `CrosstermBackend<Stdout>`)
- Add `App::run_headless(terminal: &mut Terminal<TestBackend>, max_ticks: usize)` 

### Phase 2: Render tests
- Test that user messages appear in chat panel
- Test that assistant responses appear after handling ChatChunk
- Test that tool executions appear in tool pane
- Test that error messages appear in status bar
- Test that session ID appears in header
- Test that sidebar shows workstreams
- Test empty state renders correctly

### Phase 3: Live server e2e tests
- Start TestServer with mock backend
- Create App pointed at TestServer
- Simulate: type "hello" → Enter → wait for response → verify rendered
- Simulate: multi-turn conversation → verify both messages visible
- Simulate: tool execution → verify tool pane shows tool name + output
- Simulate: connection drop → verify error message renders
- Simulate: session switch → verify chat clears and new session loads

### Phase 4: Find and fix the hang bug
- With the headless test infrastructure, reproduce the exact scenario where the TUI hangs
- The WS integration tests already proved the backend works — the bug is likely in the render/event loop timing or connection status polling
- Fix it with a test that prevents regression
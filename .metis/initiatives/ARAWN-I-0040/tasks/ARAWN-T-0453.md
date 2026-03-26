---
id: extract-event-loop-body-from-app
level: task
title: "Extract event loop body from app.run() into testable process methods"
short_code: "ARAWN-T-0453"
created_at: 2026-03-26T15:26:12.695993+00:00
updated_at: 2026-03-26T16:26:25.658365+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Extract event loop body from app.run() into testable process methods

## Objective

Refactor `app.run()` in `crates/arawn-tui/src/app/mod.rs` so the event loop body is callable from tests without a real terminal. Currently `run()` owns the terminal, the event stream, and the select loop — none of which can be exercised from a test.

Extract into:
- `process_key(&mut self, key: KeyEvent)` — already exists as `handle_key`, confirm it's sufficient
- `process_ws_message(&mut self, msg: ServerMessage)` — already exists as `handle_server_message`, confirm
- `process_tick(&mut self)` — extract the tick handler (connection status polling, ping) from the select arm
- Make `ui::render()` generic over `Backend` — currently takes `&mut Frame` which is already generic, verify it works with TestBackend

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `app.run()` still works identically for the real TUI (no behavior change)
- [ ] `process_tick()` extracted and callable independently
- [ ] `ui::render()` confirmed working with `Terminal<TestBackend>`
- [ ] Existing 153 TUI lib tests still pass
- [ ] **GATE: `cargo test --workspace` passes with zero failures before marking complete**

## Status Updates

*To be added during implementation*
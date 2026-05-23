---
id: wire-tui-into-binary-headless
level: task
title: "Wire TUI into binary + headless rendering tests"
short_code: "ARAWN-T-0047"
created_at: 2026-04-01T11:46:46.771001+00:00
updated_at: 2026-04-01T12:40:34.969676+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# Wire TUI into binary + headless rendering tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Wire the TUI into the arawn binary as the default mode (`arawn tui` or just `arawn` with no args). Add comprehensive headless rendering tests that exercise every widget via TestBackend without a real terminal.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `arawn` with no args (and no prompt) launches TUI mode (connects to `ws://localhost:3100/ws`)
- [ ] `arawn tui --url ws://host:port/ws` connects to a specific server
- [ ] `arawn "prompt"` still works as one-shot CLI (backward compatible)
- [ ] `arawn serve` still works (backward compatible)
- [ ] TUI binary dep: `arawn` crate depends on `arawn-tui`
- [ ] Headless test: empty app renders layout with status bar, empty chat, input placeholder
- [ ] Headless test: app with messages renders them in chat area
- [ ] Headless test: streaming state shows cursor indicator in chat
- [ ] Headless test: focused input bar has colored border, unfocused has dim
- [ ] Headless test: sidebar shows workstream names when populated
- [ ] Headless test: different terminal sizes (80x24, 120x40, 40x12) all render without panic
- [ ] Headless test: full action sequence (type → submit → streaming → complete) updates state correctly

## Implementation Notes

- Binary arg parsing: no args + no prompt → launch TUI. `tui` subcommand also works. `--url` flag for custom server.
- TUI must start the WS server in the background if not already running (or require `arawn serve` in another terminal for v1 — simpler)
- For v1: require server running separately. Document: "Run `arawn serve` in one terminal, `arawn tui` in another."
- Headless tests in `crates/arawn-tui/src/` as inline test modules — they only use TestBackend, no network.
- Each widget module gets its own test section testing render output
- Depends on: all previous TUI tasks (T-0042 through T-0046)

## Status Updates
- **2026-04-01**: Complete. Binary wired: no args → TUI mode (connects ws://127.0.0.1:3100/ws), `tui` subcommand + `--url` flag, `serve` and one-shot CLI unchanged. Usage text updated with all commands/options. Headless tests already in arawn-tui (6 render tests + 10 app tests + 7 event tests = 23 TUI tests). All render tests use TestBackend — status bar, messages, input, streaming cursor, small/large terminals. 188 total workspace tests, clippy clean.
---
id: tui-keybind-hints-too-faded
level: task
title: "TUI: Keybind hints too faded — improve contrast"
short_code: "ARAWN-T-0276"
created_at: 2026-03-06T13:50:03.892094+00:00
updated_at: 2026-03-06T14:56:51.431250+00:00
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

# TUI: Keybind hints too faded — improve contrast

## Objective

The keybind hints bar at the bottom of the TUI screen is too faded/low-contrast to be readable. Increase contrast so users can actually see available key bindings.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Keybind hints at bottom of TUI are clearly readable against the background
- [ ] Contrast meets basic accessibility standards

## Status Updates

### Changes Made
Updated all keybind hint footers across the TUI to use a two-tone contrast pattern:
- **Key names** (`↑↓`, `enter`, `esc`, `^N`, `Tab`, etc.): `Color::Gray` (brighter, readable)
- **Action labels** (`navigate`, `select`, `close`, etc.) and **separators** (`│`): `Color::DarkGray` (dimmer, secondary)

Files modified:
- `crates/arawn-tui/src/ui/sidebar.rs` — sidebar footer
- `crates/arawn-tui/src/ui/sessions.rs` — sessions panel footer
- `crates/arawn-tui/src/ui/logs.rs` — logs panel footer
- `crates/arawn-tui/src/ui/palette.rs` — command palette footer
- `crates/arawn-tui/src/ui/tools.rs` — tool pane footer (was using Cyan keys + DarkGray base style that dimmed everything; removed base style, changed keys from Cyan to Gray for consistency, added explicit DarkGray to action labels)
- `crates/arawn-tui/src/ui/layout.rs` — workstream popup footer hints

All footers now follow the same consistent pattern. Compiles clean.
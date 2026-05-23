---
id: tui-accept-and-plan-commands
level: task
title: "TUI /accept and /plan commands — permission mode switching via slash commands"
short_code: "ARAWN-T-0127"
created_at: 2026-04-09T16:03:03.881724+00:00
updated_at: 2026-04-09T16:13:16.426295+00:00
parent: ARAWN-I-0017
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0017
---

# TUI /accept and /plan commands — permission mode switching via slash commands

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0017]]

## Objective

Register `/accept on|off|edits` as a TUI built-in command that calls the `set_permission_mode` RPC from T-0126. Update the existing `/plan` command to also use the RPC instead of just triggering the plan mode tool. Provides runtime permission switching without editing arawn.toml.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/accept on` sets mode to `BypassPermissions`, shows confirmation in chat
- [ ] `/accept off` sets mode to `Default`, shows confirmation
- [ ] `/accept edits` sets mode to `AcceptEdits`, shows confirmation
- [ ] `/plan` sets mode to `Plan` via the RPC (in addition to existing plan mode behavior)
- [ ] `/accept` with no args shows usage help
- [ ] Commands registered in `CommandRegistry` and appear in `/help` and autocomplete
- [ ] System message confirms the mode change (e.g., "Permission mode set to BYPASS")

### Key files
- `crates/arawn-tui/src/command.rs` — register commands, add `CommandResult` variants
- `crates/arawn-tui/src/event_loop.rs` — handle new variants, call RPC

### Dependencies
- T-0126 (RPC methods must exist)

## Status Updates

*To be added during implementation*
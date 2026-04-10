---
id: slash-commands-for-workstream
level: task
title: "Slash commands for workstream management — /workstream create|list|switch"
short_code: "ARAWN-T-0115"
created_at: 2026-04-06T19:29:51.712561+00:00
updated_at: 2026-04-07T12:09:53.361351+00:00
parent: ARAWN-I-0018
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0018
---

# Slash commands for workstream management — /workstream create|list|switch

## Objective

Add hierarchical `/workstream` slash command to the TUI command system for creating, listing, and switching workstreams. The command dispatches on subcommand: `/workstream create <name>`, `/workstream list`, `/workstream switch <name>`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/workstream create <name>` calls `create_workstream` RPC with name and a default root_dir under the data dir, then switches the TUI to the new workstream (creates initial session, updates sidebar)
- [ ] `/workstream list` displays all workstreams with session counts as a system message
- [ ] `/workstream switch <name>` switches to an existing workstream by name (loads sessions, resumes most recent or creates new)
- [ ] `/workstream` with no subcommand shows usage help
- [ ] All three subcommands appear in `/help` output
- [ ] Autocomplete triggers on `/workstream` prefix

## Implementation Notes

### Key files
- `crates/arawn-tui/src/command.rs` — register "workstream" as a BuiltIn command, parse subcommands from `args`
- `crates/arawn-tui/src/app.rs` — handle `CommandResult::WorkstreamCreate`, `WorkstreamList`, `WorkstreamSwitch` variants
- `crates/arawn-tui/src/event_loop.rs` — dispatch WS interaction for create/switch (needs `client.send_request`)

### Approach
The existing command system parses `/workstream create foo` as `ParsedCommand { name: "workstream", args: "create foo" }`. The `execute_command` function returns a `CommandResult` variant. The event loop handles variants that need WS interaction (like it does for `QueryInventory` and `InvokeSkill`).

New `CommandResult` variants:
- `WorkstreamCreate(String)` — name
- `WorkstreamList`
- `WorkstreamSwitch(String)` — name

### Dependencies
- T-0117 (server RPC) needed for `create_workstream` with proper root_dir handling. Can stub with existing RPC initially.

## Status Updates

*To be added during implementation*
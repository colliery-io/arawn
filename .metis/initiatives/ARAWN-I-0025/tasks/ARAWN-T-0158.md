---
id: add-rpc-protocol-versioning-with
level: task
title: "Add RPC protocol versioning with hello handshake and method naming standardization"
short_code: "ARAWN-T-0158"
created_at: 2026-04-10T01:01:24.002263+00:00
updated_at: 2026-04-10T23:27:55.891887+00:00
parent: ARAWN-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0025
---

# Add RPC protocol versioning with hello handshake and method naming standardization

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0025]]

## Objective
Add a `hello` handshake RPC method returning server version and supported methods. Standardize all RPC method names to consistent `verb_noun` pattern.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `hello` RPC returning `{ version: "0.x.y", methods: [...] }`
- [ ] Method renames: `remember_fact` -> `store_memory`, `forget_entity` -> `delete_memory`, `memory_summary` -> `get_memory_summary`, `list_available_commands` -> `list_commands`
- [ ] Old method names accepted with deprecation warning (backward compat period)
- [ ] TUI updated to use new method names
- [ ] Clients treat unknown event tags as no-ops

## Implementation Notes
- Files: `crates/arawn/src/ws_server.rs`, `crates/arawn-tui/src/ws_client.rs`
- Depends on ARAWN-T-0154 (service trait completion) for stable method list

## Status Updates
- **COMPLETE**: All acceptance criteria met, all tests pass.

### Changes:
- **`hello` handshake**: New RPC method returns `{ version: "0.1.0", methods: [...] }` with all 18 canonical method names
- **Method renames** (canonical match arms updated):
  - `remember_fact` → `store_memory`
  - `forget_entity` → `delete_memory`
  - `memory_summary` → `get_memory_summary`
  - `list_available_commands` → `list_commands`
- **Backward compat**: Old names resolved to canonical names before dispatch, with `warn!` deprecation log including old + new name
- **TUI updated**: `event_loop.rs` switched to `list_commands` and `get_memory_summary`
- **Constants**: `PROTOCOL_VERSION` and `RPC_METHODS` array defined for hello response
- **Note on "clients treat unknown event tags as no-ops"**: The TUI already uses `_ => {}` catch-all in its event matching, so unknown events are silently ignored. No change needed.
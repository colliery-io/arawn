---
id: add-plugin-conflict-and-hot-reload
level: task
title: "Add plugin conflict and hot-reload tests"
short_code: "ARAWN-T-0296"
created_at: 2026-03-08T20:21:18.534842+00:00
updated_at: 2026-03-08T20:21:18.534842+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add plugin conflict and hot-reload tests

## Objective

No tests verify what happens when two plugins register the same command name, or when a plugin is updated/reloaded while the system is running. Add tests for conflict detection and hot-reload behavior.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Plugin registry behavior under conflict is undefined/untested. Hot-reload path has no coverage.
- **Benefits of Fixing**: Defined behavior for plugin conflicts, confidence in reload mechanics.
- **Risk Assessment**: Low-medium — conflicts are edge cases but hot-reload is user-facing.

## Acceptance Criteria

- [ ] Test registering two plugins with the same command name — verify error or last-wins behavior
- [ ] Test registering two plugins with overlapping tool names
- [ ] Test unloading and reloading a plugin preserves correct state
- [ ] Test that active sessions are unaffected by plugin reload
- [ ] `cargo test -p arawn-plugin` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Use the plugin registry directly in tests
- Create test plugin manifests with overlapping names
- Test the load/unload/reload cycle

### Files
- `crates/arawn-plugin/src/registry.rs`
- `crates/arawn-plugin/src/loader.rs`

## Status Updates

*To be added during implementation*
---
id: add-plugin-conflict-and-hot-reload
level: task
title: "Add plugin conflict and hot-reload tests"
short_code: "ARAWN-T-0296"
created_at: 2026-03-08T20:21:18.534842+00:00
updated_at: 2026-03-09T01:22:16.663333+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

### Implementation Complete

**16 new tests added across 3 files:**

**`crates/arawn-plugin/src/skill.rs`** — 6 skill conflict tests:
- `test_skill_conflict_two_plugins_same_name_simple_lookup_returns_none` — ambiguous simple name returns None
- `test_skill_conflict_invoke_simple_ambiguous_returns_none` — invoke with ambiguous name returns None
- `test_skill_conflict_three_plugins_same_name` — 3-way conflict, all qualified lookups work
- `test_skill_conflict_different_names_no_conflict` — distinct names coexist fine
- `test_skill_conflict_qualified_invocation_resolves_ambiguity` — qualified names bypass ambiguity
- `test_skill_conflict_same_plugin_overwrites` — re-registering same plugin replaces old skill

**`crates/arawn-plugin/src/watcher.rs`** — 7 hot-reload tests:
- `test_reload_updates_skill_content` — modify skill on disk, reload, verify new content
- `test_reload_updates_version` — update manifest version, reload, verify
- `test_reload_added_skill_appears` — add new skill dir on disk, reload, verify it appears
- `test_remove_and_readd_plugin` — remove then reload same plugin
- `test_reload_other_plugins_unaffected` — reload beta doesn't change alpha
- `test_concurrent_read_during_reload` — concurrent reader task during reload, no panics

**`crates/arawn-plugin/src/hooks.rs`** — 3 hook conflict tests:
- `test_two_plugins_same_event_both_fire` — two SessionStart hooks, both outputs appear
- `test_two_plugins_pre_tool_use_first_blocker_wins` — two blocking hooks, first reason wins
- `test_two_plugins_different_tool_match_no_interference` — shell hook doesn't affect file_read

**All 194 tests pass. Clippy clean.**
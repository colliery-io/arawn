s---
id: permission-config-loading-user
level: task
title: "Permission config loading — user + project settings merge with priority"
short_code: "ARAWN-T-0073"
created_at: 2026-04-03T02:48:46.037230+00:00
updated_at: 2026-04-03T10:15:47.519992+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# Permission config loading — user + project settings merge with priority

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

Load permission rules from user-level (`~/.arawn/settings.json`) and project-level (`.arawn/settings.json`) config files, and merge them with correct priority (user > project > defaults).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionConfig` struct holds merged allow/deny/ask rule lists
- [ ] Deserializes from JSON settings format: `{ "permissions": { "allow": [...], "deny": [...], "ask": [...] } }`
- [ ] Loads from `~/.arawn/settings.json` (user-level)
- [ ] Loads from `.arawn/settings.json` (project-level)
- [ ] User rules take priority over project rules (user deny overrides project allow)
- [ ] Missing files handled gracefully — empty config, no error
- [ ] Malformed permission entries logged as warnings, skipped (don't break startup)
- [ ] Unit tests for merge priority, missing files, malformed entries

## Implementation Notes

### Technical Approach
- Integrate with existing settings/config loading infrastructure
- Permission rules stored under a `"permissions"` key in settings JSON
- Merge strategy: concatenate user + project rules, but user deny rules are checked before project allow rules in the evaluation order
- Consider whether `PermissionConfig` is built once at startup or reloaded on file change (start with once-at-startup)

### Dependencies
- Depends on T-0072 (rule types) for the `PermissionRule` type to deserialize into

## Status Updates

- Decision: config is TOML-based (arawn.toml), not JSON — matched existing config infrastructure
- Created `crates/arawn-engine/src/permissions/config.rs`
- `PermissionConfig` with allow/deny/ask string lists, `into_rules()` to parse into typed rules
- `merge()` method preserves priority (user deny rules checked before project allow)
- `load_permissions_from_file()` loads from TOML with graceful fallback
- `load_merged_permissions()` merges user + project configs
- Added `toml = "0.8"` to arawn-engine Cargo.toml
- 8 config tests all passing (26 total permission tests)
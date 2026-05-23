---
id: plugin-discovery-component-loading
level: task
title: "Plugin discovery + component loading integration tests"
short_code: "ARAWN-T-0106"
created_at: 2026-04-05T17:17:10.845249+00:00
updated_at: 2026-04-05T17:46:04.551532+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Plugin discovery + component loading integration tests

## Objective

Integration tests for plugin discovery, manifest parsing, and component loading (skills, hooks, agents) into engine registries. Tests go in `crates/arawn-tests/tests/plugin_components.rs`. This task does NOT require TestHarnessBuilder — it tests the plugin loader APIs directly with temp directories.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: create temp dir with `cache/{marketplace}/{plugin}/plugin.json`, `discover_plugins()` finds it
- [ ] Test: `load_plugin_dir()` parses manifest, returns `LoadedPlugin` with correct fields
- [ ] Test: `load_plugin_components()` loads skills from plugin's skills directory into `PluginComponents`
- [ ] Test: `load_plugin_components()` loads agents from plugin's agents directory
- [ ] Test: `load_plugin_components()` loads hooks from plugin's hooks file
- [ ] Test: `register_plugin_skills()` adds plugin skills to `SkillRegistry` with `plugin_name:skill_name` namespacing
- [ ] Test: disabled plugin (`enabled: false`) skipped by loader
- [ ] Test: invalid/malformed plugin.json gracefully skipped — errors collected, other plugins still load
- [ ] Test: plugin with mixed valid/invalid components — valid ones load, invalid ones produce errors in `PluginComponents.errors`
- [ ] All tests pass with `angreal test integration`

## Implementation Notes

### Key APIs
- `discover_plugins(plugins_root: &Path) -> Vec<LoadedPlugin>` — scans `cache/{marketplace}/{plugin}/` dirs
- `load_plugin_dir(dir: &Path) -> Option<LoadedPlugin>` — loads single plugin from directory
- `load_plugin_components(plugin: &LoadedPlugin) -> PluginComponents` — loads agents, skills, hooks
- `register_plugin_skills(registry: &SkillRegistry, components: &PluginComponents)` — merges into registry
- `merge_plugin_hooks(base: &mut HookConfig, plugin_hooks: Option<HookConfig>)` — merges hook configs

### Temp directory structure for tests
```
tmp/
  cache/
    test-marketplace/
      my-plugin/
        plugin.json          # PluginManifest
        skills/
          greeting.md        # Skill markdown
        agents/
          helper.md          # Agent definition
        hooks.json           # Hook configuration
```

### Dependencies
None — independent of TestHarnessBuilder. Can run in parallel with T-0102.

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/plugin_components.rs` with 10 tests
- Discovered: cache structure is `cache/{marketplace}/{plugin}/{version}/` (has version dir)
- Discovered: hooks loading requires explicit `hooks` path in manifest (auto-discovery sets path but component loader ignores it when manifest.hooks is None)
- Discovered: hooks.json must be wrapped in `{"hooks": {...}}` settings format
- All 10 tests pass
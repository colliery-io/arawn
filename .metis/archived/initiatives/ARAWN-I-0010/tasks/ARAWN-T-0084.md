---
id: plugin-discovery-and-loading
level: task
title: "Plugin discovery and loading pipeline — scan directories, validate, load components"
short_code: "ARAWN-T-0084"
created_at: 2026-04-04T03:19:38.150406+00:00
updated_at: 2026-04-04T11:44:48.378773+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin discovery and loading pipeline — scan directories, validate, load components

## Objective

Build the `PluginLoader` that discovers plugin directories, reads and validates manifests, and returns a list of `LoadedPlugin` structs ready for component loading. Also includes the `PluginRegistry` for tracking loaded plugins at runtime.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PluginLoader::discover(user_dir, project_dir) -> Vec<LoadedPlugin>` scans both directories
- [ ] Scans `~/.arawn/plugins/*/plugin.json` (user-global plugins)
- [ ] Scans `.arawn/plugins/*/plugin.json` (project-local plugins)
- [ ] `LoadedPlugin` struct: manifest, plugin_dir (resolved absolute path), source (User/Project/BuiltIn), enabled flag
- [ ] Reads and deserializes `plugin.json` from each plugin directory
- [ ] Calls `manifest.validate()` and collects errors — invalid plugins are skipped with warning
- [ ] Resolves relative component paths against plugin_dir (e.g. `./agents` → `/abs/path/to/plugin/agents`)
- [ ] `PluginRegistry`: stores loaded plugins, queryable by name, tracks enable/disable state
- [ ] Handles missing directories gracefully (returns empty list)
- [ ] Handles duplicate plugin names: project-local takes priority over user-global
- [ ] Unit tests: discover from temp dirs, missing dirs, duplicate names, invalid manifests skipped

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/loader.rs`
- Depends on T-0083 for PluginManifest
- This task does NOT load components (agents, skills, etc.) — that's T-0085
- The loader just discovers, validates, and resolves paths

## Status Updates

*To be added during implementation*
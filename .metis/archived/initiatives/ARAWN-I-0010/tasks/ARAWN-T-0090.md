---
id: plugin-installation-install
level: task
title: "Plugin installation — install/uninstall into versioned cache, installed_plugins.json registry"
short_code: "ARAWN-T-0090"
created_at: 2026-04-04T13:05:29.968053+00:00
updated_at: 2026-04-04T13:37:38.557814+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin installation — install/uninstall into versioned cache, installed_plugins.json registry

## Objective

Implement `install_plugin()` and `uninstall_plugin()` — resolve a plugin from a marketplace, clone/download its content into the versioned cache, register in `installed_plugins.json`, and update `enabledPlugins` in settings.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `install_plugin(identifier, scope)`: resolves plugin from marketplace, clones source into `cache/{marketplace}/{plugin}/{version}/`, registers in installed_plugins.json, enables in settings
- [ ] `uninstall_plugin(identifier, scope)`: removes from installed_plugins.json, disables in settings, optionally removes cache dir
- [ ] `InstalledPluginsRegistry` struct: reads/writes `installed_plugins.json` (version 2 format)
- [ ] Each install entry tracks: scope (user/project), install_path, version, installed_at timestamp
- [ ] Plugin source resolution: look up plugin in marketplace manifest, determine git repo + path + ref
- [ ] Git clone into cache: `git clone --depth 1 --branch {ref}` into temp, then copy plugin subdirectory into cache path
- [ ] If plugin has a `path` field in marketplace (monorepo), extract only that subdirectory
- [ ] Re-install (update): replace existing version in cache, update installed_plugins.json
- [ ] Enable in settings: add `"name@marketplace": true` to `enabledPlugins` in the appropriate scope's settings.json
- [ ] Integration test: install a plugin from a local git marketplace, verify cache structure and registry

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/installer.rs`
- Depends on T-0088 (cache structure, identifiers) and T-0089 (marketplace resolution)
- Git operations via `tokio::process::Command` for async
- Cache path: `~/.arawn/plugins/cache/{marketplace}/{plugin}/{version}/`
- Monorepo support: marketplace entry has `source.path` for subdirectory extraction

## Status Updates

*To be added during implementation*
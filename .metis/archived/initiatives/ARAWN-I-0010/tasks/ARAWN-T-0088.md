---
id: claude-code-compatibility-claude
level: task
title: "Claude Code compatibility — .claude-plugin/plugin.json path, cache directory structure, name@marketplace identifiers"
short_code: "ARAWN-T-0088"
created_at: 2026-04-04T13:05:27.414426+00:00
updated_at: 2026-04-04T13:34:00.836725+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Claude Code compatibility — .claude-plugin/plugin.json path, cache directory structure, name@marketplace identifiers

## Objective

Update the existing `plugins/` module to be Claude Code-compatible: read `.claude-plugin/plugin.json`, scan the versioned cache directory structure, and use `name@marketplace` identifiers throughout.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `manifest.rs`: Look for `.claude-plugin/plugin.json` first, fall back to `plugin.json` at root
- [ ] `loader.rs`: Scan cache structure `cache/{marketplace}/{plugin}/{version}/` instead of flat `plugins/*/`
- [ ] `loader.rs`: `discover_plugins()` takes `plugins_root: &Path` (e.g. `~/.arawn/plugins/`) and walks `cache/` subdirectory
- [ ] `PluginIdentifier` struct: `name` + `marketplace` fields, displays as `name@marketplace`, parses from string
- [ ] `LoadedPlugin` includes `PluginIdentifier` instead of just name
- [ ] `settings.rs`: `enabledPlugins` is `HashMap<String, bool>` where keys are `name@marketplace` format
- [ ] `--plugin-dir` support: Load plugin from arbitrary directory path, tagged as `name@inline`
- [ ] Unit tests: manifest found at `.claude-plugin/plugin.json`, cache dir traversal, identifier parsing/display

## Implementation Notes

- Update existing files, don't create new ones
- The cache structure is: `~/.arawn/plugins/cache/{marketplace-name}/{plugin-name}/{version}/`
- Inside each version dir, the manifest is at `.claude-plugin/plugin.json`
- Keep backward compat: also accept `plugin.json` at root for simple plugins

## Status Updates

*To be added during implementation*
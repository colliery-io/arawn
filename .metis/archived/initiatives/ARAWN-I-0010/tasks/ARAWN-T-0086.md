---
id: plugin-enable-disable-and-settings
level: task
title: "Plugin enable/disable and settings — per-project config, user config from manifest"
short_code: "ARAWN-T-0086"
created_at: 2026-04-04T03:19:40.720112+00:00
updated_at: 2026-04-04T11:47:46.347730+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin enable/disable and settings — per-project config, user config from manifest

## Objective

Add per-project enable/disable control for plugins and support for user-configurable settings declared in plugin manifests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Enable/disable:**
- [ ] Read `enabledPlugins` and `disabledPlugins` arrays from `.arawn/settings.json`
- [ ] Default behavior: all discovered plugins are enabled unless explicitly in `disabledPlugins`
- [ ] If `enabledPlugins` is set, only those plugins are enabled (whitelist mode)
- [ ] PluginLoader respects enable/disable when building the loaded plugin list
- [ ] Disabled plugins are still discovered (for listing) but their components are not loaded

**User config:**
- [ ] Parse `userConfig` field from plugin manifest into `HashMap<String, UserConfigField>`
- [ ] Load user-provided values from `.arawn/settings.json` under `pluginConfigs.<plugin-name>.options`
- [ ] Validate required fields are present — warn if missing
- [ ] Make config values available to hooks via environment variables (e.g. `PLUGIN_<KEY>`)
- [ ] Make config values available to MCP server env vars via `${user_config.KEY}` substitution

**Tests:**
- [ ] Unit test: plugin disabled by `disabledPlugins` not loaded
- [ ] Unit test: whitelist mode with `enabledPlugins`
- [ ] Unit test: user config loaded and validated
- [ ] Unit test: missing required config field produces warning

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/settings.rs`
- Depends on T-0083 (manifest), T-0084 (loader)
- No keychain/sensitive config for MVP — all values stored in settings.json
- Claude Code uses `${user_config.KEY}` substitution in hook commands and MCP env

## Status Updates

*To be added during implementation*
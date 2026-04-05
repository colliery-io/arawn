---
id: plugin-manifest-schema
level: task
title: "Plugin manifest schema — PluginManifest struct, plugin.json deserialization, validation"
short_code: "ARAWN-T-0083"
created_at: 2026-04-04T03:19:36.789165+00:00
updated_at: 2026-04-04T11:43:32.803564+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin manifest schema — PluginManifest struct, plugin.json deserialization, validation

## Objective

Define the `PluginManifest` struct and deserialization from `plugin.json`. This is the foundation type that everything else reads — pure types and validation, no I/O beyond reading a single file.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PluginManifest` struct with all fields matching I-0010 design: name (required), version, description, author, agents, skills, commands, hooks, mcpServers, tools, settings, userConfig
- [ ] `PluginAuthor` struct: name (required), email, url
- [ ] `UserConfigField` struct: type (string/number/boolean), title, description, required, default
- [ ] Serde Deserialize from JSON — all optional fields use `#[serde(default)]`
- [ ] Component paths (agents, skills, commands, tools) are `Option<String>` relative to plugin root
- [ ] `hooks` field: either inline `HookConfig` or a string path to `hooks.json`
- [ ] `mcpServers` field: `HashMap<String, McpServerConfig>` matching MCP manager's config format
- [ ] `PluginManifest::validate()` method: checks name is present, paths start with `./`, returns `Vec<PluginError>`
- [ ] `PluginError` enum for structured error reporting (MissingField, InvalidPath, ParseError)
- [ ] Unit tests: valid manifest, minimal manifest (name only), missing name errors, invalid paths, hooks as string vs inline

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/manifest.rs`
- The `hooks` field needs a custom deserializer: either a `HookConfig` object or a string path
- `mcpServers` should reuse or mirror the config type from `arawn-mcp`
- Follow Claude Code's naming: kebab-case names, `./` relative paths

## Status Updates

*To be added during implementation*
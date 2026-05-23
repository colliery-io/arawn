---
id: built-in-plugin-registration-code
level: task
title: "Built-in plugin registration — code-defined plugins with same manifest interface"
short_code: "ARAWN-T-0087"
created_at: 2026-04-04T03:19:41.729184+00:00
updated_at: 2026-04-04T11:48:51.163797+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Built-in plugin registration — code-defined plugins with same manifest interface

## Objective

Allow plugins to be defined in Rust code (compiled into the binary) using the same `LoadedPlugin` interface as disk plugins. This lets us ship default skills, hooks, and agent defs as built-in plugins that users can enable/disable.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BuiltinPluginDef` struct: name, description, version, skills (Vec<SkillDefinition>), hooks (Option<HookConfig>), agents (Vec<AgentDefinition>)
- [ ] `register_builtin_plugin(def: BuiltinPluginDef) -> LoadedPlugin` converts to LoadedPlugin with source=BuiltIn
- [ ] `builtin_plugins() -> Vec<LoadedPlugin>` returns all registered built-in plugins
- [ ] Built-in plugins appear in PluginRegistry alongside disk plugins
- [ ] Built-in plugins are enabled by default but can be disabled via `disabledPlugins`
- [ ] Disk plugins with the same name take priority over built-in plugins
- [ ] At least one example built-in plugin (e.g., a "core" plugin providing default skills)
- [ ] Unit tests: built-in registered, disk overrides built-in, disable built-in

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/builtin.rs`
- Depends on T-0083 (manifest), T-0084 (loader), T-0085 (components)
- Claude Code uses `registerBuiltinPlugin()` with a similar pattern
- Built-in plugins don't have a directory on disk — their components are provided inline

## Status Updates

*To be added during implementation*
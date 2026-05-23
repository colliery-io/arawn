---
id: plugin-component-loading-agents
level: task
title: "Plugin component loading — agents, skills, hooks, MCP servers, tools from plugin directories"
short_code: "ARAWN-T-0085"
created_at: 2026-04-04T03:19:39.392865+00:00
updated_at: 2026-04-04T11:46:31.083004+00:00
parent: ARAWN-I-0010
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0010
---

# Plugin component loading — agents, skills, hooks, MCP servers, tools from plugin directories

## Objective

For each loaded plugin, load its declared components into the engine's registries: agents into AgentDefRegistry, skills into SkillRegistry, hooks into HookConfig, MCP server configs into McpManager, and .arawn_tool files into ToolRegistry.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Agents:**
- [ ] If manifest has `agents` path, load `.md` files from that directory using `load_agents_dir()` (existing)
- [ ] Agents namespaced with plugin name: `plugin-name:agent-name`
- [ ] Loaded agents merged into the engine's agent list

**Skills:**
- [ ] If manifest has `skills` path, load skills from that directory using `load_skills_dir()` (existing)
- [ ] Skills get `SkillSource::Plugin(plugin_name)` source
- [ ] Skills namespaced: `plugin-name:skill-name`
- [ ] Loaded skills registered into SkillRegistry

**Hooks:**
- [ ] If manifest has `hooks` field (inline or path), load hook config
- [ ] If string path, load from that file using `load_hooks_from_file()`
- [ ] If inline object, deserialize directly as HookConfig
- [ ] Plugin hooks merged into engine's HookConfig

**MCP Servers:**
- [ ] If manifest has `mcpServers`, extract server configs
- [ ] Forward configs to MCP manager for connection (existing McpManager API)

**Tools (.arawn_tool):**
- [ ] If manifest has `tools` path, load .arawn_tool dylibs using existing PluginLoader
- [ ] Tools registered as plugin tools in ToolRegistry

**Integration:**
- [ ] `load_plugin_components(plugin: &LoadedPlugin, ...)` function that does all of the above
- [ ] Errors in individual components don't prevent other components from loading (log and continue)
- [ ] Integration test: plugin with agents + hooks + skills loads all three successfully

## Implementation Notes

- Create `crates/arawn-engine/src/plugins/components.rs`
- Depends on T-0083 (manifest) and T-0084 (loader)
- Reuses all existing loading functions — this is the glue layer
- Namespacing prevents collisions between plugins providing same-named agents/skills

## Status Updates

*To be added during implementation*
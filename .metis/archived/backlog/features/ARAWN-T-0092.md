---
id: hot-reload-arawn-toml-watch-config
level: task
title: "Hot-reload arawn.toml — watch config file and live-update permissions, MCP servers, and engine settings"
short_code: "ARAWN-T-0092"
created_at: 2026-04-04T15:03:36.093094+00:00
updated_at: 2026-04-04T16:36:06.576695+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Hot-reload arawn.toml — watch config file and live-update permissions, MCP servers, and engine settings

## Objective

Arawn is a long-lived process. When a user edits `arawn.toml`, the changes should take effect without a restart. This means watching the config file and live-updating: permissions, MCP server connections, and engine settings.

### Priority: P1 | Effort: M

## Current state

- `arawn.toml` is read once at startup
- Permissions are loaded into `PermissionChecker` and never updated
- MCP servers are connected via `McpManager::connect_all()` once and never re-checked
- Engine config (model, max_iterations, etc.) is baked into `QueryEngineConfig` at startup
- Plugin MCP servers are also connect-once

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Config watcher:**
- [ ] `notify`-based file watcher on `arawn.toml` (and `settings.json`) — debounced, spawned as background task in serve mode
- [ ] On change: re-parse config, diff against current state, apply updates

**Permissions hot-reload:**
- [ ] `PermissionChecker` needs an `update_rules()` method (currently rules are immutable after construction)
- [ ] Watcher reloads permission rules from config and calls `update_rules()`
- [ ] Active sessions pick up new rules on next tool call (no session restart needed)

**MCP server hot-reload:**
- [ ] `McpManager` needs `connect_server()` to be public (it already is internally)
- [ ] `McpManager` needs a `disconnect_server(name)` method — drop the `RunningService`, unregister its tools from `ToolRegistry`
- [ ] Watcher diffs MCP server list: new servers → connect, removed servers → disconnect, changed servers → reconnect
- [ ] Tool registry needs `unregister_by_prefix(prefix)` to remove all tools from a disconnected MCP server (tools are named `mcp__{server}__{tool}`)

**Engine config hot-reload:**
- [ ] `ArawnConfig` behind `Arc<RwLock<ArawnConfig>>` — single shared config object
- [ ] All config reads go through the shared reference — values update on next use
- [ ] `QueryEngine` reads model, max_iterations, max_tokens from shared config each turn instead of caching in owned fields
- [ ] `ConfigWatcher` simply re-parses `arawn.toml` and swaps the value under the lock

**Plugin MCP servers:**
- [ ] Same diff logic as config MCP servers — new plugin MCP servers connect, removed ones disconnect
- [ ] Plugin cache watcher (already exists) should also trigger MCP reconnection

## Implementation Notes

### Architecture

```
ConfigWatcher (background task)
  │
  ├── watches arawn.toml
  ├── watches settings.json
  │
  ├── on arawn.toml change:
  │   ├── reload ArawnConfig
  │   ├── diff permissions → update PermissionChecker
  │   ├── diff MCP servers → connect/disconnect via McpManager
  │   └── diff engine config → update QueryEngineConfig (if feasible)
  │
  └── on settings.json change:
      └── reload plugin enable/disable → re-apply to PluginRegistry
```

### Key changes needed

1. **`PermissionChecker`** — wrap rules in `RwLock<Vec<PermissionRule>>`, add `update_rules()`. Currently in `crates/arawn-engine/src/permissions/checker.rs`.

2. **`McpManager`** — add `disconnect_server(name)`, make it require `Arc<ToolRegistry>` so it can unregister tools. Currently in `crates/arawn-mcp/src/manager.rs`.

3. **`ToolRegistry`** — add `unregister_by_prefix(prefix: &str)` to bulk-remove MCP tools when a server disconnects. Currently in `crates/arawn-engine/src/tool.rs`.

4. **`ConfigWatcher`** — new struct in `crates/arawn/src/` that watches files and dispatches updates. Needs `Arc` references to PermissionChecker, McpManager, PluginRuntime.

5. **`main.rs` serve mode** — wrap PermissionChecker and McpManager in Arc, pass to ConfigWatcher, spawn watcher task.

### What NOT to do
- Don't reload LLM client credentials live (security risk, restart is fine)
- Don't hot-swap the LLM provider (too many moving parts)
- Don't re-read config on every tool call (too expensive) — file watcher is event-driven

## Status Updates

*To be added during implementation*
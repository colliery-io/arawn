# Plugins

*Reference. Plugin manifest, marketplaces, install scopes, components, lifecycle.*

Plugins extend arawn with new agent tools, sub-agent types, skills, and MCP servers. They live under `<data_dir>/plugins/` and are loaded at server startup with hot-reload on file change.

Source: `crates/arawn-engine/src/plugins/`. CLI: `crates/arawn/src/plugin_cmd.rs`.

## Lifecycle in one paragraph

A plugin is a directory or git-fetchable archive containing a `plugin.json` manifest plus the assets it declares. You add a marketplace (the source-of-truth registry), `arawn plugin install <name>@<marketplace>` fetches and unpacks it, and the next time arawn loads or hot-reloads, the plugin's components (tools / agents / skills / MCP servers) become available. Hot-reload watches the install root via `notify` and reloads on disk changes.

## Manifest format

`plugin.json` (parsed by `crates/arawn-engine/src/plugins/manifest.rs::PluginManifest`):

```jsonc
{
  "name": "my-plugin",
  "version": "0.1.0",
  "description": "Short summary shown in `/plugins` and `arawn plugin list`.",
  "commands": "./commands",         // directory containing slash-command markdown files
  "tools": "./tools",               // directory containing tool dylib + manifest
  "agents": "./agents",             // directory containing agent definition .md files
  "skills": "./skills",             // directory containing skill .md files
  "mcpServers": [                   // array of MCP server defs (same shape as [[mcp.servers]])
    { "name": "...", "command": "...", "args": [...], "env": {...}, "enabled": true }
  ],
  "userConfig": { /* optional plugin-specific user config schema */ }
}
```

> **Note:** the manifest deserializer is `#[serde(rename_all = "camelCase")]` (see `arawn-engine/src/plugins/manifest.rs`). Use `mcpServers` and `userConfig` — snake_case keys won't parse.

Every component path is optional — a plugin can declare only the surfaces it cares about.

## Install scopes

Pass `--scope user|project` to `arawn plugin install` and `arawn plugin uninstall`. Scope is an enablement / registry attribute — it controls which installs are *visible* to the running server, not the on-disk cache path. All plugins land under `<data_dir>/plugins/cache/<marketplace>/<plugin>/<version>/` regardless of scope; the install record (in `installed_plugins.json`) records whether it's a `user`- or `project`-scoped install, and `project` installs may carry an optional `project_path` constraint.

## Marketplaces

A marketplace is a source-of-truth for resolving `name@marketplace` to a fetchable URL. Three source types:

| Source | Form | Example |
|---|---|---|
| GitHub `org/repo` | bare `org/repo` slug | `dstorey/arawn-plugins` |
| Arbitrary git URL | full URL | `git@github.com:my-org/marketplace.git` |
| Local directory | filesystem path | `/Users/dylan/code/local-plugins` |

Register one:

```sh
arawn plugin marketplace add dstorey/arawn-plugins
arawn plugin marketplace add git@github.com:my-org/marketplace.git
arawn plugin marketplace add /Users/dylan/code/local-plugins
```

List:

```sh
arawn plugin marketplace list
```

## Install / uninstall / enable / disable

```sh
arawn plugin install my-plugin@dstorey/arawn-plugins
arawn plugin install my-plugin@dstorey/arawn-plugins --scope project
arawn plugin uninstall my-plugin@dstorey/arawn-plugins
arawn plugin enable my-plugin@dstorey/arawn-plugins
arawn plugin disable my-plugin@dstorey/arawn-plugins
arawn plugin list
```

Disable preserves install state but skips loading. Re-enable to bring it back without re-installing.

## On-disk layout

```
<data_dir>/plugins/
├── installed_plugins.json   # registry: which plugins, which version, install scope
├── tools/                   # plugin tool dylibs (per V1 DataLayout)
├── build/                   # plugin compilation scratch
└── <plugin-name>/           # one directory per installed plugin
    ├── plugin.json
    └── <component-dirs>/
```

`<data_dir>/settings.json` carries the enabled/disabled state and per-plugin `user_config` overrides.

## Components

| Component | Where it lands | Documentation |
|---|---|---|
| Tools | New agent tools, callable like built-ins. | [Agent tools reference](./agent-tools.md). |
| Agents | New sub-agent types selectable via the `agent` tool's `subagent_type`. | [Sub-agents reference](./sub-agents.md). |
| Skills | New `/skill-name` slash commands and skill-tool invocations. | [Skills reference](./skills.md). |
| MCP servers | New MCP servers loaded alongside `[[mcp.servers]]` from `arawn.toml`. | [MCP reference](./mcp.md). |
| Commands | Plugin-provided slash commands distinct from skills. | Plugin authoring guide (forthcoming). |

The same hot-reload watcher picks up changes inside any of these subdirectories.

## Built-in plugins

Built-ins live in `crates/arawn-engine/src/plugins/builtin.rs` and are loaded unconditionally. Currently lightweight; most arawn functionality is in the engine proper rather than as plugins.

## Inspecting at runtime

```
/plugins
```

Lists installed plugins, version, scope, enabled state.

```sh
arawn plugin list
```

CLI equivalent. The CLI command operates against `installed_plugins.json` directly so you can inspect without a running server.

## Hot-reload

`crates/arawn-engine/src/plugins/runtime.rs` watches the plugin install root and the per-project root via `notify`. Changes (a new manifest, a swapped dylib, a deleted directory) trigger a reload pass that:

1. Re-discovers components.
2. Diffs against the current set.
3. Adds/removes/updates the live tool/agent/skill/MCP registries.

Hot-reload is bounded — a broken plugin doesn't break arawn; the failed plugin is logged and the rest continue.

## Related

- [CLI reference](./cli.md) — `arawn plugin {install,uninstall,enable,disable,list,marketplace}`.
- [MCP reference](./mcp.md) — plugins can declare MCP servers.
- [Skills reference](./skills.md) — plugins can declare skills.
- [Sub-agents reference](./sub-agents.md) — plugins can declare agent types.
- [Data directory reference](./data-directory.md) — where plugin files live.

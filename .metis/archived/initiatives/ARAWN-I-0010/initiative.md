---
id: plugin-framework-unified-plugin
level: initiative
title: "Plugin framework — unified plugin manifest for tools, agents, skills, hooks, and MCP servers"
short_code: "ARAWN-I-0010"
created_at: 2026-04-03T02:06:19.695516+00:00
updated_at: 2026-04-04T17:46:00.770424+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: plugin-framework-unified-plugin
---

# Plugin framework — unified plugin manifest for tools, agents, skills, hooks, and MCP servers Initiative

## Context

The goal is to natively support Claude Code plugins in arawn — **a user says "arawn go get this plugin" and it works because we support the structure day one.** No rewrites, no arawn-specific format. Claude Code plugins work in arawn.

Claude Code's plugin system uses: marketplace sources (GitHub repos, URLs, npm packages) → plugin discovery → versioned cache (`~/.claude/plugins/cache/{marketplace}/{plugin}/{version}/`) → `.claude-plugin/plugin.json` manifest → component loading. Plugins are identified as `name@marketplace` and managed via `enabledPlugins` in settings.json.

We've already built the component subsystems: hooks (I-0008), permissions (I-0009), MCP client (I-0007), agent defs (T-0052), skills (T-0058). We've also built manifest parsing, component loading, settings, and built-in plugins (T-0083 through T-0087). What's missing is the marketplace/installation/cache layer that makes `arawn plugin install metis@colliery-io-metis` work.

### What's done (T-0083 through T-0087)
- PluginManifest parsing and validation
- Plugin discovery from directories
- Component loading (agents, skills, hooks, MCP, tools)
- Enable/disable with user config
- Built-in plugin registration

### What's missing
- `.claude-plugin/plugin.json` manifest path (we only look for `plugin.json` at root)
- Marketplace system — fetch marketplace.json from GitHub/URL/git sources
- Plugin installation — clone/download into versioned cache
- Cache directory structure: `~/.arawn/plugins/cache/{marketplace}/{plugin}/{version}/`
- `installed_plugins.json` registry tracking installs per scope
- `known_marketplaces.json` registry of marketplace sources
- `name@marketplace` identifier format in `enabledPlugins`
- `--plugin-dir` flag for local development
- `arawn plugin install/uninstall/enable/disable` CLI commands

## Goals & Non-Goals

**Goals:**
- **Claude Code plugin compatibility** — read `.claude-plugin/plugin.json` natively, same directory structure
- **Marketplace system** — register marketplace sources (GitHub repos, git URLs), fetch marketplace.json, discover available plugins
- **Plugin installation** — clone/download plugins into versioned cache `~/.arawn/plugins/cache/{marketplace}/{plugin}/{version}/`
- **`name@marketplace` identifiers** — `enabledPlugins` in settings.json uses this format
- **`installed_plugins.json`** — tracks what's installed, at what version, in what scope (user/project)
- **`known_marketplaces.json`** — registry of marketplace sources
- **`--plugin-dir` flag** — point at a local plugin directory for development (no install needed)
- **Component loading** — agents, skills, hooks, MCP servers, commands from plugin directories (done)
- **Enable/disable per project** — settings.json scoping (done)
- **Built-in plugins** — code-defined, same interface (done)

**Non-Goals:**
- npm/pip package sources (GitHub/git only for now)
- Plugin dependency resolution
- Enterprise policy enforcement / managed plugins
- Keychain-backed sensitive config
- MCPB bundles, LSP servers, output styles, channels
- Interactive TUI plugin manager (CLI only for now)
- Auto-update / version pinning with semver ranges

## Architecture

### Overview

Arawn natively consumes Claude Code plugins. The manifest lives at `.claude-plugin/plugin.json` (Claude Code's standard path). Plugins are discovered via marketplaces, installed into a versioned cache, and identified as `name@marketplace`.

### Directory Structure (matching Claude Code)

```
~/.arawn/plugins/
├── known_marketplaces.json           # Registry of marketplace sources
├── installed_plugins.json            # What's installed, versions, scopes
├── marketplaces/                     # Cloned marketplace repos
│   ├── colliery-io-metis/
│   │   └── .claude-plugin/
│   │       └── marketplace.json
│   └── my-company/
│       └── .claude-plugin/
│           └── marketplace.json
├── cache/                            # Installed plugin content
│   ├── colliery-io-metis/
│   │   └── metis/
│   │       └── 2.0.4/
│   │           ├── .claude-plugin/
│   │           │   └── plugin.json   # ← The manifest
│   │           ├── agents/
│   │           ├── skills/
│   │           ├── hooks/
│   │           └── commands/
│   └── another-marketplace/
│       └── some-plugin/
│           └── 1.0.0/
└── data/                             # Per-plugin persistent state
    └── metis@colliery-io-metis/
```

### Plugin Identifiers

Plugins use `name@marketplace` format:
- `metis@colliery-io-metis`
- `formatter@anthropic-tools`
- `my-plugin@inline` (for `--plugin-dir`)

### Settings Format

```json
{
  "enabledPlugins": {
    "metis@colliery-io-metis": true,
    "formatter@anthropic-tools": true,
    "unwanted@some-market": false
  },
  "extraKnownMarketplaces": {
    "my-company": {
      "source": { "source": "github", "repo": "myorg/plugins" }
    }
  }
}
```

### Installation Flow

```
arawn plugin install metis@colliery-io-metis
  │
  ├── Look up marketplace "colliery-io-metis" in known_marketplaces.json
  ├── Load marketplace.json → find "metis" plugin entry
  ├── Clone/download plugin source → cache/{marketplace}/{plugin}/{version}/
  ├── Read .claude-plugin/plugin.json → validate manifest
  ├── Register in installed_plugins.json
  └── Add to settings.json: enabledPlugins["metis@colliery-io-metis"] = true
```

### Startup Loading Flow

```
Startup
  │
  ├── Read settings.json → enabledPlugins map
  ├── For each enabled plugin:
  │       ├── Resolve cache path: cache/{marketplace}/{plugin}/{version}/
  │       ├── Read .claude-plugin/plugin.json
  │       ├── Load components (agents, skills, hooks, MCP, commands)
  │       └── Register into engine registries
  │
  ├── Load --plugin-dir plugins (session-only, no cache)
  └── Load built-in plugins
```

### Marketplace Source Types (MVP)

| Source | Example | How it works |
|--------|---------|-------------|
| `github` | `{ "source": "github", "repo": "colliery-io/metis" }` | Clone repo, read marketplace.json |
| `git` | `{ "source": "git", "url": "https://..." }` | Clone URL |
| `directory` | `{ "source": "directory", "path": "/local/path" }` | Read directly |

## Detailed Design

### Marketplace Manifest (marketplace.json)

```json
{
  "name": "colliery-io-metis",
  "plugins": [
    {
      "name": "metis",
      "version": "2.0.4",
      "description": "Flight Levels methodology plugin",
      "source": {
        "source": "github",
        "repo": "colliery-io/metis",
        "path": "plugins/metis"
      }
    }
  ]
}
```

### known_marketplaces.json

```json
{
  "colliery-io-metis": {
    "source": { "source": "github", "repo": "colliery-io/metis" },
    "installLocation": "~/.arawn/plugins/marketplaces/colliery-io-metis",
    "lastUpdated": "2026-04-04T12:00:00Z"
  }
}
```

### installed_plugins.json

```json
{
  "version": 2,
  "plugins": {
    "metis@colliery-io-metis": [
      {
        "scope": "user",
        "installPath": "~/.arawn/plugins/cache/colliery-io-metis/metis/2.0.4",
        "version": "2.0.4",
        "installedAt": "2026-04-04T12:00:00Z"
      }
    ]
  }
}
```

### Existing code to update

The existing `plugins/` module (T-0083–T-0087) needs:
- `manifest.rs`: Also look for `.claude-plugin/plugin.json` (not just `plugin.json` at root)
- `loader.rs`: Scan the cache directory structure instead of flat `plugins/*/`
- `settings.rs`: `enabledPlugins` as `HashMap<String, bool>` with `name@marketplace` keys instead of arrays

## Testing Strategy

- **Unit tests** for marketplace manifest parsing and validation
- **Unit tests** for cache directory resolution (`cache/{market}/{plugin}/{version}/.claude-plugin/plugin.json`)
- **Integration test** for full install flow: add marketplace → install plugin → verify in cache → load components
- **Integration test** for startup loading: pre-populated cache → read settings → load enabled plugins
- **Test with real Claude Code plugin**: install metis@colliery-io-metis, verify agents/skills/hooks load

## Alternatives Considered

- **Arawn-specific plugin format**: Invent our own manifest and directory structure. Rejected — creates friction for the existing Claude Code plugin ecosystem. Compatibility is the whole point.
- **Symlink Claude Code's cache**: Just read `~/.claude/plugins/cache/` directly. Rejected — assumes Claude Code is installed, doesn't let arawn manage its own installs.
- **npm/pip as package managers**: Delegate to existing package managers. Rejected for MVP — adds heavy dependencies. GitHub clone is sufficient and matches Claude Code's primary source.

## Implementation Plan

**Phase 1 (done, T-0083–T-0087):** Manifest parsing, component loading, settings, built-in plugins.

**Phase 2 — Claude Code compatibility updates:**
Update existing `plugins/` module to read `.claude-plugin/plugin.json`, scan cache directory structure, use `name@marketplace` identifiers.

**Phase 3 — Marketplace system:**
`MarketplaceSource` types (github, git, directory), `known_marketplaces.json` read/write, marketplace fetching (git clone), marketplace manifest parsing.

**Phase 4 — Plugin installation:**
`install_plugin()` — resolve from marketplace, clone/download into cache, register in `installed_plugins.json`, enable in settings. `uninstall_plugin()` — remove from cache and registries.

**Phase 5 — CLI commands:**
`arawn plugin install/uninstall/enable/disable/list/marketplace add/marketplace list`

**Phase 6 — --plugin-dir flag:**
Session-only plugin loading from arbitrary directory path, tagged as `name@inline`.

### Future work (not in scope)
- npm/pip source types
- Plugin dependency resolution
- Enterprise policy enforcement
- Keychain-backed sensitive config
- MCPB bundles, LSP servers, output styles
- Interactive TUI plugin manager
- Auto-update with semver ranges
- Hot-reload on plugin file changes
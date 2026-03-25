# File Paths & Directory Layout

Complete reference for all files and directories used by Arawn.

## Config Directory

**Location:** `~/.config/arawn/` (or `ARAWN_CONFIG_DIR` or `$XDG_CONFIG_HOME/arawn/`)

| Path | Type | Description |
|------|------|-------------|
| `config.toml` | File | Primary configuration file (TOML format) |
| `client.yaml` | File | Client-specific configuration |
| `env` | File | Environment variable overrides (sourced at startup) |
| `arawn-wrapper.sh` | File | Shell wrapper script for daemon management |
| `arawn.pid` | File | PID file for the running server process |
| `oauth-tokens.json` | File | Cached OAuth 2.0 access and refresh tokens |
| `identity.age` | File | age encryption identity key for the secret store |
| `secrets.age` | File | Age-encrypted JSON map of all secrets |
| `logs/` | Directory | Log file directory |
| `plugins/` | Directory | User-level plugin installations |
| `plugins.json` | File | Runtime plugin state (enabled/disabled, subscriptions) |
| `workflows/` | Directory | Workflow definition files (TOML) |
| `memory.db` | File | SQLite database for persistent memory (facts, conversations, thoughts) |
| `memory.graph.db` | File | Graph database for memory relationships |
| `workstreams.db` | File | SQLite database for workstream metadata |
| `pipeline.db` | File | SQLite database for pipeline engine state |

---

## Data Directory

**Location:** `~/.arawn/` (or `ARAWN_BASE_PATH` or `[paths].base_path`)

This directory stores workstream file data -- the actual files and directories that workstreams operate on.

| Path | Type | Description |
|------|------|-------------|
| `<workstream-name>/` | Directory | Named workstream root |
| `scratch/` | Directory | Scratch (ephemeral) workstream root |

---

## Project-Local Files

Files that can exist in any project directory.

| Path | Type | Description |
|------|------|-------------|
| `./arawn.toml` | File | Project-local configuration overlay. Merged on top of user config. |
| `./plugins/` | Directory | Project-local plugin installations |
| `.arawn/plugins.json` | File | Project-specific plugin subscriptions and enabled state |

---

## Installation Files

| Path | Type | Platform | Description |
|------|------|----------|-------------|
| `~/.local/bin/arawn` | File | All | Arawn binary (user-local install) |
| `~/Library/LaunchAgents/io.colliery.arawn.plist` | File | macOS | launchd service definition for auto-start |
| `~/.config/systemd/user/arawn.service` | File | Linux | systemd user service unit file |

---

## Backup Directory

**Location:** `~/.arawn-backups/`

Backups are stored in timestamped directories. The number of retained backups is controlled by `ARAWN_KEEP_BACKUPS` (default: 30).

| Path | Type | Description |
|------|------|-------------|
| `~/.arawn-backups/` | Directory | Backup root |
| `~/.arawn-backups/YYYY-MM-DD_HHMMSS/` | Directory | Individual backup snapshot |

Each backup snapshot contains a copy of the config directory databases and workstream metadata at the time of creation.

---

## Workstream Directory Structure

Workstreams organize file data into isolated directory trees.

### Named Workstreams

Named workstreams have a persistent, named directory under the base path.

```
~/.arawn/<workstream-name>/
├── production/              # Stable, committed work products
└── work/                    # Active working area for the agent
```

| Directory | Description |
|-----------|-------------|
| `production/` | Finalized outputs. Files promoted from work/ when complete. |
| `work/` | Active workspace. The agent reads and writes files here during sessions. |

### Scratch Workstreams

Scratch workstreams are ephemeral and session-scoped.

```
~/.arawn/scratch/
└── sessions/
    └── <session-id>/
        └── work/            # Session-scoped working area
```

| Directory | Description |
|-----------|-------------|
| `sessions/<id>/work/` | Isolated working directory for a single session. Cleaned up after `scratch_cleanup_days` (default: 7 days). |

---

## Log Files

**Location:** `~/.config/arawn/logs/`

Arawn uses daily log rotation with two output formats.

| Path | Format | Description |
|------|--------|-------------|
| `logs/arawn.log.YYYY-MM-DD` | File | Daily rotated log file |

Each log file contains entries in two formats:
- **Console format:** Human-readable, colored output for terminal viewing
- **JSON format:** Structured JSON lines for machine parsing and log aggregation

Log rotation creates a new file at midnight. Old log files are retained indefinitely (managed by external log rotation tools or manual cleanup).

---

## WASM Cache

Compiled WASM modules used by the pipeline engine are cached using SHA-256 content addressing.

| Path | Description |
|------|-------------|
| `~/.config/arawn/wasm-cache/` | Compiled WASM module cache directory |
| `~/.config/arawn/wasm-cache/<sha256-hash>.wasm` | Individual cached compiled module |

The cache key is the SHA-256 hash of the WASM binary content. Recompilation is skipped when a cached module with a matching hash exists.

---

## Config File Resolution

Configuration is loaded by discovering and merging files in order (later overrides earlier):

| Priority | Source | Path |
|----------|--------|------|
| 1 (lowest) | User config | `~/.config/arawn/config.toml` |
| 2 | Project config | `./arawn.toml` |
| 3 (highest) | CLI arguments | (passed at runtime) |

Both files are optional. When neither exists, built-in defaults are used.

---

## Secret Store Layout

The secret store uses age encryption for sensitive values. All secrets are stored
in a single encrypted JSON map file.

| Path | Description |
|------|-------------|
| `~/.config/arawn/identity.age` | age identity (private key) for decryption |
| `~/.config/arawn/secrets.age` | Encrypted JSON map of all secrets (`{"name": "value", ...}`) |

Secrets are referenced in tool parameters using `${{secrets.<name>}}` syntax. The `arawn secrets` CLI manages the store:

| Command | Description |
|---------|-------------|
| `arawn secrets set <name>` | Encrypt and store a secret |
| `arawn secrets list` | List stored secret names (never displays values) |
| `arawn secrets delete <name>` | Remove a secret |

---

## Database Files

All databases use SQLite.

| Database | Path | Contents |
|----------|------|----------|
| Memory | `~/.config/arawn/memory.db` | Facts, conversations, thoughts, user messages, assistant messages, web content, file content |
| Memory Graph | `~/.config/arawn/memory.graph.db` | Relationship graph between memory entries |
| Workstreams | `~/.config/arawn/workstreams.db` | Workstream metadata, session records, file manifests |
| Pipeline | `~/.config/arawn/pipeline.db` | Workflow definitions, execution history, schedule state |

---

## Plugin Installation Paths

| Path | Scope | Description |
|------|-------|-------------|
| `~/.config/arawn/plugins/<plugin-name>/` | User | Global plugin installation |
| `./plugins/<plugin-name>/` | Project | Project-local plugin installation |
| `<custom-dir>/<plugin-name>/` | Custom | From `[plugins].dirs` config entries |

Within each plugin directory, the expected structure is:

```
<plugin-name>/
├── .claude-plugin/
│   └── plugin.json
├── skills/
├── agents/
├── hooks/
└── tools/
```

See [Plugin Manifest Reference](plugin-manifest.md) for details.

---

## Workflow Files

| Path | Description |
|------|-------------|
| `~/.config/arawn/workflows/<name>.toml` | Workflow definition file |

Workflow definitions are TOML files created via the `workflow` tool's `create` action or manually placed in the workflows directory.

---

## Summary of Key Paths

| Purpose | Default Path | Override |
|---------|-------------|---------|
| Configuration | `~/.config/arawn/` | `ARAWN_CONFIG_DIR` |
| Workstream data | `~/.arawn/` | `ARAWN_BASE_PATH` |
| Backups | `~/.arawn-backups/` | — |
| Binary | `~/.local/bin/arawn` | — |
| Project config | `./arawn.toml` | — |
| Project plugins | `./plugins/` | — |

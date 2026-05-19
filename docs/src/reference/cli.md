# CLI flags and subcommands

*Reference. Every flag and subcommand the `arawn` binary accepts.*

`arawn` is a single binary. With no subcommand it sends a one-shot prompt to a running server. With a subcommand (`serve`, `tui`, `plugin`, `doctor`, `usage`) it does whatever that subcommand says.

Source: `crates/arawn/src/main.rs`; plugin subtree in `crates/arawn/src/plugin_cmd.rs`.

## Top-level

```
arawn [GLOBAL OPTIONS] [SUBCOMMAND | PROMPT]
```

### Global options

| Flag | Type | Default | Description |
|---|---|---|---|
| `--data-dir <path>` | path | `~/.arawn` (or `ARAWN_DATA_DIR`) | Data directory containing the config file, databases, feed data, plugin install, token store, etc. |
| `--session <uuid>` | UUID | none | Resume an existing session. |
| `--list-sessions` | bool | false | Print all resumable sessions and exit. |
| `<prompt>` | trailing | none | One-shot prompt when no subcommand is given. Sent to the running server; streams the response and exits. |

### One-shot mode

```sh
arawn "draft a one-line commit message for the diff in this repo"
```

Sends the prompt to a `serve`-running server and streams the response. Useful for shell scripts.

## `arawn serve`

Start the WebSocket server.

| Flag | Default | Description |
|---|---|---|
| `--port <u16>` | `3100` | TCP port to listen on. The server binds `127.0.0.1:<port>`. |

The server hosts the engine, the integration registry, the feed runtime, the plugin runtime, the workflow scheduler, and the WebSocket endpoint at `ws://127.0.0.1:<port>/ws`.

## `arawn tui`

Launch the TUI client.

| Flag | Default | Description |
|---|---|---|
| `--url <ws-url>` | `ws://127.0.0.1:3100/ws` | WebSocket URL of a running server. |

The TUI is a thin client — it shows what the server reports. Quit with `Ctrl+C`; the server keeps running.

## `arawn plugin`

Plugin management. Subcommands:

| Subcommand | Description |
|---|---|
| `install <name@marketplace> [--scope user\|project]` | Install a plugin from a marketplace. |
| `uninstall <name@marketplace>` | Remove an installed plugin. |
| `enable <name@marketplace>` | Enable a previously-disabled plugin. |
| `disable <name@marketplace>` | Disable without uninstalling. |
| `list` | List installed plugins and their enabled state. |
| `marketplace add <org/repo \| git-url \| path>` | Register a marketplace source. Three forms: GitHub `org/repo`, arbitrary git URL, local directory path. |
| `marketplace list` | List registered marketplaces. |

`--scope user|project` controls where the install lands. `user` (default) writes to `<data_dir>/plugins/`; `project` writes to a per-project sub-tree (useful when running arawn against a checked-out project).

See [plugins reference](./plugins.md) for the manifest format, lifecycle, and on-disk layout.

## `arawn doctor`

Run diagnostic checks against the local install. Validates config, integration credentials, sandbox availability, the embedder model, the data directory layout, and provider reachability.

| Flag | Description |
|---|---|
| `--json` | Emit machine-readable JSON instead of human-readable text. |

Exit code is non-zero if any check fails.

## `arawn usage`

Show token usage rollups recorded by the local LLM tracker.

| Flag | Default | Description |
|---|---|---|
| `--period <day\|week\|month\|all>` | `week` | Window to aggregate over. |
| `--model <name>` | none | Filter to one model. |
| `--by-site` | false | Group rollups by `call_site` tag (engine, extractor, compactor, ceremony, etc.) as well as model. |
| `--json` | false | Emit JSON instead of human-readable text. |

## Environment variables

`--data-dir` also reads from `ARAWN_DATA_DIR`. The full env-var list is at [environment variables reference](./env-vars.md).

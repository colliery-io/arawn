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

## `arawn init`

Write a starter `arawn.toml` with one LLM profile into the data directory.

| Flag | Default | Description |
|---|---|---|
| `--provider <name>` | `groq` | LLM provider: `groq`, `openai`, `anthropic`, `ollama`, or a different OpenAI-compatible provider. |
| `--model <name>` | built-in default | Model name. |
| `--api-key-env <VAR>` | per provider | The env var that holds the API key, for example `GROQ_API_KEY`. |
| `--force` | off | Replace an existing `arawn.toml`. |

## `arawn setup`

Set up the integrations: Google (Gmail, Calendar, Drive), Slack, Atlassian (Jira, Confluence) and GitHub. For each provider, `arawn setup` shows the console steps, the scopes to add and the redirect URL. Then it asks for the credentials and writes the `[integrations.<provider>]` table into `arawn.toml`.

```sh
arawn setup              # Walk through all providers
arawn setup slack        # Set up one provider
```

If `arawn.toml` does not exist, `arawn setup` writes a starter config first, as `arawn init` does. Comments and other tables in `arawn.toml` stay as they are. The file is written with mode `0600`, because it can hold client secrets.

For scripts, give the provider and the credentials as flags. Then `arawn setup` asks no questions.

| Flag | Description |
|---|---|
| `<provider>` | `google`, `slack`, `atlassian` or `github`. `gmail`, `calendar`, `drive`, `jira` and `confluence` are also accepted. |
| `--client-id <id>` | OAuth client ID. |
| `--client-secret <secret>` | OAuth client secret. To keep the secret out of your shell history, give `--client-id` without `--client-secret` and set `ARAWN_SETUP_CLIENT_SECRET`. |
| `--secret-from-env` | Do not write the secret into `arawn.toml`. arawn reads it from `ARAWN_<PROVIDER>_CLIENT_SECRET` when the server starts. |
| `--app-id <id>` | GitHub App ID. |
| `--app-slug <slug>` | GitHub App slug. |
| `--private-key-path <path>` | GitHub App private key (`.pem`). `arawn setup` makes sure that the key can sign a token before it writes the config. |
| `--llm-provider <name>` | LLM provider for the starter config, when `arawn.toml` does not exist. Default `groq`. |

```sh
arawn setup google --client-id ID.apps.googleusercontent.com --client-secret GOCSPX-…
arawn setup atlassian --client-id ID --secret-from-env
arawn setup github --app-id 42 --app-slug my-arawn --private-key-path ~/keys/arawn.pem
```

An env var such as `ARAWN_GMAIL_CLIENT_ID`, or a per-service table such as `[integrations.gmail]`, comes before the table that `arawn setup` writes. If one of them hides the new client, `arawn setup` shows a warning that names it. See [Integrations config](./integrations-config.md#the-lookup-precedence).

After `arawn setup`, restart `arawn serve` and connect each service with `/connect <service>` in the TUI.

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

Doctor shows one `integration:<service>` line for each integration, with the fix:

| Result | Meaning | Fix shown |
|---|---|---|
| PASS | Connected. | — |
| SKIP | Not configured. | `run: arawn setup <provider>` |
| SKIP | Configured, but not connected. | `/connect <service>` |
| FAIL | A client ID is in `arawn.toml`, but no client secret is set. | Export the named `ARAWN_*_CLIENT_SECRET`, or run `arawn setup <provider>`. |
| FAIL | The GitHub App is incomplete, or its key file cannot be read. | `run: arawn setup github` |
| FAIL | The stored token cannot be read. | `/disconnect <service>`, then `/connect <service>` |

The same states are in the `status` RPC, the TUI `/status` output and the web health page. There, a state of `restart needed` means that the integration is in `arawn.toml`, but the running server did not load it. Restart `arawn serve`.

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

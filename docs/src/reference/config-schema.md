# Configuration schema

*Reference. Every `[section]` and key in `~/.arawn/arawn.toml`.*

Source: `crates/arawn/src/config.rs`; MCP in `crates/arawn-mcp/src/config.rs`; permissions in `crates/arawn-engine/src/permissions/config.rs`.

## Top-level shape

```toml
[llm.<name>]      # one or more named LLM profiles
[engine]          # which LLM is the engine, iteration cap, etc.
[compactor]       # context-compaction model + thresholds
[extraction]     # palace extractor model
[server]          # WebSocket host/port
[storage]         # data directory
[prompts]         # system-prompt token budget
[sandbox]         # shell sandbox network tools
[integrations.*]  # OAuth credentials per provider
[routing.hints]   # ModelHint → profile map
[routing.providers] # local/remote profile names
[ceremonies.<kind>] # per-ceremony overrides
[permissions]     # allow/deny/ask rules + mode
[[mcp.servers]]   # MCP server entries
```

## `[llm.<name>]`

A named LLM profile. Multiple entries allowed; the `[engine].llm` key picks which one is the engine. Keys:

| Key | Type | Default | Description |
|---|---|---|---|
| `provider` | string | `groq` | Provider name (`groq`, `openai`, `ollama`, `anthropic`) or a base URL (e.g. `https://ollama.com/v1`). |
| `model` | string | `openai/gpt-oss-20b` | Provider-specific model id. |
| `api_key` | string | none | Direct key value. Takes precedence over `api_key_env`. |
| `api_key_env` | string | `GROQ_API_KEY` | Name of env var holding the key. Set to `""` for keyless providers. |
| `base_url` | string | none | Override the provider's default API base URL. |
| `context_window` | u32 | `128000` | Context window in tokens. |
| `max_tokens` | u32 | `4096` | Max tokens per response. |

## `[engine]`

The agent loop's runtime knobs.

| Key | Type | Default | Description |
|---|---|---|---|
| `llm` | string | `default` | Name of the `[llm.<name>]` profile used for the main interaction. |
| `max_iterations` | usize | `20` | Maximum tool-use iterations per turn. |
| `max_result_size` | usize | `50000` | Maximum bytes from a single tool result that get fed back to the LLM. Excess is truncated. |
| `tool_timeout_secs` | u64 | `120` | Default wall-clock timeout per tool call. `ARAWN_TOOL_TIMEOUT_SECS` env var overrides. |

## `[compactor]`

Context compaction (when the conversation approaches the model's context window).

| Key | Type | Default | Description |
|---|---|---|---|
| `llm` | string | `engine`'s LLM | Profile used for compaction. |
| `compaction_threshold` | f32 | `0.85` | Fraction of context window at which to compact. |
| `keep_recent` | usize | `6` | Number of most-recent messages to preserve verbatim. |

## `[extraction]`

The per-workstream extractor that builds palaces.

| Key | Type | Default | Description |
|---|---|---|---|
| `llm` | string | `engine`'s LLM | Profile used for extraction. A cheaper/local model is reasonable here. |

## `[server]`

| Key | Type | Default | Description |
|---|---|---|---|
| `host` | string | `127.0.0.1` | Bind host. Anything other than a loopback address (127.0.0.0/8, ::1, `localhost`) prints a startup warning — arawn has no auth layer today, so non-loopback binds require a trusted network. |
| `port` | u16 | `3100` | TCP port. |

## `[storage]`

| Key | Type | Default | Description |
|---|---|---|---|
| `data_dir` | string | `~/.arawn` | Data directory root. Overridden by `--data-dir` and `ARAWN_DATA_DIR`. |

## `[prompts]`

| Key | Type | Default | Description |
|---|---|---|---|
| `token_budget` | u32 | `6000` | Token budget for the system prompt (excluding tool definitions). |

## `[sandbox]`

| Key | Type | Default | Description |
|---|---|---|---|
| `network_tools` | list&lt;string&gt; | `gh, kubectl, gcloud, aws, az, npm, npx, yarn, pnpm, cargo, rustup, pip, pip3, poetry, uv, gem, bundle, go, docker, podman, terraform, helm, curl, wget, fetch, git, ssh, scp, rsync, brew` | Tools that receive network access when invoked via the `shell` tool. Other commands run with network blocked. |

## `[integrations.<provider>]`

OAuth credentials. Plaintext in TOML — keep it out of version control. Env vars (`ARAWN_<SERVICE>_CLIENT_ID` / `_SECRET`) override these at startup.

### Standard OAuth providers

`slack`, `google`, `gmail`, `calendar`, `drive`, `atlassian`:

| Key | Type | Description |
|---|---|---|
| `client_id` | string | OAuth client id. |
| `client_secret` | string | OAuth client secret. |

`gmail` / `calendar` / `drive` fall back to `[integrations.google]` when empty. Recommended default is the shared `[integrations.google]` block.

### `[integrations.github]` — GitHub App, not OAuth

| Key | Type | Description |
|---|---|---|
| `app_id` | string | Numeric App ID. |
| `app_slug` | string | URL slug — visible on the App's public page. |
| `private_key_path` | string | Path to the App's RSA private key (`.pem` file). |

Env overrides: `ARAWN_GITHUB_APP_ID`, `ARAWN_GITHUB_APP_SLUG`, `ARAWN_GITHUB_PRIVATE_KEY_PATH` (preferred), or `ARAWN_GITHUB_PRIVATE_KEY_PEM` (inline).

## `[routing.hints]`

Maps `ModelHint` tiers to LLM profile names. Unset = falls back to the engine LLM.

| Key | Type | Description |
|---|---|---|
| `lightweight` | string | Profile name for `hint:lightweight`. |
| `medium` | string | Profile name for `hint:medium`. |
| `heavy` | string | Profile name for `hint:heavy`. |

## `[routing.providers]`

Health-aware routing.

| Key | Type | Description |
|---|---|---|
| `local` | string | Profile to use as the Local provider. Unset = "always Remote" (no fallback). |
| `remote` | string | Profile to use as Remote. Falls back to the engine profile if unset. |

## `[ceremonies.<kind>]`

Per-ceremony overrides. `<kind>` is `daily`, `weekly`, or `retro`. Absent table = built-in default. All keys are optional.

| Key | Type | Description |
|---|---|---|
| `enabled` | bool | When `false`, skip wiring this ceremony entirely. Defaults to enabled. |
| `schedule` | string | Cron expression overriding the plugin's default. Invalid expressions log a warning and fall back. |
| `timezone` | string | `local` (default) or an IANA zone. |
| `model` | string | LLM profile or hint shortcut (`hint:medium`) for the compose call. |
| `cadence` | string | **Retro only.** `"weekly"` (default), `"biweekly"`, or `"monthly"`. Runtime updates via the `retro_set_cadence` agent tool override this default; both end up in the same `ceremony_config` table. See [ceremonies tools reference](./ceremonies-tools.md#retro-cadence). |

## `[backfill]`

Boot-time ceremony recovery knobs.

| Key | Type | Default | Description |
|---|---|---|---|
| `ceremony_lookback_days` | int | `14` | How far back to walk on `arawn serve` boot when looking for missed daily/weekly tablets. `0` disables back-fill entirely. Retro is always excluded. See [ceremonies tools reference](./ceremonies-tools.md#back-fill). |

## `[permissions]`

Permission rules. See [permissions reference](./permissions.md) for full semantics.

| Key | Type | Default | Description |
|---|---|---|---|
| `allow` | list&lt;string&gt; | `[]` | Tool patterns to allow without prompt. |
| `deny` | list&lt;string&gt; | `[]` | Tool patterns to always deny. |
| `ask` | list&lt;string&gt; | `[]` | Tool patterns to always prompt for. |
| `autonomy` | string | `"ask"` | Starting permission posture. One of `ask` (default-safe), `edits` (auto-allow file writes; ask for shell), `full` (no prompts), `plan` (side-effects denied). Mirrors the `/autonomy` slash command. Invalid values warn and fall back to defaults. |

Pattern syntax: `tool_name` (exact), `tool_*` (glob), `tool_name(content-glob)` (tool name + content match).

## `[[mcp.servers]]`

Array of MCP server entries. Each:

| Key | Type | Default | Description |
|---|---|---|---|
| `name` | string | required | Server name (used in tool naming: `mcp__name__tool`). |
| `command` | string | required | Command to spawn the server process. |
| `args` | list&lt;string&gt; | `[]` | Arguments for the command. |
| `env` | map | `{}` | Environment variables for the spawned process. |
| `enabled` | bool | `true` | When `false`, the server is registered but not started. |

Example:

```toml
[[mcp.servers]]
name = "sqlite"
command = "uvx"
args = ["mcp-server-sqlite", "--db", "test.db"]
```

See [MCP reference](./mcp.md).

# Environment Variables

Complete reference for all environment variables used by Arawn.

## API Keys

Variables for configuring LLM provider authentication. These are checked by the `arawn-llm` crate when resolving API keys for each backend.

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ANTHROPIC_API_KEY` | — | API key for Anthropic (Claude) models | `arawn-llm` |
| `OPENAI_API_KEY` | — | API key for OpenAI-compatible backends | `arawn-llm` |
| `GROQ_API_KEY` | — | API key for Groq inference API | `arawn-llm` |
| `OLLAMA_API_KEY` | — | API key for Ollama (if auth is enabled) | `arawn-llm` |
| `LLM_API_KEY` | — | Generic fallback API key, used when a backend-specific key is not set | `arawn-llm` |

API keys can also be stored in the encrypted secret store (`~/.config/arawn/secrets.age`) and referenced in tool parameters using `${{secrets.<name>}}` handles. Environment variables take precedence over config file values.

---

## Authentication

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_API_TOKEN` | — | Bearer token for authenticating client requests to the Arawn server | `arawn-server` |

When set, all API requests to the server must include this token in the `Authorization: Bearer <token>` header.

---

## Paths

Variables controlling filesystem locations for configuration and data.

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_CONFIG_DIR` | `~/.config/arawn` | Override the configuration directory. Takes precedence over `XDG_CONFIG_HOME`. | `arawn-config` |
| `ARAWN_BASE_PATH` | `~/.arawn` | Override the base data directory for workstream file storage | `arawn-config` |

### Resolution Order for Config Directory

1. `ARAWN_CONFIG_DIR` environment variable (if set)
2. `XDG_CONFIG_HOME/arawn` (if `XDG_CONFIG_HOME` is set)
3. `~/.config/arawn` (platform default)

### Resolution Order for Base Path

1. `ARAWN_BASE_PATH` environment variable (if set)
2. `[paths].base_path` value in `config.toml` (if configured)
3. `~/.arawn` (default)

---

## Server

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_SERVER_URL` | `http://localhost:8080` | URL of the Arawn server for client connections | `arawn-client` |

Used by the CLI client and API clients to locate the running Arawn server.

---

## Features

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_MONITORING_ENABLED` | `true` | Enable or disable filesystem monitoring for workstreams. Accepts `true`, `false`, or `1`. | `arawn-config` |

When set, this overrides the `[paths.monitoring].enabled` config value. Filesystem monitoring watches workstream directories for changes and triggers event-driven updates.

---

## Backup

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_KEEP_BACKUPS` | `30` | Number of backup snapshots to retain before automatic cleanup | `arawn-server` |

Backups are stored in `~/.arawn-backups/` with timestamped directories. When the count exceeds this value, the oldest backups are removed.

---

## OAuth

Variables for configuring OAuth 2.0 authentication flow. These override values from `[oauth]` in `config.toml`.

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `ARAWN_OAUTH_CLIENT_ID` | — | OAuth client ID | `arawn-server` |
| `ARAWN_OAUTH_AUTHORIZE_URL` | — | Authorization endpoint URL | `arawn-server` |
| `ARAWN_OAUTH_TOKEN_URL` | — | Token exchange endpoint URL | `arawn-server` |
| `ARAWN_OAUTH_REDIRECT_URI` | — | Redirect URI after authorization | `arawn-server` |
| `ARAWN_OAUTH_SCOPE` | — | Space-separated list of OAuth scopes | `arawn-server` |

All five variables must be set for OAuth to be active. Tokens are cached in `~/.config/arawn/oauth-tokens.json`.

---

## System

Standard system environment variables that Arawn reads.

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `HOME` | (system) | User home directory. Used as the base for default path resolution. | `arawn-config` |
| `EDITOR` | `vim` | Preferred text editor. Used when opening files for interactive editing. | `arawn` (CLI) |
| `SHELL` | (system) | User's default shell. Used for shell tool execution and PTY spawning. | `arawn-agent-tools` |
| `XDG_CONFIG_HOME` | `~/.config` | XDG base directory for user configuration. Arawn appends `/arawn` to this path. | `arawn-config` |

---

## Plugin

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `CLAUDE_PLUGIN_ROOT` | (set per-plugin) | Set to the plugin's root directory when executing hook commands and CLI tool commands. Not user-configurable; set automatically by the plugin system. | `arawn-plugin` |

This variable enables portable paths in plugin manifests. For example, a hook command can reference `${CLAUDE_PLUGIN_ROOT}/hooks/validate.sh` to locate scripts relative to the plugin directory regardless of installation location.

---

## Search Provider Keys

Additional API keys for web search providers, used by the `web_search` tool.

| Variable | Default | Description | Component |
|----------|---------|-------------|-----------|
| `BRAVE_API_KEY` | — | API key for Brave Search | `arawn-agent-tools` |
| `SERPER_API_KEY` | — | API key for Serper (Google Search) | `arawn-agent-tools` |
| `TAVILY_API_KEY` | — | API key for Tavily search | `arawn-agent-tools` |

When none of these are set, the `web_search` tool falls back to DuckDuckGo (no API key required).

---

## Precedence Rules

When the same setting can be configured in multiple places, the following precedence applies (highest priority first):

1. **Environment variables** -- always override config file values
2. **Project-local config** (`./arawn.toml`) -- overrides user config
3. **User config** (`~/.config/arawn/config.toml`) -- base configuration
4. **Built-in defaults** -- used when no other source provides a value

# Configuration Reference

Complete reference for all Arawn configuration options. Configuration files use TOML format.

## Config Load Order

Configuration is loaded with cascading resolution. Later sources override earlier ones.

| Priority | Source | Path |
|----------|--------|------|
| 1 (lowest) | User config | `~/.config/arawn/config.toml` |
| 2 | Project-local config | `./arawn.toml` |
| 3 (highest) | CLI arguments | `arawn start --port 9090` |

Environment variables override config file values where noted. Use `arawn config which` to see which files are loaded.

---

## [llm]

Default LLM backend configuration. All agents use this backend unless overridden by a named profile.

```toml
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
base_url = "https://api.anthropic.com"
retry_max = 3
retry_backoff_ms = 500
max_context_tokens = 200000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | string | *(required)* | LLM provider: `anthropic`, `openai`, `groq`, `ollama`, `custom`, `claude-oauth` |
| `model` | string | *(required)* | Model identifier (e.g., `claude-sonnet-4-20250514`) |
| `api_key` | string | | API key. **Not recommended in config files.** Use `arawn config set-secret` or env vars. |
| `base_url` | string | *(provider default)* | Custom API base URL for proxies or self-hosted endpoints |
| `retry_max` | u32 | | Max retry attempts for transient failures |
| `retry_backoff_ms` | u64 | | Millisecond delay between retries |
| `max_context_tokens` | usize | | Max context window size in tokens |

### [llm.\<profile\>]

Named LLM profiles. Agents reference profiles by name via `[agent.<name>] llm = "<profile>"`. Each profile accepts the same fields as the `[llm]` section.

```toml
[llm.claude]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000

[llm.fast]
backend = "groq"
model = "llama-3.3-70b-versatile"
max_context_tokens = 32768

[llm.local]
backend = "ollama"
base_url = "http://localhost:11434"
model = "llama3"
```

---

## [agent.\<name\>]

Per-agent settings. The `default` key applies to all agents unless overridden by a named agent section.

```toml
[agent.default]
llm = "claude"
max_iterations = 25
max_tokens = 8192

[agent.summarizer]
llm = "fast"
max_iterations = 10
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | string | | LLM profile name (references `[llm.<name>]`) |
| `name` | string | | Display name for the agent |
| `description` | string | | Agent description |
| `system_prompt` | string | | System prompt override |
| `max_iterations` | u32 | | Max tool loop iterations |
| `max_tokens` | u32 | | Max tokens per LLM response |

---

## [server]

HTTP/WebSocket server settings.

```toml
[server]
port = 8080
bind = "127.0.0.1"
rate_limiting = true
api_rpm = 120
request_logging = true
trust_proxy = false
ws_allowed_origins = []
workspace = "/home/user/projects"
bootstrap_dir = "/home/user/.config/arawn/bootstrap"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | u16 | `8080` | HTTP listen port |
| `bind` | string | `"127.0.0.1"` | Bind address |
| `rate_limiting` | bool | `true` | Enable per-IP rate limiting |
| `api_rpm` | u32 | `120` | API requests per minute per IP |
| `request_logging` | bool | `true` | Enable HTTP request logging |
| `trust_proxy` | bool | `false` | Trust `X-Forwarded-For` headers for IP resolution |
| `ws_allowed_origins` | string[] | `[]` | WebSocket allowed origins. Empty list allows all origins. |
| `workspace` | path | | Server working directory for file operations |
| `bootstrap_dir` | path | | Directory containing bootstrap prompt files |

### Runtime Server Settings

These settings are controlled programmatically or via environment variables, not the TOML file.

| Setting | Default | Description |
|---------|---------|-------------|
| `cors_origins` | *(empty)* | CORS allowed origins |
| `tailscale_users` | *(none)* | Allowed Tailscale users |
| `reconnect_grace_period` | 30s | Grace period for WebSocket session reconnect |
| `max_ws_message_size` | 1 MB | Max WebSocket message size |
| `max_body_size` | 10 MB | Max REST request body size |
| `ws_connections_per_minute` | 30 | Max WS connections per minute per IP |

Authentication is configured via `ARAWN_API_TOKEN` or the keyring, not in the config file.

---

## [embedding]

Embedding provider configuration for memory search and indexing.

```toml
[embedding]
provider = "local"
dimensions = 384
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | string | `"local"` | Embedding provider: `local` (ONNX), `openai`, or `mock` |
| `dimensions` | usize | *(provider default)* | Output embedding dimensions |

### [embedding.local]

Settings for the local ONNX-based embedding provider.

```toml
[embedding.local]
model_path = "/path/to/model.onnx"
tokenizer_path = "/path/to/tokenizer.json"
model_url = "https://example.com/model.onnx"
tokenizer_url = "https://example.com/tokenizer.json"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model_path` | path | | Custom ONNX model path |
| `tokenizer_path` | path | | Custom tokenizer.json path |
| `model_url` | string | | Auto-download URL for ONNX model |
| `tokenizer_url` | string | | Auto-download URL for tokenizer |

### [embedding.openai]

Settings for the OpenAI embedding provider.

```toml
[embedding.openai]
model = "text-embedding-3-small"
dimensions = 1536
base_url = "https://api.openai.com"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | string | `"text-embedding-3-small"` | OpenAI model name |
| `dimensions` | usize | | Override embedding dimensions |
| `base_url` | string | | Custom endpoint URL |
| `api_key` | string | | API key. Prefer `OPENAI_API_KEY` env var. |

---

## [memory]

Memory system configuration.

```toml
[memory]
database = "memory.db"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `database` | path | | SQLite database path (relative to data dir) |

### [memory.recall]

Active recall settings for injecting relevant memories into conversations.

```toml
[memory.recall]
enabled = true
limit = 5
threshold = 0.6
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable active recall |
| `limit` | usize | `5` | Max memories to recall per turn |
| `threshold` | f32 | `0.6` | Min similarity score (0.0-1.0) |

### [memory.indexing]

Session indexing pipeline for extracting memories from conversations.

```toml
[memory.indexing]
enabled = true
backend = "openai"
model = "gpt-4o-mini"
ner_model_path = "/path/to/gliner.onnx"
ner_tokenizer_path = "/path/to/tokenizer.json"
ner_threshold = 0.5
ner_model_url = "https://example.com/gliner.onnx"
ner_tokenizer_url = "https://example.com/tokenizer.json"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable session indexing pipeline |
| `backend` | string | `"openai"` | LLM backend for extraction |
| `model` | string | `"gpt-4o-mini"` | Model for extraction/summarization |
| `ner_model_path` | path | | GLiNER ONNX model path for local NER |
| `ner_tokenizer_path` | path | | GLiNER tokenizer JSON path |
| `ner_threshold` | f32 | `0.5` | NER confidence threshold (0.0-1.0) |
| `ner_model_url` | string | | Auto-download URL for GLiNER model |
| `ner_tokenizer_url` | string | | Auto-download URL for GLiNER tokenizer |

### [memory.confidence]

Memory confidence scoring parameters. Controls how memory relevance decays over time.

```toml
[memory.confidence]
fresh_days = 30.0
staleness_days = 365.0
staleness_floor = 0.3
reinforcement_cap = 1.5
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fresh_days` | f32 | `30.0` | Days before staleness decay begins |
| `staleness_days` | f32 | `365.0` | Days at which staleness reaches the floor |
| `staleness_floor` | f32 | `0.3` | Minimum staleness multiplier |
| `reinforcement_cap` | f32 | `1.5` | Maximum reinforcement multiplier |

---

## [tools.output]

Maximum output size limits for tool results.

```toml
[tools.output]
max_size_bytes = 102400
shell = 102400
file_read = 512000
web_fetch = 204800
search = 51200
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_size_bytes` | usize | `102400` | Default max output size (100 KB) |
| `shell` | usize | `102400` | Max output for shell tool (100 KB) |
| `file_read` | usize | `512000` | Max output for file_read tool (500 KB) |
| `web_fetch` | usize | `204800` | Max output for web_fetch tool (200 KB) |
| `search` | usize | `51200` | Max output for search/grep/glob tools (50 KB) |

### [tools.shell]

```toml
[tools.shell]
timeout_secs = 30
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout_secs` | u64 | `30` | Shell command execution timeout in seconds |

### [tools.web]

```toml
[tools.web]
timeout_secs = 30
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout_secs` | u64 | `30` | Web request timeout in seconds |

---

## [plugins]

Plugin system configuration.

```toml
[plugins]
enabled = true
dirs = ["~/.config/arawn/plugins", "./plugins"]
hot_reload = true
auto_update = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable the plugin system |
| `dirs` | path[] | `[]` | Additional plugin directories to scan |
| `hot_reload` | bool | `true` | Enable file-watching hot reload |
| `auto_update` | bool | `true` | Auto-update subscribed plugins on startup |

### [[plugins.subscriptions]]

Plugin subscription entries. Each entry defines a plugin source.

```toml
[[plugins.subscriptions]]
source = "github"
repo = "author/plugin-name"
ref = "main"
enabled = true

[[plugins.subscriptions]]
source = "url"
url = "https://git.example.com/plugin.git"
ref = "v1.0"

[[plugins.subscriptions]]
source = "local"
path = "/path/to/local/plugin"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | string | *(required)* | Source type: `github`, `url`, or `local` |
| `repo` | string | | GitHub `owner/repo` (`github` source) |
| `url` | string | | Git clone URL (`url` source) |
| `path` | path | | Local filesystem path (`local` source) |
| `ref` | string | `"main"` | Git ref (branch, tag, or commit) |
| `enabled` | bool | `true` | Enable this subscription |

---

## [mcp]

Model Context Protocol server configuration.

```toml
[mcp]
enabled = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable MCP globally |

### [[mcp.servers]]

MCP server entries. Each entry defines a connection to an MCP server.

```toml
# Stdio transport (spawns a child process)
[[mcp.servers]]
name = "sqlite"
transport = "stdio"
command = "mcp-server-sqlite"
args = ["--db", "data.db"]
env = [["DEBUG", "1"]]
enabled = true

# HTTP transport (connects to a remote server)
[[mcp.servers]]
name = "remote"
transport = "http"
url = "http://localhost:3000/mcp"
headers = [["Authorization", "Bearer token"]]
timeout_secs = 30
retries = 3
```

| Field | Type | Default | Applies To | Description |
|-------|------|---------|------------|-------------|
| `name` | string | *(required)* | both | Unique server name |
| `transport` | string | `"stdio"` | both | Transport type: `stdio` or `http` |
| `enabled` | bool | `true` | both | Enable this server |
| `command` | string | | stdio | Command to spawn |
| `args` | string[] | `[]` | stdio | Command arguments |
| `env` | [key, value][] | `[]` | both | Environment variables |
| `url` | string | | http | Server URL |
| `headers` | [key, value][] | `[]` | http | HTTP headers |
| `timeout_secs` | u64 | `30` | http | Request timeout |
| `retries` | u32 | `3` | http | Retry count for failed requests |

---

## [pipeline]

Pipeline engine for workflow automation.

```toml
[pipeline]
enabled = true
database = "pipeline.db"
workflow_dir = "workflows"
max_concurrent_tasks = 4
task_timeout_secs = 300
pipeline_timeout_secs = 600
cron_enabled = true
triggers_enabled = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable pipeline engine |
| `database` | path | | SQLite state database path |
| `workflow_dir` | path | | Workflow TOML definitions directory |
| `max_concurrent_tasks` | usize | `4` | Max concurrent task executions |
| `task_timeout_secs` | u64 | `300` | Per-task timeout in seconds |
| `pipeline_timeout_secs` | u64 | `600` | Per-pipeline timeout in seconds |
| `cron_enabled` | bool | `true` | Enable cron-based scheduling |
| `triggers_enabled` | bool | `true` | Enable event-based triggers |

---

## [logging.interactions]

Structured interaction logging to JSONL files.

```toml
[logging.interactions]
enabled = true
path = "~/.arawn/logs"
retention_days = 90
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable interaction logging |
| `path` | path | | JSONL log directory |
| `retention_days` | u32 | `90` | Days to retain log files before cleanup |

---

## [delegation]

Subagent delegation and result compaction settings.

```toml
[delegation]
max_result_len = 8000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_result_len` | usize | `8000` | Max subagent result length (chars) before compaction |

### [delegation.compaction]

LLM-based compaction of large subagent results.

```toml
[delegation.compaction]
enabled = false
threshold = 8000
backend = "default"
model = "gpt-4o-mini"
target_len = 4000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable LLM-based result compaction |
| `threshold` | usize | `8000` | Min character length to trigger compaction |
| `backend` | string | `"default"` | LLM profile name for compaction |
| `model` | string | `"gpt-4o-mini"` | Model for compaction |
| `target_len` | usize | `4000` | Target output length after compaction |

---

## [session]

Session cache settings.

```toml
[session]
max_sessions = 10000
cleanup_interval_secs = 60
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_sessions` | usize | `10000` | Max sessions in LRU cache before eviction |
| `cleanup_interval_secs` | u64 | `60` | Seconds between cache cleanup runs |

---

## [workstream]

Workstream data management.

```toml
[workstream]
database = "workstreams.db"
data_dir = "workstreams"
session_timeout_minutes = 60
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `database` | path | | SQLite database path (relative to data dir) |
| `data_dir` | path | | JSONL message history directory |
| `session_timeout_minutes` | i64 | `60` | Session timeout in minutes |

### [workstream.compression]

LLM-based session compression for workstreams.

```toml
[workstream.compression]
enabled = false
backend = "default"
model = "claude-sonnet"
max_summary_tokens = 1024
token_threshold_chars = 32000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable auto-compression |
| `backend` | string | `"default"` | LLM profile for compression |
| `model` | string | `"claude-sonnet"` | Summarization model |
| `max_summary_tokens` | u32 | `1024` | Max tokens in generated summary |
| `token_threshold_chars` | usize | `32000` | Character threshold to trigger compression (~8k tokens) |

---

## [paths]

Data path configuration and disk usage monitoring.

```toml
[paths]
base_path = "~/.arawn"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_path` | path | `~/.arawn` | Base directory for all Arawn data |

### [paths.usage]

Disk usage warning thresholds.

```toml
[paths.usage]
total_warning_gb = 10
workstream_warning_gb = 1
session_warning_mb = 200
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_warning_gb` | u64 | `10` | Total disk usage warning threshold (GB) |
| `workstream_warning_gb` | u64 | `1` | Per-workstream warning threshold (GB) |
| `session_warning_mb` | u64 | `200` | Per-session warning threshold (MB) |

### [paths.cleanup]

Automatic cleanup settings for inactive scratch data.

```toml
[paths.cleanup]
scratch_cleanup_days = 7
dry_run = false
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `scratch_cleanup_days` | u32 | `7` | Days before inactive scratch data is cleaned up |
| `dry_run` | bool | `false` | Log cleanup actions without deleting |

### [paths.monitoring]

Filesystem monitoring configuration for change detection.

```toml
[paths.monitoring]
enabled = true
debounce_ms = 500
polling_interval_secs = 30
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable filesystem monitoring |
| `debounce_ms` | u64 | `500` | Event debounce interval in milliseconds |
| `polling_interval_secs` | u64 | `30` | Polling fallback interval in seconds |

---

## [rlm]

Recursive Learning Machine (exploration/research agent) configuration.

```toml
[rlm]
model = "claude-sonnet-4-20250514"
max_turns = 50
max_context_tokens = 150000
compaction_threshold = 0.8
max_compactions = 5
max_total_tokens = 500000
compaction_model = "gpt-4o-mini"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | string | *(inherit from backend)* | Exploration model |
| `max_turns` | u32 | | Max agent turns before stopping |
| `max_context_tokens` | usize | | Max context window before compaction triggers |
| `compaction_threshold` | f32 | | Fraction of `max_context_tokens` to trigger compaction (0.0-1.0) |
| `max_compactions` | u32 | | Max compaction cycles before stopping |
| `max_total_tokens` | usize | | Cumulative token budget for entire exploration |
| `compaction_model` | string | | Separate cheaper model for compaction summaries |

---

## [oauth]

OAuth PKCE flow overrides for the `claude-oauth` backend. All fields are optional. Unset fields fall through to environment variables (`ARAWN_OAUTH_*`) and then built-in defaults.

```toml
[oauth]
client_id = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
authorize_url = "https://claude.ai/oauth/authorize"
token_url = "https://console.anthropic.com/v1/oauth/token"
redirect_uri = "https://console.anthropic.com/oauth/code/callback"
scope = "org:create_api_key user:profile user:inference"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `client_id` | string | *(built-in)* | OAuth client ID |
| `authorize_url` | string | *(built-in)* | Authorization endpoint URL |
| `token_url` | string | *(built-in)* | Token exchange endpoint URL |
| `redirect_uri` | string | *(built-in)* | OAuth redirect URI |
| `scope` | string | *(built-in)* | OAuth scopes (space-separated) |

Resolution order: `[oauth]` config values, then `ARAWN_OAUTH_*` env vars, then built-in defaults.

---

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ARAWN_CONFIG` | Config file path override |
| `ARAWN_SERVER_URL` | Server URL for CLI commands |
| `ARAWN_BASE_PATH` | Override base data path |
| `ARAWN_MONITORING_ENABLED` | Enable/disable filesystem monitoring (`true`/`false`) |
| `ARAWN_API_TOKEN` | Server authentication token |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GROQ_API_KEY` | Groq API key |
| `OLLAMA_API_KEY` | Ollama API key |
| `ARAWN_OAUTH_CLIENT_ID` | OAuth client ID override |
| `ARAWN_OAUTH_AUTHORIZE_URL` | OAuth authorization endpoint override |
| `ARAWN_OAUTH_TOKEN_URL` | OAuth token endpoint override |
| `ARAWN_OAUTH_REDIRECT_URI` | OAuth redirect URI override |
| `ARAWN_OAUTH_SCOPE` | OAuth scopes override |
| `LLM_API_KEY` | Generic fallback API key (used when a backend-specific key is not set) |
| `BRAVE_API_KEY` | API key for Brave Search (web_search tool) |
| `SERPER_API_KEY` | API key for Serper/Google Search (web_search tool) |
| `TAVILY_API_KEY` | API key for Tavily search (web_search tool) |

---

## API Key Resolution

API keys are resolved in this order for each backend:

1. `--api-key` CLI flag (highest priority, overrides all below)
2. Age-encrypted secret store (`arawn secrets set <name>`)
3. System keyring (set via `arawn config set-secret <backend>`)
4. Environment variable (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.)
5. `[llm] api_key` or `[llm.<profile>] api_key` in config (lowest priority)

The `claude-oauth` backend uses OAuth tokens instead of API keys. Authenticate via `arawn auth login`.

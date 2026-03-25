# CLI Command Reference

Complete reference for the `arawn` command-line interface. All commands connect to a running Arawn server unless otherwise noted.

## Global Flags

These flags apply to every command.

| Flag | Short | Type | Env Var | Description |
|------|-------|------|---------|-------------|
| `--verbose` | `-v` | bool | | Enable verbose output |
| `--json` | | bool | | Output as JSON for scripting |
| `--server` | | string | `ARAWN_SERVER_URL` | Server URL (overrides current context) |
| `--context` | | string | | Use a specific named context |

---

## arawn start

Launch the Arawn server.

```
arawn start [FLAGS]
```

**Description:** Starts the HTTP/WebSocket server. CLI flags override values from the config file. When `--daemon` is set, the server forks into the background and writes its PID to `~/.config/arawn/arawn.pid`.

| Flag | Short | Type | Env Var | Description |
|------|-------|------|---------|-------------|
| `--daemon` | `-d` | bool | | Run server in background (daemon mode) |
| `--port` | `-p` | u16 | | Port to listen on (overrides config) |
| `--bind` | `-b` | string | | Address to bind to (overrides config) |
| `--token` | | string | `ARAWN_API_TOKEN` | API token for authentication |
| `--backend` | | string | | LLM backend: anthropic, openai, groq, ollama, custom, claude-oauth |
| `--api-key` | | string | | API key (overrides config and keyring) |
| `--base-url` | | string | | Custom base URL (overrides config) |
| `--model` | | string | | Model identifier (overrides config) |
| `--workspace` | | path | | Working directory for file operations |
| `--bootstrap-dir` | | path | | Path to directory containing bootstrap files |
| `--prompt-file` | | path | | Additional prompt file to load (repeatable) |
| `--config` | | path | | Path to config file (overrides default discovery) |
| `--seed` | | bool | | Seed the server with test workstreams and sessions (dev mode) |

**Examples:**

```bash
arawn start                       # Start with config file defaults
arawn start -p 9090               # Start on port 9090
arawn start -d                    # Start as a background daemon
arawn start --backend anthropic   # Start with a specific LLM backend
arawn start --token my-secret     # Start with an explicit auth token
```

---

## arawn stop

Stop a running Arawn daemon.

```
arawn stop
```

**Description:** Reads the PID from `~/.config/arawn/arawn.pid` and sends SIGTERM to the process. Fails if no PID file exists.

**Flags:** None.

---

## arawn status

Show server status.

```
arawn status
```

**Description:** Queries the server health endpoint. Displays whether the server is running, its version, and the server URL. Respects `--json` and `--server` global flags.

**Flags:** None (uses global flags).

**JSON output:**

```json
{
  "running": true,
  "version": "0.1.0",
  "server_url": "http://localhost:8080"
}
```

**Examples:**

```bash
arawn status                      # Check default server
arawn status --json               # Machine-readable output
arawn status --server http://remote:8080
```

---

## arawn ask

Send a one-shot question to the agent.

```
arawn ask <prompt> [FLAGS]
```

**Description:** Sends a single prompt to the agent and streams the response to stdout. The connection closes after the response completes.

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `<prompt>` | | string | *(required)* | The question or prompt to send |
| `--session` | `-s` | string | | Continue an existing session |
| `--no-memory` | | bool | false | Skip memory context |

**Examples:**

```bash
arawn ask "Explain ownership in Rust"
arawn ask -s abc123 "Follow up on that"
arawn ask --no-memory "What is 2+2?"
```

---

## arawn chat

Enter interactive chat mode (REPL).

```
arawn chat [FLAGS]
```

**Description:** Opens a readline-based interactive session. Type messages at the prompt and receive streamed responses.

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--session` | `-s` | string | | Resume an existing session |
| `--new` | `-n` | bool | false | Force start a new session |

**Examples:**

```bash
arawn chat                        # Start a new chat session
arawn chat -s abc123              # Resume an existing session
arawn chat --new                  # Force a fresh session
```

---

## arawn memory

Memory operations.

```
arawn memory <subcommand>
```

### arawn memory search

Semantic search through memories.

```
arawn memory search <query> [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `<query>` | | string | *(required)* | Search query |
| `--limit` | `-l` | usize | 10 | Maximum results to return |

### arawn memory recent

Show recent memories.

```
arawn memory recent [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-l` | usize | 10 | Number of recent memories to show |

### arawn memory stats

Show memory database statistics. No additional flags.

### arawn memory reindex

Re-embed all memories with the current configured embedding provider.

```
arawn memory reindex [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--dry-run` | | bool | false | Show what would be done without doing it |
| `--yes` | `-y` | bool | false | Skip confirmation prompt |

### arawn memory export

Export memories to JSON.

```
arawn memory export [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--output` | `-o` | string | stdout | Output file path |

**Examples:**

```bash
arawn memory search "Rust ownership"
arawn memory search "API design" -l 5
arawn memory recent
arawn memory stats
arawn memory export -o memories.json
arawn memory reindex --dry-run
```

---

## arawn notes

Note management.

```
arawn notes <subcommand>
```

### arawn notes add

Add a quick note.

```
arawn notes add <content> [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `<content>` | | string | *(required)* | Note content |
| `--tags` | `-t` | string | | Tags for the note (repeatable) |

### arawn notes list

List all notes.

```
arawn notes list [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-l` | usize | 20 | Maximum notes to show |

### arawn notes search

Search notes.

```
arawn notes search <query>
```

| Flag | Type | Description |
|------|------|-------------|
| `<query>` | string | Search query (required) |

### arawn notes show

Show a specific note.

```
arawn notes show <id>
```

### arawn notes delete

Delete a note.

```
arawn notes delete <id>
```

**Examples:**

```bash
arawn notes add "Remember to refactor auth" -t todo -t backend
arawn notes list
arawn notes search "refactor"
arawn notes show <id>
arawn notes delete <id>
```

---

## arawn config

Configuration management.

```
arawn config <subcommand>
```

### arawn config show

Show resolved configuration and all LLM profiles. With `--verbose`, also prints raw TOML.

### arawn config which

Show which config files are loaded and their search-order precedence.

### arawn config edit

Open the configuration file in `$EDITOR` (defaults to `vim`).

### arawn config init

Initialize a config file with defaults.

```
arawn config init [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--local` | bool | false | Create project-local config (`./arawn.toml`) instead of user config |

### arawn config path

Print the configuration file path to stdout.

### arawn config set-secret

Store an API key in the system keyring.

```
arawn config set-secret <backend>
```

| Argument | Description |
|----------|-------------|
| `<backend>` | Backend name: `anthropic`, `openai`, `groq`, `ollama`, `custom`, `claude-oauth` |

Prompts interactively for the API key (input hidden).

### arawn config delete-secret

Remove an API key from the system keyring.

```
arawn config delete-secret <backend>
```

### arawn config current-context

Print the current context name.

### arawn config get-contexts

List all available contexts in a table.

### arawn config use-context

Switch to a different context.

```
arawn config use-context <name>
```

### arawn config set-context

Create or update a context.

```
arawn config set-context <name> [FLAGS]
```

| Flag | Type | Description |
|------|------|-------------|
| `<name>` | string | Context name (required) |
| `--server` | string | Server URL (required when creating a new context) |
| `--workstream` | string | Default workstream for this context |
| `--timeout` | u64 | Connection timeout in seconds |

### arawn config delete-context

Delete a context by name.

```
arawn config delete-context <name>
```

**Examples:**

```bash
arawn config show
arawn config which
arawn config edit
arawn config init
arawn config init --local
arawn config set-secret anthropic
arawn config set-context prod --server http://prod:8080
arawn config use-context prod
arawn config get-contexts
```

---

## arawn auth

Authentication management.

```
arawn auth <subcommand>
```

### arawn auth login

Authenticate with Claude MAX via OAuth PKCE flow. Opens a browser URL, prompts for the authorization code, and exchanges it for tokens stored locally.

### arawn auth status

Show current authentication status, including OAuth token expiry and API token presence.

### arawn auth logout

Clear stored OAuth tokens from disk.

### arawn auth token

Generate or display an API token.

```
arawn auth token [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--generate` | bool | false | Generate a new UUID token and print setup instructions |

Without `--generate`, prints the current `ARAWN_API_TOKEN` value.

**Examples:**

```bash
arawn auth login
arawn auth status
arawn auth token --generate
arawn auth logout
```

---

## arawn session

View and manage chat sessions.

```
arawn session <subcommand>
```

### arawn session list

List all sessions. Supports `--json` for machine-readable output.

### arawn session show

Show session conversation history including tool calls.

```
arawn session show <id>
```

**Examples:**

```bash
arawn session list
arawn session show <id>
arawn session show <id> --json
```

---

## arawn plugin

Plugin management.

```
arawn plugin <subcommand>
```

### arawn plugin add

Subscribe to a plugin from a git URL or GitHub shorthand.

```
arawn plugin add <source> [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `<source>` | | string | *(required)* | GitHub shorthand (`owner/repo`) or full git URL |
| `--ref` | `-r` | string | `main` | Git ref (branch, tag, or commit) to checkout |
| `--project` | | bool | false | Add to project-local config instead of global |

### arawn plugin list

List all installed plugins.

```
arawn plugin list [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--subscribed` | bool | false | Show only subscribed plugins |
| `--local` | bool | false | Show only local plugins |

### arawn plugin update

Update subscribed plugins.

```
arawn plugin update [name]
```

| Argument | Description |
|----------|-------------|
| `[name]` | Plugin name to update (updates all if omitted) |

### arawn plugin remove

Unsubscribe and remove a plugin.

```
arawn plugin remove <name> [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<name>` | string | *(required)* | Plugin name or subscription ID |
| `--project` | bool | false | Remove from project-local config |
| `--delete-cache` | bool | false | Also delete cached plugin files |

**Examples:**

```bash
arawn plugin add owner/repo
arawn plugin add owner/repo --ref v1.0
arawn plugin add https://git.example.com/plugin.git
arawn plugin update
arawn plugin update my-plugin
arawn plugin remove my-plugin
arawn plugin list
```

---

## arawn agent

Subagent management.

```
arawn agent <subcommand>
```

### arawn agent list

List all available subagents.

```
arawn agent list [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--plugin` | string | | Filter agents by source plugin name |

### arawn agent info

Show detailed information about a specific agent.

```
arawn agent info <name>
```

Displays the agent's description, plugin source, model, max iterations, allowed tools, and system prompt.

**Examples:**

```bash
arawn agent list
arawn agent list --plugin mimir
arawn agent info researcher
```

---

## arawn mcp

MCP (Model Context Protocol) server management.

```
arawn mcp <subcommand>
```

### arawn mcp list

List configured MCP servers.

```
arawn mcp list [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--tools` | bool | false | Show tools available from each server (requires connecting) |

### arawn mcp add

Add a new MCP server configuration.

```
arawn mcp add <name> <target> [FLAGS] [-- args...]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `<name>` | | string | *(required)* | Unique name for this MCP server |
| `<target>` | | string | *(required)* | Command to spawn (stdio) or URL (http) |
| `--http` | | bool | false | Use HTTP transport instead of stdio |
| `--env` | `-e` | string | | Environment variables in `KEY=VALUE` format (repeatable) |
| `--header` | `-H` | string | | HTTP header in `KEY=VALUE` format (http only, repeatable) |
| `--timeout` | | u64 | 30 | Request timeout in seconds (http only) |
| `--retries` | | u32 | 3 | Number of retries for failed requests (http only) |
| `--disabled` | | bool | false | Start server disabled (don't auto-connect on startup) |
| `[-- args...]` | | string[] | | Arguments to pass to the command (stdio only) |

### arawn mcp test

Test connection to an MCP server.

```
arawn mcp test <name> [FLAGS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--full` | bool | false | Show full tool schemas |

### arawn mcp remove

Remove an MCP server configuration.

```
arawn mcp remove <name>
```

**Examples:**

```bash
arawn mcp list
arawn mcp list --tools
arawn mcp add postgres npx -- @anthropic/mcp-postgres
arawn mcp add api http://localhost:3001 --http
arawn mcp add search uvx -- mcp-search -e API_KEY=sk-xxx
arawn mcp test postgres
arawn mcp remove postgres
```

---

## arawn secrets

Manage the age-encrypted secret store.

```
arawn secrets <subcommand>
```

### arawn secrets set

Store a secret in the encrypted store. Prompts interactively for the value (input hidden).

```
arawn secrets set <name>
```

Reference stored secrets in tool parameters with `${{secrets.<name>}}`.

### arawn secrets list

List all stored secret names. Never displays values.

### arawn secrets delete

Delete a secret from the encrypted store.

```
arawn secrets delete <name>
```

**Examples:**

```bash
arawn secrets set github_token
arawn secrets list
arawn secrets delete github_token
```

---

## arawn logs

View operational logs.

```
arawn logs [FLAGS]
```

**Description:** Reads local log files by default. With `--remote`, fetches logs from the running server via the API.

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--lines` | `-n` | usize | 25 | Number of lines to show |
| `--follow` | `-f` | bool | false | Follow log output continuously (tail -f) |
| `--file` | | string | latest | Log file to read (by name) |
| `--remote` | `-r` | bool | false | Fetch logs from the running server instead of local files |
| `--list-files` | | bool | false | List available log files (use with `--remote`) |

**Examples:**

```bash
arawn logs                          # Show recent local logs
arawn logs -n 50                    # Show last 50 lines
arawn logs -f                       # Follow log output (tail -f)
arawn logs --file launchd-stdout    # Read a specific log file
arawn logs --remote                 # Fetch logs from running server
arawn logs --remote --list-files    # List available server log files
```

---

## arawn tui

Launch the terminal user interface.

```
arawn tui [FLAGS]
```

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--workstream` | `-w` | string | from context or "default" | Workstream to open |

**Examples:**

```bash
arawn tui                         # Launch TUI with default workstream
arawn tui -w my-project           # Open a specific workstream
```

---

## arawn backup

Backup all Arawn data (databases, config, workstreams).

```
arawn backup [FLAGS]
```

**Description:** Runs the backup shell script, archiving all data to the specified destination.

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--output` | `-o` | string | `~/.arawn-backups` | Backup destination directory |
| `--keep` | `-k` | u32 | 30 | Number of backups to keep |

---

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ARAWN_SERVER_URL` | Default server URL for CLI commands |
| `ARAWN_API_TOKEN` | Server authentication token (alias: `ARAWN_AUTH_TOKEN`) |
| `ARAWN_CONFIG_DIR` | Config directory path override |
| `ARAWN_BASE_PATH` | Override base data path |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GROQ_API_KEY` | Groq API key |
| `OLLAMA_API_KEY` | Ollama API key |
| `LLM_API_KEY` | Generic fallback API key (used when a backend-specific key is not set) |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (connection failure, invalid arguments, server error) |

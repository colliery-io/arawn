# Plugin Manifest Reference

Complete reference for the Arawn plugin system. Plugins follow the Claude Code plugin format for compatibility.

## Directory Layout

A plugin is a directory containing a `.claude-plugin/plugin.json` manifest and optional component subdirectories:

```
my-plugin/
├── .claude-plugin/
│   └── plugin.json           # Manifest (required)
├── skills/
│   └── <skill-name>/
│       └── SKILL.md           # Skill definition
├── agents/
│   └── <agent-name>.md        # Agent definition
├── hooks/
│   └── hooks.json             # Hook configuration
└── tools/
    └── <tool-name>/
        └── tool.json           # CLI tool definition
```

---

## plugin.json Schema

The manifest at `.claude-plugin/plugin.json` declares plugin metadata and component paths.

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | — | Unique identifier. Must be kebab-case, start with a letter, no consecutive hyphens, no trailing hyphen. |
| `version` | string | No | `"0.0.0"` | Semantic version (e.g., `"1.0.0"`, `"1.0.0-alpha"`). Minimum two dot-separated parts. No leading zeros. |
| `description` | string | No | `""` | Human-readable description |
| `author` | object | No | None | Author information (see below) |
| `homepage` | string | No | None | Documentation URL |
| `repository` | string | No | None | Source repository URL |
| `license` | string | No | None | SPDX license identifier (e.g., `"MIT"`, `"Apache-2.0"`) |
| `keywords` | array\<string\> | No | `[]` | Discovery keywords |
| `skills` | string \| array\<string\> | No | None | Path(s) to skills directories, relative to plugin root |
| `agents` | string \| array\<string\> | No | None | Path(s) to agent files or directories |
| `hooks` | string \| array\<string\> | No | None | Path(s) to hooks.json config files |
| `commands` | string \| array\<string\> | No | None | Path(s) to CLI tool directories |
| `mcpServers` | object \| string | No | None | Inline MCP server config or path to `.mcp.json` |
| `lspServers` | object \| string | No | None | Inline LSP server config or path |
| `outputStyles` | string \| array\<string\> | No | None | Path(s) to output style directories |

### Author Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Author name |
| `email` | string | No | Author email address |
| `url` | string | No | Author website URL |

### Path Fields

All path fields accept either a single string or an array of strings. Paths are resolved relative to the plugin root directory.

```json
{ "skills": "./skills/" }
{ "skills": ["./skills/", "./extra-skills/"] }
```

### Minimal Example

```json
{ "name": "my-plugin" }
```

### Full Example

```json
{
  "name": "journal",
  "version": "0.1.0",
  "description": "Notes and journal plugin",
  "author": {
    "name": "Your Name",
    "email": "you@example.com"
  },
  "homepage": "https://example.com/journal",
  "repository": "https://github.com/you/journal-plugin",
  "license": "MIT",
  "keywords": ["notes", "journaling"],
  "skills": "./skills/",
  "agents": "./agents/",
  "hooks": "./hooks/hooks.json",
  "commands": "./tools/"
}
```

### Name Validation Rules

- Must start with a lowercase letter (`a-z`)
- Allowed characters: lowercase letters, digits, hyphens
- No consecutive hyphens (`my--plugin` is invalid)
- No trailing hyphen (`plugin-` is invalid)
- No uppercase letters (`MyPlugin` is invalid)

### Version Validation Rules

- Minimum two dot-separated numeric parts (`1.0`)
- No leading zeros (`01.0.0` is invalid)
- Non-numeric parts are invalid (`1.x.0`)
- Pre-release suffixes are allowed (`1.0.0-alpha`)

---

## Skill Format

Skills are markdown files with YAML frontmatter, located at `skills/<skill-name>/SKILL.md`.

### YAML Frontmatter Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | — | Skill name (used for `/name` invocation) |
| `description` | string | No | `""` | Human-readable description |
| `uses_tools` | array\<string\> | No | `[]` | Tools this skill uses |
| `args` | array\<object\> | No | `[]` | Declared arguments |

### Skill Argument Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Argument name (used as `{name}` placeholder in body) |
| `description` | string | No | Argument description |
| `required` | boolean | No | Whether the argument is required |

### Example

```markdown
---
name: review-pr
description: Review a pull request
uses_tools:
  - github
args:
  - name: pr_number
    description: PR number to review
    required: true
---

# PR Review

Fetch the diff for PR {pr_number} and review it for:
- Security vulnerabilities
- Performance issues
- Code style violations
```

### Invocation

Skills are invoked in user messages using slash syntax:

- `/skill-name args` -- invokes the skill by name
- `/plugin-name:skill-name args` -- namespaced invocation when multiple plugins define the same skill name

---

## Agent Config Format

Agent definitions are markdown files with YAML frontmatter, located at `agents/<agent-name>.md`.

### YAML Frontmatter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Agent name (used with the `delegate` tool) |
| `description` | string | No | Human-readable description |
| `model` | string | No | LLM model override |
| `tools` | array\<string\> | No | Constrained tool set for this agent |
| `max_iterations` | integer | No | Maximum conversation turns |

### Example

```markdown
---
name: code-reviewer
description: Code review specialist
model: claude-sonnet
tools:
  - file_read
  - grep
  - glob
max_iterations: 15
---

You are a code review specialist. Analyze code for:

- Security vulnerabilities
- Performance issues
- Code style violations
- Logic errors

Provide specific, actionable feedback.
```

The markdown body after the frontmatter becomes the agent's system prompt.

---

## hooks.json Schema

Hook configuration files define event-driven handlers that fire at lifecycle events in the agent turn loop.

### Top-Level Structure

```json
{
  "hooks": {
    "<EventType>": [
      {
        "matcher": "<tool-name-or-glob>",
        "hooks": [
          {
            "type": "command",
            "command": "<shell-command>",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

### Event Types

| Event | Description | Non-Zero Exit Behavior |
|-------|-------------|----------------------|
| `PreToolUse` | Fires before a tool executes | Blocks the tool execution |
| `PostToolUse` | Fires after a tool completes successfully | Informational only (no blocking) |
| `PostToolUseFailure` | Fires after a tool fails | Informational only (no blocking) |
| `SessionStart` | Fires when a new session or turn begins | Informational only (no blocking) |
| `SessionEnd` | Fires when a session or turn ends | Informational only (no blocking) |
| `Stop` | Fires when the agent produces a final response | Informational only (no blocking) |
| `SubagentStop` | Fires when a subagent stops | Informational only (no blocking) |
| `SubagentStarted` | Fires when a background subagent starts | Informational only (no blocking) |
| `PreCompact` | Fires before context compaction | Informational only (no blocking) |
| `UserPromptSubmit` | Fires when the user submits a prompt | Informational only (no blocking) |
| `Notification` | Fires when a notification is sent | Informational only (no blocking) |
| `PermissionRequest` | Fires when a permission request is made | Informational only (no blocking) |

Only `PreToolUse` can block execution. All other events are informational.

### Hook Matcher Fields

| Field | Type | Description |
|-------|------|-------------|
| `matcher` | string | Tool name to match. Supports glob patterns (e.g., `file_*`, `*`). |

### Hook Definition Fields (HookDef)

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `event` | string | Yes | — | Event type: `PreToolUse` or `PostToolUse` |
| `tool_match` | string | No | None | Glob pattern matching tool names |
| `match_pattern` | string | No | None | Regex pattern matching against tool parameters (JSON) |
| `command` | string | Yes | — | Shell command to execute |
| `timeout` | integer | No | 10 | Subprocess timeout in seconds |

### Hook Execution

1. When a matching event fires, the hook command is spawned as a shell subprocess.
2. JSON context is written to the subprocess's stdin, containing the tool name, parameters, and event metadata.
3. The subprocess runs in the plugin's directory as its working directory.
4. **PreToolUse only:** A non-zero exit code blocks the tool execution and returns the subprocess output as an error.
5. PostToolUse hooks are fire-and-forget; exit codes are logged but do not affect execution.

Default subprocess timeout is 10 seconds.

### Example

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "shell",
        "hooks": [
          {
            "type": "command",
            "command": "./hooks/validate-shell.sh",
            "timeout": 5
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "file_write",
        "hooks": [
          {
            "type": "command",
            "command": "./hooks/log-file-changes.sh"
          }
        ]
      }
    ]
  }
}
```

---

## Plugin Subscription Config

Plugins can be sourced from remote repositories or local paths using subscriptions in `config.toml`.

### TOML Schema

```toml
[[plugins.subscriptions]]
source = "github"          # "github", "url", or "local"
repo = "owner/repo"        # For source = "github"
url = "https://..."        # For source = "url"
path = "/local/path"       # For source = "local"
ref = "v1.0.0"            # Git ref (branch, tag, commit). Default: "main"
enabled = true             # Default: true
```

### Subscription Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `source` | string | Yes | — | Source type: `github`, `url`, `local` |
| `repo` | string | Conditional | None | GitHub repository in `owner/repo` format (required when source is `github`) |
| `url` | string | Conditional | None | Git clone URL (required when source is `url`) |
| `path` | string | Conditional | None | Local filesystem path (required when source is `local`) |
| `ref` | string | No | `"main"` | Git ref to checkout |
| `enabled` | boolean | No | `true` | Whether subscription is active |

### Runtime Plugins Configuration

In addition to TOML subscriptions, runtime plugin state is stored in JSON files:

| File | Scope | Description |
|------|-------|-------------|
| `~/.config/arawn/plugins.json` | Global | User-level runtime subscriptions and enabled state |
| `.arawn/plugins.json` | Project | Project-specific subscriptions and enabled state |

Format:

```json
{
  "enabledPlugins": {
    "journal@local": true,
    "github-tools@github.com/author/repo": false
  },
  "subscriptions": [
    { "source": "github", "repo": "author/repo", "ref": "v1.0.0" }
  ]
}
```

---

## Plugin Management Commands

| Command | Description |
|---------|-------------|
| `arawn plugin add <source>` | Add a plugin subscription |
| `arawn plugin list` | List installed plugins and their components |
| `arawn plugin update` | Update all subscribed plugins |
| `arawn plugin remove <name>` | Remove a plugin |

---

## Plugin Loading

### Scan Directories

Plugins are discovered from these directories (in order):

1. `$XDG_CONFIG_HOME/arawn/plugins/` (or `~/.config/arawn/plugins/`) -- user-level
2. `./plugins/` -- project-local
3. Additional directories from `[plugins].dirs` in config

### Load Process

1. Scan each directory for subdirectories containing `.claude-plugin/plugin.json`.
2. Parse and validate the manifest (name format, version format).
3. Validate that declared component paths exist on disk.
4. Discover skills, agents, hooks, and CLI tools from declared paths.
5. Register components with their respective managers.

### Plugin System Configuration

```toml
[plugins]
enabled = true           # Enable/disable the plugin system (default: true)
dirs = []                # Additional plugin directories
hot_reload = true        # File watching for live reload (default: true)
auto_update = true       # Update subscribed plugins on startup (default: true)
```

---

## CLAUDE_PLUGIN_ROOT

The `CLAUDE_PLUGIN_ROOT` environment variable is set to the plugin's root directory when executing hook commands and CLI tool commands. Use it in manifest paths for portability:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "${CLAUDE_PLUGIN_ROOT}/server"
    }
  }
}
```

---

## Hot Reload Behavior

When `hot_reload = true` (default), the plugin system watches plugin directories for filesystem changes. When a plugin file is modified:

1. The affected plugin is re-scanned.
2. Updated skills, agents, and hooks are re-registered.
3. Changes take effect without restarting the server.

File change events are debounced (default 500ms) to coalesce rapid edits.

---

## Claude Code Compatibility

The plugin format is designed for compatibility with Claude Code plugins:

- `.claude-plugin/plugin.json` manifest structure is identical.
- Skill format (`skills/<name>/SKILL.md` with YAML frontmatter) is the same.
- Agent definition format matches.
- Hook event names (`PreToolUse`, `PostToolUse`) and execution model are compatible.
- The `CLAUDE_PLUGIN_ROOT` environment variable follows the same convention.
- Path fields accept both single strings and arrays.
- `mcpServers` field supports both inline config and path references.

Plugins written for Claude Code can be used with Arawn without modification, provided they use only the supported component types listed above.

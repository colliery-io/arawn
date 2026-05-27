# Configure hooks

*How-to. Wire shell commands into arawn's lifecycle events.*

Hooks let you extend or modify arawn's behavior without writing Rust. Arawn fires a hook event at each lifecycle moment — before a tool runs, after a session starts, when a permission prompt opens, etc. — and any shell command you've registered for that event executes with the event's payload on stdin. A hook can return `Allow` (let arawn proceed), `Block` (abort the event with a reason the model or user sees), or `Modify` (rewrite the payload). This is arawn's port of [Claude Code's hook system](https://docs.claude.com/en/docs/claude-code/hooks); existing Claude Code hook configs are largely portable.

## Where hooks live

Arawn loads hooks at startup from two paths, merged:

- **User-level**: `~/.arawn/settings.json` — applies to every lens.
- **Project-level**: `<lens-root>/.arawn/settings.json` — applies only when that lens is active.

Both files are optional. Missing files are silently treated as "no hooks for this scope." Hook subprocesses run with `cwd = lens root` so your scripts can reference project files with relative paths.

## File shape

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "file_write",
        "hooks": [
          { "type": "command", "command": "prettier --write \"$ARAWN_TOOL_ARG_path\"", "timeout": 10 }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "shell",
        "hooks": [
          { "type": "command", "command": "scripts/block-dangerous-shell.sh" }
        ]
      }
    ]
  }
}
```

Each event key (`PostToolUse`, `PreToolUse`, etc.) maps to an array of **hook groups**. Each group has:

| Field | Type | Required | Description |
|---|---|---|---|
| `matcher` | string \| object | optional | Selector. For tool events: tool name or regex (e.g., `"file_write"`, `"file_*"`). For Notification: `notification_type`. Omit to match everything. |
| `hooks` | array | required | Commands to run when this group matches. Each entry: `type` (always `"command"` in V1), `command` (shell string), optional `timeout` (seconds, default 5). |

Commands run in parallel within a group; multiple matching groups also run in parallel. Block decisions aggregate — if any matching hook returns Block, the event is blocked.

## Return values

A hook command returns a result via exit code + stdout:

- **Exit 0**: Allow. The event proceeds. Stdout (if any) is surfaced as a log line.
- **Exit 2**: Block. Stderr becomes the block reason the model (or user) sees. Only honored for blocking events (see below).
- **Any other non-zero exit**: treated as Allow with a warning logged. Don't let a broken script accidentally halt arawn.
- **JSON on stdout** (advanced): the hook can return a structured response like `{"decision": "block", "reason": "secret detected"}` for fine-grained control.

## V1 events (17 fired today)

| Event | Fires | Can block? |
|---|---|---|
| `PreToolUse` | Before each tool's `execute()` | **Yes** — block aborts the tool call; the agent sees the block reason |
| `PostToolUse` | After successful tool execution | No |
| `PostToolUseFailure` | After tool error or timeout | No |
| `PermissionRequest` | Before a modal permission prompt is raised | No |
| `PermissionDenied` | When a tool is rejected (rule, user prompt, or mode fallback) | No |
| `UserPromptSubmit` | When the user sends a message, before the model is called | **Yes** — block short-circuits the turn; the user sees the block reason as the assistant reply |
| `Stop` | When the model produces a final response | No |
| `StopFailure` | When the model stream errors mid-turn | No |
| `SessionStart` | After a new session is created | No |
| `PreCompact` | Before context-window compaction runs | No |
| `PostCompact` | After successful compaction | No |
| `Notification` | On every `ServerNotice` broadcast (hot-reload, integration events, etc.) | No |
| `SubagentStart` | When a sub-agent is spawned via the `agent` tool | No |
| `SubagentStop` | When a sub-agent completes (any exit path) | No |
| `TaskCreated` | When a background task is registered | No |
| `TaskCompleted` | When a background task finishes | No |

## V2 events (defined but not fired yet)

These eight events are defined in the schema for forward compatibility but don't yet have production fire sites:

- `Setup` (no first-run flow exists yet)
- `SessionEnd` (arawn sessions are persistent — no natural "ended" boundary)
- `WorktreeCreate` / `WorktreeRemove` (worktree subsystem doesn't exist)
- `CwdChanged` (no stateful cwd tracking)
- `FileChanged` (no user-file watcher)
- `TeammateIdle` (no teammate-agent system)
- `Elicitation` / `ElicitationResult` (no structured-input request flow)
- `PromptInjectionVerdict` (prompt-injection guard has no production runner)

You can register hooks for them — they'll just never fire until the underlying feature lands.

## Examples

### 1. Auto-format on file write

`PostToolUse` hook that runs Prettier whenever the `file_write` tool writes a file. Non-blocking — even if Prettier fails, arawn proceeds.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "file_write",
        "hooks": [
          { "type": "command", "command": "prettier --write \"$ARAWN_TOOL_ARG_path\" 2>/dev/null || true" }
        ]
      }
    ]
  }
}
```

### 2. Audit log on session end

Append `{session_id, timestamp}` to a JSONL file every time `SessionStart` fires (V1 substitute for `SessionEnd`, which isn't fired yet).

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "jq -nc --arg sid \"$ARAWN_SESSION_ID\" --arg ts \"$(date -u +%FT%TZ)\" '{session_id: $sid, started_at: $ts}' >> ~/.arawn/audit.jsonl"
          }
        ]
      }
    ]
  }
}
```

### 3. Block dangerous shell commands

`PreToolUse` hook that rejects any shell call whose command matches `rm -rf /`. The block reason gets surfaced to the agent so the model can adapt.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "shell",
        "hooks": [
          {
            "type": "command",
            "command": "if echo \"$ARAWN_TOOL_ARG_command\" | grep -qE 'rm -rf */'; then echo 'Refused: rm -rf / is never safe' >&2; exit 2; fi"
          }
        ]
      }
    ]
  }
}
```

## Troubleshooting

**My hook doesn't fire.**
- Check the event name spelling — events are CamelCase (`PreToolUse`, not `pre_tool_use`).
- Check the `matcher` — for tool events it matches the tool name (`shell`, `file_write`, `calendar_upcoming`, etc.). Omit the matcher to fire for every event of this type.
- Check that arawn loaded your settings file — the startup log line `hooks loaded user_settings_present=… project_settings_present=… total_hook_groups=…` will tell you whether the files were found.

**My PreToolUse hook returns exit 2 but the tool still runs.**
- For exit-2 to block, your script must write the block reason to **stderr** (not stdout). Stdout is reserved for the structured-response JSON form.

**My hook subprocess hangs / takes too long.**
- Default per-hook timeout is 5 seconds. Override with `"timeout": 30` (or whatever) in the group definition. Hooks that exceed the timeout are killed and treated as Allow with a warning.

## See also

- [LLM providers](../reference/llm-providers.md) — Groq backend hardening that pairs well with hook-based audit logging.
- [Configuration schema](../reference/config-schema.md) — the full `[*]` table reference for `arawn.toml` (separate from `settings.json`).

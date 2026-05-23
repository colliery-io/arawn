# Lock down permissions

*How-to. Three named permission setups — paranoid, hands-off CI, strict review — copy-paste ready.*

arawn lets an LLM run shell commands and edit files. The [permission model](../reference/permissions.md) controls what the agent can do without asking. This page gives three preset setups for common postures.

## Prerequisites

- A working `~/.arawn/arawn.toml`.

## Paranoid: always deny anything destructive

Default permission mode stays at `default` (read-only auto, write/shell ask), but a hard deny list catches the most-destructive shell calls regardless of what the agent or a follow-up rule says.

```toml
[permissions]
deny = [
    "shell(rm -rf *)",
    "shell(sudo *)",
    "shell(*--force*)",
    "shell(git push *--force*)",
    "shell(git reset --hard *)",
    "shell(*~)",                # backup file globs we don't want touched
]
allow = [
    "Read", "Glob", "Grep",
    "file_read",
    "shell(git status*)",
    "shell(git diff*)",
    "shell(git log*)",
    "shell(cargo check*)",
    "shell(cargo test*)",
]
# Everything else falls through to the default mode (ask).
```

Use this when you want explicit allow-listed commands to run without prompt, but anything not on the allow list still asks, and dangerous shell patterns can never run no matter what.

## Hands-off CI: full autonomy with a thin deny list

Full autonomy skips all permission prompts. Use it only when there's no human at the keyboard and you're confident in the deny list:

```toml
[permissions]
autonomy = "full"
deny = [
    "shell(rm -rf /*)",
    "shell(sudo *)",
    "shell(curl *| sh*)",
]
```

> **Warning:** `autonomy = "full"` means file writes and shell calls run without asking. Don't use this on a development machine; it's for unattended runs (CI, scheduled workflows on a sandboxed host) where you've reviewed what the agent might do.

## Strict review: ask before every side effect

The other direction — even reads ask. Useful when you're tinkering with permissions and want to see exactly what the agent reaches for:

```toml
[permissions]
autonomy = "ask"
allow = [
    "Read", "Glob", "Grep",
]
# Everything else asks — including all file_write and shell.
```

The agent will prompt before any `file_write`, `file_edit`, `shell`, or `web_fetch`.

## Switching modes at runtime

The TUI's `/autonomy` slash command toggles the posture without editing TOML:

```
/autonomy full    # full autonomy (everything auto-allowed)
/autonomy edits   # auto-allow file writes; ask for shell
/autonomy ask     # default-safe (read-only auto, write/shell ask)
/autonomy plan    # plan mode: side-effects denied (not asked)
```

> Content patterns match against the tool's primary argument
> (`shell` → `command`, `file_*` → `path`, `web_fetch` → `url`,
> etc.), so the recipes below match naturally. See
> [permissions reference](../reference/permissions.md) for the
> per-tool field mapping.

## What's next

- Full rule syntax + evaluation order: [permissions reference](../reference/permissions.md).
- The deny > allow > ask philosophy: [explanation: permission model](../explanation/permission-model.md).
- The OS-level sandbox underneath the rules: [shell sandbox reference](../reference/shell-sandbox.md).

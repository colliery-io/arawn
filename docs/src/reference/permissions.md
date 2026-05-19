# Permission model

*Reference. Rule syntax, evaluation order, modes, and the audit log.*

Source: `crates/arawn-engine/src/permissions/`.

## Concept

A **permission rule** is `allow`, `deny`, or `ask` plus a pattern. Rules let you say things like *"always allow Read, never allow shell calls that contain `rm -rf`, ask before any web_fetch."* They sit above the OS sandbox — the sandbox is your floor, the rules are your policy.

When a tool call doesn't match any rule, the active **permission mode** decides what happens.

## Rule syntax

```toml
[permissions]
allow = [
    "Read",                  # exact tool name
    "file_*",                # glob on tool name
    "shell(git *)",          # tool name + content pattern
    "shell(cargo *)",
]
deny = [
    "shell(rm -rf *)",
    "shell(curl *)",
]
ask = [
    "web_fetch",             # everything else falls through to mode default
]
```

- The **tool name** is matched first (exact or glob with `*`).
- If the rule has a `(content pattern)`, the tool's first-positional-string argument must also match that glob.
- `shell(...)` matches against the command string; `web_fetch(...)` against the URL; `Read(...)` / `file_*(...)` against the path.

## Evaluation order

```
deny  >  allow  >  ask  >  mode fallback
```

First match within each category wins. If no rule matches at all, the active mode decides.

## Permission modes

The mode controls what happens when no explicit rule matches. Each tool's *category* (ReadOnly / FileWrite / Shell / Other — see [agent tools reference](./agent-tools.md)) is what the mode actually keys off.

| Mode | ReadOnly | FileWrite | Shell | Other |
|---|---|---|---|---|
| `default` | allow | ask | ask | ask |
| `accept_edits` | allow | allow | ask | ask |
| `bypass` | allow | allow | allow | allow |
| `plan` | allow | deny | deny | deny |

`plan` mode is special: any side-effect tool is **denied outright** (not asked). The agent can think, read, and search, but it can't act. `enter_plan_mode` and `exit_plan_mode` are exempt — they're how the agent toggles modes.

The default mode is `default`. Set a starting mode with:

```toml
[permissions]
permission_mode = "default"  # or "accept_edits" / "bypass" / "plan"
```

Or change at runtime with the [`/accept` and `/plan` slash commands](./slash-commands.md).

## Per-decision responses

When the engine prompts for permission (an `ask` outcome), the response is one of:

| Response | Behavior |
|---|---|
| `AllowOnce` | This call only. Same shape next time will ask again. |
| `AllowAlways` | Add a session-grant; same shape auto-allows for the rest of the session. Not persisted to TOML. |
| `Deny` | Reject this call. |

Session grants live in-memory only. To make a grant permanent, edit `[permissions]` in `arawn.toml`.

## Audit log

Every allow / deny / ask decision is recorded. View with:

```
/permissions
```

The slash command shows the active rule set, the current mode, and the recent decisions. The full audit log is at `<data_dir>/logs/server.log` at `DEBUG` level — `RUST_LOG=arawn=debug` exposes it.

## Loading rules

Rules can come from:

1. `~/.arawn/arawn.toml` — `[permissions]` section.
2. A per-project `.arawn/permissions.toml` (if present in the working directory).
3. The active workstream's `arawn.md` (rare; used for workstream-specific exceptions).

The three sources are merged at load time. See `crates/arawn-engine/src/permissions/load.rs::load_merged_permissions`.

## Recipes

Three preset setups — paranoid, hands-off CI, strict review — are at [lock down permissions how-to](../how-to/lock-down-permissions.md).

## Related

- [Shell sandbox reference](./shell-sandbox.md) — the OS-level layer the rules sit on top of.
- [Permission model explanation](../explanation/permission-model.md) — why deny > allow > ask, why plan mode exists.
- [Agent tools reference](./agent-tools.md) — tool catalog with category column.

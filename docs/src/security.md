# Security & Permissions

Arawn lets an LLM run shell commands and edit files on your machine. Two
layers limit what the agent can actually do:

1. **OS-level sandbox** for the shell tool — restricts what any spawned
   process can read, write, or talk to over the network.
2. **Permission rules** evaluated by the engine before each tool call —
   user-visible policy on top of the sandbox.

## Shell sandbox

The shell tool runs every command inside an OS sandbox:

| Platform | Backend |
|---|---|
| macOS | `sandbox-exec` |
| Linux | `bubblewrap` |
| Windows | (not yet supported — shell tool fails closed) |

What the sandbox enforces by default:

- **Write access** is restricted to the active workstream sandbox directory
  (`<data_dir>/workstreams/<workstream>/...`). Writes anywhere else fail.
- **Sensitive paths** are denied for reading. The deny list lives in
  `crates/arawn-engine/src/tools/sensitive_paths.rs` and includes:
  - System auth: `/etc/shadow`, `/etc/sudoers`, `/etc/ssl/private`
  - SSH/GPG: `~/.ssh`, `~/.gnupg`
  - Cloud creds: `~/.aws`, `~/.azure`, `~/.config/gcloud`, `~/.kube`
  - Container creds: `~/.docker/config.json`
  - Package tokens: `~/.npmrc`, `~/.netrc`
  - Shell history (selective)
  - macOS keychains and Safari cookies
- **Network access** is blocked by default. A command gets network access
  only if it invokes a binary listed in `[sandbox].network_tools` in
  `arawn.toml` (defaults: `gh`, `git`, `curl`, `wget`, `cargo`, `npm`,
  `pip`, `gcloud`, `kubectl`, `docker`, ...).
- **Environment scrubbing**: spawned processes get a sanitized environment
  (`PATH`, `HOME`, `USER`, locale, build-tool homes), not the parent's full
  env. API keys held by the arawn process (`OPENAI_API_KEY`,
  `GROQ_API_KEY`, etc.) are NOT inherited by shell children. The full
  allowlist is in `crates/arawn-engine/src/tools/safe_env.rs`.

> **Caveat**: `sandbox-exec` is deprecated by Apple and may be removed in
> future macOS versions. We'll need to migrate to a different backend before
> that happens.

> **Caveat**: `bubblewrap` must be installed separately on Linux
> (`apt install bubblewrap` / `pacman -S bubblewrap` / etc.). If it's
> missing, the shell tool fails closed rather than running unsandboxed.

## Permission rules

Above the sandbox, the engine evaluates **permission rules** before
running any tool. Rules let you say "always allow `Read`, never allow
`shell` calls that contain `rm -rf`, ask before any `web_fetch`."

### Rule format

A rule is one of `allow`, `deny`, or `ask`, plus a pattern:

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

- The tool name is matched first (exact or glob).
- If the rule has a `(content pattern)`, the tool's input must also match
  that glob.
- Evaluation order: **deny > allow > ask**. First match in each category
  wins. If nothing matches, the active permission *mode* decides.

### Permission modes

The mode controls what happens when no explicit rule matches.

| Mode | Read-only tools | File-write tools | Shell tools |
|---|---|---|---|
| `default` | allow | ask | ask |
| `accept_edits` | allow | allow | ask |
| `bypass` | allow | allow | allow |
| `plan` | allow | deny | deny |

`plan` mode is special: any tool with side effects is denied outright
(not asked) so the agent can plan without acting. `enter_plan_mode` and
`exit_plan_mode` are exempt — they're how the agent toggles modes.

The default mode is `default`. The TUI exposes a mode switcher (work in
progress — see backlog T-0195).

## Limiting blast radius

For a paranoid setup:

```toml
# Always-deny anything destructive, regardless of mode.
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

For a hands-off setup (CI, automation):

```toml
# Bypass mode + a deny-list of things you never want even from "trusted" runs.
permission_mode = "bypass"

[permissions]
deny = [
    "shell(rm -rf /*)",
    "shell(sudo *)",
    "shell(curl *| sh*)",
]
```

For a strict review setup (every tool call gates on you):

```toml
permission_mode = "default"

[permissions]
allow = [
    "Read", "Glob", "Grep",
]
# Everything else asks — including all file_write and shell.
```

## Auditing tool use

A `/permissions` (or similar) command that prints the active rule set and
the recent allow/deny/ask decisions for the session is on the backlog
(T-0196 follow-up work). Until it lands, the server log records every
permission decision at `DEBUG` level — run with `RUST_LOG=arawn=debug` to
see them.

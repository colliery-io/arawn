# Shell sandbox

*Reference. The OS-level sandbox that the `shell` tool runs every command through.*

Source: `crates/arawn-engine/src/tools/{shell,safe_env,sensitive_paths}.rs`.

## Backends

The shell tool spawns each command in an OS sandbox:

| Platform | Backend | Notes |
|---|---|---|
| macOS | `sandbox-exec` | Apple has deprecated this; migration to a different backend is on the roadmap. |
| Linux | `bubblewrap` (`bwrap`) | Must be installed separately (`apt install bubblewrap` / `pacman -S bubblewrap`). Foreground shell calls fall back to **unsandboxed** with a `[WARNING: Command ran without sandbox protection ...]` prefix when `bwrap` is missing; only background-task shell calls fail closed. |
| Windows | (unsupported backend) | Same fallback as missing `bwrap` — foreground shell calls run **unsandboxed with a warning**; background-task shell calls fail closed. |

> **Security note:** the foreground unsandboxed-fallback behavior is a sharp edge — a misconfigured Linux host (no `bwrap`) or a Windows host will execute commands without the deny-list / network-block protections described below. If you need hard guarantees, run on macOS or a Linux host with `bwrap` installed.

## What the sandbox enforces by default

### Write access

Only the active workstream's `workspace/` directory is writable:

```
<data_dir>/workstreams/<active>/workspace/
```

Writes anywhere else fail with `Permission denied`. The agent should be calling `file_write` (which is path-aware and operates inside the workspace) rather than reaching for `shell` for writes — `file_write` is the right tool for most cases.

### Sensitive-path deny list

The sandbox denies reads on a hardcoded list of sensitive paths. Source: `crates/arawn-engine/src/tools/sensitive_paths.rs`.

| Category | Examples |
|---|---|
| System auth | `/etc/shadow`, `/etc/sudoers`, `/etc/ssl/private` |
| SSH / GPG | `~/.ssh`, `~/.gnupg` |
| Cloud creds | `~/.aws`, `~/.azure`, `~/.config/gcloud`, `~/.kube` |
| Container creds | `~/.docker/config.json` |
| Package tokens | `~/.npmrc`, `~/.netrc` |
| Shell history | `~/.bash_history`, `~/.zsh_history` (selective) |
| macOS-specific | Keychains, Safari cookies |

These are denied at the sandbox layer — even if a permission rule says `allow = ["shell(*)"]`, the sandbox blocks the read.

### Network access

Network is **blocked by default**. A command gets network access only if it invokes a binary listed in `[sandbox].network_tools` in `arawn.toml`.

The default allowlist (see [config schema](./config-schema.md)):

```
gh, kubectl, gcloud, aws, az, npm, npx, yarn, pnpm, cargo, rustup,
pip, pip3, poetry, uv, gem, bundle, go, docker, podman, terraform,
helm, curl, wget, fetch, git, ssh, scp, rsync, brew
```

Override by setting `[sandbox].network_tools = [...]` in `arawn.toml`.

### Environment scrubbing

Spawned processes get a **sanitized environment**, not the parent's full env. The allowlist (source: `crates/arawn-engine/src/tools/safe_env.rs`) is small and explicit:

- Exact-match: `PATH`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TERM`, `LANG`, `LC_ALL`, `TMPDIR`, `TMP`, `TEMP`, `PWD`, `OLDPWD`, `CARGO_HOME`, `RUSTUP_HOME`, `GOPATH`, `GOROOT`, `NPM_CONFIG_PREFIX`, `PIP_CACHE_DIR`.
- Prefix-match: `LC_*` (locale categories), `XDG_*` (XDG Base Directory).

Anything not in that set is dropped. So `JAVA_HOME`, `GRADLE_*`, `GOMODCACHE`, `EDITOR`, `VISUAL`, `PAGER`, `DISPLAY`, `WAYLAND_DISPLAY`, and the rest are **not** forwarded — commands that need them will see them empty.

**Not inherited:** API keys held by the arawn process — `OPENAI_API_KEY`, `GROQ_API_KEY`, `ANTHROPIC_API_KEY`, etc. So if the agent calls `shell("echo $OPENAI_API_KEY")`, it sees an empty string. This is intentional: the agent has the keys via the LLM provider abstraction; shell children don't need them.

## Permission rules on top

The sandbox is the floor. On top, the [permission model](./permissions.md) lets you say *"never run shell with `rm -rf`"* or *"always allow `shell(git status*)`"*. The two layers compose:

1. Permission rule must say allow (explicitly or via mode default).
2. Sandbox must permit the actual syscalls.

If the rule says allow but the sandbox denies (e.g. the command tries to read `~/.ssh`), the syscall fails — the command runs but errors out.

## Inspecting the sandbox at runtime

```sh
arawn doctor --json
```

Reports sandbox backend availability + whether the binary (`sandbox-exec` / `bwrap`) is present.

```sh
RUST_LOG=arawn_engine::tools::shell=debug arawn serve
```

Logs each `shell` invocation with its sandbox profile and the spawned command.

## Caveats

- **`sandbox-exec` deprecation** (macOS): Apple has marked it for removal. Migration to `apple-sandbox` profile generators or a containerization backend is on the roadmap.
- **`bubblewrap`** (Linux): not installed by default on most distros. The shell tool fails closed when `bwrap` is missing rather than running unsandboxed.
- **Windows**: not supported. `shell` returns an error explaining the platform isn't covered.

## Related

- [Permissions reference](./permissions.md) — the rule layer above the sandbox.
- [Lock down permissions how-to](../how-to/lock-down-permissions.md) — preset configs.

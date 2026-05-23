---
id: shell-tool-inherits-all-env-vars
level: task
title: "Shell tool inherits all env vars in unsandboxed/fallback mode — API keys leak via shell output"
short_code: "ARAWN-T-0172"
created_at: 2026-04-16T17:08:11.904183+00:00
updated_at: 2026-04-16T17:27:25.394042+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Shell tool inherits all env vars in unsandboxed/fallback mode — API keys leak via shell output

## Objective

When the shell sandbox is unavailable (fallback path, `shell.rs:395-409`) or when running background commands (`shell.rs:54-196`), the spawned process inherits all environment variables from the arawn parent process. This includes `AWS_SECRET_ACCESS_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, and any other secrets set in the user's shell. An agent running `env | grep KEY` in unsandboxed mode gets full access.

Fix: sanitize the environment before spawning any shell command. Whitelist safe env vars and strip everything else.

### Priority
- [x] P1 - High (credential exfiltration vector)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All shell command spawns (foreground, background, sandboxed, unsandboxed) use a filtered environment
- [ ] Whitelist of safe env vars defined in a shared constant (PATH, HOME, USER, SHELL, TERM, LANG, LC_*, TMPDIR, XDG_*, CARGO_HOME, RUSTUP_HOME)
- [ ] API key env vars (`*_API_KEY`, `*_TOKEN`, `*_SECRET*`) are never passed to child processes
- [ ] Sandboxed commands: filtered env passed to sandbox-exec/bwrap child
- [ ] Unsandboxed fallback: filtered env (not empty — tools need PATH etc.)
- [ ] Test: `env` command output does not contain any `*_API_KEY` or `*_TOKEN` vars that exist in parent process
- [ ] Existing shell tests still pass (tools that need PATH, cargo, git, etc. still work)

## Implementation Notes

- `shell.rs` — all command spawn points
- `Command::env_clear()` + `Command::envs(filtered)` is the Rust stdlib approach
- The sandboxed path already constructs a `Command` — add `.env_clear().envs(safe_env())` before spawn
- Some tools (curl, git, gh) may need specific env vars — could add a per-detected-tool env var allowlist on top of the base whitelist
- The arawn process itself needs API keys (for LLM calls) — those should be read at startup and not kept in env, but that's a separate concern

## Status Updates

- Created `crates/arawn-engine/src/tools/safe_env.rs` with `safe_env() -> HashMap<String,String>` and `is_safe_env_name(&str)`. Allowlist: PATH, HOME, USER, LOGNAME, SHELL, TERM, LANG, TMPDIR/TMP/TEMP, PWD/OLDPWD, CARGO_HOME, RUSTUP_HOME, GOPATH/GOROOT, NPM_CONFIG_PREFIX, PIP_CACHE_DIR, LC_ALL plus prefixes LC_* and XDG_*.
- Wired `.env_clear().envs(safe_env())` into all three shell spawn sites: `spawn_background`, `execute_sandboxed`, `execute_unsandboxed`.
- Tests added to `safe_env.rs` (allowlist + secret blocking + safe_env() filtering) and to `shell.rs`: `shell_env_does_not_leak_secrets` sets a parent env var, runs `env` via the shell tool, and asserts neither the name nor value appears in child output. `shell_env_preserves_path` confirms PATH is forwarded.
- All 183 tools tests pass.
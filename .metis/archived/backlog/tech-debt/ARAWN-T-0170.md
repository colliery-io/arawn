---
id: background-shell-commands-bypass
level: task
title: "Background shell commands bypass sandbox — run_in_background executes unsandboxed"
short_code: "ARAWN-T-0170"
created_at: 2026-04-16T17:08:09.234518+00:00
updated_at: 2026-04-16T17:33:27.654145+00:00
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

# Background shell commands bypass sandbox — run_in_background executes unsandboxed

## Objective

When `run_in_background: true` is set on a shell command, the entire sandbox is bypassed (`shell.rs:54-196`). The command runs with full filesystem access and inherits all environment variables. An agent could spawn a background command to read `~/.ssh`, `~/.aws`, stored OAuth tokens, or any other sensitive path that the foreground sandbox explicitly blocks.

Fix: apply the same `sandbox-exec` / bubblewrap sandbox profile to background commands. The comment at line 68-69 says "sandbox requires sync lifecycle management that doesn't fit background execution" — this needs to be solved, likely by wrapping the background spawn in the same sandbox profile but with async output collection.

### Priority
- [x] P1 - High (security boundary violation)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Background shell commands run inside the same sandbox as foreground commands
- [ ] Sensitive path deny list (`~/.ssh`, `~/.aws`, etc.) applies to background commands
- [ ] Write restrictions (workdir + /tmp only) apply to background commands
- [ ] Network control (default blocked, allowed for detected tools) applies to background commands
- [ ] If sandbox unavailable, background commands should **fail** rather than silently run unsandboxed
- [ ] Existing background command tests still pass
- [ ] New test: background `cat ~/.ssh/id_rsa` is denied

## Implementation Notes

- `shell.rs:54-196` — background command path that skips sandbox
- `shell.rs:200-431` — foreground sandbox setup (sensitive paths, write restrictions, network)
- The sandbox profile is built as a string/config and passed to `sandbox-exec -p` or `bwrap` — this should be extractable into a shared function that both foreground and background paths use
- Key challenge: foreground uses synchronous `Command::output()` while background uses `Command::spawn()` with a tokio task collecting output later. The sandbox wrapper should work with both — it's a process-level constraint, not tied to output collection

## Status Updates

- Added `init_sandbox_for_background()` helper in `shell.rs` that initializes a fresh `Arc<SandboxManager>`, runs `check_dependencies` + `initialize` + `wrap_with_sandbox`, and returns the manager paired with the wrapped command. The manager is handed off to the reader task and `reset()` is called after the child exits — solves the "sync lifecycle" comment.
- `spawn_background` now reuses `build_sandbox_config` (same profile as foreground) so all the sensitive path denies, write restrictions (workdir + /tmp), and network control inherited from `network_tools` apply to background commands too.
- If sandboxing is unavailable on this platform OR sandbox dependencies/init fail, background spawn returns `ToolError::ExecutionFailed` rather than silently running unsandboxed (per acceptance criteria).
- Background spawn also uses `.env_clear().envs(safe_env())` (T-0172 plumbing) so secrets don't leak into the child.
- Removed the prior "WARNING: UNSANDBOXED" log and updated the success message to read `(sandboxed)`.
- Added two new tests: `background_command_runs_sandboxed` (writes a marker file inside the sandbox dir and verifies completion) and `background_command_sandbox_blocks_sensitive_read` (attempts `ls ~/.ssh`, asserts no real SSH filenames appear in the captured output). Skipped on platforms without sandbox support or without ~/.ssh on the host.
- All 21 shell tests pass; full workspace `angreal test unit` is green.
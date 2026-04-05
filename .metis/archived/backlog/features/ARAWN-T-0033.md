---
id: dangerous-command-detection-block
level: task
title: "Dangerous command detection — block/warn on destructive shell commands"
short_code: "ARAWN-T-0033"
created_at: 2026-04-01T11:01:56.707494+00:00
updated_at: 2026-04-02T12:35:31.383477+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# OS-level shell sandbox — sandbox-exec (macOS) / bubblewrap (Linux)

## Objective

Replace the current unsandboxed `ShellTool` with OS-level process sandboxing. Today, `ShellTool` sets `cwd` to the workspace but the subprocess can escape via absolute paths, read credentials, or write anywhere on the filesystem. The fix is to use `sandbox-runtime` (which wraps macOS `sandbox-exec` and Linux `bubblewrap`) to enforce filesystem and network restrictions at the OS level.

### Priority
- P1 — LLM can currently execute any shell command with full filesystem access

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] ShellTool executes commands through `sandbox-runtime` instead of raw `Command`
- [ ] Write access restricted to the session/workstream sandbox directory only
- [ ] Sensitive paths denied for reading (~/.ssh, ~/.aws, ~/.gnupg, keychains, credentials, shell histories)
- [ ] Network access denied by default (no outbound connections from shell)
- [ ] Platform detection at startup — macOS sandbox-exec (built-in), Linux bubblewrap+socat
- [ ] If sandbox is unavailable, ShellTool returns an error explaining how to install dependencies (not silently unsandboxed)
- [ ] Timeout enforcement through the sandbox layer
- [ ] Existing ShellTool tests pass with sandbox (echo, exit codes, timeout)
- [ ] New tests: write outside sandbox blocked, read sensitive path blocked, absolute path escape blocked

## Implementation Notes

### Technical Approach

1. Add `sandbox-runtime` dependency to `arawn-engine`
2. Build sandbox config from `ToolContext` inside `ShellTool::execute`:
   - `write_paths`: `[ctx.working_dir]`
   - `deny_read_paths`: sensitive paths list (from backup's `SandboxConfig::default_deny_read_paths()`)
   - `working_dir`: `ctx.working_dir`
   - `allowed_domains`: empty (no network)
3. Platform detection via `SandboxStatus::detect()` — check for `sandbox-exec` (macOS) or `bwrap`+`socat` (Linux)
4. No separate crate — sandbox logic lives inside `ShellTool` since it's the only tool that needs OS-level sandboxing (file tools already have path validation)

### Prior Art

The previous arawn codebase had a full `arawn-sandbox` crate (in `backup/crates/arawn-sandbox/`) with:
- `SandboxConfig` — write-allow model, deny-read list, domain allowlist, timeout, env vars
- `SandboxManager` — wraps `sandbox-runtime`, platform detection, command execution
- `Platform` enum — MacOS, Linux, Unsupported with dependency checking
- Comprehensive test suite including sandboxed echo, write-allowed, write-denied, timeout

### Dependencies
- `sandbox-runtime` crate

## Status Updates

*To be added during implementation*
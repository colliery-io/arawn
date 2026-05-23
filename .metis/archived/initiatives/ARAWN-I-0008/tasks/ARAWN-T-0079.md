---
id: command-hook-executor-subprocess
level: task
title: "Command hook executor — subprocess spawning, stdin piping, exit code handling, timeouts"
short_code: "ARAWN-T-0079"
created_at: 2026-04-04T02:16:23.685831+00:00
updated_at: 2026-04-04T02:31:03.518115+00:00
parent: ARAWN-I-0008
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0008
---

# Command hook executor — subprocess spawning, stdin piping, exit code handling, timeouts

## Objective

Implement the `CommandHookExecutor` that spawns shell subprocesses for command hooks, pipes hook input as JSON on stdin, captures stdout/stderr, interprets exit codes, and enforces timeouts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `CommandHookExecutor::execute(command, hook_input) -> HookResult` async function
- [ ] Spawns shell subprocess via `tokio::process::Command` (using user's shell or `sh`)
- [ ] Serializes `HookInput` as JSON and writes to subprocess stdin
- [ ] Captures stdout and stderr from subprocess
- [ ] Exit code 0 → `HookResult::Allow` (stdout available but suppressed by default)
- [ ] Exit code 2 → `HookResult::Block` with stderr as the reason
- [ ] Other exit codes → `HookResult::Warn` with stderr as the message
- [ ] Configurable timeout (default 10s) — kills subprocess and returns `Block` on timeout
- [ ] Handles subprocess spawn failures gracefully (returns `Warn`, doesn't panic)
- [ ] Integration tests with real shell commands (`echo`, `exit 0`, `exit 2`, `exit 1`, `sleep` for timeout)

## Implementation Notes

- Depends on T-0078 for `HookResult` and `HookInput` types
- Use `tokio::time::timeout` wrapping the subprocess wait
- Consider working directory — hooks should run in the project root
- Claude Code pipes JSON on stdin; we follow the same pattern for compatibility

## Status Updates

*To be added during implementation*
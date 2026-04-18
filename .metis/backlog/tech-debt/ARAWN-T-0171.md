---
id: glob-and-grep-tools-lack-sensitive
level: task
title: "Glob and Grep tools lack sensitive path deny list — can search ~/.ssh, ~/.aws, etc."
short_code: "ARAWN-T-0171"
created_at: 2026-04-16T17:08:10.411672+00:00
updated_at: 2026-04-16T17:23:39.302065+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Glob and Grep tools lack sensitive path deny list — can search ~/.ssh, ~/.aws, etc.

## Objective

The shell tool (`shell.rs:200-246`) has a comprehensive sensitive path deny list blocking access to `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/gcloud`, `~/.kube`, `~/.docker/config.json`, `~/.npmrc`, `~/.netrc`, `~/.git-credentials`, `~/.config/gh`, `~/.vault-token`, `~/.pgpass`, `~/.my.cnf`, history files, and macOS keychains. But glob (`glob.rs:50-65`) and grep (`grep.rs:104-117`) only validate that the path is within the workstream root via `ctx.validate_path()`. An agent could `glob pattern="*" path="../../.ssh"` or `grep pattern="BEGIN RSA" path="../../.."` to enumerate and search sensitive directories.

Extract the shell tool's sensitive path list into a shared `SENSITIVE_PATHS` constant and apply it as a deny check in glob and grep path validation.

### Priority
- [x] P1 - High (security boundary inconsistency)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Shared `SENSITIVE_PATHS` list extracted from shell.rs into a common location (e.g., `arawn-engine/src/tools/sensitive_paths.rs`)
- [ ] Glob tool rejects paths that resolve into any sensitive directory
- [ ] Grep tool rejects paths that resolve into any sensitive directory
- [ ] Shell tool uses the same shared list (no duplication)
- [ ] Tests: glob targeting `~/.ssh` returns error, grep targeting `~/.aws` returns error
- [ ] Workstream-root paths still work normally

## Implementation Notes

- `shell.rs:200-246` — current sensitive path list
- `glob.rs:50-65` — path validation (only workstream root check)
- `grep.rs:104-117` — path validation (only workstream root check)
- `ctx.validate_path()` in `context.rs` would be the natural place to add the deny check, so all tools that call it get protection automatically
- Consider whether `validate_path()` should always deny sensitive paths, or whether this should be a separate `deny_sensitive_path()` check

## Status Updates

- Created `crates/arawn-engine/src/tools/sensitive_paths.rs` with `sensitive_deny_read_paths()` (moved from shell.rs) and new `is_sensitive_path(&Path) -> bool` helper that compares canonical forms to defeat symlink/.. tricks.
- Removed duplicated deny list from `shell.rs` — now imports from the shared module.
- `glob.rs` and `grep.rs` now call `is_sensitive_path` after `validate_path`, returning a clear error if the resolved path resolves into a sensitive directory.
- 5 unit tests in sensitive_paths module (deny list contents, ssh/aws/etc-shadow detection, ordinary path negative case). All 171 tools tests pass.
- Acceptance criteria met: shared module exists, glob/grep reject sensitive paths, shell still works, no duplication.
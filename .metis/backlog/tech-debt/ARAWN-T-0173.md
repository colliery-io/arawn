---
id: file-read-write-edit-tools-have-no
level: task
title: "File read/write/edit tools have no secret-file deny list — can read .env, token storage, credentials"
short_code: "ARAWN-T-0173"
created_at: 2026-04-16T17:08:12.967571+00:00
updated_at: 2026-04-16T17:25:41.678800+00:00
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

# File read/write/edit tools have no secret-file deny list — can read .env, token storage, credentials

## Objective

The file_read, file_write, and file_edit tools validate that paths stay within the workstream root (via canonical path checks), but they have no awareness of secret files. Within the workstream root itself, `.env`, `.env.local`, `secrets.json`, `config/secrets.yml`, and similar files are fully readable. More critically, once we add OAuth token storage to the arawn data dir, the agent's `allowed_paths` mechanism (which currently permits reading `~/.arawn/*.md` context files) could be a vector for reading stored tokens if the token files are co-located.

Add a secret-file pattern deny list to the file tools. Files matching common secret patterns should be blocked regardless of path validation.

### Priority
- [x] P2 - Medium (currently low exposure, becomes critical with token storage)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Deny list of secret file patterns: `.env`, `.env.*`, `*.secret`, `*.key`, `*.pem`, `credentials.json`, `secrets.yml`, `secrets.yaml`, `secrets.toml`, `token.json`, `tokens.json`, `*.p12`, `*.pfx`
- [ ] file_read rejects files matching deny patterns with clear error message
- [ ] file_write rejects writes to deny-listed filenames
- [ ] file_edit rejects edits to deny-listed filenames
- [ ] Deny check uses filename/basename matching, not full path (so `.env` is blocked whether it's `./app/.env` or `../../.env`)
- [ ] Legitimate files like `env.rs`, `.env.example`, `environment.ts` are NOT blocked (pattern must be precise)
- [ ] Test: file_read of `.env` in workstream root returns error
- [ ] Test: file_read of `src/env.rs` succeeds (no false positive)

## Implementation Notes

- `file_read.rs:54-93` — path validation
- `file_write.rs:44-73` — path validation
- `file_edit.rs:52-93` — path validation
- Best location: a `is_secret_file(path: &Path) -> bool` function in the shared sensitive paths module (same place as T-0171's `SENSITIVE_PATHS`)
- Check should happen after path resolution but before file I/O
- The `allowed_paths` escape hatch should NOT bypass the secret-file check — even explicitly allowed paths shouldn't let the agent read `.env`
- Future consideration: when token storage is added (integration toolkit initiative), the token directory should be outside `allowed_paths` entirely

## Status Updates

- Added `is_secret_file(&Path) -> bool` to `tools/sensitive_paths.rs`. Matches by basename so `./app/.env` and `../../.env` are both caught.
- Patterns: exact (`.env`, `.envrc`, `credentials.{json,yml,yaml}`, `secrets.{yml,yaml,json,toml}`, `token.json`, `tokens.json`); extensions (`.secret`, `.key`, `.pem`, `.p12`, `.pfx`); `.env.*` with allowlist for `example`, `sample`, `template`, `dist`, `default`.
- Wired into `file_read.rs`, `file_write.rs`, `file_edit.rs` immediately after path-traversal check. Runs even on `allowed_paths`, so the escape hatch can't be used to read .env.
- Added unit tests for the helper (positive and negative) plus integration tests in each file tool: `.env` blocked, `env.rs` not blocked.
- All 177 tools tests pass.
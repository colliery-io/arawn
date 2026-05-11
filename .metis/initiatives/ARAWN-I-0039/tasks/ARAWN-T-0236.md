---
id: drive-recent-backfill-stalls-on
level: task
title: "drive/recent backfill stalls on boundary file — sub-ms precision loss on cursor round-trip"
short_code: "ARAWN-T-0236"
created_at: 2026-05-11T12:58:14.119075+00:00
updated_at: 2026-05-11T13:21:28.123678+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# drive/recent backfill stalls on boundary file — sub-ms precision loss on cursor round-trip

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P2 — drive backfill is functionally broken when the most-recent file's
`modifiedTime` doesn't round-trip cleanly through chrono. Gmail's
`existing_message_path` skip masks the same class of issue there;
drive doesn't have an equivalent. Caught during T-0234 smoke testing
on 2026-05-11.

## Reproduction

```
/watch drive/recent smoke-drive since=2026-04-10T00:00:00Z
```

Iter 1 returns 8 files, cursor advances to the newest file's
modifiedTime `"2026-05-10T05:59:47.306+00:00"`. Iter 2 issues
`modifiedTime > '2026-05-10T05:59:47.306+00:00'` to Drive — and Drive
returns the same boundary file again. Template writes it; `new_latest`
stays equal (string compare); spawn-loop stall guard:

```
backfill cursor stalled — template returned items but cursor unchanged
```

## Root cause

`google_drive3` parses Drive's modifiedTime into `DateTime<Utc>` at
millisecond precision (Drive's internal precision is higher).
`f.modified_time.to_rfc3339()` formats that truncated DateTime. We
store the truncated string as the cursor.

On the next call we pass that same truncated string back as the floor.
Drive evaluates `modifiedTime > '2026-05-10T05:59:47.306+00:00'`
against its own internal timestamp (e.g.
`2026-05-10T05:59:47.306789Z`) and the file *legitimately* satisfies
`>` — but our cursor can't represent the precision we'd need to
exclude it.

## Fix

Mirror gmail's idempotence pattern: skip the write (and the
items_written++) when the destination path already exists. The day
partition is known up front from `file.modified_time`, so the check
is one `path.exists()` call — no scan needed.

```rust
let path = day_dir.join(format!("{}.json", file.id));
if path.exists() {
    continue;
}
```

With that, iter 2's returned-boundary-file becomes a no-op; the
spawn-loop sees `items_written == 0` and converges cleanly.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `drive/recent::run` skips the write + items_written++ when
  `<day>/<file_id>.json` already exists on disk.
- [ ] New unit test: simulate iter 2 receiving a file already on disk,
  assert items_written == 0 and cursor unchanged (which now produces
  `no-new-items`, not a stall).
- [ ] Smoke-test: re-run T-0234 scenario, verify backfill converges
  cleanly (`last_status="ok"`, not `backfill-failed`).
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Out of scope

- Storing a higher-precision cursor. Doesn't fix the underlying
  chrono-truncation issue, just shifts the boundary.
- Bumping `>` to `>=` plus an id-set skip. More complex than the
  path-exists check and offers no extra robustness.

## Status Updates

### 2026-05-11 — landed: path.exists() skip in drive/recent::run

Mirrors gmail's `existing_message_path` idempotence pattern. When
Drive's `modifiedTime > '<cursor>'` query returns a file we've already
written (boundary precision tie), the template now skips the write
without incrementing `items_written`. Spawn-loop sees iter 2 as a
clean no-op and converges with `status="no-new-items"` instead of
firing the cursor-stalled guard.

### Implementation

- `crates/arawn-feeds/src/templates/drive/recent.rs` — `path.exists()`
  check before `create_dir_all` + `write_file_metadata`. Day partition
  is known up-front from `file.modified_time`, so it's a single
  filesystem stat — no scan needed.
- 1 new integration test in `tests/drive_recent.rs`:
  `second_run_skips_already_archived_boundary_file` — queues the same
  file twice, asserts iter 2 returns `items_written=0` and
  `status="no-new-items"`.

### Smoke retest

`/watch drive/recent smoke-drive since=2026-04-10T00:00:00Z` →
iter 1 wrote files, iter 2 hit the path.exists() skip, spawn-loop
exited cleanly. Final: `last_status="no-new-items"`, `runs=2`, no
stall.

6/6 drive_recent tests green. Workspace + clippy clean.
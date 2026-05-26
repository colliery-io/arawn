---
id: filesystem-folder-keep-a-durable
level: task
title: "filesystem/folder: keep a durable local copy of ingested files"
short_code: "ARAWN-T-0427"
created_at: 2026-05-26T18:55:22.839540+00:00
updated_at: 2026-05-26T21:54:14.391486+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# filesystem/folder: keep a durable local copy of ingested files

## Objective

The `filesystem/folder` feed (ARAWN-I-0057) watches a folder and indexes file
content for `feed_search`. But the **original file bytes are not retained** — if
the watched folder lives on detachable / sync-backed storage (Google Drive,
Dropbox, a USB drive, an external SSD), the source can vanish: Drive evicts a
locally-cached file, the drive is unplugged, a folder is moved. When that
happens the feed can no longer serve the actual file, only whatever made it into
the projection.

Make the feed keep a **durable local copy** of each ingested file inside arawn's
data dir, so watched content is always available regardless of the source's
availability.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [ ] P0 / [ ] P1 / [x] P2 - Medium (resilience improvement; the feed works
      today, this hardens it against detachable storage)

### Business Justification
- **User Value**: people will inevitably point this at Google Drive / Dropbox /
  removable media. Today an evicted or unplugged source silently degrades the
  feed. A local copy means "watch my Drive notes folder" just works, offline and
  after the cloud client garbage-collects the local cache.
- **Effort Estimate**: M.

## Context / Current Behavior

- `crates/arawn-feeds/src/clients/filesystem.rs` scans the root, fingerprints
  `(mtime, size)`, and emits created/modified/deleted signals to
  `signals.jsonl`.
- `crates/arawn-projections/src/filesystem.rs` reads file content into
  `body_text` **capped at 256 KB** and only for UTF-8 text (binary/larger files
  are indexed as empty). So even the projection isn't a faithful copy.
- Nothing writes the file bytes anywhere arawn controls.

## Acceptance Criteria

## Acceptance Criteria

- [x] On created/modified signals, the feed copies the file to
      `<feed_dir>/files/<rel_path>` (`sync_copy`), parent dirs created as needed.
- [x] Copies survive source **unavailability**: an unmounted/unplugged root
      makes `scan()` error out before diffing, so no deletions fire and copies
      are retained until the source returns. (Genuine per-file deletion while the
      root is readable is mirrored — see below.)
- [x] Bounded: per-file `MAX_COPY_BYTES` = 25 MB; oversize files are skipped with
      a `warn!` and still produce a signal. (No total cap in v1.)
- [x] Deletion policy: **mirror** (user decision) — a deleted source file removes
      its copy; idempotent if the copy is already gone.
- [x] Re-copy is signal-gated: only files that produced a created/modified signal
      this scan are copied; unchanged files emit nothing → no recopy.
- [x] Default: `copy_files` **on**, opt-out via `copy_files=false`; declared in
      `param_schema()` (shows in the `/watch` modal); documented in
      `feed-templates.md`.
- [x] Tests: copy-on-create (nested path), mirror-delete, copy-disabled,
      oversize-skip.

## Decisions (locked with user 2026-05-26)

- **On by default, opt-out** via `copy_files: bool` (serde default true) +
  a `ParamKind::Bool` schema entry.
- **Layout**: mirror the tree (`files/<rel_path>`); content-addressed store
  rejected as overkill for v1.
- **Deletion**: mirror. The unplug/evict resilience comes from `scan()` erroring
  on an absent root, not from retention.
- **Size cap**: per-file 25 MB const, skip + warn; no param, no total cap in v1.
- **Projection unification** (read `body_text` from the copy, lifting the 256 KB
  cap): **deferred** — `body_text` already persists in the DB at ingest, so
  indexing isn't what's at risk; this task is about serving the bytes. Possible
  follow-up.

## Dependencies
Extends the `filesystem/folder` template from ARAWN-I-0057 (completed). The
`copy_files` knob, if added, should be declared in `param_schema()`
(ARAWN-I-0058) so it shows up in the `/watch` modal.

## Risk Considerations
- Disk usage — must be bounded (size cap, optional total cap).
- Copying large/binary trees is out of the feed's "raw text drops" design
  intent; size cap + skip keeps it honest.
- Don't block the scan on slow copies (detachable storage can be slow); consider
  copy failures as non-fatal (log + still signal).

## Status Updates

*Filed 2026-05-26 from a user request.*

**2026-05-26 — Implemented + tested.**
- `FilesystemFeedParams.copy_files: bool` (serde default true) + `param_schema()`
  entry (`ParamKind::Bool`, default true) → shows in the `/watch` modal.
- `run()` mirrors created/modified files into `<feed_dir>/files/<rel_path>` and
  removes the copy on `deleted`, via `sync_copy()`. `MAX_COPY_BYTES` = 25 MB
  (skip + warn). Copy failures are non-fatal (logged; signal still stands).
- Unplug/evict safety: a missing root errors in `scan()` before the diff, so
  copies are retained rather than mass-deleted.
- Tests: `run_copies_files_into_feed_dir_by_default`, `run_mirrors_source_deletion`,
  `run_skips_copy_when_disabled`, `sync_copy_skips_oversize_files`. Updated the
  two schema-key tests (filesystem now has 5 params). Docs updated in
  `feed-templates.md`.
- `angreal check workspace` clean; `cargo test -p arawn-feeds` + service tests
  green; `angreal docs build` clean.
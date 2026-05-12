---
id: drive-projection-drive-files
level: task
title: "Drive projection — drive_files"
short_code: "ARAWN-T-0244"
created_at: 2026-05-12T03:28:17.262856+00:00
updated_at: 2026-05-12T12:52:13.065562+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Drive projection — drive_files

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Implement the `drive_files` projection on top of T-0242's plumbing. Each Drive file (Doc / Sheet / Slide / generic) becomes a projection row with extracted text + metadata, embedded + FTS-indexed.

## Scope

- `drive_files` table: id, feed_id, source_id (Drive fileId), source_ts (modifiedTime), name, path (computed from parents), mime_type, owner, body_text (extracted), size_bytes, created_at/updated_at, UNIQUE(feed_id, source_id).
- FTS5 over `name + body_text`.
- Embedding over `body_text` (truncated to first ~16k chars for large docs — flag in row).
- Mirror-to-projection adapter in `arawn-feeds::templates::drive::folder_sync`.
- Backfill walks the existing mirror.
- Tests: projection row + FTS searchability for a small fixture Drive folder.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `drive_files` table created with FTS + embedding, populated after `drive-folder-sync` feed runs.
- [ ] Idempotent on re-run (UNIQUE on `(feed_id, source_id)`).
- [ ] Files with empty body_text (binary / unsupported) still get rows (name + metadata) but no embedding.
- [ ] Backfill walks the existing mirror.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

- Follow T-0242's `Projection` pattern; add one variant.
- The mirror already has extracted text (htmd-converted markdown for Docs); reuse it directly — no re-extraction.
- Updates: Drive files are mutable. UPSERT on `(feed_id, source_id)` and refresh embedding only if `body_text` changed (cheap hash compare).

### Dependencies

- T-0242 (projection plumbing).

## Status Updates

### 2026-05-12 — Drive adapter landed

- `crates/arawn-projections/src/drive.rs` — `DriveFileProjection`. `walk_feed_dir` reads `meta.json` (tolerating both `cursor.files` and top-level `files` shapes) and reads each file's bytes off disk.
- Text bodies capped at 64KB (truncate, keep going). Non-UTF8 or unreadable files still produce a metadata-only row so the agent tool can surface them by name even if it can't search the body.
- 4 unit tests: walk-files-from-meta, missing-meta-empty, top-level-files-key tolerance, missing-local-file metadata row.
- Wired into dispatcher (`drive` provider). `angreal check workspace` + `clippy` clean.

Body-hash UPSERT path handles Drive's mutable-files case: re-running the feed against an updated Doc refreshes both the row and the FTS5 index in one transaction (UPDATE branch in `ProjectionStore::write_batch`).
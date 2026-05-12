---
id: drive-projection-drive-files
level: task
title: "Drive projection — drive_files"
short_code: "ARAWN-T-0244"
created_at: 2026-05-12T03:28:17.262856+00:00
updated_at: 2026-05-12T03:28:17.262856+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
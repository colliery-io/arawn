---
id: drive-feed-templates-folder-sync
level: task
title: "Drive feed templates — folder-sync + recent"
short_code: "ARAWN-T-0221"
created_at: 2026-05-08T21:01:09.972817+00:00
updated_at: 2026-05-08T22:46:40.029540+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Drive feed templates — folder-sync + recent

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Land the two Google Drive feed templates from I-0039 Phase 4. Split out from T-0217 so each provider can be reviewed in isolation.

- `drive/folder-sync` — rsync-style mirror of a Drive folder, native files on disk. Param: `folder` (path or id).
- `drive/recent` — personal feed: files modified in the last N days. Auto-created on `/connect google_drive`.

Reference: I-0039 Detailed Design; existing `arawn-integrations/src/drive/`.

## Type / Priority

- Feature, P1.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Both templates registered in `arawn_feeds::default_registry`.
- [ ] **DriveFeedClient trait + RealDriveClient adapter** mirror the slack/calendar/gmail pattern. Trait surface: list/changes + get-metadata + download body (+ export for Google natives).
- [ ] **Cursors**:
  - `folder-sync` — per-file `etag/md5` map persisted in cursor; re-fetch only on change.
  - `recent` — `modifiedTime > cursor` window; `pageToken` not required for the simpler "last N days" semantics.
- [ ] **Disk layout**:
  - `drive/folder-sync/<feed_id>/<original_path>` — native file body on disk; preserves Drive's folder structure.
  - `drive/recent/<feed_id>/YYYY-MM-DD/<file_id>.json` — metadata only (recent doesn't mirror bodies).
- [ ] `validate(params)`:
  - `folder-sync` requires `folder` non-empty; resolves to folder ID at registration time.
  - `recent` no required params; optional `days_back: u32` (default 7, validated 1..=90).
- [ ] `defaults(params)`: cadence `1h` for `folder-sync`, `30m` for `recent`.
- [ ] **folder-sync semantics**:
  - One-way pull only.
  - Preserves Drive's folder structure under `feed_dir/`.
  - Google native files (Docs/Sheets/Slides) exported per the `drive_read` mime-dispatch policy (Doc → markdown, Sheet → CSV, Slide → plain text).
  - Deleted files are deleted locally (mirror semantics, not append).
- [ ] **Auto-create** `drive/recent` on `/connect google_drive`. Idempotent. (May be deferred to T-0219 alongside `/feeds` UX, see notes.)
- [ ] **Failure modes**: token expired/scope removed → `FeedError::Auth`; rate-limit → `FeedError::RateLimited(retry_after)`.
- [ ] **Tests** in `crates/arawn-feeds/tests/drive_*.rs`:
  - `folder_sync_mirrors_native_files_and_exports_google_natives`.
  - `folder_sync_deletes_local_when_remote_deleted`.
  - `folder_sync_skips_unchanged_via_etag_cursor`.
  - `recent_writes_per_file_metadata_partitioned_by_modified_date`.
  - `recent_only_returns_files_within_window`.
  - `validate_rejects_missing_required_params`.
  - `returns_auth_when_drive_not_connected`.
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Implementation Notes

### Technical Approach

Same shape as gmail/calendar:
1. `clients/drive.rs` — `DriveFeedClient` trait + `RealDriveClient` adapter wrapping `arawn_integrations::drive::GoogleDriveIntegration`.
2. `templates/drive/{folder_sync.rs, recent.rs, common.rs}` — shared helpers for mime-dispatch export and path-safe writes.
3. Hoist Drive Arc in main.rs the same way calendar/gmail were; `RealClients::with_drive(...)` builder.

Mime-dispatch lives in the existing `arawn-integrations/src/drive/tools.rs` (`drive_read` tool) — re-use the export-mime mapping rather than duplicating.

### Dependencies

- T-0214 (feed runtime, landed).
- Existing `arawn-integrations::drive` client.

### Risk Considerations

- folder-sync's mirror semantics mean we can delete local files; ensure the path we delete is always under `feed_dir/` (no escape via `..` from a hostile filename).
- Google's `files.list` is paginated; helper must follow `nextPageToken` to fully enumerate large folders.

## Status Updates

### 2026-05-08 — drive/{folder-sync, recent} landed

Both Drive templates done in one pass.

**Trait surface** (`DriveFeedClient`):
- `resolve_folder(path_or_id)` — supports `"root"`, raw folder ids, and slash-delimited paths under My Drive (one segment at a time).
- `list_folder_children(folder_id)` — non-recursive; templates own walk semantics with cycle protection + 32-level depth cap.
- `list_modified_since(since, max)` — RFC3339 query, used by `recent`.
- `download(file_id, export_mime)` — `Some(mime)` for Google natives, `None` for raw alt=media.

Plus a `DriveFile` snapshot type kept Serializable so `recent` can write it verbatim, and `export_for(mime)` / `is_unsupported_google_native(mime)` helpers re-using the policy from `drive_read`.

**drive/recent**: per-day partitioned metadata under `<feed_dir>/YYYY-MM-DD/<file_id>.json`. Cursor `{ latest_modified_iso }` advances exactly (Drive's `modifiedTime >` is timestamp-grained, not day-grained, so no in-template dedupe needed). Cadence `*/30 * * * *`.

**drive/folder-sync**: rsync-style mirror with three departures worth calling out:
- **Storage**: bodies at the same relative path Drive uses, `sanitize_path_component` per segment (rejects `..`, `.`, empty; maps `/`, `\`, NUL, control chars to `_`). Google natives get an extension matching their export (`.md`, `.csv`, `.txt`, `.png`).
- **Cursor**: per-file `{ token, path }` map keyed by Drive id. Token is `md5:<hash>` for binaries, `mtime:<rfc3339>` for natives. Renames/moves are first-class — when path changes for a known id, the helper deletes the old path before writing the new one.
- **Mirror**: files in the prior cursor that aren't in the current remote walk get deleted locally (defense-in-depth: every `remove_file` runs through `is_under(feed_dir, _)` first; we refuse to ever touch a path outside `feed_dir`). Empty subdirs pruned on the way out.

Cadence `0 * * * *` (hourly) — folder content changes far less often than chat.

**Departures from the AC**:
- AC said "etag/md5 map"; landed as a "change token" abstraction that handles both binary checksum and Google-native modifiedTime uniformly. Same effect, simpler cursor shape.
- AC said `pageToken` for changes-API style cursoring on `recent`. Landed as straightforward `modifiedTime > <since>` because the semantics ask for "last N days" which doesn't need the changes API's cross-Drive scope. Trivial to add later if we need it.
- Auto-create on `/connect google_drive`: deferred to T-0219 alongside the rest of the auto-create wiring.

**Tests** (12 integration tests + 5 unit tests for helpers): drive_recent.rs covers per-day partitioning, cursor-as-`since` on second run, empty no-op, auth failure, validate gates. drive_folder_sync.rs covers native + export mirror, change-token skip, remote-deletion → local-deletion, rename → old-path cleanup, unsupported native skipped, auth failure, validate gates.

**Production wiring**: hoisted Drive Arc in main.rs the same way as Slack/Calendar/Gmail; `RealClients::with_drive(...)` picks it up. All existing test mocks gained a no-op `drive()` impl.

104 arawn-feeds tests green. `angreal check workspace` and `angreal check clippy` clean.
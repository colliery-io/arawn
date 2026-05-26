---
id: filesystem-folder-keep-a-durable
level: task
title: "filesystem/folder: keep a durable local copy of ingested files"
short_code: "ARAWN-T-0427"
created_at: 2026-05-26T18:55:22.839540+00:00
updated_at: 2026-05-26T18:55:22.839540+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


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

- [ ] On created/modified signals, the feed writes a copy of the file under the
      feed's data dir (e.g. `<feed_dir>/files/<rel_path>` or a content-addressed
      store) before/alongside emitting the signal.
- [ ] Copies survive the source disappearing: after the watched root is
      unmounted/evicted/deleted, previously-ingested files are still readable
      from arawn's copy.
- [ ] Deterministic, bounded behavior: a configurable max file size and/or total
      cap so a huge tree can't fill the disk; oversize files are skipped with a
      logged reason (and still produce a signal).
- [ ] Deletion policy decided + implemented: when the source file is deleted,
      does the local copy stay (archival) or get removed? (Lean: **retain** —
      durability is the whole point — but make it explicit and tested.)
- [ ] Re-copy is idempotent / fingerprint-gated: unchanged files aren't recopied
      each scan.
- [ ] Opt-out or opt-in decided (see Open Questions) and documented in
      `feed-templates.md`.
- [ ] Unit tests: copy-on-create, no-recopy-on-unchanged, oversize-skip,
      survives-source-deletion.

## Open Questions / Decisions Needed

- **On by default, or a `copy_files` param?** Durability-by-default is friendlier
  but uses disk; a bool param (default on?) gives control. Needs a `ParamSpec`
  entry either way (ties into ARAWN-I-0058 schema).
- **Layout**: mirror the tree (`files/<rel_path>`) — human-browsable — vs
  content-addressed (`blobs/<hash>`) with the path map in the cursor —
  dedups identical content, immune to renames. Lean mirror for transcripts.
- **Relationship to the projection**: should `body_text` read from the local
  copy (removing the 256 KB / text-only limitation), making the copy the
  source of truth for indexing too? Likely yes — unifies the two.
- **Size cap defaults** and whether to cap total feed-dir size.

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

*Filed 2026-05-26 from a user request; not yet scheduled.*
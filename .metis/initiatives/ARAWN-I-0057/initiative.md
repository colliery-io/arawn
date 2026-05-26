---
id: local-filesystem-feed-watch-a
level: initiative
title: "Local filesystem feed — watch a folder, emit signals on text-file changes"
short_code: "ARAWN-I-0057"
created_at: 2026-05-25T15:44:24.377793+00:00
updated_at: 2026-05-25T16:30:09.594652+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: local-filesystem-feed-watch-a
---

# Local filesystem feed — watch a folder, emit signals on text-file changes Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

Today every arawn feed is a remote API source (Atlassian, Calendar, Drive, GitHub, Gmail, Slack). There is no way to ingest signal from a **local folder** — e.g. a directory where transcripts, voice-memo text exports, or hand-dropped meeting notes land. Users wanting to feed that material into arawn's KB have to manually paste it.

This initiative adds a `FilesystemFeedTemplate` that points at a folder, walks it on a cadence, and emits `created` / `modified` / `deleted` signals for files matching include globs. It plugs into the existing feed runtime exactly like remote feeds — `/watch` to register, projected through the extractor pipeline, searchable via `feed_search`.

**Primary use case:** transcripts and other raw text dropped into a curated folder. Not a code-project watcher; default excludes catch the common foot-guns but the design assumes the user is pointing at a notes directory, not a repo.

## Goals & Non-Goals

**Goals:**
- A working `filesystem/folder` feed template usable via `/watch /path/to/notes` with sensible defaults
- Scan-and-diff cycle that emits `created`, `modified`, `deleted` signals for matching files
- Initial run on an empty cursor emits `created` for every existing match (auto-indexes a folder you just pointed at)
- Reuse the existing feed dispatch, signal projection, and `feed_search` paths — no new architectural pattern
- Configurable include/exclude globs with sane defaults

**Non-Goals:**
- Sub-second latency (cron-poll model, 30s default cadence)
- Content extraction inside the feed itself — that stays the extractor pipeline's job
- True rename detection — renames are best-effort observed as `deleted` + `created`
- Watching directory mounts or remote filesystems (NFS/SMB) — design targets local POSIX
- Hash-based change detection — `(mtime, size)` is the fingerprint; rapid-edit-within-a-second collisions are accepted

## Architecture

### Fit with existing feed model

Existing feeds implement `FeedTemplate::run(ctx, params, feed_dir, cursor)`. cloacina schedules `run()` on the feed's cadence. The template owns its cursor (opaque JSON).

`FilesystemFeedTemplate` follows the same contract — **no new traits, no push lifecycle, no new dependency**:

1. `run()` walks the configured root (using `walkdir`, which is already a transitive dep via `ignore`)
2. Applies include + exclude globs (`globset`, already a workspace dep)
3. Builds a current file map: `path -> { mtime, size }`
4. Diffs against the previous map in cursor
5. Emits one signal per `(path, event)` pair: created (new path), modified (path exists but fingerprint changed), deleted (was in cursor, not in current scan)
6. Returns the new map as the next cursor

This is operationally identical to a polling REST feed — every cycle is a complete state read + diff against the last cursor.

### Params

```rust
pub struct FilesystemFeedParams {
    pub root: PathBuf,              // absolute path required
    pub recursive: bool,            // default: true
    pub include: Vec<String>,       // default: ["**/*"]
    pub exclude: Vec<String>,       // default: see below
}
```

Default `exclude`:
```
[".git/**", "target/**", "node_modules/**", ".venv/**", "__pycache__/**", ".DS_Store"]
```

These are defense-in-depth against accidentally pointing the watcher at a code dir. The intended use is transcripts/notes folders that won't have these subdirs anyway.

### Cursor

```rust
struct FilesystemFeedCursor {
    files: BTreeMap<PathBuf, FileFingerprint>,
}

struct FileFingerprint {
    mtime: i64,    // unix seconds
    size: u64,
}
```

`BTreeMap` so the serialized cursor is order-stable (helps debugging + makes test fixtures deterministic).

### Signal shape

One signal per event:
```json
{
  "source": "filesystem",
  "path": "/abs/path/to/file.txt",
  "rel_path": "transcripts/2026-05/meeting.txt",
  "event": "created",
  "ts": "2026-05-25T15:44:00Z",
  "size_bytes": 12345,
  "mtime": "2026-05-25T15:43:58Z"
}
```

(`size_bytes` and `mtime` are `null` on `deleted`.)

### `/watch` integration

`FeedRuntime` already supports dynamic feed registration via `/watch` (see Phase 6 in `runtime.rs:111`). The filesystem feed registers exactly like a Slack channel: `/watch filesystem/folder root=/abs/path/to/notes` (params syntax matches existing feeds).

### Validation rules

`FilesystemFeedTemplate::validate`:
- `root` must be an absolute path
- `root` must exist and be a directory
- `root` must have **≥ 3 path components** (rejects `/`, `/home`, `/Users/$USER` — too broad)
- Each `include` and `exclude` must be a valid glob (parse via `globset`)
- `root` must be readable

## Detailed Design

### Why scan-and-diff over `notify`

Considered: push-driven `notify` + debouncer. Rejected because:
- Adds a new architectural pattern (push lifecycle) that the rest of the feed system doesn't have
- Cross-platform quirks (macOS rename = delete+create; Linux gives both; Windows is its own story)
- Debounce window logic is a real source of bugs; cron-poll has no such concept
- For the actual use case (text files arriving in a notes folder), 30s latency is fine — the extractor pipeline is the long pole anyway

scan-and-diff gives us:
- One file of new code (no trait change)
- No new dependencies (walkdir + globset already present)
- Trivially testable (fixture dirs + `tempdir` + assert against emitted signals)
- Self-healing — if the cursor gets corrupted, next cycle reconstructs

### Performance envelope

A scan touches:
- One `walkdir` traversal of root (respects `recursive`)
- One `stat()` per file (built into `walkdir::DirEntry::metadata()`)
- Glob match per path (cheap; `globset` compiles once)

For 100 files at 30s cadence: negligible. For 10,000 files at 30s cadence: still <100ms scan time on local SSD. For 100,000+ files: user should tighten globs or raise cadence — we document this, don't engineer around it.

### Lifecycle of a new file

1. User drops `meeting.txt` into watched folder at t=0
2. Cron fires `run()` at t≤30s
3. Scan sees `meeting.txt`, cursor doesn't → emits `created` signal
4. Signal routed through existing dispatch → projection → extractor
5. KB now contains entity references from `meeting.txt`
6. `feed_search "meeting"` finds it

### Lifecycle of a delete

1. User removes `old-meeting.txt`
2. Cron fires `run()`
3. Scan misses `old-meeting.txt`; cursor has it → emits `deleted` signal
4. Downstream consumers can decide what "deleted" means (the extractor pipeline may want to tombstone projections; that's a follow-up if it becomes a problem)

## Alternatives Considered

**A. `notify`-driven push feed.** Sub-second latency; new architectural pattern; complex lifecycle, debounce, cross-platform fragility. Rejected — wrong tool for transcript-arrival use case.

**B. Cron-poll + content hash instead of `(mtime, size)`.** More robust against rapid edits at the same size; requires reading every file every cycle. Rejected — `(mtime, size)` collisions are extremely rare for human-typed text and we can add a hash field later non-breakingly.

**C. Read file contents in the feed `run()` and emit the body as part of the signal.** Bypasses the extractor pipeline; couples watcher to extraction policy. Rejected — signal is metadata; content access happens downstream.

**D. Auto-discover all text files at startup with no globs.** Rejected — explicit include list keeps the feed predictable and reviewable.

## Implementation Plan

Four tasks, executed serially via ralph-tasks:

- **T-A — Template + params/cursor types.** Skeleton `FilesystemFeedTemplate` in `crates/arawn-feeds/src/clients/filesystem.rs`. `FilesystemFeedParams` with serde. `FilesystemFeedCursor` with `BTreeMap`. `validate()` covering root-depth, glob-syntax, dir-exists. Unit tests for validation and param parsing.
- **T-B — Scan-and-diff core.** `run()` implementation: walkdir + globset + diff against cursor + emit signals. Synthetic-`created`-on-empty-cursor behavior. Unit tests with `tempdir` fixtures: empty cursor emits N created; modify file → modified; remove file → deleted; exclude globs honored.
- **T-C — Wire into FeedRuntime + register template.** Add to `register_default_templates` in `crates/arawn-feeds/src/runtime.rs`. Wire signals through the standard projection path so `feed_search` finds them. Add a projection table if needed (or piggyback on existing).
- **T-D — `/watch` UX + docs + UAT.** Verify `/watch filesystem/folder root=/some/path` works end-to-end through the existing `/watch` flow. Docs page describing the feed (params, defaults, semantics). One UAT scenario: drop a fixture transcript into a tempdir, assert `feed_search` returns it.

## Exit Criteria

- [ ] `FilesystemFeedTemplate` registered + visible in feed listing
- [ ] `/watch filesystem/folder root=/path` end-to-end produces signals visible via `feed_search`
- [ ] Initial-scan emits `created` for every existing match on first run
- [ ] Cursor round-trips correctly across `run()` invocations (delete one file, next run emits exactly one `deleted` signal)
- [ ] Default excludes (`.git`, `target`, etc.) prevent accidental ingestion of build artifacts
- [ ] UAT scenario passes (drop transcript → searchable)
- [ ] Docs page exists explaining params + defaults
- [ ] Full `angreal test uat` green
---
id: projections-plumbing-gmail
level: task
title: "Projections plumbing + gmail_messages reference impl"
short_code: "ARAWN-T-0242"
created_at: 2026-05-12T03:28:15.356110+00:00
updated_at: 2026-05-12T03:28:15.356110+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Projections plumbing + gmail_messages reference impl

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Stand up the per-feed-type projection layer described in I-0040 Phase 2: normalized sqlite tables per feed item type, embedded + FTS-indexed, written on every successful feed run. Land the shared plumbing + the `gmail_messages` projection as a worked reference. Subsequent tasks (T-0243–T-0246) implement the remaining feed types on this foundation.

Projections are the second of the three-layer architecture (feeds → projections → palaces). They give cross-feed semantic search without any workstream declared, and are the input to the per-workstream extractor in Phase 4.

## Scope

### Crate / module layout

New crate `arawn-projections` (or a sibling module in `arawn-memory`, TBD by simplest cargo dep graph). Houses:

- A `Projection` trait or enum surface — one variant per feed type, each with its own normalized fields.
- `projections.db` (or a table set in the existing memory db — open question, see notes).
- A shared writer that handles embedding + FTS in one transactional dual-write, mirroring the T-0240 pattern in `arawn-memory::store`.

### Storage schema

Per-feed-type table. For the gmail reference impl:

```sql
CREATE TABLE gmail_messages (
    id TEXT PRIMARY KEY,              -- stable hash of (feed_id, source_id)
    feed_id TEXT NOT NULL,            -- which feed produced this
    source_id TEXT NOT NULL,          -- provider's id (gmail message id)
    source_ts TEXT NOT NULL,          -- RFC3339; the item's authored timestamp
    sender TEXT,
    recipients TEXT,                  -- JSON array
    subject TEXT,
    body_text TEXT NOT NULL,
    thread_id TEXT,
    labels TEXT,                      -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(feed_id, source_id)
);

CREATE VIRTUAL TABLE gmail_messages_fts USING fts5(
    projection_id UNINDEXED,
    sender, subject, body_text,
    tokenize = 'unicode61'
);

CREATE TABLE gmail_messages_embeddings (
    projection_id TEXT PRIMARY KEY,
    embedding BLOB NOT NULL
    -- (sqlite-vec virtual table or raw blob — match arawn-memory's pattern)
);
```

Other feed types follow the same shape with their own normalized field set.

### Embedding pipeline

Reuse `arawn-embed` (already loaded by `arawn-memory` for all-MiniLM-L6-v2). Batch projection rows on insert (configurable batch size, default 32). Throttle config respects the feed-runtime backoff settings already in `arawn-feeds::runtime`.

### Dispatch hook

`crates/arawn-feeds/src/dispatch.rs::run_feed` already returns per-run stats. Add a post-run callback that:

1. Reads new items from the feed mirror (since last projection cursor).
2. Normalizes each item into the corresponding `Projection` variant.
3. Writes to the projection table + FTS + embedding inside one tx.
4. Advances the per-feed projection cursor.

### Backfill

On first `MemoryStore::open` / projection-db open, walk every feed mirror's existing items and project them. Same spawn-loop convergence pattern as I-0039 / T-0227. The cursor handles incremental subsequent runs naturally.

### `feed_search` interface

Not in this task — T-0247. This task just ensures the storage shape supports it (entity_id + ranked-FTS-MATCH + vector-similarity primitives).

### What's deferred

- Other feed types (T-0243 Slack, T-0244 Drive, T-0245 Atlassian, T-0246 Calendar).
- `feed_search` agent tool (T-0247).
- Per-workstream extraction (Phase 4).

## Acceptance Criteria

- [ ] `arawn-projections` (or shared module) exists with a `Projection` trait/enum + writer that handles embedding + FTS dual-write in one transaction.
- [ ] `gmail_messages` projection wired end-to-end: gmail feed run produces projection rows after the run completes.
- [ ] Backfill loop projects pre-existing gmail mirror items on first projection-db open.
- [ ] Idempotent on re-run — duplicate items (matched by `(feed_id, source_id)`) update rather than duplicate.
- [ ] Embedding pipeline batches + throttles per config; throughput documented.
- [ ] Unit tests cover: projection write, embedding store, FTS row creation, dedup on re-run.
- [ ] Integration test: run a small fixture gmail feed → assert projection rows present + searchable via raw FTS.
- [ ] `angreal check workspace` + `angreal check clippy` clean.
- [ ] Open design questions resolved and decisions logged in this task's status updates:
  - Where do projections live — separate `projections.db` or extra tables in the memory db?
  - One crate vs. one module per feed type vs. shared crate with one file per feed type?

## Implementation Notes

### Technical approach

1. **Decide storage location.** Two options: (a) extra tables in the existing memory db so search joins are local; (b) separate `projections.db` so projection volume doesn't bloat the memory db (gmail alone could be 100k+ rows). Default to (b) unless join requirements emerge.
2. **Projection trait.** Each feed type implements `from_mirror_item(&MirrorItem) -> Projection`. The trait is sealed; new feed types add a variant (closed-enum policy mirroring `EntityType`).
3. **Writer.** Single `ProjectionWriter` with a `write_batch(&[Projection]) -> Result<()>` that runs inside `with_tx`. Per projection: insert row → FTS upsert → embed + store. Same transactional dual-write pattern as `arawn-memory::store`.
4. **Embedding policy.** Embed `body_text` (or `subject + body_text` for gmail). 384-d all-MiniLM-L6-v2 to match memory. Batched embedding via `arawn-embed`'s batch interface.
5. **Dispatch hook.** Add `dispatch::run_feed` hook that calls the projection writer after `finalize_backfill_success`. Hook returns errors via the existing FeedError surface but doesn't fail the feed run on projection-side issues (per the "warn and continue" pattern from T-0237).
6. **Backfill.** On projection-db open, query each feed's mirror for items not yet in the projection (LEFT JOIN on `source_id`). Walk results in batches. Cursor unnecessary if dedup is `(feed_id, source_id)` UNIQUE.

### Dependencies

- T-0240 (memory infrastructure as the FTS+vector dual-write template).
- `arawn-feeds::dispatch::run_feed` (hook point).
- `arawn-embed` (embedding pipeline).

### Risk considerations

- **DB write contention.** Projections + memory + feeds all writing to sqlite. WAL is enabled. Separate `projections.db` reduces lock surface.
- **Embedding throughput on backfill.** 100k gmail messages at ~20ms each = 30+ min. Throttling needs a checkpoint so backfill survives restarts. Cursor on backfill is OK; cursor on incremental is optional.
- **Schema drift across feeds.** Each feed type has its own normalized fields. Avoid a god-table; one table per type. Forces the closed-enum policy in Rust.
- **Idempotency under updated items.** Gmail messages don't update; jira issues do. The reference impl can ignore updates; document that update handling is per-feed (revisit in T-0245 for Atlassian).

## Status Updates **[REQUIRED]**

*To be added during implementation*
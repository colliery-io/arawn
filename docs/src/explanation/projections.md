# Projections

*Explanation. Why projections are flat, why the embedding column exists, when to read which layer.*

Projections are the middle layer between raw [feeds](./feeds.md) and curated [palaces](./palaces.md). After a feed fetches content, the dispatcher writes a normalized row into `projections.db` — one table per feed type, with shared columns plus a per-type `metadata` JSON blob.

For the schemas, see [projection tables reference](../reference/projection-tables.md). For the agent-facing search tool, see [`feed_search` tool reference](../reference/feed-search-tool.md). This page is about *why* projections look the way they do.

## What projections give you that feeds don't

Two things:

1. **Cross-feed semantic search.** `feed_search "what did the team decide about postgres?"` walks every projection table — gmail, slack, jira, confluence, drive — and returns ranked hits. No workstream required. The feeds layer can't do this; it's file-on-disk, no FTS, no embedding.
2. **A flat substrate the extractor can walk.** The palace extractor (`arawn_extractor::cot::CotChain`) reads projection rows in `source_ts` order via a cursor and turns them into typed entities. It needs a relational table, not a file tree.

## Why flat

Projections have **no relations between rows**. Two emails are two rows; if one quotes the other, the projection layer doesn't know. Why design it that way?

- **Relations are interpretation, not data.** Two Slack messages might be a reply chain that matters for workstream A and noise for workstream B. Encoding the relation at the projection layer would lock in one workstream's view.
- **The query patterns benefit from flatness.** Cross-feed FTS works at table-scan speed because there's no edge traversal. Vector search works because the embedding column is a per-row property, not a per-edge one.
- **The extractor wants flat input.** The CoT chain processes rows sequentially with a `WHERE source_ts > cursor` filter. A graph-shaped substrate would force expensive traversal at extractor entry.

Relations belong in the palace layer, where they're typed (`supersedes`, `relates_to`, `mentions`) and per-workstream.

## Why an embedding column

`feed_search` does hybrid retrieval — FTS5 keyword match RRF-fused with vector similarity. The vector half needs an embedding per row. Two design choices:

- **Embeddings live in the projection table**, not in a sidecar. `sqlite-vec` extension makes this fast and atomic with the row writes. No "wait for sync" between two stores.
- **A background pass fills them in.** Writing the embedding inline with the row would block the feed dispatcher on inference latency. Instead, rows ship with `embedding IS NULL`; a 5-minute tokio task walks unfilled rows and embeds in batches.

If the embedder model isn't installed, embeddings stay NULL. `feed_search` degrades to FTS-only — slower at semantic queries but functional. The graceful fallback is intentional.

## Why one table per feed type

`gmail_messages`, `slack_messages`, `jira_issues`, `confluence_pages`, `drive_files`, `calendar_events`, `github_notifications`, `github_issues`, `github_prs`, `github_reviews`, etc. Why not a single big table with a `kind` column?

- **Per-type metadata.** A Gmail message has `sender, recipients, subject, thread_id, labels`. A Jira issue has `key, status, assignee, components, labels`. No useful single schema covers both. Per-table = honest schema per type.
- **FTS index granularity.** SQLite FTS5 wants per-table tokenization. A "search Slack only" query benefits from indexing just `slack_messages.body_text`, not stripping out 95% of rows by `kind`.
- **Migration scope.** Adding a new feed type adds a new table. Old tables don't move. The schema migration story is local.

The trade-off: queries that span types (`feed_search` with no `feed_types` filter) iterate per table. The dispatch code is uglier than a single-table query, but the per-table benefits dominate.

## Why append-only

Rows are append-only from the projections layer's perspective. They get rewritten only when a feed re-fetches the same `source_id` (idempotent on `id` primary key — same content, same row).

- The `id` is deterministic: a hash of `feed_id + source_id`. Same inputs, same row.
- Idempotency means re-running a feed is safe — repeated `INSERT OR REPLACE` produces the same end state.
- Backfill across rate-limit waves works because the cursor walks forward; even if cron re-fires before the previous run finished, the de-dup is automatic.

No `UPDATE` semantics for content changes. If a Jira issue's status changes, the feed dispatcher writes a new projection row for the issue (the issue's `source_ts` advances on update). The palace layer's `EXTRACTED_FROM` provenance can still trace back; the steward can de-duplicate entities if needed.

## Why provenance is one-way

Every projection row gets a UUID derived from its `id` (Uuid v5, namespace OID). Palace entities that get extracted from that row get an `EXTRACTED_FROM` edge to the UUID. **The steward is forbidden from removing those edges.** Why?

- **Trust.** If the agent says "this decision came from that email," the user needs to be able to walk the link. Removable provenance means removable trust.
- **Rollback.** The steward's apply/rollback contract relies on `EXTRACTED_FROM` edges to find which entities a re-shelve operation affected. Removing them would break rollback.

The price is a small write per entity. Cheap.

## When to read projections vs. feeds vs. palace

| Question | Layer | Why |
|---|---|---|
| What did the message *say* literally? | feeds (file read) | The bytes are there; no transformation needed. |
| Find any content that mentions X across all feeds. | projections (`feed_search`) | Cross-feed FTS + vector is what projections give you. |
| Find an entity / decision / convention in *one workstream*. | palace (`signal_search`) | Typed entities are a palace-layer thing. |
| Filter entities by type or tag. | palace (`signal_query`) | Same. |
| Chronological "what happened in workstream X." | palace (`signal_timeline`) | Same. |
| Hydrate a specific entity back to its source content. | palace → projections → feeds | Walk the `EXTRACTED_FROM` edge → `ProjectionStore::get_row` → file read. |

The walk-down-the-stack pattern is rare but supported. Most queries live in one layer.

## What's NOT in projections

- **Relations.** Edge-shaped data lives in the palace.
- **User curation.** Projections are mechanical — raw item from upstream becomes a projection row. The user doesn't refine them.
- **Workstream scope.** A projection row isn't "for" any workstream — it's a per-feed-type fact. Workstream binding determines which extractors process it, not which projection rows exist.

## Why a separate DB file

`projections.db` is its own SQLite file, separate from per-workstream `memory.db` and global `memory.db`. Trade-offs considered:

- **Single shared DB**: simpler, but locking issues at high write rates (Slack channel-archive fires every 15 min across many channels).
- **One DB per feed**: clean isolation, but cross-feed search would need to ATTACH every DB on every query.
- **One DB for all projections**: chosen. Cross-feed search via `JOIN` or `UNION` across in-DB tables. Locking is fine because writes are batched per-feed-run.

## Related

- [Projection tables reference](../reference/projection-tables.md) — per-table schemas.
- [`feed_search` tool reference](../reference/feed-search-tool.md) — the agent-facing read surface.
- [The three-layer data model](./three-layer-data-model.md) — projections in context.
- [Extraction explanation](./extraction.md) — what consumes projection rows.
- [Feeds explanation](./feeds.md) — what produces projection rows.

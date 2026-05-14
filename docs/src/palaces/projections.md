# Projections

Projections are the middle layer between raw [feeds](../feeds/index.md)
and curated [workstream palaces](./index.md). After a feed fetches
content from upstream, the dispatcher writes a normalized row into
`projections.db` — one table per feed type, with shared columns
(`id`, `feed_id`, `source_id`, `source_ts`, `title`, `body_text`,
plus an embedding column for vector search) and a per-type
`metadata` JSON blob.

Projections give you two things the file-based feed layer doesn't:

1. **Cross-feed semantic search** via the
   [`feed_search`](../feeds/feed-search.md) agent tool — FTS5 + vector
   similarity (RRF-fused) over every projection table, no workstream
   required.
2. **The substrate the extractor reads** when building workstream
   palaces. The CoT chain (`classify → extract → link → write`) walks
   projection rows in `source_ts` order and turns them into typed
   entities.

## Projection tables

One table per feed type. Each carries the shared columns plus
per-type metadata:

| Feed type | Source | Per-type metadata |
|---|---|---|
| `gmail_messages` | Gmail feed templates (inbox-archive, etc.) | sender, recipients, subject, thread_id, labels |
| `slack_messages` | Slack `channel-archive` (top-level posts) | channel_id, sender_id, thread_ts, reactions |
| `slack_thread_messages` | Slack `channel-archive` (thread replies) | same as above + `is_thread_reply: true` |
| `drive_files` | Drive `folder-sync` / `recent` | file_id, path, mime_type, owners |
| `jira_issues` | Jira `project-tracker` / `assignee-tracker` | key, status, assignee, components, labels |
| `jira_comments` | Jira issue comments | issue_key, author |
| `jira_history` | Jira changelog entries | issue_key, field, from, to |
| `confluence_pages` | Confluence `space-archive` | space_key, page_id, version, author |
| `calendar_events` | Calendar `upcoming` | event_id, start, end, attendees, location |

Shape of a single row (the type-erased view):

```rust
pub struct ProjectionRow {
    pub id: String,             // stable per-row id (deterministic from feed_id + source_id)
    pub feed_id: String,        // which feed produced it
    pub source_id: String,      // upstream id (gmail msg id, slack ts, jira key, ...)
    pub source_ts: DateTime<Utc>,
    pub title: String,
    pub body_text: String,
    pub feed_type: String,
    pub metadata: serde_json::Value,
}
```

Every projection row that backs a workstream entity has an
`EXTRACTED_FROM` edge from the entity to a UUID derived from
`projection_id`. That's the provenance link the agent (and the
journal-based rollback) uses to trace any palace entity back to the
content that spawned it.

## How rows get written

The dispatcher (`arawn_feeds::dispatch::run_feed`) is the only writer.
When a feed's cloacina-scheduled task fires:

1. Template fetches new items from upstream.
2. Each item is materialized as a typed projection struct
   (`GmailMessageProjection`, `SlackMessageProjection`, ...).
3. The struct implements `Projection::row()` which produces the
   type-erased `ProjectionRow`.
4. `ProjectionStore::write_batch` inserts rows in one transaction.
5. After the write, the dispatcher fires the extractor hook for every
   active workstream that has the feed bound — the per-workstream
   extractor picks up the new rows via its cursor.

Rows are append-only from the projections layer's perspective. They
get rewritten only when a feed re-fetches the same `source_id`
(idempotent on `id` primary key — same content, same row).

## Embeddings

Each projection table has an `embedding` column. A background pass
(`arawn_projections::run_embed_pass`) walks rows whose embedding is
`NULL` and fills them in via the configured embedder. Runs every
5 minutes on a tokio task.

`feed_search` uses these embeddings for the vector half of its
hybrid ranking. The extractor's `link_by_name` stage does NOT use
embeddings — it uses FTS5 + the entity titles. Embeddings are a
projection-layer concern, not a palace-layer concern.

## When to read projections vs palace

| Question | Reach for… |
|---|---|
| What did the message *say*? | The raw feed file (template-catalog has the layout) |
| Find any content that mentions X across all feeds | `feed_search` (projections) |
| Find an entity / decision / convention in *one workstream* | `signal_search` (palace) |
| Filter entities by type or tag in one workstream | `signal_query` (palace) |
| Chronological "what happened in workstream X" | `signal_timeline` (palace) |
| Hydrate a specific projection row from an entity's `EXTRACTED_FROM` edge | Resolve the UUID back to `projection_id`, then `ProjectionStore::get_row` |

Projections are deliberately *flat*. There are no relations between
projection rows. The graph structure (entities + edges) is the
palace's job — projections feed it.

## See also

- [Feeds](../feeds/index.md) — the layer below; how content gets here.
- [Feed search tool](../feeds/feed-search.md) — the agent-facing
  interface to projections.
- [Extraction](./extraction.md) — how projection rows become entities.
- [Palaces](./index.md) — the layer above.

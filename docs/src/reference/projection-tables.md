# Projection tables

*Reference. Per-feed-type schemas + the `ProjectionRow` struct + embedding mechanics.*

Projections are the middle layer between raw [feeds](./feeds-overview.md) and curated [lens palaces](./palace-types.md). After a feed fetches content from upstream, the dispatcher writes a normalized row into `projections.db` — one table per feed type, with shared columns plus a per-type `metadata` JSON blob.

For the conceptual framing, see [projections explanation](../explanation/projections.md). For the agent-facing search interface, see [`feed_search` tool reference](./feed-search-tool.md).

Source: `crates/arawn-projections/`.

## Shared columns

Every projection table has:

| Column | Type | Purpose |
|---|---|---|
| `id` | string | Stable per-row id (deterministic hash from `feed_id + source_id`). |
| `feed_id` | string | Which feed produced this row. |
| `source_id` | string | Upstream id — Gmail message id, Slack ts, Jira issue key, etc. |
| `source_ts` | RFC3339 | The item's authored timestamp. |
| `title` | string | One-line summary used by FTS. |
| `body_text` | string | Searchable body content. |
| `body_hash` | string | Hash of `body_text` used to detect updates between runs. |
| `metadata` | JSON | Per-type fields (see below). |

The `feed_type` shown on the `ProjectionRow` struct below is **populated at hydration time** from the originating table name — it is not a column on the data tables themselves (each `<feed_type>` table's identity is its name).

### Sibling tables per feed type

For each `<feed_type>` data table, two siblings exist:

| Table | Purpose |
|---|---|
| `<feed_type>_fts` | FTS5 virtual index over `title + body_text`. |
| `<feed_type>_embeddings` | Bookkeeping rows with a `status` enum (`pending` / `embedded` / `skipped`) tracking the embedding pass. |
| `<feed_type>_vec` | sqlite-vec `vec0` virtual table holding the embedding vectors. |

Embeddings live in `<feed_type>_vec`, **not** as a column on the data table. The 5-minute embed-pass walks `<feed_type>_embeddings WHERE status = 'pending'` and writes vectors into `<feed_type>_vec`.

## Per-table metadata

The 16 projection tables (source: `crates/arawn-projections/src/`):

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
| `calendar_events` | Calendar `upcoming-archive` | event_id, start, end, attendees, location |
| `github_notifications` | github/notifications | id, reason, repository, subject, updated_at |
| `github_issues_and_prs` | github/issues-and-prs | repo, number, kind (issue/pr), state, labels, author |
| `github_review_queue` | github/review-queue | repo, pr_number, reviewer, requested_at |
| `github_repo_commits` | github/repo-mirror | repo, sha, author, committed_at |
| `github_repo_issues` | github/repo-mirror | repo, number, state, labels, assignees, author |
| `github_repo_prs` | github/repo-mirror | repo, number, state, labels, requested_reviewers, author |
| `github_issue_or_pr_comments` | github/repo-mirror | repo, issue_or_pr_number, author |

## `ProjectionRow` (type-erased view)

```rust
pub struct ProjectionRow {
    pub id: String,             // stable per-row id (deterministic from feed_id + source_id)
    pub feed_id: String,        // which feed produced it
    pub source_id: String,      // upstream id
    pub source_ts: DateTime<Utc>,
    pub title: String,
    pub body_text: String,
    pub feed_type: String,      // populated at hydration from the originating table name
    pub metadata: serde_json::Value,
}
```

Every projection row that backs a lens entity has an `EXTRACTED_FROM` edge from the entity to a UUID derived from `projection_id`. That's the provenance link the agent (and the journal-based rollback) uses to trace any palace entity back to the content that spawned it.

## How rows get written

The dispatcher (`arawn_feeds::dispatch::run_feed`) is the only writer. When a feed's cron-scheduled task fires:

1. The template fetches new items from upstream.
2. Each item is materialized as a typed projection struct (`GmailMessageProjection`, `SlackMessageProjection`, etc.).
3. The struct implements `Projection::row()` which produces the type-erased `ProjectionRow`.
4. `ProjectionStore::write_batch` inserts rows in one transaction.
5. After the write, the dispatcher fires the extractor hook for every active lens that has the feed bound — the per-lens extractor picks up the new rows via its cursor.

Rows are **append-only** from the projections layer's perspective. They get rewritten only when a feed re-fetches the same `source_id` (idempotent on `id` primary key — same content, same row).

## Embeddings

Each projection table has an `embedding` column. A background pass (`arawn_projections::run_embed_pass`) walks rows whose embedding is `NULL` and fills them in via the configured embedder. Runs every 5 minutes on a tokio task.

`feed_search` uses these embeddings for the vector half of its hybrid ranking. The extractor's `link_by_name` stage does NOT use embeddings — it uses FTS5 + the entity titles. Embeddings are a projection-layer concern, not a palace-layer concern.

If the embedder isn't installed (`<data_dir>/models/all-MiniLM-L6-v2/model.onnx`), embeddings stay `NULL`. `feed_search` then degrades to FTS-only — slower at semantic queries but functional.

## When to read projections vs. palace

| Question | Reach for… |
|---|---|
| What did the message *say*? | Raw feed file (see [feed templates reference](./feed-templates.md)) |
| Find any content that mentions X across all feeds | [`feed_search`](./feed-search-tool.md) (projections) |
| Find an entity / decision / convention in *one lens* | `signal_search` (palace, see [lens tools reference](./lens-tools.md)) |
| Filter entities by type or tag in one lens | `signal_query` (palace) |
| Chronological "what happened in lens X" | `signal_timeline` (palace) |
| Hydrate a specific projection row from an entity's `EXTRACTED_FROM` edge | Resolve the UUID back to `projection_id`, then `ProjectionStore::get_row` |

Projections are deliberately **flat**. There are no relations between projection rows. The graph structure (entities + edges) is the palace's job — projections feed it.

## Related

- [Palace types reference](./palace-types.md) — the entity/relation catalog the layer above uses.
- [Projections explanation](../explanation/projections.md) — why flat, when to read which type.
- [`feed_search` tool reference](./feed-search-tool.md).
- [Feeds overview reference](./feeds-overview.md).

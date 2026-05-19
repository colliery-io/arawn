# The three-layer data model

*Explanation. Why arawn has feeds, projections, AND palaces — and which question each layer answers.*

```
Feeds        →  raw bytes from upstream (Slack, Gmail, Drive, Jira, …)
Projections  →  per-feed-type normalized rows in a single sqlite db
Palaces      →  per-workstream graphqlite KB of typed entities + relations
```

Three layers because each answers a different kind of question. The agent reaches for the lowest layer that answers what you asked.

## The three layers, made concrete

| Layer | Storage | Owns | Built by |
|---|---|---|---|
| **Feeds** | Files on disk under `~/.arawn/data/<provider>/<template>/<feed_id>/` | Fidelity — bytes look like what the provider returned. | The feed runtime (cloacina cron job per feed). |
| **Projections** | `projections.db` (SQLite + sqlite-vec), one table per feed type | Findability — cross-feed semantic + structured search. | The feed dispatcher writes a row per fetched item. |
| **Palaces** (workstream KBs) | `<data_dir>/workstreams/<name>/memory.db` (SQLite + graphqlite) | Curated meaning — typed entities + relations, with provenance back to projection rows. | The per-workstream extractor on each new projection row + the steward on a cadence. |

Each layer can be queried independently. The agent reaches for the lowest layer that answers the question:

- *"What did the email say verbatim?"* → feeds (file read).
- *"Find every message that mentions Postgres."* → projections ([`feed_search`](../reference/feed-search-tool.md)).
- *"What did we decide about the ledger service?"* → palace (`signal_search`, `signal_query`).

## Why three layers instead of one

A single layer would have to be optimized for one of three contradictory access patterns:

- **Fidelity** says "store exactly what the provider sent." File-on-disk is best — a JSONL line per Slack message, a `.json` per Gmail thread, the raw Confluence XML body.
- **Findability** says "let me search across all of it." File-on-disk loses here — there's no FTS index, no vector search, no way to ask "every item mentioning X across all my feeds."
- **Meaning** says "what's the *decision* about X, regardless of which email or Slack thread it landed in?" Findability still loses — searching for "Postgres" returns 1500 messages, not "we decided to use Postgres 16 because…"

Three layers, three answers. Each layer's job is small and focused; the next layer builds on it.

## Why feeds are file-on-disk

Files are the most boring possible storage. The agent reads them with `Read`, `Glob`, `Grep` — the same tools it uses on source code. There's no special query API, no schema migration, no "how do I get the bytes out of arawn?" problem. The mirrored bytes are exactly what the provider sent, in a tree you can `cat` or `rsync` or back up.

Disk usage grows with what you mirror, but you choose what to mirror via `/watch`. Local-first by design — no upstream summarizer, no cloud index.

[Feeds explanation](./feeds.md) goes deeper.

## Why projections are flat

`projections.db` has one table per feed type — `gmail_messages`, `slack_messages`, `jira_issues`, etc. Each row carries shared columns (`id`, `feed_id`, `source_id`, `source_ts`, `title`, `body_text`, `embedding`) plus a per-type `metadata` JSON blob.

**No relations between projection rows.** Two emails are two rows; if one quotes the other, the projection layer doesn't know. Why?

- Relations are a property of the *workstream's interpretation*, not of the data. A reply chain in #design that matters for workstream A is noise for workstream B.
- Projections are the substrate for cross-feed search and the extractor's input. Both want flat tables — one for FTS5 indexing, one for cursor walks.
- Relations as edges live in the palace layer where they're typed (`supersedes`, `contradicts`, `relates_to`) and per-workstream.

[Projections explanation](./projections.md) goes deeper.

## Why palaces are typed

The third layer is where the agent actually answers "what do you know about X?" Some content is decisions; some is conventions; some is just notes. The closed set of entity types (`decision`, `convention`, `fact`, `preference`, `person`, `note`) gives the agent a language for filtering — `signal_query { entity_type: "decision", since: "2026-04-01" }` is a meaningful query.

Relations between entities (`supersedes`, `contradicts`, `supports`, `mentions`, etc.) are also typed. The agent can traverse: "this decision supersedes that question; what was the original framing?"

[Palaces explanation](./palaces.md) covers the metaphor and lifecycle.

## Per-workstream palaces, not one global palace

Each workstream gets its own palace at `<data_dir>/workstreams/<name>/memory.db`. Why not one global graph?

- **Locality.** A decision in your "work" workstream shouldn't pollute a query in your "home" workstream. Cross-contamination would make the search worse.
- **Ontology.** Each workstream has its own closed tag ontology (5-12 slugs). One ontology can't reasonably span "ledger service" and "kid's soccer schedule."
- **Operability.** Backing up, archiving, or deleting a workstream is `rm -rf` the directory. No graph surgery.

Per-workstream isolation does mean cross-workstream identity is something the steward has to opt into via the `doorwatch` subroutine — but that's the steward's job, not the storage layer's.

## How the layers compose at read time

Most agent queries hit one layer:

| You ask | Agent calls | Layer |
|---|---|---|
| "What's in this Slack channel today?" | `Read` or `Glob` on `data/slack/channel-archive/...` | feeds |
| "Find any mention of $term across feeds." | `feed_search` | projections |
| "What did we decide about X?" | `signal_search` | palace |
| "Show me decisions tagged `postgres` since April." | `signal_query` | palace |
| "Trace this decision back to the original email." | `signal_query` → walk `EXTRACTED_FROM` → resolve to projection row → optional `feed read` | palace → projections (→ feeds) |

The walk-down-the-stack pattern (palace → projection → feed) is real but rare. The agent uses it when you ask for original-source content. Most workflows live in one layer.

## How the layers compose at write time

```
Feed cron tick:
  template.fetch() → upstream API → in-memory items
       │
       ▼
  ProjectionStore.write_batch(items) → projections.db
       │
       ▼
  For each workstream bound to this feed:
       extractor.run(new rows)
            │
            ▼
       palace KB updated (entities + EXTRACTED_FROM edges)
```

The dispatcher (`arawn_feeds::dispatch::run_feed`) is the only writer for projections. The extractor (`arawn_extractor::cot::CotChain`) is the only writer for palace entities. Files in `data/<provider>/<template>/<feed_id>/` are template-controlled — they own their slice of the directory.

## When to bother with palaces

Palaces aren't free — extraction costs LLM calls. They pay off when you'll ask the agent about a workstream's state *repeatedly*. A one-off "what did X mean by Y" is a feeds question; a recurring "what's the state of our migration to Postgres" is a palace question.

[When palaces make sense](./palaces.md#when-to-bother) has the trade-off table.

## Related

- [Feeds explanation](./feeds.md) — layer 1.
- [Projections explanation](./projections.md) — layer 2.
- [Palaces explanation](./palaces.md) — layer 3.
- [Extraction explanation](./extraction.md) — how layer 2 becomes layer 3.
- [Workstreams explanation](./workstreams.md) — the container layer 3 lives inside.

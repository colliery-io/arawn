# Workstream Palaces

A **workstream palace** is the third and most curated layer of arawn's
local-first data system. Where [feeds](../feeds/index.md) give you raw
mirrored content and [projections](./projections.md) give you a
searchable normalized view of that content, a palace is the typed
*knowledge graph* the agent has built up about one specific thing you
track: a project, a person, a hobby, a campaign.

```
Feeds        →  raw bytes from upstream (Slack, Gmail, Drive, Jira, …)
Projections  →  per-feed-type normalized rows in a single sqlite db
Palaces      →  per-workstream graphqlite KB of typed entities + relations
```

The metaphor is the memory palace — locality over global graphs;
relationships before attributes; vivid extraction over flattened
records; continuous gentle curation; provenance walks. Each workstream
keeps its own palace so the agent can ask "what do I know about
*this*?" without spanning every concept you've ever mentioned.

## The three layers, made concrete

| Layer | Storage | Owns | Built by |
|---|---|---|---|
| **Feeds** | Files on disk under `~/.arawn/data/<provider>/<template>/<feed_id>/` | Fidelity — the bytes look like what the provider returned. | The feed runtime (a [cloacina](../workflows.md) cron job per feed) |
| **Projections** | `projections.db` (sqlite) with one table per feed type | Findability — cross-feed semantic + structured search. | The feed dispatcher writes a row per fetched item |
| **Palaces (workstream KBs)** | `~/.arawn/workstreams/<name>/memory.db` (sqlite + graphqlite extension) | Curated meaning — typed entities + relations, with provenance back to the projection rows that spawned them. | The per-workstream extractor on each new projection row + the steward on a cadence |

Each layer can be queried independently. The agent reaches for the
lowest layer that answers the question:

- "What did the email say verbatim?" → feeds (file read).
- "Find every message that mentions Postgres" → projections
  ([`feed_search`](../feeds/feed-search.md)).
- "What did we decide about the ledger service?" → palace
  ([`signal_search`](./agent-read-patterns.md), `signal_query`).

## What lives in a palace

Closed-set entity types (the same six from arawn-memory):

- **Decision** — a choice made, often with rationale.
- **Convention** — a rule or norm the workstream follows.
- **Fact** — a state-of-the-world claim.
- **Preference** — what someone prefers.
- **Person** — an individual mentioned across the workstream.
- **Note** — anything else worth keeping.

Closed-set relation types:

- `relates_to`, `supports`, `contradicts`, `supersedes`, `mentions`, `belongs_to`,
- plus two special ones: `extracted_from` (provenance back to the
  projection row) and `summarizes` (a dust summary pointing at the
  entities it compressed).

Per-workstream **tag ontology**: a closed list of slug tags the
extractor is allowed to attach to an entity's `tags_ontology` field.
This is what makes clustering ([`workstream_dust`](./steward.md),
[`signal_query`](./agent-read-patterns.md) tag filters) reliable.

Each entity also carries a `tags_discovered` field — free-form tags
the LLM emitted but that aren't in the ontology yet. Discovered tags
are the raw material from which the ontology grows.

[`extraction.md`](./extraction.md) covers the ontology / discovered
split in detail.

## Workstream lifecycle

```
/workstream create <name> --description "…"
        │
        │  Agent (via the workstream-create skill) walks you through
        │  description → propose ontology → confirm → finalize.
        ▼
WORKSTREAM EXISTS with declared ontology
        │
        │  /workstream bind <name> <feed_id>
        ▼
EXTRACTION runs over new projection rows from bound feeds.
For each row:
   classify scope → extract entities → link by name → write
        │
        ▼
STEWARD runs every pass over the resulting KB:
   • re-shelve   — merge near-duplicates
   • map         — propose new relations (manual accept)
   • door-watch  — propose cross-workstream identity (manual accept)
   • tag-promoter— propose vocab additions (manual accept)
   • dust        — manual trigger, summarize cold clusters
        │
        ▼
AGENT READS via signal_search / signal_query / signal_timeline.
ACTIONS journal'd: workstream_journal / workstream_refine /
                  workstream_apply / workstream_rollback.
```

## Key design decisions

Three ADRs anchor how palaces work. Read them when you need to know
*why* something is the shape it is:

- **[ADR-0002](#)** — Memory storage on graphqlite, schema enforced in
  Rust at the public API. FTS5 + vector indexes colocate in the same
  sqlite file as the graph tables.
- **[ADR-0003](#)** — Steward bounded blast radius. The four
  subroutines have an explicit verb allowlist; every action is
  journaled write-ahead with enough payload to undo.
- **[ADR-0004](#)** — Tag ontologies are required at workstream
  creation; vocabulary grows via the Extract→Suggest→Add cycle (the
  `tag-promoter` proposes, the user accepts via `workstream_apply`).

## When palaces make sense

| Use a palace when… | Skip the palace, stay in projections, when… |
|---|---|
| You want the agent to reason about *one thing*'s state over time | You want cross-feed semantic recall (use `feed_search`) |
| You need typed entities + relations (decisions, conventions, ...) | You just want to read the raw content |
| You want continuous curation (dedupe / summarization / vocabulary growth) | One-off lookup is fine |
| You'll run the agent against this workstream repeatedly | The data is too generic for a focused KB |

A workstream without a binding is harmless — extraction simply has
nothing to do. A workstream with a binding but no use case piles up
data the agent never references. Bind workstreams to the topics
you actually ask about.

## Related reading

- [Projections](./projections.md) — the middle layer; `feed_search`.
- [Extraction](./extraction.md) — how content turns into entities; the
  ontology / discovered tag split.
- [Steward](./steward.md) — the four subroutines + the journal /
  apply / rollback contract.
- [Agent read patterns](./agent-read-patterns.md) — `signal_search`,
  `signal_query`, `signal_timeline`, `workstream_dust`, and the
  refine / apply / rollback / tag flow.
- [Feeds](../feeds/index.md) — the layer below.
- [Memory](../memory.md) — the L1/L2 auto-memory system (separate from
  workstream palaces; lives in the global tier).

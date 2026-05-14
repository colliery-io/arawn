# Extraction

The extractor turns [projection rows](./projections.md) into typed
entities in a [workstream palace](./index.md). It's a 4-stage
chain-of-thought process that runs whenever new projection rows arrive
for a workstream that has bound the producing feed.

## The 4 stages

```
classify  →  extract  →  link-by-name  →  write
   │           │               │              │
   │           │               │              └─ Persist entities + relations
   │           │               │                 + EXTRACTED_FROM provenance
   │           │               │
   │           │               └─ Resolve "to_name: 'open question…'"
   │           │                  references to existing KB entities via FTS
   │           │
   │           └─ Pull typed entities out of the row's content
   │              (decisions, conventions, facts, notes, people).
   │
   └─ Is this row in scope for this workstream? Out-of-scope rows
      get skipped; the workstream's description gates the call.
```

Each stage is one LLM call. The chain reads the workstream's
**description** and **declared tag ontology** to scope decisions and
constrain tag emission. The whole flow is implemented in
`arawn_extractor::cot::CotChain`.

## The tag model (ADR-0004)

Every entity carries two tag fields:

- **`tags_ontology`** — closed list, drawn exclusively from the
  workstream's declared ontology. The extractor's prompt shows the
  current ontology and the LLM is told to use the exact strings.
  Rust-side filtering drops anything the LLM emits that isn't in the
  list. This is the substrate dust and `signal_query` cluster on.
- **`tags_discovered`** — free-form, LLM-emitted. Carries content the
  ontology hasn't absorbed yet. Searchable via `signal_search` for
  recall. Raw material for the [`tag-promoter`](./steward.md)
  steward subroutine.

### Why two fields

We tried free-form-only first. UAT showed it failed two ways:

- Without vocabulary pressure, the LLM minted variants
  (`falcon-project`, `falcon`, `falcon_project`) for the same concept
  across rows. Clustering broke.
- With "prefer existing tags" pressure, the LLM over-corrected:
  generic tags (`infrastructure`, `eng-org`) absorbed everything
  specific. Half of the dnd workstream had empty tags.

The hybrid is the recovery. The closed ontology gives clustering a
deterministic substrate. The free-form set keeps the LLM's recall
intact and provides growth signal for the
[Extract→Suggest→Add cycle](./steward.md#extract-suggest-add).

ADR-0004 has the full rationale.

## What `extract` returns per candidate

The LLM emits a JSON array; each item is:

```json
{
  "entity_type": "decision" | "convention" | "fact" | "preference" | "person" | "note",
  "title": "short title",
  "content": "optional longer text",
  "tags_ontology": ["postgres", "ledger"],
  "tags_discovered": ["rfc-0042", "jsonb"]
}
```

The Rust write step:

1. Filter `tags_ontology` against the workstream's declared list
   (lowercase + trim before lookup). Anything outside the list drops
   silently — the LLM doesn't get to invent ontology tags here.
2. Normalize `tags_discovered` (lowercase + trim) and drop empties.
3. Anchor freshness: `entity.created_at = entity.updated_at =
   row.source_ts`. The knowledge is as old as its source content,
   not as old as extraction. (Reinforcement on re-extract still
   resets `updated_at` to now via `MemoryStore::reinforce_entity`.)
4. `store_fact` writes via search-before-create dedup: existing
   entities with the same case-insensitive title get reinforced;
   new ones get inserted.

## `link-by-name`

After extracting candidates, the chain asks the LLM for relations
between them and any existing entities, by *name* not by id:

```json
[
  { "from": "use postgres", "rel": "supersedes",
    "to_name": "open question: which db?" }
]
```

Rust resolves `to_name` via FTS (`MemoryStore::search` against the
workstream tier first, then global). If a high-enough match exists,
the relation gets written. Unresolved links are dropped silently —
the LLM doesn't see UUIDs, only titles.

This is what makes "supersede this question with that decision" work
across multiple extraction passes without the chain having to know
internal ids.

## Provenance

Every written entity gets an `EXTRACTED_FROM` edge to a UUID derived
from the source projection row's id (Uuid v5, namespace OID). That
edge survives every steward subroutine — ADR-0003 makes provenance
removal non-negotiable.

Want to know where an entity came from? Walk the `EXTRACTED_FROM`
edge to its target UUID, reverse-lookup the projection row, read the
source bytes. The agent doesn't usually do this directly, but the
data path is intact.

## Cursor + idempotency

The extractor tracks a per-(workstream, feed_type) cursor in
`extractor_cursors`. Each pass walks `WHERE source_ts > cursor LIMIT
batch_size`, processes the batch, advances the cursor to the latest
processed row's `source_ts`. A crash mid-batch leaves the cursor
where it was — re-run picks up cleanly.

Two run modes:

- **Steady-state**: triggered by the feed dispatch hook after new
  projection rows land. Runs one batch (no loop).
- **Backfill**: triggered by `/workstream bind` when a workstream
  attaches to a feed that already has projection rows. Runs the
  batch loop until exhausted or a 10-minute wall-clock cap hits.
  Subsequent triggers resume cleanly via the cursor.

## Configuration

The extractor uses the engine LLM by default. To use a different
provider/model for extraction (e.g. a cheaper model since the chain
is high-volume), set:

```toml
[extraction]
llm = "haiku"   # name of an entry in [llm.<name>]
```

If unset, falls back to the engine LLM.

## See also

- [Steward](./steward.md) — the four subroutines that curate what the
  extractor produces.
- [Projections](./projections.md) — what the extractor reads.
- [Agent read patterns](./agent-read-patterns.md) — how the agent
  queries the resulting palace.
- [Memory](../memory.md) — the two-tier scope (global vs workstream)
  the entities land in.

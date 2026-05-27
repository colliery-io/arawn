# Palace types

*Reference. Entity types and relation types in a lens palace.*

A lens palace is a typed knowledge graph the extractor builds on top of projections. Every entity has a type from a closed set; every relation has a type from a closed set. This page catalogs both.

For the rationale ("why a palace, why a closed set"), see [palaces explanation](../explanation/palaces.md). For the agent-facing query tools, see [lens tools reference](./lens-tools.md). For the extractor's behavior, see [extraction explanation](../explanation/extraction.md).

Source: `crates/arawn-memory/src/types.rs`.

## Entity types

Closed set — every entity in a palace is one of these six:

| Type | Default scope | Use |
|---|---|---|
| `decision` | lens | A choice made, often with rationale. "We use Postgres 16 for the ledger service." |
| `convention` | lens | A rule or norm the lens follows. "Tests live inline in the same file as the code." |
| `fact` | lens | A state-of-the-world claim. "The config lives at `~/.arawn/arawn.toml`." |
| `preference` | global | What someone prefers. "Dylan prefers terse responses." |
| `person` | global | An individual mentioned across the lens. |
| `note` | lens | Anything else worth keeping. |

`preference` and `person` are **scope-locked to global** — they always live in `<data_dir>/memory.db`, never inside a lens palace. Everything else is lens-scoped.

## Relation types

Closed set — every edge between entities is one of these:

| Relation | Direction | Meaning |
|---|---|---|
| `relates_to` | undirected | Generic linkage. |
| `supports` | A → B | A is evidence for B. |
| `contradicts` | A ↔ B | A and B make incompatible claims. |
| `supersedes` | A → B | A replaces B (B is deprecated). |
| `mentions` | A → B | A's content references B. |
| `belongs_to` | A → B | A is part of B. |
| `extracted_from` | entity → projection | Provenance: this entity came from that projection row. **Never removed** — provenance is a one-way write. |
| `summarizes` | A → [B, C, …] | A is a dust summary that compresses cold material B, C, …. |

The `extracted_from` and `summarizes` edges are "special":

- `extracted_from` is per-entity, points at a stable UUID derived from `projection_id`. The agent (and the steward's rollback) uses it to trace entities back to source content. The steward is **forbidden** from removing these edges.
- `summarizes` is created by the dust subroutine when it compresses a cluster. Rollback of a dust summary uses DETACH DELETE to clean up the `SUMMARIZES` edges along with the summary entity.

## Tag ontology

Per-lens **tag ontology** is a closed list of slug tags the extractor is allowed to attach to an entity's `tags_ontology` field. The ontology is declared at lens creation (5-12 tags typical), and grows via the `tag-promoter` steward subroutine (see [steward subroutines reference](./steward-subroutines.md)).

Each entity also carries a `tags_discovered` field — free-form tags the LLM emitted but that aren't in the ontology yet. These are the raw material from which the ontology grows.

## Entity fields

Every palace entity has:

| Field | Type | Description |
|---|---|---|
| `id` | UUID | Stable id. |
| `entity_type` | enum | One of the six types above. |
| `title` | string | Short label (used by FTS). |
| `body` | string | Longer content. |
| `tags_ontology` | list&lt;string&gt; | Tags from the lens's closed ontology. |
| `tags_discovered` | list&lt;string&gt; | Free-form tags the LLM emitted; not in ontology yet. |
| `confidence` | enum | `stated` (1.0) / `observed` (0.7) / `inferred` (0.5). |
| `reinforcement_count` | int | Number of times the extractor has re-extracted this entity. Higher = more attested. |
| `created_at` / `updated_at` | RFC3339 | Timestamps. |

## Confidence

| Source | Base score | Meaning |
|---|---|---|
| `stated` | 1.0 | User explicitly said it. |
| `observed` | 0.7 | Inferred from behavior. |
| `inferred` | 0.5 | Extraction pipeline guessed it. |

Search results are ranked partly by confidence — a stated preference outranks an inferred one.

## Where palaces live

```
<data_dir>/lenses/<name>/memory.db
```

Each lens has its own SQLite database with FTS5 + sqlite-vec extensions + graphqlite for relations.

## Related

- [Palaces explanation](../explanation/palaces.md) — the palace metaphor, lifecycle, ADR pointers.
- [Extraction explanation](../explanation/extraction.md) — how content turns into entities.
- [Lens tools reference](./lens-tools.md) — `signal_*` query tools.
- [Steward subroutines reference](./steward-subroutines.md) — the curation loop.
- [Memory model reference](./memory-model.md) — the global-tier knowledge base sibling.

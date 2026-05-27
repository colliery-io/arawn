# Palaces

*Explanation. The palace metaphor, the lifecycle, the ADRs that anchor the design.*

A **lens palace** is arawn's third and most curated data layer: a typed knowledge graph the agent has built about one specific thing you track. Where [feeds](./feeds.md) give you raw mirrored content and [projections](./projections.md) give you a searchable normalized view, a palace gives you typed entities with relations — the *meaning* layer.

For the entity/relation catalog, see [palace types reference](../reference/palace-types.md). For the user-facing tools, see [lens tools reference](../reference/lens-tools.md). This page is about the design.

## The metaphor

The "memory palace" idea is ancient — orators imagined rooms in a building, each room holding one piece of an argument, and walked the rooms to recall their speech. The structure was physical (or imagined-physical); the meaning lived in the layout.

A lens palace borrows the principle:

- **Locality.** Each palace is per-lens. Knowledge about your "work" lens lives in a different room from your "home" lens. Queries scope to a palace.
- **Relationships before attributes.** Entities are nodes; relations are edges. A "decision" entity has a `supersedes` edge to the "open question" it replaced. The graph structure is the most interesting query target.
- **Vivid extraction over flattened records.** The extractor pulls *meaning* out of projection rows — "we decided X because Y" — not just normalized fields.
- **Continuous gentle curation.** The steward proposes maintenance (merge near-duplicates, add relations, summarize cold material). Bounded blast radius; journaled writes.
- **Provenance walks.** Every entity has an `EXTRACTED_FROM` edge to the projection row that spawned it. You can always walk the link.

Each lens keeps its own palace so the agent can ask "what do I know about *this*?" without spanning every concept you've ever mentioned.

## When a palace makes sense

| Use a palace when... | Skip the palace, stay in projections, when... |
|---|---|
| You want the agent to reason about *one thing*'s state over time | You want cross-feed semantic recall (use `feed_search`) |
| You need typed entities + relations (decisions, conventions, ...) | You just want to read the raw content |
| You want continuous curation (dedupe / summarization / vocabulary growth) | One-off lookup is fine |
| You'll run the agent against this lens repeatedly | The data is too generic for a focused KB |

A lens without a binding is harmless — extraction has nothing to do. A lens with a binding but no use case piles up data the agent never references. Bind lenses to topics you actually ask about.

## What lives in a palace

Closed-set entity types (same six as the global memory tier):

- **Decision** — a choice made, often with rationale.
- **Convention** — a rule or norm the lens follows.
- **Fact** — a state-of-the-world claim.
- **Preference** — what someone prefers. (Scope-locked to global; doesn't actually land in palaces.)
- **Person** — an individual. (Also scope-locked to global.)
- **Note** — anything else worth keeping.

Closed-set relation types: `relates_to`, `supports`, `contradicts`, `supersedes`, `mentions`, `belongs_to`, plus the two special ones: `extracted_from` (provenance) and `summarizes` (dust output).

Per-lens **tag ontology**: a closed list of slug tags the extractor is allowed to attach. This is what makes clustering — `lens_dust`, `signal_query` tag filters — reliable. Each entity also carries `tags_discovered` (free-form LLM tags outside the ontology), which is the raw material from which the ontology grows.

[Extraction explanation](./extraction.md) covers the ontology vs. discovered split in detail.

## Lens lifecycle

```
/lens create <name>
        │
        │  Agent (via the lens-create skill) walks you through:
        │  description → propose ontology → confirm → finalize.
        ▼
LENS EXISTS with declared ontology
        │
        │  /lens bind <name> <feed_id_or_uri>
        ▼
EXTRACTION runs over new projection rows from bound feeds.
For each row:
   classify scope → extract entities → link by name → write
        │
        ▼
STEWARD runs every pass over the resulting KB:
   • re-shelve   — merge near-duplicates
   • map         — propose new relations (manual accept)
   • door-watch  — propose cross-lens identity (manual accept)
   • tag-promoter— propose vocab additions (manual accept)
   • dust        — manual trigger, summarize cold clusters
        │
        ▼
AGENT READS via signal_search / signal_query / signal_timeline.
ACTIONS journal'd: lens_journal / lens_refine /
                  lens_apply / lens_rollback.
```

## Key design decisions

Three ADRs anchor how palaces work:

- **ADR-0002 — Memory storage on graphqlite.** Schema enforced in Rust at the public API; FTS5 + vector indexes colocate in the same SQLite file as the graph tables. One sqlite database per lens.
- **ADR-0003 — Steward bounded blast radius.** The four auto-pass subroutines have an explicit verb allowlist; every action is journaled write-ahead with enough payload to undo. Nothing changes the graph without being recoverable.
- **ADR-0004 — Tag ontologies are required at lens creation.** Vocabulary grows via the Extract→Suggest→Add cycle — `tag-promoter` proposes, the user accepts via `lens_apply`. Free-form-only failed in UAT (variant explosion); ontology-only over-corrected (generic tags absorbed everything specific). The hybrid (`tags_ontology` + `tags_discovered`) is the recovery.

## Why per-lens, not global

A global palace would force one ontology to span everything you track. That doesn't work:

- Your "work" lens's tags (`postgres`, `ledger`, `migration`) don't help in your "home" lens's queries.
- Cross-lens noise would degrade `signal_search` quality.
- Operational consequences: archiving a lens becomes "delete its sqlite file" instead of "graph surgery."

The trade-off: cross-lens identity (the same person mentioned in two lenses) becomes something the steward has to opt into via `doorwatch`. That's a per-lens concern, deliberately bounded.

## How curation works

The steward runs every lens's palace through five subroutines on a periodic pass:

| Subroutine | Mode | What it does |
|---|---|---|
| `reshelve` | mutating | Dedupe near-duplicate entities; merge content; mark erroneous ones for deletion. |
| `map` | proposal-only | Suggest new relations between entities. |
| `doorwatch` | proposal-only | Suggest cross-lens identity matches. |
| `tag-promoter` | proposal-only | Propose adding a recurring `tags_discovered` value to the ontology. |
| `dust` | manual-only | Summarize stale clusters into a single Note. |

The proposal-only subroutines write journal rows; you accept via `lens_apply`. `reshelve` is the only auto-mutating one — its journal payload carries the pre-state so rollback works.

[Steward explanation](./steward.md) goes deeper on the bounded-blast-radius rationale.

## Reading vs. writing

The agent reads palaces with `signal_*` tools. The agent never writes palace entities directly — only the extractor (writing from projection rows) and the steward (applying journaled proposals) write. This separation:

- Keeps the agent's tool surface simple (read-only on palaces).
- Makes provenance honest (`EXTRACTED_FROM` always points at real content the LLM didn't invent).
- Lets curation stay bounded (the steward's verbs are the only verbs).

You, the user, can refine via `lens_refine` / `lens_apply` — those are the user-facing curation surfaces. You don't bypass extraction by hand-writing entities; if you want to seed something, write it to global memory via `/remember`.

## Related

- [Palace types reference](../reference/palace-types.md).
- [Lens tools reference](../reference/lens-tools.md).
- [Steward subroutines reference](../reference/steward-subroutines.md).
- [Extraction explanation](./extraction.md).
- [Steward explanation](./steward.md).
- [Lenses explanation](./lenses.md).
- [Three-layer data model](./three-layer-data-model.md).

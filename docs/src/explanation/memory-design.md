# Memory design

*Explanation. Why two tiers (global + workstream), why some entity types are scope-locked, why FTS + vector instead of one or the other.*

arawn keeps a persistent knowledge base of facts, decisions, preferences, and people. The agent reads from and writes to it autonomously; you can store and inspect entries directly with `/remember`, `/memory`, `/forget`.

For the entity types and schema, see [memory model reference](../reference/memory-model.md). This page is about *why* memory is shaped the way it is.

## Two tiers

| Scope | Lives in | Holds |
|---|---|---|
| **Global** | `<data_dir>/memory.db` (+ `memory.graph.db`) | Things true across every workstream — preferences, important people, system-wide decisions. |
| **Workstream** | `<data_dir>/workstreams/<name>/memory.db` | Project-scoped facts, decisions, conventions, notes. |

Why split?

### 1. Locality

A single global KB would mix every concern. "What did we decide about X?" would return X-related decisions from every workstream you've ever touched — work, hobbies, household stuff. The agent's `memory_search` results would degrade with scope creep.

Per-workstream KBs scope queries naturally: searching while in workstream `work` returns work's decisions; switching to `home` returns home's. The agent doesn't have to filter by tag or date to keep things relevant.

### 2. Operability

Backing up, archiving, or wiping a workstream is `rm -rf <data_dir>/workstreams/<name>`. No surgery on a shared graph. The directory boundary is the workstream boundary.

### 3. Cost

Embedding all entities into one giant FAISS-style index would mean the index gets re-warmed on every server start; vector queries would span everything. Per-workstream sqlite-vec indexes keep the working set small per query.

## Why preferences and people are scope-locked to global

`preference` and `person` entities always go to global, regardless of which workstream you're in when you create them. Why?

### Preferences

A preference is about *you*: "I prefer terse responses," "I write tests inline with code," "I dislike `unwrap` in production code." These hold across every workstream. Locking them to global means:

- The agent applies them everywhere automatically. No need to re-state preferences when you switch to a new workstream.
- One canonical answer to "what does Dylan prefer?" — no per-workstream forks.

### People

A person is *one person*. "Alice the security lead" is one entity, not "Alice-in-work-workstream" and "Alice-in-personal-workstream." The cross-workstream identity case is what `doorwatch` proposes to handle (and currently only records, doesn't merge — see [steward explanation](./steward.md)). Locking `person` to global avoids the duplication problem in the first place.

The trade-off: a person mentioned in one workstream is also visible to queries in another. That's usually right (Alice is Alice). If you really need workstream-specific personhood, file under `note` instead.

## Why decisions, conventions, facts, notes are workstream-locked

They make sense only inside a specific workstream's context:

- A `decision` ("use Postgres 16 for the ledger") only matters where you'd ask about the ledger.
- A `convention` ("tests live inline with code") is project-specific.
- A `fact` ("the config lives at ~/.arawn/arawn.toml") could go global, but most are project-specific.
- A `note` is by definition free-form workstream content.

If a fact really is global ("the timezone of the office is UTC-7"), you can write it directly to global with `/remember` and the scope-router will accept it as a global-scoped fact in practice. But the *default* placement for `fact` and `note` is workstream.

## Why FTS5 + vector instead of one or the other

`memory_search` does both:

- **FTS5 keyword search** over entity titles and bodies. Always available. Returns ranked hits by `bm25` score.
- **Vector similarity** over entity content embeddings. Available only if the embedder model is installed at `<data_dir>/models/all-MiniLM-L6-v2/model.onnx`. Returns ranked hits by cosine similarity.

When both are available, each side scores hits separately (FTS uses `1/(1+bm25_rank)`, vector uses `1/(1+cosine_distance)`) and they're combined via a weighted linear sum: `composite = 0.4 × semantic + 0.3 × fts + 0.3 × confidence`. Why both?

- **FTS catches exact-term recall.** Searching for "Postgres" returns every entity that contains the word "Postgres." Cheap, deterministic, no model dependency.
- **Vectors catch semantic recall.** Searching for "the database we picked" returns the "Postgres 16" decision even if the entity text doesn't contain the word "database." This is what makes follow-up questions natural ("the framework I mentioned" → cloacina).
- **Each fails in different ways.** FTS misses paraphrases; vectors miss exact technical terms. Hybrid is best.

If the embedder isn't installed, search degrades silently to FTS-only. The agent still works; you just lose semantic recall.

## Why graphqlite for relations

Relations between entities (`supersedes`, `contradicts`, `mentions`, etc.) live in `memory.graph.db` — a graphqlite (Cypher-over-SQLite) database that colocates with the FTS-indexed entity table.

Why a graph DB instead of a relation table?

- **Traversal queries.** "What supersedes this decision?" is a graph query. SQL joins work for one hop; multi-hop traversals are awkward.
- **One file per workstream.** `memory.db` holds entities + FTS + vec; `memory.graph.db` holds the graph. Both colocate, both back up together.
- **Schema enforced in Rust.** graphqlite is permissive; the public API in `arawn-memory` enforces "only these entity types, only these relation types." The graphqlite layer is a tactical storage choice, not a contract.

## Why confidence is a closed set

Every entity carries a `ConfidenceSource`:

- `stated` (1.0) — user explicitly said it.
- `observed` (0.7) — inferred from behavior.
- `inferred` (0.5) — extraction pipeline guessed it.

Why three, not a free-form float?

- **The signal-to-noise ratio is low.** A free-form `0.81` confidence from an LLM is mostly noise; the LLM isn't a calibrated probability estimator.
- **Three buckets are coarse enough to be honest.** "User said it" / "We saw them do it" / "The LLM thinks so" are meaningfully different. A scalar would imply false precision.
- **Ranking benefits.** When `memory_search` returns ties, breaking on confidence (stated > observed > inferred) gives a stable order users can predict.

## How memory composes with palaces

Memory is the **L1**: small, fast-write, conversational. Palaces are **L2**: bigger, extracted from feed projections, curated by the steward.

- `memory_store` is what the agent calls during a turn when the user states a preference or makes an in-conversation decision. Lands in either global or the active workstream's `memory.db` based on entity type.
- The extractor writes to the **same** workstream `memory.db` — but its writes are constrained to the workstream's tag ontology (extractor entities can't invent ontology tags; user/agent memory writes can).

Both tiers share the same schema. From the agent's perspective, `memory_search` returns entities from either tier transparently.

## What memory ISN'T

- **A scratchpad for one-turn state.** The compactor handles turn-to-turn context. Memory is across-session knowledge.
- **A todo list.** Use `todo_*` tools and `/todo`. Todos are first-class with their own table.
- **A documentation system.** If you want to write internal docs about how your project works, write them in your repo. Memory is about *facts and preferences*, not narrative.

## Related

- [Memory model reference](../reference/memory-model.md) — the schema + tools.
- [Palaces explanation](./palaces.md) — the L2 / workstream-tier KB.
- [Extraction explanation](./extraction.md) — what writes palace entities.
- [Slash commands reference](../reference/slash-commands.md) — `/remember`, `/memory`, `/forget`.

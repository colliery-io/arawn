# Memory design

*Explanation. Why memory is one global store, how it differs from signals (the
lens-extracted stream), why FTS + vector instead of one or the other.*

arawn keeps a persistent **memory** — global statements of fact and behavioral
tuning. The agent reads from and writes to it autonomously; you can store and
inspect entries directly with `/remember`, `/memory`, `/forget`.

For the entity types and schema, see [memory model reference](../reference/memory-model.md).
This page is about *why* memory is shaped the way it is.

## Memory vs. signals

**Memory** and **signals** are different concerns, stored separately:

| | Memory | Signals |
|---|---|---|
| What it holds | Global statements of fact / behavioral tuning ("Pat Collins is someone I manage", "prefer terse responses", "we use Postgres 16") | Extracted activity from feeds — decisions, mentions, events, conventions surfaced by a lens's standing extractor |
| Where it lives | `<data_dir>/memory.db` | `<data_dir>/lenses/<slug>/memory.db` (one per lens) |
| Who writes it | You / the agent, deliberately (`/remember`, `memory_store`) | Each lens's extractor, continuously, from feed material |
| Who reads it | The chat (`memory_search`) and every lens's extractor (when classifying scope) | The chat (`signal_search` / `signal_query` / `signal_timeline`), cross-lens, with each hit labeled by source lens |
| Scope | Global. There is no lens dimension. | Per-lens by construction (a signal belongs to the lens that extracted it) |

The "Postgres 16" decision lives in memory if you say `/remember we use
Postgres 16 in production` — it's a standing fact. It lives as a signal in a
lens if the lens's extractor pulled it out of an architecture-review email.
These don't compete; they have different jobs.

## Why memory is global

A lens-scoped memory would force you to remember which lens a fact "belongs to"
before you could write it down. That's the wrong shape: facts and preferences
are about the user, not about a topic. "I prefer terse responses" doesn't
belong to a project. "Pat Collins is someone I manage" doesn't belong to a
project — it's a fact lenses *consume* (the people-I-manage lens uses it when
deciding scope, the team-health lens uses it when ranking signals, etc.).

A single global memory:

- **Lets you write a fact once.** Everything that needs it reads from the same
  store.
- **Lets lenses be memory-aware extractors.** When a lens classifies an
  incoming row, it consults memory for "who matters, what's a known project,
  what conventions apply." That's how the people-I-manage lens recognizes a
  1:1 transcript as in-scope even when the row body never says "manager."
- **Avoids the cross-lens identity problem.** A person mentioned in many lenses
  is *one person* in memory.

## Why FTS5 + vector

`memory_search` does both:

- **FTS5 keyword search** over entity titles and bodies. Always available.
  Returns ranked hits by `bm25` score.
- **Vector similarity** over entity content embeddings. Available only if the
  embedder model is installed at `<data_dir>/models/all-MiniLM-L6-v2/model.onnx`.
  Returns ranked hits by cosine similarity.

When both are available, each side scores hits separately
(`1/(1+bm25_rank)` and `1/(1+cosine_distance)`) and they're combined via
`composite = 0.4 × semantic + 0.3 × fts + 0.3 × confidence`. Why both?

- **FTS catches exact-term recall.** Searching for "Postgres" returns every
  entity that contains the word. Cheap, deterministic, no model dependency.
- **Vectors catch semantic recall.** Searching for "the database we picked"
  returns the "Postgres 16" memory even if the title doesn't contain
  "database." This is what makes follow-up questions natural.
- **Each fails in different ways.** FTS misses paraphrases; vectors miss exact
  technical terms. Hybrid is best.

If the embedder isn't installed, search degrades silently to FTS-only.

## Why graphqlite for relations

Relations between entities (`supersedes`, `contradicts`, `mentions`, etc.) live
in a separate graphqlite (Cypher-over-SQLite) database that colocates with the
FTS-indexed entity table.

- **Traversal queries.** "What supersedes this?" is a graph query. SQL joins
  work for one hop; multi-hop traversals are awkward.
- **Schema enforced in Rust.** graphqlite is permissive; `arawn-memory`
  enforces "only these entity types, only these relation types." The
  storage layer is a tactical choice, not a contract.

## Why confidence is a closed set

Every memory entity carries a `ConfidenceSource`:

- `stated` (1.0) — you explicitly said it.
- `observed` (0.7) — inferred from behavior.
- `inferred` (0.5) — the extraction pipeline guessed it.

Why three, not a free-form float?

- **Three buckets are coarse enough to be honest.** "User said it" / "We saw
  them do it" / "The LLM thinks so" are meaningfully different. A scalar from
  an LLM would be false precision.
- **Ranking benefits.** When ties happen, breaking on confidence (stated >
  observed > inferred) gives a stable, predictable order.

## How memory composes with signals

- **You write memory.** `/remember`, `memory_store`. These land in the global
  store. They're durable, intentional, and few.
- **Lenses extract signals.** Each lens's extractor reads incoming feed rows,
  consults memory for context, and writes typed entities to its own KB
  (`signal_search` reads from these, cross-lens).
- **Both feed the chat.** A turn that searches recalls memory and signals from
  their respective tools. Memory tells the agent who you are and what you
  prefer; signals tell it what's been happening.

## What memory ISN'T

- **A signal store.** Extracted activity belongs in lens palaces. If something
  is "what happened" rather than "what's true," it's a signal.
- **A scratchpad for one-turn state.** The compactor handles turn-to-turn
  context.
- **A todo list.** Use `todo_*` tools and `/todo`.
- **A documentation system.** Memory is about facts and preferences, not
  narrative — keep that in your repo.

## Related

- [Memory model reference](../reference/memory-model.md) — the schema + tools.
- [Lenses](./lenses.md) — what a lens is and how it consumes memory.
- [Palaces](./palaces.md) — where signals live.
- [Extraction](./extraction.md) — how signals get written.
- [Slash commands reference](../reference/slash-commands.md) — `/remember`, `/memory`, `/forget`.

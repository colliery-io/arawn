# Memory

Arawn keeps a persistent **knowledge base** of facts, decisions,
preferences, and people across sessions. The agent reads from and writes
to it autonomously during conversations; you can also store and inspect
entries directly (UI work-in-progress — see backlog T-0197).

## Two stores

| Scope | Lives in | Holds |
|---|---|---|
| **Global** | `<data_dir>/memory.db` | Things true across every workstream — your preferences, important people, system-wide decisions |
| **Workstream** | `<data_dir>/workstreams/<workstream>/memory.db` | Project-scoped facts, decisions, conventions, notes |

Some entity types are scope-locked: `Preference` and `Person` always go
to global; `Decision`, `Convention`, `Note`, and `Fact` always go to
workstream.

## What gets stored

### Entity types

| Type | Default scope | Use |
|---|---|---|
| `fact` | workstream | Things observed about the project — "the config lives at `~/.arawn/arawn.toml`" |
| `decision` | workstream | Resolved choices with rationale — "we use TOML not env vars for config" |
| `convention` | workstream | Project rules — "tests live inline in the same file as the code" |
| `preference` | global | User preferences — "Dylan prefers terse responses" |
| `person` | global | People — "Alice is the security lead" |
| `note` | workstream | Free-form notes |

### Relations

Entities can be linked: `relates_to`, `contradicts`, `supports`,
`supersedes`, `extracted_from`, `mentions`, `belongs_to`. The agent uses
these to navigate context — e.g. when retrieving a fact, it can also
surface things that contradict or supersede it.

### Confidence

Every entity carries a `ConfidenceSource`:

| Source | Base score | Meaning |
|---|---|---|
| `stated` | 1.0 | User explicitly said it |
| `observed` | 0.7 | Inferred from behavior |
| `inferred` | 0.5 | Extraction pipeline guessed it |

Search results are ranked partly by confidence — a stated preference
beats an inferred one.

## How it's retrieved

Two paths:

1. **FTS keyword search** — SQLite full-text index on entity titles and
   bodies. Always available.
2. **Vector similarity** — sentence embeddings of entity content.
   Available only if an embedding model is loaded.

The agent's `memory_search` tool uses both when both are available, then
merges and re-ranks. If the embedder isn't loaded, search silently
degrades to FTS-only — semantic matches ("the framework I mentioned
yesterday" → matching "cloacina") stop working, but exact-term recall
still does.

> **Heads-up**: arawn currently logs `embedding model unavailable —
> memory system will use FTS only` to the server log if the embedder
> can't load. A user-facing TUI banner is on the backlog (T-0197).

### Embedding model

The default embedder is `all-MiniLM-L6-v2` loaded via ONNX. The model
file is **not bundled** with the binary — it's expected at
`~/.arawn/models/all-MiniLM-L6-v2/model.onnx`. We'll automate the
download in a future release; for now you'll need to fetch it yourself
or accept FTS-only mode.

## How the agent uses it

Two tools are registered for the LLM:

- **`memory_store`** — write a fact. The agent calls this when the user
  states a preference, when a decision is made in conversation, or when
  it derives something worth remembering. Output is the stored entity ID.
- **`memory_search`** — retrieve relevant entities. The agent calls this
  at the start of a conversation to load context, and during a turn when
  it needs background ("did we already decide about X?"). Returns ranked
  entities with confidence scores.

A typical flow:

> **User**: "From now on, prefer Tokio over async-std."
>
> *Agent calls `memory_store({ "type": "preference", "title": "Prefer Tokio over async-std", "source": "stated" })`*
>
> *Next session, agent calls `memory_search({"query": "async runtime preference"})` and gets the preference back before suggesting code.*

## Direct access (work-in-progress)

UI for direct user interaction is partial today:

- `/remember <text>` — would store a user-supplied fact. Currently a stub
  in the TUI command list (see T-0195 + T-0197).
- `/memory` — would list recent entries. Same status.
- `/forget <id>` — same status.

Until those land, you can read the SQLite databases directly with
`sqlite3 <data_dir>/memory.db` if you need to.

## Storage layout

```
<data_dir>/
├── memory.db                              # global KB
├── memory.graph.db                        # global KB graph (relations)
└── workstreams/
    └── <workstream>/
        └── memory.db                      # workstream KB
```

`*-shm` and `*-wal` files alongside each `.db` are SQLite's
write-ahead-log files; safe to ignore but don't delete while arawn is
running.

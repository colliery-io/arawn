# Memory model

*Reference. Entity types + relations + confidence + storage layout for the global knowledge base.*

The memory model is the **global-tier** knowledge base — the things true across every lens. Per-lens palaces are the lens-tier version, sharing the same entity/relation vocabulary; see [palace types reference](./palace-types.md). For the rationale (why two tiers, what's scope-locked), see [memory design explanation](../explanation/memory-design.md).

Source: `crates/arawn-memory/`.

## Two stores

| Scope | Lives in | Holds |
|---|---|---|
| **Global** | `<data_dir>/memory.db` (+ `memory.graph.db`) | Things true across every lens — preferences, important people, system-wide decisions. |
| **Lens** | `<data_dir>/lenses/<name>/memory.db` | Project-scoped facts, decisions, conventions, notes. |

Some entity types are scope-locked: `preference` and `person` always go to global; `decision`, `convention`, `note`, and `fact` always go to lens.

## Entity types

Same closed set as palace entities:

| Type | Default scope | Use |
|---|---|---|
| `fact` | lens | Observed facts — "config lives at `~/.arawn/arawn.toml`" |
| `decision` | lens | Resolved choices with rationale — "we use TOML, not env vars, for config" |
| `convention` | lens | Project rules — "tests live inline" |
| `preference` | global | User preferences — "prefers terse responses" |
| `person` | global | People — "Alice is the security lead" |
| `note` | lens | Free-form notes |

See [palace types reference](./palace-types.md) for entity fields and confidence levels.

## Relations

Same closed set as palace relations (see [palace types reference](./palace-types.md)): `relates_to`, `contradicts`, `supports`, `supersedes`, `extracted_from`, `mentions`, `belongs_to`, `summarizes`.

The agent uses these to navigate context — e.g. when retrieving a fact, it can surface things that contradict or supersede it.

## Retrieval

Two paths:

1. **FTS keyword search** — SQLite full-text index on entity titles and bodies. Always available.
2. **Vector similarity** — sentence embeddings of entity content. Available only if the embedder model is loaded.

The `memory_search` agent tool scores each side and combines via a weighted linear sum — `composite = 0.4 × semantic + 0.3 × fts + 0.3 × confidence` (see `crates/arawn-engine/src/tools/memory_search.rs`). If the embedder isn't loaded, search silently degrades to FTS-only — semantic matches ("the framework I mentioned yesterday" → "cloacina") stop working, but exact-term recall still does.

### Embedding model

The default embedder is `all-MiniLM-L6-v2` loaded via ONNX. The model file is **not bundled** with the binary — it's expected at:

```
<data_dir>/models/all-MiniLM-L6-v2/model.onnx
```

Automatic install is on the roadmap. Without it, memory + projections degrade to FTS-only.

## Agent surface

| Tool | Description |
|---|---|
| `memory_store` | Write a fact. Agent calls this when the user states a preference, when a decision is made in conversation, or when it derives something worth remembering. |
| `memory_search` | Hybrid FTS + vector search. Returns ranked entities with confidence. |

Slash commands:

| Command | Description |
|---|---|
| `/remember <text>` | Store a user-supplied fact (scope-routes the same way as `memory_store`). |
| `/memory` | Show a summary of the active scope's knowledge base. |
| `/forget <entity>` | Remove an entity by short code or exact name. |

## Storage layout

```
<data_dir>/
├── memory.db                              # global KB (entities + FTS index)
├── memory.graph.db                        # global KB graph relations (graphqlite)
└── lenses/
    └── <lens>/
        └── memory.db                      # lens KB (entities, graph, FTS, vec)
```

`*-shm` and `*-wal` files alongside each `.db` are SQLite's write-ahead-log files; safe to ignore but don't delete while arawn is running.

## A typical flow

The agent's autonomous write/read pattern:

```
User: "From now on, prefer Tokio over async-std."

Agent calls memory_store({ "type": "preference",
                           "title": "Prefer Tokio over async-std",
                           "source": "stated" })
    → preference written to <data_dir>/memory.db (scope-locked global)

Next session, agent calls memory_search({ "query": "async runtime preference" })
    → preference returned with confidence=stated, score≈1.0
    → agent uses it to inform code suggestions
```

## Related

- [Memory design explanation](../explanation/memory-design.md) — two-tier rationale, FTS-vs-vector trade-off.
- [Palace types reference](./palace-types.md) — the lens-tier equivalent.
- [Slash commands reference](./slash-commands.md) — `/remember`, `/memory`, `/forget`.
- [Agent tools reference](./agent-tools.md) — `memory_store`, `memory_search`.

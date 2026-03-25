# Setting Up Memory and Knowledge

This tutorial shows you how to configure Arawn's memory system so it remembers
facts across sessions, builds a knowledge graph of entities and relationships,
and recalls relevant context automatically during conversations.

**Time:** 20 minutes
**Prerequisites:** A working Arawn installation ([Getting Started](getting-started.md))

---

## How memory works

Arawn's memory system has three layers that work together:

1. **Fact store** -- individual pieces of information stored in SQLite with
   vector embeddings for semantic search.
2. **Knowledge graph** -- entities (people, projects, concepts) and the
   relationships between them, stored in a separate graph database.
3. **Recall engine** -- a hybrid search that blends vector similarity, graph
   context, and confidence scoring to find the most relevant memories.

When you have a conversation, Arawn can automatically extract facts and entities
at session close. When you start a new conversation, the recall engine searches
memory for context relevant to your query and injects it into the prompt.

## Step 1: Review the default memory configuration

Memory is enabled by default. Check what you have:

```bash
arawn config show
```

Look for the `[memory]` section. With defaults, you should see something like:

```
[memory]
  database = "memory.db"

[memory.recall]
  enabled = true
  threshold = 0.6
  limit = 5

[embedding]
  provider = "local"
  dimensions = 384
```

The key settings:

| Setting | Default | Meaning |
|---------|---------|---------|
| `memory.recall.enabled` | `true` | Recall is active during conversations |
| `memory.recall.threshold` | `0.6` | Minimum score to include a memory (0.0--1.0) |
| `memory.recall.limit` | `5` | Maximum memories injected per turn |
| `embedding.provider` | `"local"` | Uses local ONNX model (offline, 384 dimensions) |

If you are happy with these defaults, you can skip to Step 3. Otherwise,
continue to Step 2 to customize.

## Step 2: Configure embedding and indexing

Open (or create) your configuration file:

```bash
${EDITOR:-nano} ~/.config/arawn/config.toml
```

### Embedding provider

The default local provider uses the `all-MiniLM-L6-v2` ONNX model. It runs
offline and produces 384-dimensional embeddings. No API key required.

If you prefer higher-dimensional embeddings from OpenAI (1536 dimensions),
switch the provider:

```toml
[embedding]
provider = "openai"

[embedding.openai]
model = "text-embedding-3-small"
dimensions = 1536
# API key read from OPENAI_API_KEY env var automatically
```

For this tutorial, we will stick with the local provider. No changes needed.

### Session indexing

Session indexing automatically extracts facts and entities when a session ends.
It uses an LLM to analyze the conversation and store what it learned.

Add or verify these settings:

```toml
[memory.indexing]
enabled = true
backend = "openai"           # LLM backend for extraction
model = "gpt-4o-mini"        # A smaller model works well for extraction
```

> **Note:** Indexing uses a separate LLM call. `gpt-4o-mini` is recommended
> because extraction is a structured task that does not need a large model, and
> it keeps costs low.

If you do not want automatic indexing (you prefer to store facts manually), set
`enabled = false`. You can always store facts manually regardless of this
setting.

### Confidence tuning

The confidence model controls how memories age:

```toml
[memory.confidence]
fresh_days = 30.0            # Memories are "fresh" for 30 days (no decay)
staleness_days = 365.0       # Linear decay over a year
staleness_floor = 0.3        # Never drop below 30% confidence
reinforcement_cap = 1.5      # Max boost from repeated reinforcement
```

The formula is: `score = base * reinforcement * staleness`

- A stated fact starts at 1.0 and stays there for 30 days.
- After 30 days, it decays linearly toward 0.3 over the next 335 days.
- Each time the same fact is reinforced, the score gets a 10% boost (capped
  at 1.5x).
- Superseded facts always score 0.0.

Leave these at defaults unless you have a specific reason to change them.

## Step 3: Store a fact manually

Start a chat session and tell Arawn something it should remember:

```bash
arawn chat
```

```
arawn> My preferred programming language is Rust and I work at Acme Corp.

I've noted that! Your preferred programming language is Rust and you work
at Acme Corp. I'll remember this for future conversations.
```

The agent uses the `memory_search` and `think` tools internally to persist this.
You can also store facts through the REST API:

```bash
curl -X POST http://localhost:8080/api/v1/memory \
  -H "Content-Type: application/json" \
  -d '{"content": "My preferred editor is Helix", "content_type": "preference", "source": "stated"}'
```

Or simply tell the agent in a chat session -- it will store the fact automatically.

## Step 4: Search memory

Use semantic search to find stored facts:

```bash
arawn memory search "what programming language do I use"
```

Expected output:

```
Results (1 match):

  [0.87] My preferred programming language is Rust and I work at Acme Corp.
         Source: stated | Created: 2 minutes ago | Confidence: 1.00
```

The number in brackets is the similarity score. Try different queries to see how
semantic search works -- it matches meaning, not keywords:

```bash
arawn memory search "where do I work"
```

```
Results (1 match):

  [0.82] My preferred programming language is Rust and I work at Acme Corp.
         Source: stated | Created: 3 minutes ago | Confidence: 1.00
```

Even though the query says "where do I work" and the stored fact says "work at
Acme Corp", the semantic search finds it because the meanings are related.

## Step 5: See recall in action

Now start a new session and ask a question that should trigger recall:

```bash
arawn chat
```

```
arawn> Can you recommend a good project template for me?

Based on what I know about you -- you prefer Rust and use Helix as your
editor -- I'd recommend starting with `cargo init` for a new project.
For a template with more structure, try `cargo generate` with the
`rust-github/template` template...
```

Notice that the agent recalled your language preference and editor choice without
you mentioning them. The recall engine searched memory using your query,
found relevant facts above the 0.6 threshold, and included them in the context.

## Step 6: Check memory statistics

Get an overview of what is stored:

```bash
arawn memory stats
```

Expected output:

```
Memory Statistics
─────────────────
Total memories:     3
  Stated:           2
  Observed:         1
  Inferred:         0
  Superseded:       0

Graph entities:     2
Graph relationships: 1

Embedding dimensions: 384
Vector index size:    3 entries
```

## Step 7: Explore the knowledge graph

When session indexing runs (or when you store facts that mention entities), Arawn
extracts named entities and relationships into a knowledge graph.

From the facts we stored, the indexer would have extracted:

- **Entities:** "Rust" (Concept), "Acme Corp" (Organization), "Helix" (Tool)
- **Relationships:** You -- `prefers` -- Rust, You -- `works_at` -- Acme Corp,
  You -- `uses` -- Helix

The graph powers the "graph expansion" phase of recall. When you search for
"Rust", the graph finds that you also prefer Helix and work at Acme Corp, which
boosts those related memories in the results.

The recall engine uses a blended score combining text similarity, graph
connectivity, and memory confidence to rank results. For details on how scoring
works, see [Memory & Knowledge Graph](../explanation/memory-and-knowledge.md).

## Step 8: Observe contradiction detection

Store a fact that contradicts an earlier one via the API:

```bash
curl -X POST http://localhost:8080/api/v1/memory \
  -H "Content-Type: application/json" \
  -d '{"content": "My preferred programming language is Go", "content_type": "preference", "source": "stated"}'
```

Now search for your language preference:

```bash
arawn memory search "preferred programming language"
```

Expected output:

```
Results (1 match):

  [0.91] My preferred programming language is Go
         Source: stated | Created: just now | Confidence: 1.00
```

The old "preferred programming language is Rust" fact has been superseded.
Arawn detected that both facts share the same subject ("my preferred programming
language") and replaced the old one. Superseded facts score 0.0 and are
effectively invisible to recall.

Let us fix that by storing the correct fact via the API:

```bash
curl -X POST http://localhost:8080/api/v1/memory \
  -H "Content-Type: application/json" \
  -d '{"content": "My preferred programming language is Rust", "content_type": "preference", "source": "stated"}'
```

## Step 9: Tune recall for your needs

If you find recall too aggressive (injecting irrelevant memories) or too quiet
(missing things it should find), adjust the threshold and limit.

Edit `~/.config/arawn/config.toml`:

```toml
[memory.recall]
enabled = true
threshold = 0.7    # Raise threshold to be more selective (default: 0.6)
limit = 3          # Fewer memories per turn (default: 5)
```

Guidelines:

| Threshold | Behavior |
|-----------|----------|
| 0.5 | Broad -- includes loosely related memories |
| 0.6 | Balanced -- default, good for most use cases |
| 0.7 | Selective -- only closely related memories |
| 0.8 | Strict -- very high relevance required |

A higher limit is useful for research-heavy workflows where you want maximum
context. A lower limit keeps the prompt lean and reduces token usage.

## Step 10: Verify the full pipeline

Run through this end-to-end test to confirm everything works:

```bash
# Store a fact via the API
curl -X POST http://localhost:8080/api/v1/memory \
  -H "Content-Type: application/json" \
  -d '{"content": "The Arawn project uses SQLite for storage", "content_type": "fact", "source": "stated"}'

# Search for it
arawn memory search "what database does Arawn use"

# Check stats
arawn memory stats

# Start a chat and see recall
arawn chat
```

In the chat, ask: "What do you know about me and my tools?" The agent should
recall your stored preferences and use them in its response.

---

## What you learned

- How Arawn's three-layer memory system works: fact store, knowledge graph,
  and recall engine
- How to configure embedding providers (local ONNX vs. OpenAI)
- How to enable session indexing for automatic fact extraction
- How to store and search memories via the CLI
- How recall injects relevant context into new conversations
- How the blended scoring formula ranks memories
- How contradiction detection supersedes outdated facts
- How to tune recall threshold and limit for your workflow

## Next steps

- [Building Your First Workflow](first-workflow.md) -- automate tasks with
  pipelines
- [Memory Guidelines](../reference/memory-guidelines.md) -- best practices for
  what to store and when
- [Configuration Reference](../reference/configuration.md) -- full memory and
  embedding config options

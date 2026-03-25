# Memory & Knowledge Graph

Most AI chat interfaces are stateless. You close the window and everything is gone. Arawn takes a different approach: conversations produce lasting knowledge. Facts, entities, and relationships are extracted, scored, and stored so that future conversations benefit from everything the agent has learned.

This page explains how persistent memory works, why it is designed the way it is, and how the various components -- vector search, knowledge graph, confidence scoring, staleness detection -- combine into a coherent recall system.

## The Problem Memory Solves

Without memory, every conversation starts from zero. You tell the agent that your project uses Rust, that the database is SQLite, that the deployment target is ARM64 -- and next time, you tell it all again. The agent has no continuity.

Memory gives the agent continuity. When you say "how should I optimize the database queries?", the agent recalls that your project uses SQLite with WAL mode, that you care about single-binary deployment, and that you previously discussed indexing strategies. The response is grounded in your actual context rather than generic advice.

## Storage Architecture

The memory system combines three storage backends in a single `MemoryStore`:

**SQLite** (via `rusqlite`) stores the memories themselves: content, metadata, confidence scores, timestamps, and supersession chains. SQLite was chosen because it aligns with Arawn's zero-dependency philosophy and provides ACID transactions for the complex contradiction detection logic.

**sqlite-vec** extends SQLite with vector similarity search through a virtual table (vec0). When you store a memory, its embedding vector is stored alongside it. When you query, the vec0 table finds the nearest neighbors by distance. This runs entirely in-process -- no separate vector database needed.

**graphqlite** provides a knowledge graph on top of SQLite, supporting entity storage and neighbor traversal. Entities (people, projects, concepts, technologies) and relationships (works_on, depends_on, created_by) are stored here and queried during recall to enrich results with structural context.

### Why Not a Dedicated Vector Database?

Vector databases like Pinecone, Weaviate, or Qdrant are optimized for scale -- millions of vectors, distributed search, real-time updates. Arawn's memory store holds thousands to tens of thousands of memories for a single user. At this scale, SQLite with the vec0 extension is fast enough (sub-100ms for 100 memories) and eliminates an external dependency.

The trade-off is that if you had millions of memories, performance would degrade. For a personal agent, that day is far away.

## Memory Types

Every memory has a content type that describes what it represents:

| Type | Description | Typical Source |
|------|-------------|----------------|
| `Note` | User-created notes via the `note` tool | Direct user action |
| `Fact` | Extracted factual claims | Session indexing |
| `UserMessage` | Things the user said | Conversation history |
| `AssistantMessage` | Things the agent said | Conversation history |
| `Thought` | Agent reasoning traces | Think tool output |

The content type matters during recall because you can filter by type. If you want only facts, you do not need to wade through conversation messages.

## Embedding

Before a memory can participate in vector similarity search, it needs a vector representation. Arawn supports two embedding backends:

**Local ONNX** generates 384-dimensional embeddings entirely offline using a pre-loaded ONNX model. This is the default because it requires no API key, no network connection, and no per-request cost. The trade-off is that 384 dimensions capture less semantic nuance than larger models.

**OpenAI embeddings** generate 1536-dimensional vectors via the OpenAI API. These capture more semantic detail but require an API key and network access. The choice between local and remote is a configuration decision.

The embedding dimensionality is stored in the vec0 table schema, so switching backends requires re-embedding existing memories. This is by design -- mixing embeddings from different models in the same vector space would produce meaningless similarity scores.

## The Recall Flow

Recall is the core retrieval operation. It runs at the start of every agent turn to inject relevant context before the LLM call. Here is what happens step by step:

### Step 1: Embed the Query

The user's message is converted to an embedding vector using the same model that embedded the stored memories. This ensures the query and the stored vectors live in the same semantic space.

### Step 2: Vector Similarity Search

The query embedding is compared against all stored embeddings via the vec0 virtual table. The search returns 2x the requested limit of candidates, sorted by distance. Fetching extra candidates allows for filtering without falling below the target count.

### Step 3: Filter

Each candidate is checked against:
- **Session ID**: If the query specifies a session, only memories from that session pass.
- **Time range**: Today, this week, this month, or all time.
- **Superseded flag**: Memories that have been superseded by newer, contradicting facts are excluded.
- **Content type**: Optional filter for specific memory types.

### Step 4: Graph Enrichment

For each surviving candidate, the system queries the knowledge graph for related entities via `GraphStore.get_neighbors()`. If a memory about "Rust async" is linked to entities "tokio" and "async-std", those entities are included in the result. This enrichment gives the agent structural context beyond raw text similarity.

### Step 5: Blended Scoring

Each memory receives a composite score from three signals:

**With graph context available:**
```
score = similarity * 0.4 + graph_score * 0.3 + confidence * 0.3
```

**Without graph context:**
```
score = similarity * 0.6 + confidence * 0.4
```

The weights reflect a design judgment: semantic similarity is the strongest signal, but confidence (how trustworthy is this memory?) and graph connectivity (how central is this memory in the knowledge graph?) provide meaningful corrections. A memory with high similarity but low confidence (inferred, old, not reinforced) should rank below a slightly less similar memory that the user explicitly stated recently.

The graph score is computed as `min(neighbor_count, 5) / 5` -- a memory connected to five entities scores 1.0, while an isolated memory scores 0.0. The cap at 5 prevents well-connected memories from overwhelming similarity.

### Step 6: Staleness Check

Each memory's citation is checked for freshness:
- **File citations**: The cited file's mtime is compared to the stored mtime. If the file has changed, the memory is marked as potentially stale.
- **Web citations**: A 7-day age threshold applies. Web content fetched more than a week ago may be outdated.
- **Session and user citations**: These do not go stale.

Staleness is informational -- it is included in the recall result so the agent can caveat its response ("this information may be outdated"), but stale memories are not automatically excluded.

### Step 7: Sort, Filter, Return

Results are sorted by composite score (highest first). A `min_score` threshold can exclude low-scoring noise. The top N results are returned to the agent.

## Confidence Scoring

Every memory has a confidence score that represents how trustworthy it is. The score is computed from three factors:

### Base Confidence

Depends on the memory's source:

| Source | Base Score | Rationale |
|--------|-----------|-----------|
| Stated | 1.0 | The user explicitly said it -- highest trust |
| System | 0.9 | System-derived data is usually reliable |
| Observed | 0.7 | Inferred from behavior -- reasonable but uncertain |
| Inferred | 0.5 | Logical deduction -- plausible but unverified |

### Reinforcement

When the same fact is encountered again, its reinforcement count increases. Each reinforcement adds 0.1 to the multiplier, capped at 1.5:

```
reinforcement_multiplier = min(1.0 + 0.1 * count, 1.5)
```

A fact stated once has a multiplier of 1.0. A fact reinforced 5 times has 1.5. This means a frequently confirmed fact scores up to 50% higher than a one-time mention.

### Staleness Decay

Memories lose confidence over time. The decay is linear from full confidence (within 30 days) to a floor (at 365 days):

- **Fresh** (< 30 days): No decay.
- **Decaying** (30-365 days): Linear interpolation to the staleness floor.
- **Floor** (> 365 days): Confidence bottoms out at 0.3.

The floor at 0.3 is deliberate. A year-old fact is less trustworthy but not worthless. "David's favorite language is Rust" may still be true after a year; it should rank lower than recent information but not disappear entirely.

### Superseded Memories

When a new fact contradicts an existing fact with the same subject and predicate, the old fact is marked as superseded and its confidence drops to 0.0. It is effectively invisible to recall while remaining in the database for audit purposes. The `superseded_by` field links the old fact to its replacement.

## Knowledge Graph

Beyond flat memory storage, Arawn builds a knowledge graph of entities and relationships.

### Entity Extraction

When a session closes, the indexing pipeline extracts entities using one of two strategies:

**GLiNER (local NER)**: A vendored Rust implementation of the GLiNER model that runs ONNX inference locally. It identifies named entities (people, projects, organizations, technologies) in the conversation text without any API call. When GLiNER is available, the LLM is only used for fact extraction, reducing API costs.

**LLM-based extraction**: When GLiNER is not available, the LLM performs full extraction of entities, facts, and relationships in a single call. This is more expensive but requires no local model.

### Why a Graph?

Consider a conversation where you discuss three projects that all depend on tokio. In flat memory, these are three separate facts. In a graph, there is a single "tokio" entity node with "depends_on" edges from each project. When you later ask about tokio, the recall system finds not just memories that mention tokio by text similarity, but also all projects connected to it via graph traversal.

The graph provides structural relationships that text similarity alone cannot capture. Two memories might use completely different words to describe the same relationship, but if they are linked through graph entities, the connection is preserved.

### Graph Storage

Entities and relationships are stored in graphqlite, which uses SQLite tables internally. Entity nodes have an ID, label (e.g., "Person", "Project"), name, and property map. Relationships connect two entities with a type (e.g., "works_on", "depends_on") and optional properties.

## Session Indexing

The bridge between ephemeral conversations and persistent memory is the session indexing pipeline, which runs asynchronously when a session closes.

### The Indexing Pipeline

1. **Extract**: NER (GLiNER or LLM) identifies entities and relationships in the conversation. The LLM extracts factual claims as structured data.

2. **Store entities**: Each entity is upserted into the knowledge graph. If the entity already exists, its properties are merged.

3. **Store facts**: Each fact is embedded and stored through contradiction detection:
   - **No existing match**: Insert as a new memory.
   - **Same subject, same content**: Reinforce (bump the reinforcement count).
   - **Same subject, different content**: Supersede the old fact and insert the new one.

4. **Store relationships**: Entity relationships are added to the graph.

5. **Summarize**: The LLM generates a summary of the entire session. This summary is embedded and stored as a memory, providing a high-level anchor point for future recall.

### Why Asynchronous?

Indexing involves embedding generation, LLM calls for extraction, and multiple database writes. Running it synchronously would block the session close response for several seconds. By spawning a background task (`tokio::spawn`), the user gets an immediate 204 response while indexing happens in the background.

## Active Recall

Every time the agent processes a user message, it performs "active recall": embedding the message, querying the memory store, and injecting relevant memories as a system message at position 1 in the conversation (after the bootstrap prompt but before conversation history).

This is invisible to the user. The agent simply has better context because it remembers relevant things from past conversations. The recalled memories appear as structured context that the LLM can reference in its response.

## Fallback: Text Search

When vector embeddings are not available (the vec0 extension failed to load, or no embeddings have been generated yet), the memory store falls back to SQL LIKE-based text search. This is less semantically powerful -- it finds exact substring matches rather than meaning-based similarity -- but it ensures the system degrades gracefully rather than failing entirely.

## Design Trade-offs

**Local vs. cloud embeddings**: Local ONNX embeddings (384-dim) are fast and free but less nuanced. OpenAI embeddings (1536-dim) are better quality but require network access and cost money. The system supports both, letting the user choose based on their priorities.

**Graph enrichment cost**: Graph traversal adds latency to every recall query. The implementation mitigates this by only querying neighbors (not full graph traversal) and capping the graph score component. For most use cases, the added latency is negligible compared to the subsequent LLM API call.

**Contradiction detection scope**: Contradictions are detected by matching on subject and predicate. This catches "David's language is Python" vs. "David's language is Rust" but not more subtle contradictions that require reasoning. Full logical contradiction detection would require an LLM call per fact storage, which is too expensive for the indexing pipeline.

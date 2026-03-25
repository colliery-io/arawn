# Sessions & Workstreams

Arawn organizes conversations at two levels: sessions (individual conversations) and workstreams (persistent contexts that span multiple sessions). This page explains why both exist, how they interact, and what design decisions shape their behavior.

## Why Two Levels?

A session is a single conversation. You open a chat, exchange messages, and close it. Sessions are short-lived and focused on immediate tasks.

But real work spans multiple conversations. A research project might involve dozens of sessions over weeks. Between sessions, you want the agent to remember what was discussed, what files were created, and what conclusions were reached. That continuity is what workstreams provide.

Without workstreams, each session would be isolated. The agent would forget everything between conversations, and you would need to re-establish context every time. Without sessions within workstreams, there would be no boundary between distinct conversations -- everything would blur into a single unbounded stream.

The two-level model gives you both: session-level boundaries for individual interactions and workstream-level continuity for long-running projects.

## Sessions

A session represents a single conversation with the agent. It has a unique ID, a sequence of turns (user messages, agent responses, tool calls), and a lifecycle.

### Session Lifecycle

```
create -> turns -> close -> index
```

1. **Create**: A session is created automatically when the first message arrives (or explicitly via the API). The session gets a unique ID and is associated with a workstream.

2. **Turns**: Each turn consists of a user message, memory recall, LLM inference, optional tool execution, and an agent response. Turns are appended to the session history.

3. **Close**: When the session ends (explicitly via DELETE, WebSocket disconnect, or TTL expiration), it is removed from the active session store.

4. **Index**: After closing, a background task extracts facts, entities, and relationships from the conversation and stores them in the memory system. This is how ephemeral session content becomes persistent knowledge.

### Session Cache

Active sessions are held in an in-memory LRU cache backed by `arawn-session`. The cache provides:

- **LRU eviction**: When the cache reaches capacity (configurable, default 10,000 sessions), the least recently used session is evicted. The eviction callback can trigger persistence or cleanup.

- **TTL expiration**: Sessions have a configurable time-to-live. Expired sessions are cleaned up periodically. Each access (get_or_load) resets the TTL timer.

- **Consolidated get_or_load**: A single write lock covers the check-load-insert sequence. This prevents the race condition where two concurrent requests both miss the cache and both load the same session from persistence, wasting work and potentially creating inconsistencies.

- **Persistence hooks**: The cache is generic over a `PersistenceHook` trait. The hook defines how sessions are loaded from disk (`load`), saved to disk (`save`), and cleaned up on eviction (`on_evict`). The workstream system implements this trait to manage JSONL message files.

### Why LRU?

An agent server might handle many sessions simultaneously, but only a fraction are actively in use at any moment. LRU eviction keeps the most active sessions in memory while allowing the total session count to exceed what fits in RAM. The evicted sessions are not lost -- they can be reloaded from persistence on the next access.

### Session Persistence

Session messages are persisted as JSONL (JSON Lines) files within the workstream directory structure. Each message is a single JSON object on one line, including role, content, timestamp, and metadata. JSONL was chosen over a database table because:

- It is append-only, which is efficient for the write pattern (new messages are always appended).
- It is human-readable for debugging.
- It survives partial writes (a crash mid-write corrupts at most the last line).
- It does not require schema migrations.

## Workstreams

A workstream is a persistent context that spans multiple sessions. It provides a named workspace with its own filesystem boundary, message history, and metadata.

### Scratch vs. Named Workstreams

Arawn supports two kinds of workstreams with different isolation models:

**Scratch workstreams** are ephemeral. When you start a chat without specifying a workstream, you get a scratch session. The filesystem boundary is scoped to a session-specific directory: `scratch/sessions/<session-id>/work/`. When the session ends, the scratch directory can be cleaned up. Scratch workstreams are for quick, throwaway interactions that do not need to persist.

**Named workstreams** are persistent. They have a name, a `production/` directory for outputs, and a `work/` directory for scratch space. All sessions within the same named workstream share these directories. This means the agent can write a file in one session and read it in the next. Named workstreams are for projects that span multiple conversations.

### Why the Distinction?

The scratch/named split exists because the security model needs to scope filesystem access, and the appropriate scope depends on the use case.

For scratch sessions, per-session isolation prevents one session's tool calls from interfering with another's. If two scratch sessions run concurrently, they cannot read or write each other's files.

For named workstreams, cross-session access is the whole point. Session 2 needs to see the files session 1 created. The `production/` directory is where finished artifacts live; the `work/` directory is where in-progress files are staged.

### Workspace Isolation

Each workstream has its own filesystem boundary enforced by the `FsGate`. This isolation is not advisory -- it is enforced at the path validation level (Layer 4 of the security model) and the OS sandbox level (Layer 6). A tool executing in workstream A physically cannot access workstream B's files.

This isolation matters because a multi-workstream user might have a "personal research" workstream and a "client project" workstream. Files from the client project should never leak into personal research context, even if the agent hallucinates a path.

### Promoting Scratch to Named

Sometimes a scratch session turns into something worth keeping. Arawn supports "promoting" a scratch workstream to a named workstream. This moves the scratch session's files to a named workstream's `production/` directory and establishes the named workstream for future sessions.

This workflow reflects how real work evolves: you start with a quick experiment, realize it is going somewhere, and decide to make it persistent. The system supports this transition without losing work.

## Session Ownership

WebSocket-connected sessions have an ownership model to prevent conflicts when multiple clients observe the same session.

### The Subscribe Model

When a WebSocket client connects to a session, it subscribes. The first subscriber becomes the **owner** and can send messages. Additional subscribers are **read-only** -- they receive streaming events (text, tool starts, tool completions) but cannot send messages.

This prevents the corruption that would result from two clients simultaneously sending messages to the same agent turn. The agent loop is sequential; interleaving messages from different clients would produce incoherent conversations.

### Reconnect Tokens

Network connections drop. When a WebSocket disconnects, the session does not immediately lose its owner. Instead, there is a 30-second grace period during which the original client can reconnect and reclaim ownership using a reconnect token.

Without this grace period, a brief network hiccup would cause the user to lose ownership. Another client could subscribe during the gap and become the owner. The reconnect token prevents this race: the original client gets 30 seconds to come back before ownership is released.

### Why Not Multiple Writers?

Supporting multiple writers would require conflict resolution: what happens when two users send a message at the same time? Who gets to respond first? How are tool calls from different contexts interleaved? These are solvable problems, but they add complexity that a personal agent does not need. The single-writer model is simpler and correct for the primary use case.

## Disk Management

Workstreams accumulate data over time: JSONL message files, files created by tool execution, production outputs. Without management, disk usage would grow unbounded.

Arawn provides per-workstream disk limits and monitoring:

- **Usage tracking**: The `DirectoryManager` can report the total size of a workstream's directories.
- **Cleanup policies**: Configurable limits on message history size. When a workstream exceeds its limit, older message files can be archived or summarized.
- **Compression**: JSONL files can be compressed to reduce storage overhead for long-running workstreams.

### Context Window Management

Large workstreams may have hundreds or thousands of messages. Loading all of them into an LLM context window is impractical (and impossible if the total exceeds the model's context limit).

When loading a workstream session, Arawn applies context window management:
1. Keep the most recent messages verbatim (they have the most immediate relevance).
2. Summarize older messages (compress history without losing key information).
3. Include all tool results from recent turns (tool output is often the most information-dense part of a conversation).

This strategy balances context quality (recent messages are exact) with context breadth (older messages are summarized but present).

## Integration Between Sessions and Memory

Sessions and memory are connected through the indexing pipeline:

1. **During a session**: The agent uses active recall to retrieve relevant memories before each turn. Memories are injected as context, improving response quality.

2. **When a session closes**: The indexing pipeline extracts facts, entities, and relationships from the session's message history. These become new memories.

3. **In future sessions**: The new memories are available for recall, creating a feedback loop where each conversation makes subsequent conversations better.

This cycle -- conversation produces facts, facts improve conversations -- is the mechanism by which Arawn becomes more useful over time. The session is the unit of interaction; the memory system is the unit of persistence; and the indexing pipeline is the bridge between them.

## Design Trade-offs

**JSONL vs. database for message storage**: JSONL is simple and append-only but makes queries (e.g., "find all sessions that mentioned X") expensive. A database would enable richer queries but add complexity and migration burden. For a personal agent where message queries are rare, JSONL's simplicity wins.

**LRU cache size**: The default of 10,000 sessions is generous for a single user. A larger cache uses more memory but reduces disk I/O from reloading sessions. A smaller cache conserves memory but increases latency for session access. The value is configurable because the right answer depends on the user's hardware and usage patterns.

**Single-writer sessions**: This simplifies the ownership model but prevents collaborative real-time editing. If Arawn ever needed to support teams, the session ownership model would need to evolve toward operational transforms or CRDTs. For now, simplicity is the right trade-off.

**Workstream-level vs. session-level memory**: Memories are stored globally, not scoped to a workstream. A fact learned in workstream A is available for recall in workstream B. This is deliberate -- knowledge about your preferences, your tech stack, and your conventions is useful across all contexts. The trade-off is that project-specific knowledge (e.g., internal API details) also leaks across workstreams. A future enhancement could add workstream-scoped recall filtering, but the current global model reflects the assumption that for a personal agent, all knowledge is relevant everywhere.

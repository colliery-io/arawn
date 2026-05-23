---
id: agentic-foundation-query-engine
level: initiative
title: "Agentic Foundation — Query Engine, Tools, and Workspace/Session Core"
short_code: "ARAWN-I-0001"
created_at: 2026-03-31T16:37:28.546614+00:00
updated_at: 2026-03-31T22:33:54.296488+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: agentic-foundation-query-engine
---

# Agentic Foundation — Query Engine, Tools, and Workspace/Session Core Initiative

## Context

Clean-slate rebuild of Arawn, using Claude Code's architecture as the reference model for how to structure an agentic coding assistant. The previous attempt grew too large without testing and never stabilized. This time we start with the minimum viable foundation: the agentic loop, a tool system, and workspace/session management.

Key architectural insight from Claude Code: the entire system is built around a **query engine** that implements a streaming tool-call loop (`prompt → LLM → tool_use → execute → feed result → loop`). Everything else — chat, commands, agents, scheduled work — is built on top of that core loop.

The critical divergence from Claude Code: Arawn needs **workstreams and sessions as first-class domain concepts** woven through the system. Claude Code is stateless per invocation (CWD = workspace, each run = new session). Arawn's tools, permissions, and persistence are all scoped to a workstream/session context.

### Reference Material
- `01-overview.md` through `06-c4-diagrams.md` — Claude Code architecture analysis
- `claude-code/` — Claude Code source for reference
- `.metis/vision.md` — Arawn vision document

## Goals & Non-Goals

**Goals:**
- Establish the workspace Cargo layout with 4 crates (`arawn`, `arawn-core`, `arawn-llm`, `arawn-engine`)
- Implement the domain model: workstreams, sessions, messages
- Build a trait-based LLM client abstraction with Groq as the first provider
- Implement the agentic query engine loop (streaming tool-call loop)
- Build a tool trait + registry with at least basic tools (think, shell, file read)
- Wire workspace/session context through the engine so tools are scoped
- All code tested from day one

**Non-Goals:**
- TUI (separate initiative — this is headless/library-only)
- Watchers, schedulers, action items
- Persistence layer (SQLite/graphqlite) — use in-memory for now
- MCP support, plugin system
- Multi-agent orchestration
- Web UI

## Architecture

### Crate Structure

```
arawn/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── arawn-core/               # Domain model, workspace/session management
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── workstream.rs     # Workstream type, filesystem paths
│   │       ├── session.rs        # Session lifecycle, message history
│   │       ├── message.rs        # Message types (user, assistant, tool_use, tool_result)
│   │       └── error.rs
│   │
│   ├── arawn-llm/                # LLM client abstraction
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # LLM client trait
│   │       ├── anthropic.rs      # Anthropic API implementation
│   │       ├── types.rs          # Request/response types, streaming
│   │       └── error.rs
│   │
│   ├── arawn-engine/             # Agentic loop, tool system
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── query_engine.rs   # Core loop: prompt → LLM → tool_use → execute → loop
│   │       ├── tool.rs           # Tool trait, ToolRegistry
│   │       ├── context.rs        # Engine context (session + workstream ref)
│   │       ├── permission.rs     # Permission checking (stub for now)
│   │       └── error.rs
│   │
│   └── arawn/                    # Binary crate (minimal for now)
│       └── src/
│           └── main.rs           # CLI entrypoint, wires crates together
```

### Data Flow (mirrors Claude Code's agentic loop)

```
User Input
  → Engine creates prompt with session context + tool definitions
    → LLM streams response
      → Engine detects tool_use blocks
        → Permission check (workstream-scoped)
          → Tool executes (sandboxed to workstream filesystem)
            → tool_result fed back to LLM
              → Loop until final text response (no tool_use)
                → Response stored in session, returned to caller
```

### Key Traits

```rust
// arawn-llm
#[async_trait]
trait LlmClient: Send + Sync {
    async fn stream(&self, request: ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;
}

// arawn-engine
#[async_trait]
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, ctx: &ToolContext, params: serde_json::Value) -> Result<ToolOutput>;
}
```

## Detailed Design

### arawn-core
Owns the domain types. The core relationship is strict: **one session belongs to exactly one workstream, immutably bound at creation**. There is no mid-session workstream switching — changing workstreams means changing sessions.

- A `Workstream` knows its root directory, name, and metadata. It owns zero or more sessions.
- A `Session` holds an ordered list of `Message`s and an immutable `workstream_id`. The session's `ToolContext` is derived from its workstream and never changes during the session lifetime.
- A **scratch space** is a default workstream that always exists for ad-hoc sessions. Sessions can be promoted from scratch to a real workstream later.
- Messages use an enum matching the Anthropic API shape: `User`, `Assistant` (with optional `tool_use` blocks), `ToolResult`.

This means the engine never needs to handle workstream context swaps — the workstream binding is set once and never changes.

However, the **tool set is mutable during a session**. The `ToolRegistry` supports hot-reloading: tools can be registered or unregistered at runtime, and the engine queries available tools fresh each turn. This enables:
- **Token efficiency** — only include tool definitions relevant to the current task in the prompt
- **Long-running automation** — dynamically load tools as a workflow progresses (e.g., add summarization tool after data collection phase)
- **Safety** — strip dangerous tools mid-session when entering restricted phases

The `ToolContext` holds the immutable session/workstream binding. The `ToolRegistry` is the live, shared, hot-swappable layer.

No persistence yet — sessions live in memory. The types are designed so a persistence layer can be added later without changing the engine.

### arawn-llm
Backend-agnostic LLM client layer. Defines an `LlmClient` trait that any provider implements. The engine codes against the trait, never a concrete provider.

- `LlmClient` trait: `stream(ChatRequest) → Stream<ChatChunk>` — the only interface the engine knows
- `ChatRequest` / `ChatChunk` / `ToolUse` / `ToolResult` — provider-neutral types that map to the common subset of chat+tool-use APIs
- First implementation: **Groq** (fast, cheap, good for iteration)
- Future implementations: Anthropic (Claude), OpenAI, local models, etc.

Each provider crate/module handles its own API key resolution, SSE parsing, and type mapping to/from the neutral types. No retry logic in v1 — keep it simple.

### arawn-engine
The heart. `QueryEngine` takes an `LlmClient`, a `ToolRegistry`, and a session context. The `run` method:
1. Builds the prompt from session history + system context
2. Calls `LlmClient::stream`
3. Collects the streamed response
4. If response contains `tool_use` blocks: check permissions, execute each tool, append `tool_result` messages, goto 2
5. If response is pure text: append to session, return

`ToolRegistry` is a shared, hot-swappable container of `Tool` impls. Tools can be registered/unregistered at runtime. Each turn, the engine queries the registry for currently available tools to include in the prompt — so changes take effect on the next LLM call without session restart. `ToolContext` provides the executing tool with immutable workstream-scoped filesystem access and session metadata.

### Starter Tools
- **ThinkTool** — No-op reasoning scratchpad (like Claude Code's think tool)
- **ShellTool** — Execute a command, return stdout/stderr (workstream CWD)
- **FileReadTool** — Read a file within the workstream root

## Testing & Evaluation

Evaluation is wired in from day one. The trait-based LLM client enables deterministic end-to-end testing of the entire engine loop without hitting a real API.

### Mock LLM Client
A `MockLlmClient` that implements `LlmClient` with scripted responses. You define a sequence: "respond with these tool_use blocks, then after tool_results, respond with this text." This tests the full agentic loop deterministically.

### Test Harness (fixture-style)
A `TestHarness` builder that assembles the full stack:
- Creates a temp-dir workstream with pre-populated files
- Registers specific tools (real implementations)
- Wires in the mock LLM with a scripted conversation
- Runs the engine loop and captures results

Assertions cover: tool calls made (name, params), session history shape, filesystem side-effects, final response content.

### Layers
1. **Unit tests** — per-crate, inline (`#[cfg(test)]` modules). Test individual types, serialization, tool execution in isolation.
2. **Functional tests** — harness-driven, test the engine loop end-to-end with mock LLM + real tools + real filesystem (temp dirs).
3. **Live eval** (future) — hit Groq, assert on behavioral properties rather than exact output. Not part of this initiative.

## Alternatives Considered

1. **Single mega-crate** — Rejected. Clean boundaries between domain, LLM, and engine make testing easier and prevent circular dependencies. Lessons learned from previous attempt.
2. **Start with persistence** — Rejected. In-memory sessions first. Adding SQLite is a separate initiative once the core loop works.
3. **Start with TUI** — Rejected. The engine should be testable headless. TUI is a consumer of the engine, not part of it.
4. **Mirror Claude Code crate-for-crate** — Rejected. Their component model inspires ours but they have different constraints (TypeScript, no workspace management, IDE bridge concerns).

## Implementation Plan

Tasks will be decomposed after design approval. Rough ordering:
1. Workspace scaffolding (Cargo workspace, crate stubs, CI)
2. `arawn-core` domain types (workstream, session, message)
3. `arawn-llm` client (Anthropic streaming)
4. `arawn-engine` tool trait + registry + starter tools
5. `arawn-engine` query engine loop
6. Integration: wire binary crate to run a headless conversation
7. Test coverage for all of the above
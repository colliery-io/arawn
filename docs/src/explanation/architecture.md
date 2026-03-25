# Architecture Overview

This page explains the high-level architecture of Arawn: why it is structured the way it is, how the pieces fit together, and what design philosophy drove the decisions.

## Design Philosophy

Arawn is built around four principles that distinguish it from cloud-hosted AI assistants:

**Edge-first computing.** Arawn runs as a single binary on your machine. There is no cloud service to deploy, no infrastructure to manage, no account to create. You download a binary, run it, and everything works. This is a deliberate choice rooted in the belief that a personal AI research agent should be as personal as possible -- your data stays on your hardware, your conversations never leave your network, and the system works whether or not you have internet access.

**Embedded storage.** Rather than requiring PostgreSQL, Redis, or any external database, Arawn uses SQLite for all persistent storage. The memory database, knowledge graph, session metadata, and workstream state all live in SQLite files on disk. This eliminates an entire class of deployment and operational complexity. There is no database server to configure, no connection strings to manage, no separate backup strategy to maintain. When you back up Arawn, you copy a directory.

**Memory-centric design.** Unlike stateless chat interfaces that forget everything between sessions, Arawn maintains persistent memory. Facts learned in one conversation carry forward to the next. The agent builds a knowledge graph of entities and relationships over time. This accumulating context is what transforms a generic LLM into a personalized research assistant.

**Tool-native execution.** The LLM is an orchestrator, not the endpoint. Arawn gives the model access to tools -- shell execution, file operations, web search, memory storage -- and the model decides when and how to use them. The system is designed around this agentic loop rather than treating tool use as an afterthought.

## Why Rust?

Three properties of Rust make it the right choice for this project:

1. **Single binary distribution.** `cargo build --release` produces one self-contained executable. No runtime, no interpreter, no dependency installation. Users get a binary that works.

2. **Memory safety without garbage collection.** An agent that executes arbitrary tool calls, manages concurrent WebSocket connections, and runs background indexing tasks needs to be correct under concurrency. Rust's ownership model catches data races at compile time.

3. **Performance.** Local embedding generation (ONNX), vector similarity search, and NER extraction are computationally intensive. Rust's zero-cost abstractions mean these operations run at native speed without the overhead of a managed runtime.

## Crate Organization

Arawn is organized as a Cargo workspace with approximately 20 crates arranged in strict layers. The layering is not arbitrary -- it exists to enforce dependency rules at compile time and to keep build times manageable.

### Layer 0: Foundation

Crates with no internal dependencies. These are leaf nodes in the dependency graph.

| Crate | Purpose |
|-------|---------|
| `arawn-types` | Shared types: Message, Memory, FsGate trait, hook definitions |
| `arawn-config` | Configuration loading, TOML parsing, secret resolution |
| `arawn-oauth` | OAuth PKCE flow for Claude MAX authentication |
| `arawn-sandbox` | OS-level sandboxing (macOS sandbox-exec, Linux bubblewrap) |
| `arawn-script-sdk` | Pre-compiled SDK for WASM script execution |
| `arawn-session` | Session cache with LRU eviction and TTL |
| `arawn-mcp` | MCP (Model Context Protocol) client for external tool servers |
| `arawn-client` | HTTP/WebSocket client for connecting to Arawn servers |

### Layer 1: Services

Domain-specific functionality that depends only on foundation crates.

| Crate | Purpose |
|-------|---------|
| `arawn-llm` | LLM backends (Anthropic, OpenAI, Groq, Ollama), embedding generation |
| `arawn-memory` | Memory store with vector search, graph operations, confidence scoring |
| `arawn-pipeline` | Workflow execution via the Cloacina engine |
| `arawn-plugin` | Plugin system: skills, hooks, agents, CLI tools |

### Layer 2: Business Logic

Core agent functionality that combines services.

| Crate | Purpose |
|-------|---------|
| `arawn-agent` | Agentic loop, tool registry, context building, prompt assembly |
| `arawn-agent-indexing` | Session indexing: NER extraction, fact storage, summarization |
| `arawn-agent-tools` | Tool implementations: file, shell, search, delegate, memory, web |
| `arawn-workstream` | Persistent conversation contexts, filesystem isolation, JSONL storage |

### Layer 3: Orchestration

| Crate | Purpose |
|-------|---------|
| `arawn-domain` | Facade that orchestrates agent, session, memory, and MCP services |
| `arawn-tui` | Terminal UI built with Ratatui and Crossterm |

### Layer 4: Transport

| Crate | Purpose |
|-------|---------|
| `arawn-server` | Axum-based HTTP/WebSocket server, REST API, OpenAPI spec |

### Layer 5: Binary

| Crate | Purpose |
|-------|---------|
| `arawn` | CLI binary: commands, REPL, brings everything together |

### Why This Many Crates?

The crate structure serves three purposes:

**Compile-time dependency enforcement.** A crate in Layer 1 physically cannot import a crate from Layer 2 because Cargo will refuse to build a circular dependency. This makes architectural violations impossible, not just discouraged.

**Incremental compilation.** When you change a file in `arawn-agent-tools`, only that crate and its dependents need to recompile. The foundation crates, LLM backends, and memory store are untouched. In a single-crate project, every change would trigger a full rebuild.

**Clear ownership boundaries.** Each crate has a focused responsibility. `arawn-memory` owns storage. `arawn-sandbox` owns OS-level isolation. `arawn-plugin` owns extension discovery. When you need to understand or modify a subsystem, you know exactly where to look.

## Data Flow

The primary data flow through the system follows this path:

```
User message
  -> CLI/TUI or HTTP client
    -> arawn-server (Axum router)
      -> arawn-domain (service facade)
        -> arawn-agent (build context, enter tool loop)
          -> arawn-llm (send to LLM backend)
          <- LLM response (text or tool calls)
          -> arawn-agent-tools (execute tools if needed)
          -> arawn-memory (recall relevant memories)
        <- AgentResponse
      <- Domain response
    <- HTTP/WebSocket response
  <- Displayed to user
```

Several important things happen along this path:

1. **Memory recall** happens before the LLM call. The agent embeds the user's message, queries the memory store for semantically similar content, and injects relevant memories as a system message at position 1 in the conversation.

2. **The tool loop** is iterative. The LLM may respond with tool calls, which get executed and fed back as tool results. The LLM then decides whether to call more tools or respond with text. This loop continues until the model produces a text-only response or hits the iteration limit.

3. **Session indexing** happens asynchronously after a session closes. A background task extracts entities, facts, and relationships from the conversation and stores them in the memory system. This is how short-term conversation context becomes long-term memory.

## Why SQLite?

SQLite is used for all persistent storage in Arawn. This is worth explaining because it is an unusual choice for an application that does vector similarity search, graph traversal, and concurrent read/write access.

**Zero configuration.** SQLite is an embedded database. It requires no server process, no network configuration, no authentication setup. The database is a single file on disk. This aligns with Arawn's edge-first philosophy.

**WAL mode for concurrency.** SQLite's Write-Ahead Logging mode allows concurrent readers with a single writer. Since Arawn's write patterns are bursty (store memories after indexing, save session state) rather than continuous, WAL mode provides sufficient concurrency for a single-user agent.

**Extension ecosystem.** The `sqlite-vec` extension provides vector similarity search via virtual tables (vec0). The `graphqlite` crate provides graph operations on top of SQLite. Rather than requiring separate vector and graph databases, everything lives in one storage engine.

**Portability.** A SQLite database can be copied between machines, backed up with `cp`, and inspected with standard tooling. There are no binary log formats, no WAL segments to manage, no replication topology to understand.

The trade-off is that SQLite does not scale to millions of embeddings or thousands of concurrent users. For a personal agent running on a single machine, this trade-off is acceptable. If Arawn ever needed to scale beyond a single user, the `MemoryStore` interface is abstract enough to swap in a different backend.

## Concurrency Model

Arawn is an async Rust application built on Tokio. Understanding the concurrency model helps explain several architectural choices.

**The agent loop is sequential per session.** Within a single session, turns execute one at a time. There is no concurrent tool execution within a turn (though this could change in the future). This simplifies the conversation history management -- you never need to merge concurrent modifications to the message list.

**Sessions are concurrent across each other.** The server handles multiple simultaneous sessions via Tokio's task scheduler. Each session's agent turn runs as an independent async task. The session cache uses `RwLock` to allow concurrent reads with exclusive writes.

**Background tasks run independently.** Session indexing, plugin file watching, and TTL cleanup run as spawned Tokio tasks. They communicate with the main system through shared `Arc` references to stores and caches rather than through message passing channels. This is simpler than an actor model and sufficient for the single-user scenario.

**Blocking operations are isolated.** ONNX inference, sandbox execution, and some SQLite operations are blocking. These are run via `tokio::task::block_in_place` or `spawn_blocking` to avoid starving the async runtime. The `SandboxManager`, for example, uses `block_in_place` because the sandbox-runtime crate holds a `!Send` lock guard that prevents the future from being moved between threads.

## Key Architectural Decisions

**Trait-based abstraction for LLM backends.** The `LlmBackend` trait in `arawn-llm` abstracts over Anthropic, OpenAI, Groq, and Ollama. The agent loop does not know which backend it is using. This means switching models requires a configuration change, not a code change.

**The FsGate trait for security.** Rather than hardcoding filesystem access rules, Arawn defines a `FsGate` trait with `validate_read`, `validate_write`, `sandbox_execute`, and `working_dir` methods. The `WorkstreamFsGate` implementation scopes access to workstream boundaries. This abstraction means the tool execution pipeline does not need to understand workstream structure -- it just asks the gate whether an operation is allowed.

**Domain facade pattern.** The `arawn-domain` crate acts as a facade over the agent, session, memory, and MCP subsystems. The server layer talks to the domain; the domain coordinates the services. This prevents the server routes from becoming tangled orchestration logic and keeps the HTTP layer thin.

**Plugin compatibility with Claude Code.** The plugin system uses the `.claude-plugin/plugin.json` manifest format, skills stored as `SKILL.md` with YAML frontmatter, and agents as markdown files. This format compatibility is deliberate -- it means plugins written for Claude Code work with Arawn and vice versa, expanding the available plugin ecosystem.

**MCP for external tool servers.** The Model Context Protocol (MCP) client in `arawn-mcp` allows Arawn to connect to external tool servers that expose capabilities via a standardized protocol. Rather than building every possible tool into the binary, MCP lets you connect to specialized servers (database tools, API integrations, domain-specific utilities) that run as separate processes. The MCP client handles connection management, protocol negotiation, and tool discovery.

**Configuration cascade.** Arawn's configuration system in `arawn-config` merges settings from multiple sources: built-in defaults, system-level TOML files, project-level TOML files, environment variables, and command-line flags. Later sources override earlier ones. Secret resolution follows its own cascade: age-encrypted store, then OS keychain (via the `keyring` crate), then environment variables, then file-based fallback. This layering means you can set sensible defaults at the system level and override per-project without duplicating configuration.

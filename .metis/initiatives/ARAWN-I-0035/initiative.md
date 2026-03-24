---
id: decompose-start-rs-server
level: initiative
title: "Decompose start.rs Server Initialization"
short_code: "ARAWN-I-0035"
created_at: 2026-03-22T23:50:08.425554+00:00
updated_at: 2026-03-24T18:45:37.193402+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: decompose-start-rs-server
---

# Decompose start.rs Server Initialization

## Context

`arawn/src/commands/start.rs::run()` is a ~1,600-line async function that handles the entire server initialization sequence. It grew organically and now contains config loading, LLM backend resolution, memory store initialization, tool registry setup, plugin loading, MCP wiring, workstream management, session cache configuration, and server startup — all in a single function. This makes it hard to understand, test, and modify individual phases.

Promoted from blocked task ARAWN-T-0377.

## Goals & Non-Goals

**Goals:**
- Extract each initialization phase into a named function with clear inputs/outputs
- Each phase should be independently understandable (~100-200 lines max)
- Use a builder or accumulator pattern to pass state between phases
- Maintain identical runtime behavior — pure refactoring, no logic changes
- Make each phase individually testable

**Non-Goals:**
- Changing the initialization order or dependencies
- Adding new features or config options
- Restructuring into separate crates

## Detailed Design

Proposed function decomposition:
1. `load_config()` → `ArawnConfig` + `ConfigSource`
2. `init_llm_backends()` → `HashMap<String, SharedBackend>` + `SharedEmbedder`
3. `init_memory_store()` → `Option<Arc<MemoryStore>>`
4. `init_tools()` → `ToolRegistry`
5. `init_plugins()` → `Vec<PluginPrompt>` + `HookDispatcher` + agent configs
6. `init_mcp()` → `Option<McpManager>`
7. `init_pipeline()` → `Option<Arc<PipelineEngine>>`
8. `init_workstreams()` → `Option<WorkstreamManager>`
9. `init_server()` → `Server` (assembles AppState from all above)

Each function takes only what it needs and returns only what it produces. A `ServerInit` struct or sequential calls accumulate the results.

## Implementation Plan

- Task 1: Extract `load_config` + `init_llm_backends` (lowest coupling)
- Task 2: Extract `init_memory_store` + `init_tools`
- Task 3: Extract `init_plugins` + `init_mcp`
- Task 4: Extract `init_pipeline` + `init_workstreams` + `init_server`
- Task 5: Clean up — remove intermediate variables, add doc comments

Each task should be done in a worktree, verified with full test suite before merge.

## Deep Analysis: Initialization Phase Map

The `run()` function spans lines 143-1621. After a daemon-mode early return (lines 143-189), the non-daemon path has 15 distinct initialization phases. Below is the complete map.

### Phase 1: Daemon Fork (lines 143-189)
- **What:** If `--daemon` flag is set, re-exec the binary without `--daemon` in background, write PID file, and return early.
- **Inputs:** `args.daemon`, `args.port`, `args.bind`, `args.config`
- **Outputs:** Early return (no state produced for later phases)
- **Dependencies:** None
- **Coupling:** None — completely self-contained. Can be extracted trivially.

### Phase 2: Configuration Loading (lines 191-229)
- **What:** Loads config from an explicit path or via auto-discovery, prints warnings and verbose source info, validates config.
- **Inputs:** `args.config`, `args.workspace`, `ctx.verbose`
- **Outputs:** `config: &ArawnConfig` (the `loaded` binding)
- **Dependencies:** None
- **Coupling:** Low — `config` is consumed by every subsequent phase, but this phase itself has no upstream deps.

### Phase 3: LLM Backend Resolution (lines 236-289)
- **What:** Resolves the default LLM backend from config + CLI overrides, then resolves all named LLM profiles into a `HashMap<String, SharedBackend>`.
- **Inputs:** `config`, `args` (backend/model/api_key/base_url CLI overrides), `config.llm_profiles`, `config.oauth`
- **Outputs:** `backend: SharedBackend` (default), `backends: HashMap<String, SharedBackend>`, `resolved: ResolvedLlm`
- **Dependencies:** Phase 2 (config)
- **Coupling:** Medium — `backend` is consumed by Phase 11 (explore tool), Phase 12 (delegate tool), Phase 14 (agent builder). `backends` is consumed by Phase 15 (session indexer) and Phase 17 (session compressor). `resolved.model` is consumed by Phase 14.

### Phase 4: Server Settings Resolution (lines 291-337)
- **What:** Merges CLI args with config to determine bind address, port, workspace path, bootstrap dir, and auth token.
- **Inputs:** `args.port`, `args.bind`, `args.workspace`, `args.bootstrap_dir`, `args.token`, `config.server`
- **Outputs:** `addr: SocketAddr`, `workspace: Option<PathBuf>`, `bootstrap_dir: Option<PathBuf>`, `auth_token: Option<String>`
- **Dependencies:** Phase 2 (config)
- **Coupling:** Low — `addr` and `auth_token` feed Phase 16 (server config). `workspace` and `bootstrap_dir` feed Phase 14 (agent builder) and Phase 8 (plugin subscriptions).

### Phase 5: Embedder Initialization (lines 339-356)
- **What:** Builds the embedding spec from config and creates the embedder instance.
- **Inputs:** `config.embedding`
- **Outputs:** `embedder: Arc<dyn Embedder>`, `embedding_config`
- **Dependencies:** Phase 2 (config)
- **Coupling:** Medium — `embedder` is consumed by Phase 7 (memory store vector init), Phase 14 (agent builder), and Phase 15 (session indexer).

### Phase 6: Pipeline Engine Initialization (lines 358-410)
- **What:** Creates the pipeline engine (SQLite-backed task runner) if enabled in config.
- **Inputs:** `config.pipeline`, `data_dir`
- **Outputs:** `pipeline_engine: Option<Arc<PipelineEngine>>`, `pipeline_workflow_dir: PathBuf`, `_workflow_watcher_handle`
- **Dependencies:** Phase 2 (config)
- **Coupling:** Medium — `pipeline_engine` is consumed by Phase 9 (pipeline tool registration) and Phase 18 (graceful shutdown). `pipeline_workflow_dir` is consumed by Phase 9.

### Phase 7: Memory Store Initialization (lines 412-444)
- **What:** Opens the SQLite memory store, initializes the knowledge graph and vector extensions, runs WAL checkpoint.
- **Inputs:** `config.memory`, `data_dir`, `embedder.dimensions()`, `embedder.name()`
- **Outputs:** `memory_store: Option<Arc<MemoryStore>>`
- **Dependencies:** Phase 2 (config), Phase 5 (embedder — for dimensions/name)
- **Coupling:** High — `memory_store` is consumed by Phase 9 (memory search tool), Phase 14 (agent builder), Phase 15 (session indexer), and Phase 16 (AppState assembly).

### Phase 8: Tool Registry & Core Tools (lines 446-505)
- **What:** Creates the tool registry and registers all built-in tools (shell, file read/write, glob, grep, web fetch, web search, note, memory search) with configured timeouts and output limits.
- **Inputs:** `config.tools`, `memory_store`
- **Outputs:** `tool_registry: ToolRegistry` (mutable, extended by later phases)
- **Dependencies:** Phase 2 (config), Phase 7 (memory_store for MemorySearchTool)
- **Coupling:** Very High — `tool_registry` is a mutable accumulator consumed and extended by Phase 9 (pipeline tools), Phase 10 (MCP tools), Phase 11 (explore tool), Phase 12 (delegate tool), and finally consumed by Phase 14 (agent builder). This is the most heavily threaded piece of state.

### Phase 9: Pipeline Tool Registration (lines 507-740)
- **What:** If pipeline is enabled, loads runtime catalog, creates script executor, compiles built-in WASM runtimes, registers CatalogTool and WorkflowTool, loads existing workflow TOML files, starts hot-reload watcher.
- **Inputs:** `pipeline_engine`, `pipeline_workflow_dir`, `data_dir`, `tool_registry` (mutated)
- **Outputs:** Mutates `tool_registry` (adds CatalogTool, WorkflowTool). Side-effects: `_workflow_watcher_handle` kept alive.
- **Dependencies:** Phase 6 (pipeline_engine), Phase 8 (tool_registry)
- **Coupling:** Medium — contained within the pipeline feature. The long length (~230 lines) is mostly error-handling boilerplate for catalog/executor fallbacks and workflow loading.

### Phase 10: Plugin System (lines 742-964)
- **What:** Discovers and loads plugins from configured directories, syncs git-subscribed plugins, registers hooks, collects agent configs and skill prompt fragments, optionally starts hot-reload watcher.
- **Inputs:** `config.plugins`, `workspace`, `ctx.verbose`
- **Outputs:** `plugin_prompts: Vec<(String, String)>`, `hook_dispatcher: HookDispatcher`, `plugin_agent_configs: HashMap`, `plugin_agent_sources: HashMap`, `_watcher_handle`
- **Dependencies:** Phase 2 (config), Phase 4 (workspace for subscription manager)
- **Coupling:** Medium — outputs feed Phase 11 (hook_dispatcher), Phase 12 (agent configs for delegate tool), Phase 14 (plugin_prompts, hook_dispatcher for agent builder).

### Phase 11: MCP Server Connection (lines 966-1091)
- **What:** Creates the MCP manager, converts config entries to McpServerConfig, connects to all configured servers, adapts discovered tools into the tool registry.
- **Inputs:** `config.mcp`, `tool_registry` (mutated)
- **Outputs:** `mcp_manager: Option<McpManager>`, mutates `tool_registry`
- **Dependencies:** Phase 2 (config), Phase 8 (tool_registry)
- **Coupling:** Medium — `mcp_manager` is consumed by Phase 16 (AppState) and Phase 18 (graceful shutdown).

### Phase 12: Hook Dispatcher + Explore Tool + Delegate Tool (lines 1093-1189)
- **What:** Wraps hook_dispatcher in Arc, creates RLM exploration tool, creates delegate tool for plugin subagents. The delegate tool requires cloning the entire tool registry into an Arc and then rebuilding a new mutable registry.
- **Inputs:** `hook_dispatcher`, `config.rlm`, `backend`, `tool_registry`, `plugin_agent_configs`, `plugin_agent_sources`, `config.agent`
- **Outputs:** `shared_hook_dispatcher: Option<Arc<HookDispatcher>>`, mutates `tool_registry` (adds ExploreTool, DelegateTool). Replaces `tool_registry` entirely if delegate tool is created.
- **Dependencies:** Phase 3 (backend), Phase 8 (tool_registry), Phase 10 (hook_dispatcher, plugin agent configs)
- **Coupling:** Very High — the delegate tool section (lines 1148-1189) is the most complex coupling point. It Arc-wraps the current registry, clones all tools into a new registry, and adds the delegate tool. This is the riskiest code to extract because the `tool_registry` binding is reassigned.

### Phase 13: System Prompt & Agent Builder (lines 1191-1330)
- **What:** Builds the system prompt, configures the Agent builder with backend, tools, prompts, model, max_iterations, hooks, workspace, bootstrap dir, prompt files, secret resolver, filesystem gate resolver, memory store, and embedder. Calls `builder.build()`.
- **Inputs:** `config.agent`, `backend`, `tool_registry`, `plugin_prompts`, `resolved.model`, `shared_hook_dispatcher`, `workspace`, `bootstrap_dir`, `args.prompt_file`, `memory_store`, `embedder`, `config.workstream` (for fs gate), `data_dir`
- **Outputs:** `agent: Agent`
- **Dependencies:** Phase 3 (backend, resolved), Phase 4 (workspace, bootstrap_dir), Phase 5 (embedder), Phase 7 (memory_store), Phase 8+9+11+12 (tool_registry fully assembled), Phase 10 (plugin_prompts), Phase 12 (shared_hook_dispatcher)
- **Coupling:** Very High — this is the convergence point where nearly all prior phases contribute. The filesystem gate resolver sub-block (lines 1270-1319) creates a SandboxManager and DirectoryManager inline, adding further complexity.

### Phase 14: Session Indexer (lines 1332-1451)
- **What:** Creates the session indexer for background session summarization, optionally with GLiNER NER engine.
- **Inputs:** `memory_cfg.indexing`, `memory_store`, `backends` (for indexing backend lookup), `embedder`
- **Outputs:** `indexer: Option<SessionIndexer>`
- **Dependencies:** Phase 3 (backends), Phase 5 (embedder), Phase 7 (memory_store)
- **Coupling:** Low — self-contained once its inputs are available.

### Phase 15: Server Assembly & Startup (lines 1453-1600)
- **What:** Creates ServerConfig, assembles AppState from agent + indexer + memory_store + hook_dispatcher + mcp_manager + workstream manager + session config + compressor. Creates Server and calls `server.run()`.
- **Inputs:** `config.server`, `auth_token`, `addr`, `agent`, `indexer`, `memory_store`, `shared_hook_dispatcher`, `mcp_manager`, `config.workstream`, `data_dir`, `config.session`, `backends`, `args.seed`
- **Outputs:** Running server (blocks until shutdown)
- **Dependencies:** All prior phases
- **Coupling:** Very High — this is the final assembly point.

### Phase 16: Graceful Shutdown (lines 1602-1621)
- **What:** After server stops, shuts down pipeline engine and MCP servers.
- **Inputs:** `pipeline_engine`, `mcp_manager`
- **Outputs:** None
- **Dependencies:** Phase 6 (pipeline_engine), Phase 11 (mcp_manager)

## Shared State Flow Analysis

### Arc-wrapped resources that cross phase boundaries:
| Resource | Created in | Consumed by |
|----------|-----------|-------------|
| `backend: SharedBackend (Arc)` | Phase 3 | Phases 12, 13, 14 |
| `backends: HashMap<String, SharedBackend>` | Phase 3 | Phases 14, 15 |
| `embedder: Arc<dyn Embedder>` | Phase 5 | Phases 7, 13, 14 |
| `memory_store: Option<Arc<MemoryStore>>` | Phase 7 | Phases 8, 13, 14, 15 |
| `pipeline_engine: Option<Arc<PipelineEngine>>` | Phase 6 | Phases 9, 16 |
| `shared_hook_dispatcher: Option<Arc<HookDispatcher>>` | Phase 12 | Phases 13, 15 |
| `mcp_manager: Option<McpManager>` | Phase 11 | Phases 15, 16 |
| `tool_registry: ToolRegistry` | Phase 8 | Phases 9, 11, 12, 13 (mutated across 4 phases) |

### The `tool_registry` problem:
`tool_registry` is the most problematic shared state. It is created in Phase 8, then mutated in Phases 9, 11, and 12, and finally consumed by Phase 13. In Phase 12, it is even replaced entirely (the binding is reassigned). Any extraction must handle this accumulator pattern explicitly — either via a builder/accumulator struct or by passing ownership through a chain.

### The `config` reference problem:
`config` is borrowed immutably (`&loaded.config`) and threaded through nearly every phase. This is clean for borrowing but means extracted functions all need either the full `&ArawnConfig` or specific sub-sections. The latter is cleaner but more verbose.

## Risk Assessment

### Riskiest extractions (tightest coupling):

1. **Phase 12 (Delegate Tool) — HIGHEST RISK:** Reassigns `tool_registry` by Arc-wrapping it, cloning all tools into a new registry, and adding the delegate tool. The ownership transfer is fragile. Must be extracted together with or after Phase 11 (MCP tools), since it needs the fully-assembled registry.

2. **Phase 13 (Agent Builder) — HIGH RISK:** Convergence point consuming outputs from 10+ prior phases. The inline filesystem-gate-resolver closure (lines 1270-1319) captures `data_dir`, `config.workstream`, and creates `SandboxManager` and `DirectoryManager` — these would need to be extracted first or passed in.

3. **Phase 15 (Server Assembly) — HIGH RISK:** Another convergence point. Includes inline workstream manager creation, session config application, and compressor setup. Several sub-blocks mutate `app_state` conditionally.

4. **Phase 9 (Pipeline Tools) — MEDIUM RISK:** Long (~230 lines) but mostly self-contained. The hot-reload watcher spawns a tokio task that captures `engine` and `factory` by move — the `_workflow_watcher_handle` must remain in scope for the server's lifetime.

5. **Phase 10 (Plugins) — MEDIUM RISK:** Long (~220 lines) but produces discrete outputs. The plugin watcher handle must survive.

### Safest extractions (loosest coupling):

1. **Phase 1 (Daemon Fork):** Completely self-contained, early-return path.
2. **Phase 2 (Config Loading):** Pure input, no prior-phase deps.
3. **Phase 4 (Server Settings):** Pure merge of CLI args + config.
4. **Phase 5 (Embedder Init):** Small, depends only on config.
5. **Phase 14 (Session Indexer):** Self-contained once inputs available.

## Proposed Extraction Order (safest to riskiest)

| Order | Phase(s) | Risk | Rationale |
|-------|----------|------|-----------|
| 1 | Phase 1: Daemon fork | Trivial | Already a self-contained early-return block |
| 2 | Phase 2: Config loading + validation | Low | No upstream deps; produces the foundational `config` |
| 3 | Phase 4: Server settings resolution | Low | Pure merge logic, no side effects |
| 4 | Phase 3: LLM backend resolution | Low | Depends only on config; helper functions already exist |
| 5 | Phase 5: Embedder init | Low | Small, depends only on config |
| 6 | Phase 7: Memory store init | Low | Depends on config + embedder, self-contained |
| 7 | Phase 14: Session indexer | Low | Depends on memory_store + backends + embedder, self-contained |
| 8 | Phase 6+9: Pipeline engine + tools | Medium | Extract together since Phase 9 depends tightly on Phase 6 outputs; keep watcher handle alive |
| 9 | Phase 10: Plugin system | Medium | Long but discrete outputs; extract hook_dispatcher wrapping (Phase 12 start) separately |
| 10 | Phase 11: MCP servers | Medium | Mutates tool_registry but in a contained way |
| 11 | Phase 8: Core tool registry | Medium | Must be extracted BEFORE Phase 12 because the delegate tool reassigns it |
| 12 | Phase 12: Explore + Delegate tools | High | Requires careful ownership transfer of tool_registry |
| 13 | Phase 13: Agent builder assembly | High | Convergence point; extract filesystem gate resolver as a sub-function first |
| 14 | Phase 15: Server assembly + startup | High | Final convergence; extract workstream init and compressor init as sub-functions |
| 15 | Phase 16: Graceful shutdown | Low | Small, but must be done last since it references variables from the full scope |

## Recommended Accumulator Pattern

Rather than passing 15+ variables between functions, introduce a `ServerInit` struct:

```
struct ServerInit {
    config: ArawnConfig,
    backends: HashMap<String, SharedBackend>,
    default_backend: SharedBackend,
    resolved_model: String,
    embedder: Arc<dyn Embedder>,
    addr: SocketAddr,
    auth_token: Option<String>,
    workspace: Option<PathBuf>,
    bootstrap_dir: Option<PathBuf>,
    memory_store: Option<Arc<MemoryStore>>,
    tool_registry: ToolRegistry,
    pipeline_engine: Option<Arc<PipelineEngine>>,
    plugin_prompts: Vec<(String, String)>,
    hook_dispatcher: Option<Arc<HookDispatcher>>,
    mcp_manager: Option<McpManager>,
    plugin_agent_configs: HashMap<String, PluginAgentConfig>,
    plugin_agent_sources: HashMap<String, String>,
    // Watcher handles (must survive for server lifetime)
    _workflow_watcher: Option<WatcherHandle>,
    _plugin_watcher: Option<WatcherHandle>,
}
```

Each extracted function takes `&mut ServerInit` (or the relevant subset via a smaller intermediate struct) and populates its fields. This avoids the "18 arguments" problem while keeping the data flow explicit.
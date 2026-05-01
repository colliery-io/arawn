---
id: multi-model-llm-configuration-per
level: initiative
title: "Multi-Model LLM Configuration — per-component provider routing and client wiring"
short_code: "ARAWN-I-0027"
created_at: 2026-04-16T12:42:57.212570+00:00
updated_at: 2026-04-17T02:46:19.737908+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: multi-model-llm-configuration-per
---

# Multi-Model LLM Configuration — per-component provider routing and client wiring Initiative

## Context

Arawn's `arawn.toml` already supports **named LLM configs** — a `[llm.*]` map where each entry defines a provider, model, API key, and limits. The `[engine]` and `[compactor]` sections reference these by name, and `config.rs` has `engine_llm()` / `compactor_llm()` resolution methods that fall back correctly.

**The problem:** the wiring in `main.rs` and `local_service.rs` doesn't honour this. Today, `build_llm_client()` is called once for the engine's config, and that single `Arc<dyn LlmClient>` is passed to both the engine and compactor (`local_service.rs:249`). The compactor config field is dead code.

Beyond the compactor, there's no mechanism for other components to use a different model at all — tools, MCP servers, and any future subsystems (judge, classifier, router) all inherit the engine's client or nothing.

### Current wiring (single client)

```
arawn.toml → config.engine_llm() → build_llm_client() → Arc<dyn LlmClient>
                                                              ↓
                                                         LocalService
                                                          ├─ engine  ← uses it
                                                          └─ compactor ← also uses it (ignores compactor_llm())
```

### Target wiring (per-component)

```
arawn.toml → LlmClientPool (name → Arc<dyn LlmClient>)
                 ├─ "default"  → engine
                 ├─ "cheap"    → compactor
                 ├─ "fast"     → tool-level summarization (future)
                 └─ "judge"    → eval/grading (future)
```

## Goals & Non-Goals

**Goals:**
- Wire `compactor_llm()` to actually build and use a separate client when configured
- Introduce a lightweight `LlmClientPool` (or similar) that lazily builds clients by name from config
- Pass the pool (or individual clients) to components that need them, not a single shared client
- Ensure backward compatibility: single `[llm.default]` config still works with zero changes
- Make it straightforward to add new per-component LLM slots in the future (e.g., `[tools.summarizer]`, `[judge]`)

- Tools and agents can declare an `LlmPreference` (provider, model, capabilities) in their definition
- At execution time, the runtime resolves the preference against the pool: exact match → capability match → fallback to engine LLM
- Fallback is always available — a missing preferred model is degraded service, not a hard failure
- Tools/agents can detect whether they got their preferred model or the fallback, so they can adjust behavior (e.g., skip a summarization step if only a small model is available)

**Non-Goals:**
- Runtime model switching / hot-reload of config (restart is fine)
- Token accounting or cost tracking across models
- Multi-model routing within a single conversation turn (e.g., mixture-of-agents)
- Automatic model selection / capability inference (tools declare what they want explicitly)

## Architecture

### LlmClientPool

A thin wrapper over `HashMap<String, Arc<dyn LlmClient>>` that:
1. Takes `&ArawnConfig` at construction
2. Lazily (or eagerly) builds clients for each named `[llm.*]` entry via `build_llm_client()`
3. Wraps each in `RetryClient`
4. Provides `fn get(&self, name: &str) -> Option<Arc<dyn LlmClient>>`
5. Provides `fn engine(&self) -> Arc<dyn LlmClient>` and `fn compactor(&self) -> Arc<dyn LlmClient>` convenience methods that resolve via config names with fallback

Lives in `crates/arawn/src/` (binary crate) since it depends on `build_llm_client` and config types.

### Wiring changes

- `main.rs`: Replace single `build_llm_client()` call with `LlmClientPool::from_config(&config)`
- `LocalService::new()`: Accept the pool (or two separate clients: engine + compactor)
- `LocalService::build_engine()`: Use `pool.compactor()` for `Compactor::new()` instead of `self.llm`
- `LocalService::shared_llm()`: Continues to return the engine client (tools see engine model)

### LlmPreference — tool/agent model requests

Tools and agents declare what kind of LLM they want via an `LlmPreference` struct:

```rust
/// What a tool or agent wants from an LLM.
pub struct LlmPreference {
    /// Specific named config from arawn.toml (e.g., "cheap", "judge")
    pub named: Option<String>,
    /// Preferred provider (e.g., "anthropic", "groq")
    pub provider: Option<String>,
    /// Preferred model (e.g., "claude-sonnet-4-20250514")
    pub model: Option<String>,
    /// Minimum capability requirements
    pub capabilities: LlmCapabilities,
}

pub struct LlmCapabilities {
    /// Minimum context window (tokens)
    pub min_context_window: Option<u32>,
    /// Needs tool/function calling support
    pub tool_use: bool,
    /// Needs vision/image input
    pub vision: bool,
}
```

### Resolution order

When a tool/agent requests an LLM via preference:

1. **Named match** — if `preference.named` is set and exists in the pool, use it
2. **Provider+model match** — scan pool entries for exact provider+model match
3. **Capability match** — scan pool entries for first that satisfies all capability requirements
4. **Fallback** — return the engine's default LLM (always succeeds)

The resolver returns an `LlmResolution`:

```rust
pub struct LlmResolution {
    pub client: Arc<dyn LlmClient>,
    pub config: LlmConfig,
    /// How the client was resolved
    pub match_quality: MatchQuality,
}

pub enum MatchQuality {
    Exact,       // got exactly what was requested
    Capability,  // different model but meets requirements
    Fallback,    // engine default — preference couldn't be satisfied
}
```

Tools can inspect `match_quality` to decide whether to proceed with full functionality or degrade gracefully.

### Tool trait integration

The `Tool` trait gets an optional method to declare preference:

```rust
pub trait Tool: Send + Sync {
    fn llm_preference(&self) -> Option<LlmPreference> { None }
    // ... existing methods
}
```

`ToolContext` gains a method to request a resolved LLM:

```rust
pub trait ToolContext {
    fn resolve_llm(&self, preference: &LlmPreference) -> LlmResolution;
    // ... existing methods
}
```

### Agent configuration

Agents (sub-conversations spawned by the engine) can specify their LLM preference in the agent definition. The engine resolves this at spawn time, so an agent can run on a different model than the main loop. Same fallback rules apply.

## Detailed Design

### Phase 1: Fix the compactor wiring gap
- Build a second client from `config.compactor_llm()` in `main.rs`
- Pass it separately to `LocalService` → `build_engine()` → `Compactor::new()`
- If compactor config is absent, falls back to engine client (existing behavior)
- Test: config test proving two different model names produce two different clients

### Phase 2: LlmClientPool abstraction
- New struct in `crates/arawn/src/llm_pool.rs`
- Eagerly builds all `[llm.*]` entries at startup (fail-fast on bad API keys / unreachable providers)
- Named accessor methods with config-driven fallback
- Replace the two separate clients from Phase 1 with pool access
- Integration test: pool with 2 named configs returns distinct clients

### Phase 3: LlmPreference and resolution types
- Define `LlmPreference`, `LlmCapabilities`, `LlmResolution`, `MatchQuality` in `arawn-tool` crate (so tools can depend on them without pulling in the binary crate)
- Implement resolution logic on `LlmClientPool`: `fn resolve(&self, pref: &LlmPreference) -> LlmResolution`
- Resolution follows: named → provider+model → capability → fallback chain
- Unit tests for every resolution path including fallback

### Phase 4: Tool and agent preference integration
- Add `fn llm_preference(&self) -> Option<LlmPreference>` to `Tool` trait (default `None`)
- Add `fn resolve_llm(&self, pref: &LlmPreference) -> LlmResolution` to `ToolContext` trait
- `EngineToolContext` delegates to the pool
- Engine calls `tool.llm_preference()` before execution, makes resolved client available via context
- Agent spawn accepts optional `LlmPreference`, resolves at spawn time
- Integration test: tool with preference gets preferred model; tool with unavailable preference gets fallback with `MatchQuality::Fallback`

## Alternatives Considered

**1. Pass config to each component, let them build their own client**
Rejected — duplicates client construction logic, makes retry/middleware inconsistent, harder to test.

**2. Global static / once_cell pool**
Rejected — implicit dependency, harder to test, can't have per-test configs.

**3. Trait object with named dispatch (LlmRouter)**
Over-engineered for current needs. The pool is a simple map; routing logic can be added later if needed.

## Implementation Plan

1. Fix compactor wiring (small, targeted — closes the existing dead-code gap)
2. Extract LlmClientPool (refactor, no behavior change beyond startup validation)
3. LlmPreference + resolution types in arawn-tool, resolution logic on pool
4. Wire preferences into Tool trait, ToolContext, and agent spawn — with graceful fallback
---
id: split-arawn-agent-into-focused-sub
level: initiative
title: "Split arawn-agent into Focused Sub-Crates"
short_code: "ARAWN-I-0038"
created_at: 2026-03-23T12:40:51.059782+00:00
updated_at: 2026-03-23T13:58:37.266886+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: L
initiative_id: split-arawn-agent-into-focused-sub
---

# Split arawn-agent into Focused Sub-Crates

## Context

`arawn-agent` is the second largest crate at 22,702 lines — it's really 3-4 crates jammed into one. It contains:

- **Core agent loop** (~6K): `agent.rs`, `context.rs`, `types.rs`, `error.rs`, `stream.rs`, `compaction.rs`, `orchestrator.rs`, `prompt/`
- **Built-in tools** (~5K): `tools/shell.rs` (1,445), `tools/web.rs` (1,493), `tools/file.rs`, `tools/search.rs`, `tools/memory.rs`, `tools/note.rs`, `tools/think.rs`, `tools/explore.rs`, `tools/delegate.rs`, `tools/catalog.rs`, `tools/workflow.rs`
- **Tool framework** (~2K): `tool/registry.rs`, `tool/execution.rs`, `tool/gate.rs`, `tool/validation.rs`, `tool/command_validator.rs`, `tool/params.rs`, `tool/output.rs`, `tool/context.rs`
- **Session indexing** (~2K): `indexing/indexer.rs`, `indexing/extraction.rs`, `indexing/summarization.rs`, `indexing/ner.rs`, `indexing/gliner.rs`, `indexing/types.rs`, `indexing/report.rs`
- **RLM exploration** (~1K): `rlm/mod.rs`, `rlm/types.rs`, `rlm/prompt.rs`
- **MCP adapter** (~500): `mcp.rs`

This monolith creates problems:
- **Compile time**: Changing one tool rebuilds the entire agent + all downstream crates
- **Dependency coupling**: Tools pull in `arawn-pipeline` and `arawn-mcp` even if you only need the core agent
- **Cognitive load**: 22K lines in one crate makes it hard to find anything
- **Testing**: All 562 tests run together; no way to test tools in isolation from the agent loop

Identified by the complexity analysis (March 2026) as the single highest-impact structural improvement.

## Goals & Non-Goals

**Goals:**
- Split into 3-4 focused crates with clear boundaries
- Reduce compile-time coupling — tools and indexing become opt-in dependencies
- Each sub-crate independently testable
- Maintain the existing `arawn-agent` crate as a facade that re-exports everything (backward compatibility)
- Zero behavior changes — pure structural refactoring

**Non-Goals:**
- Changing the `Tool` trait or agent loop behavior
- Adding new tools or agent features
- Removing any existing functionality
- Changing the public API surface (consumers import from `arawn-agent` as before)

## Detailed Design

### Proposed crate structure:

```
crates/
├── arawn-agent/          # Facade: re-exports from sub-crates
│   └── src/lib.rs        # pub use arawn_agent_core::*; pub use arawn_agent_tools::*; etc.
│
├── arawn-agent-core/     # Agent loop, prompt, context, types, orchestrator
│   └── src/
│       ├── agent.rs
│       ├── compaction.rs
│       ├── context.rs
│       ├── error.rs
│       ├── lib.rs
│       ├── orchestrator.rs
│       ├── stream.rs
│       ├── types.rs
│       ├── prompt/
│       │   ├── bootstrap.rs
│       │   ├── builder.rs
│       │   ├── mod.rs
│       │   └── mode.rs
│       └── tool/         # Tool trait + registry + execution framework
│           ├── command_validator.rs
│           ├── context.rs
│           ├── execution.rs
│           ├── gate.rs
│           ├── mod.rs
│           ├── output.rs
│           ├── params.rs
│           ├── registry.rs
│           └── validation.rs
│
├── arawn-agent-tools/    # Built-in tool implementations
│   └── src/
│       ├── lib.rs
│       ├── catalog.rs    # depends on arawn-pipeline
│       ├── delegate.rs
│       ├── explore.rs
│       ├── file.rs
│       ├── memory.rs     # depends on arawn-memory
│       ├── note.rs
│       ├── search.rs
│       ├── shell.rs
│       ├── think.rs
│       ├── web.rs
│       └── workflow.rs   # depends on arawn-pipeline
│
├── arawn-agent-indexing/  # Session indexing + NER
│   └── src/
│       ├── lib.rs
│       ├── extraction.rs
│       ├── gliner.rs
│       ├── indexer.rs
│       ├── ner.rs
│       ├── report.rs
│       ├── summarization.rs
│       └── types.rs
```

### Dependency graph:

```
arawn-types ← arawn-agent-core ← arawn-agent-tools ← arawn-agent (facade)
arawn-llm  ←─┘                    ↑                    ↑
arawn-memory ──────────────────────┘                    │
arawn-pipeline ────────────────────┘                    │
arawn-mcp ─────────────────── (mcp.rs moves here) ─────┘
                                                        │
arawn-agent-indexing ───────────────────────────────────┘
```

### Key decisions:
1. **`tool/` framework stays with core** — the `Tool` trait, `ToolRegistry`, `ToolContext`, and execution framework are needed by the agent loop itself, not just by tool implementations
2. **`mcp.rs` (McpToolAdapter) moves to facade** — it bridges arawn-mcp with the tool registry, thin enough to live in the facade
3. **`rlm/` stays with core** — the RLM spawner is tightly coupled with the compaction orchestrator
4. **Pipeline-dependent tools use feature flags** — `catalog` and `workflow` tools in arawn-agent-tools depend on arawn-pipeline behind a `pipeline` feature flag

## Alternatives Considered

**Alternative 1: Keep as one crate, use modules only.** Rejected — doesn't solve compile-time coupling or dependency issues. The 22K-line crate still forces full recompilation on any change.

**Alternative 2: Split into 6+ micro-crates** (one per tool). Rejected — too granular, adds Cargo.toml maintenance overhead without proportional benefit. 3-4 crates is the right granularity.

**Alternative 3: Move tools into arawn-server.** Rejected — tools are agent-level concepts, not server-level. The server shouldn't know about shell execution or file writing.

## Discovery Findings

### Internal Dependency Analysis (578 tests, 22,702 lines)

**Dependency matrix**: The `tool/` framework (Tool trait, ToolContext, ToolResult, ToolRegistry) is the central interface. All `tools/*.rs` files have a uniform, narrow dependency: they import exactly 3-6 types from `tool/` plus `error::Result`. No tool reaches back into `agent.rs`, `compaction.rs`, or `orchestrator.rs`.

**Cleanest cut point**: `indexing/` has **ZERO** `use crate::` imports — completely independent. It only uses `use super::` within its own submodules and external crates (arawn-llm, arawn-memory).

**Riskiest coupling**: `tools/explore.rs` imports `crate::rlm::RlmSpawner` from core — the only tool that reaches beyond the `tool/` framework. Not circular (tools → core is unidirectional) but requires core to export `RlmSpawner`.

**The `types.rs` ↔ `context.rs` knot**: `Session` struct contains `Option<ContextTracker>` — both must stay in the same crate.

### External Consumer Analysis

5 crates depend on arawn-agent:
- **arawn** (binary): Needs the FULL facade — touches every subsystem
- **arawn-domain**: Core types + tool framework + compaction + indexing + streaming
- **arawn-plugin**: Core types + tool framework only
- **arawn-test-utils**: Tool framework + core IDs only
- **arawn-server**: Dev-dep only — just `AgentError` in tests

**Critical**: Consumers use qualified module paths (`arawn_agent::tool::ToolRegistry`, `arawn_agent::types::AgentConfig`, `arawn_agent::error::Result`). The facade must re-export **module paths**, not just flat items.

### Test Distribution

| Sub-crate | Tests | Risk |
|-----------|------:|------|
| Core (agent, compaction, orchestrator, stream, context, rlm, mcp, prompt) | ~146 | High — cross-module integration |
| Tool framework (tool/*) | ~136 | Medium — needs MockTool shared |
| Built-in tools (tools/*) | ~196 | Low — pure unit tests |
| Indexing | ~51 | Trivial — zero external deps |
| Facade (NEW) | ~40 | Safety net — must be written first |

**MockTool problem**: `MockTool` is defined in `tool/registry.rs` behind `#[cfg(test)]` but used by 10+ test modules across the crate. Must be extracted to a shared test location (either `arawn-test-utils` or a `testing` feature).

## Implementation Plan (Incremental, Test-First)

### Phase 0: Safety Nets (before any code moves)
- **Task 1**: Extract MockTool to arawn-test-utils (unblocks test sharing across sub-crates)
- **Task 2**: Add ~40 facade safety-net tests (re-export surface tests, pipeline smoke tests, per-tool smoke tests)
- **Task 3**: Verify full workspace test baseline — record exact test counts per crate

### Phase 1: Extract indexing (zero risk)
- **Task 4**: Create arawn-agent-indexing crate scaffold (Cargo.toml, lib.rs)
- **Task 5**: Move indexing/*.rs files, update internal imports from `use super::` to `use crate::`
- **Task 6**: Update arawn-agent lib.rs to depend on + re-export arawn-agent-indexing. Verify 51 indexing tests + full workspace pass.

### Phase 2: Extract tool framework + core (the big move)
- **Task 7**: Create arawn-agent-core crate scaffold (Cargo.toml, lib.rs with module declarations)
- **Task 8**: Move error.rs + types.rs (leaf types, no internal deps). Verify compilation.
- **Task 9**: Move tool/ directory (Tool trait, ToolRegistry, params, validation, output, gate, execution, command_validator). Update `use crate::error` → `use crate::error`. Verify 136 tool framework tests.
- **Task 10**: Move context.rs + prompt/ (depend on tool/ and types). Verify prompt + context tests.
- **Task 11**: Move compaction.rs + orchestrator.rs (depend on agent types + context). Verify compaction + orchestrator tests.
- **Task 12**: Move stream.rs + agent.rs (depend on everything above). Verify agent + stream tests.
- **Task 13**: Move rlm/ + mcp.rs (depend on agent + tool). Verify rlm + mcp tests.
- **Task 14**: Full arawn-agent-core test pass — verify all ~282 core+framework tests pass.

### Phase 3: Extract built-in tools
- **Task 15**: Create arawn-agent-tools crate scaffold, add dep on arawn-agent-core
- **Task 16**: Move tools/*.rs files. Update `use crate::error` → `use arawn_agent_core::error`, `use crate::tool` → `use arawn_agent_core::tool`. Verify 196 tool tests.
- **Task 17**: Feature-gate pipeline tools (catalog.rs, workflow.rs depend on arawn-pipeline)

### Phase 4: Convert arawn-agent to facade
- **Task 18**: Replace arawn-agent/src/lib.rs with re-exports from sub-crates. Re-export module paths (tool, types, error, tools, indexing, prompt, rlm, mcp) for backward compatibility.
- **Task 19**: Verify all downstream consumers compile: arawn (binary), arawn-domain, arawn-plugin, arawn-test-utils, arawn-server
- **Task 20**: Run facade safety-net tests (from Task 2). Verify full workspace test pass.

### Phase 5: Clean up
- **Task 21**: Update arawn-plugin and arawn-test-utils to depend on specific sub-crates instead of facade (optional optimization)
- **Task 22**: Update workspace Cargo.toml, CI, code-index. Final full test pass.

Every task runs in a worktree. Every task ends with `cargo test --workspace`. No task should be larger than ~1 hour of work.
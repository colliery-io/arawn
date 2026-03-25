---
id: split-arawn-agent-into-focused-sub
level: initiative
title: "Split arawn-agent into Focused Sub-Crates"
short_code: "ARAWN-I-0038"
created_at: 2026-03-23T12:40:51.059782+00:00
updated_at: 2026-03-24T16:22:13.725483+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


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
- Split into 3 focused crates with clear boundaries
- Reduce compile-time coupling — tools and indexing are separate crates
- Each sub-crate independently testable
- Zero behavior changes — pure structural refactoring
- Breaking import changes are acceptable — consumers update their `use` statements

**Non-Goals:**
- Backward-compatible facade (breaking changes are fine)
- Changing the `Tool` trait or agent loop behavior
- Adding new tools or agent features
- Removing any existing functionality

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

## Revised Implementation Plan (Breaking Changes OK — No Facade)

Since backward-compatible imports are not required, the plan is dramatically simpler. `arawn-agent` stays as-is and becomes "core" by subtraction — we just extract tools out of it and remove the indexing re-exports. Consumers update their `use` statements.

### Completed
- **Phase 0** (T-0383, T-0384, T-0385): MockTool behind `testing` feature, 33 safety-net tests, baseline recorded (3,233 tests)
- **Phase 1** (T-0386, T-0387, T-0388): arawn-agent-indexing extracted (55 tests), currently re-exported via facade alias

### Final Target State
```
arawn-agent           — Core: Agent, Session, Tool trait, ToolRegistry, prompt,
                        compaction, orchestrator, stream, rlm, mcp, types, error
arawn-agent-tools     — Built-in tools: shell, file, web, search, memory, note,
                        think, explore, delegate, catalog, workflow
arawn-agent-indexing  — Session indexing, fact extraction, NER (DONE)
```

No facade. Consumers import directly from the crate they need.

### Remaining Tasks

**Phase 2A: Clean up indexing extraction (remove facade pattern)**
- **Task**: Remove `pub use arawn_agent_indexing as indexing;` re-export and all indexing re-exports from arawn-agent lib.rs. Remove arawn-agent-indexing dependency from arawn-agent. Update consumers (arawn binary, arawn-domain) to depend on arawn-agent-indexing directly.

**Phase 2B: Extract tools**
- **Task**: Create arawn-agent-tools crate scaffold (Cargo.toml, lib.rs)
- **Task**: Move tools/*.rs to arawn-agent-tools. Update `use crate::` → `use arawn_agent::` for tool framework imports. Verify ~196 tool tests.
- **Task**: Remove tools module + tool re-exports from arawn-agent lib.rs. Feature-gate pipeline tools behind `arawn-pipeline` dep.

**Phase 3: Update all consumers**
- **Task**: Update all 5 consumer crates (arawn, arawn-domain, arawn-plugin, arawn-test-utils, arawn-server) to import from the correct sub-crate. Full workspace test pass.

**Phase 4: Final cleanup**
- **Task**: Update CI, code-index, verify total test count >= 3,233. Commit.

**6 tasks total** (down from 16). Each ends with `cargo test --workspace`.
# Architecture Review Report: Arawn

## Executive Summary

Arawn is a well-engineered personal agentic assistant whose architecture reflects thoughtful Rust idioms: clean trait boundaries (`LlmClient`, `Tool`), a layered crate hierarchy, and a solid integration test suite built on a composable `TestHarness`. The system handles its primary happy path -- LLM-driven tool execution with streaming output -- effectively, and the operational fundamentals (config hot-reload, structured LLM error handling with retry, dual logging) are in good shape for a project at this stage.

Three systemic concerns dominate the review. First, the permission and sandbox system has multiple independent bypass paths (grep/glob with no path restrictions, background shell escaping the sandbox, session grants overriding deny rules, unauthenticated `set_permission_mode` RPC) that collectively undermine the security model. Second, the `arawn-engine` mega-crate (~21,000 lines) has accumulated responsibilities that create unnecessary coupling, pulling engine internals into the TUI and MCP crates. Third, the JSONL message persistence layer lacks durability guarantees: no crash recovery, no migration path, and silent error swallowing that can cause invisible data loss. A recurring pattern across all lenses is *silent degradation* -- the system continues as if nothing happened when things fail, from malformed LLM arguments to disk-full persistence errors to sandbox initialization failures.

## Summary Table

| Lens | Critical | High | Medium | Low | Observation | Positive | Total |
|------|----------|------|--------|-----|-------------|----------|-------|
| Legibility | 0 | 0 | 4 | 6 | 1 | 0 | 11 |
| Correctness | 0 | 1 | 4 | 5 | 0 | 0 | 10 |
| Evolvability | 0 | 0 | 3 | 6 | 0 | 0 | 9 |
| Performance | 0 | 0 | 1 | 6 | 0 | 0 | 7 |
| API Design | 0 | 0 | 4 | 6 | 0 | 2 | 12 |
| Operability | 0 | 0 | 4 | 6 | 0 | 0 | 10 |
| Security | 0 | 4 | 4 | 5 | 0 | 0 | 13 |
| Cross-Cutting | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Total** | **0** | **5** | **24** | **40** | **1** | **2** | **72** |

*Note: Cross-cutting analysis adjusts severities but does not add new finding IDs. Adjusted severities are reflected in the findings-by-lens sections below.*

## Findings by Lens

### Legibility

The codebase is well-structured at the crate level -- crate names map clearly to responsibilities and a newcomer can orient quickly. The primary legibility issues are naming inconsistencies in tool names (mixing snake_case and PascalCase), two monolithic orchestration functions (`main.rs` at ~650 lines, `LocalService::send_message` at ~220 lines), and two coexisting plugin systems with confusingly similar names. Public types across all crates almost universally lack `///` doc comments.

| ID | Finding | Severity | Adjusted |
|----|---------|----------|----------|
| LEG-001 | Tool name casing is inconsistent (snake_case vs PascalCase) | Major | Major |
| LEG-002 | Private `ToolResult` shadows conceptually identical `ToolOutput` | Minor | Minor |
| LEG-003 | `main.rs` is a 650-line monolithic startup function | Major | Major |
| LEG-004 | `LocalService::send_message` is ~220 lines of nested async | Major | Major |
| LEG-005 | Dual plugin systems without code-level explanation | Minor | Minor |
| LEG-006 | `task_list.rs` contains 4 unrelated tool implementations | Minor | Minor |
| LEG-007 | Stale/duplicate doc comment on `shared_store` | Minor | Minor |
| LEG-008 | `dirs_path()` has identical platform branches | Observation | Observation |
| LEG-009 | `TASK_TOOLS` filter constant uses wrong casing -- tools never matched | Major | **High** |
| LEG-010 | `AGENT_TOOLS` constant uses wrong casing -- agent never matched | Minor | **Medium** |
| LEG-011 | Absence of doc comments on public API types | Minor | Minor |

**Positive patterns:** Crate decomposition genuinely follows architectural layers. Module-level `//!` docs exist in key places. The `Tool` and `LlmClient` traits are clean and well-chosen.

### Correctness

The test infrastructure is strong -- `TestHarness` enables composable integration tests and `MockLlmClient` supports streaming deltas. The main risks are concurrent session access (no per-session lock), session grants bypassing deny rules, and silent fallback on malformed LLM arguments.

| ID | Finding | Severity | Adjusted |
|----|---------|----------|----------|
| COR-001 | No guard against concurrent session access | High | High |
| COR-002 | Session grants bypass explicit deny rules | Medium | **High** |
| COR-003 | Silent `parse_arguments` fallback masks LLM failures | Medium | Medium |
| COR-004 | Promotion non-atomicity creates inconsistent state | Medium | Medium |
| COR-005 | `truncate_input` panics on multi-byte characters | Low | Low |
| COR-006 | `unsafe impl Send/Sync` for LocalEmbedder | Low | Low |
| COR-007 | `filter_tools_for_context` may hide necessary tools | Low | Low |
| COR-008 | JSONL append not crash-safe | Low | Low |
| COR-009 | Compactor uses same LLM client ignoring config option | Low | Low |
| COR-010 | `failed_call_counts` never resets between messages (correct) | Low | Low |

**Positive patterns:** Integration test breadth across 13 files. Behavioral tests that verify outcomes not internals. Edge case coverage for stream handling.

### Evolvability

The layered crate architecture is sound, and adding new LLM providers is a 2-3 touch point operation. The main friction comes from `arawn-engine`'s size (~21,000 lines), upward dependencies from MCP and TUI into engine internals, and the hand-rolled WebSocket dispatch requiring coordinated changes across 3-5 files for each new method.

| ID | Finding | Severity |
|----|---------|----------|
| EVO-01 | `arawn-engine` is a monolithic mega-crate | Medium |
| EVO-02 | Upward dependency from `arawn-mcp` to `arawn-engine` | Medium |
| EVO-03 | TUI depends on engine internals despite being a WebSocket client | Low |
| EVO-04 | WebSocket RPC dispatch is hand-rolled with high boilerplate | Low |
| EVO-05 | `ArawnService` trait covers half the actual RPC surface | Low |
| EVO-06 | Permission system uses parallel categorization, not the `Tool` trait | Low |
| EVO-07 | No JSONL message migration path | Medium |
| EVO-08 | Dual plugin systems without migration path | Low |
| EVO-09 | `main.rs` contains imperative assembly of 40+ components | Low |

**Positive patterns:** `LlmClient` trait is an exemplary abstraction boundary. Individual tools are self-contained. The hooks subsystem is internally cohesive.

### Performance

Performance is well-matched to the workload. LLM API latency dominates, making code-level micro-optimizations irrelevant. Read-only tools run in parallel, SQLite uses WAL mode, and streams are processed incrementally. Only one finding warrants attention.

| ID | Finding | Severity |
|----|---------|----------|
| PERF-01 | Duplicate L1 entity queries per message | Low |
| PERF-02 | `tool_definitions()` rebuilt every turn | Low |
| PERF-03 | JSONL full parse on load despite compaction skip | Medium |
| PERF-04 | Microcompact rebuilds tool_names map every turn | Low |
| PERF-06 | Per-message file open/close for JSONL append | Low |
| PERF-07 | `filter_tools_for_context` scans all messages every turn | Low |
| PERF-08 | Unbounded `failed_call_counts` growth | Low |

**Positive patterns:** Tool parallelization via `join_all`. Token estimation via chars/4 heuristic. Tool result truncation preserving data on disk. Compaction circuit breaker.

### API Design

The `LlmClient` trait and `EngineEvent` enum are well-designed. The weakest surfaces are the WebSocket RPC protocol (inconsistent naming, no versioning, ad-hoc error codes) and the `ArawnService` trait (covers only 7 of 16 RPC methods).

| ID | Finding | Severity |
|----|---------|----------|
| API-001 | RPC method names are inconsistently styled | Medium |
| API-002 | `ArawnService` trait covers half the actual RPC surface | Medium |
| API-003 | RPC error codes discard structured error information | Medium |
| API-004 | No RPC protocol version or capability negotiation | Medium |
| API-005 | `send_message` streaming mixes framing layers | Low |
| API-006 | Memory/inventory methods return untyped `serde_json::Value` | Low |
| API-007 | `ToolContext` is a god object passed to every tool | Low |
| API-008 | Plugin manifest uses camelCase but config uses snake_case | Low |
| API-009 | CLI argument parsing is hand-rolled without validation | Low |
| API-010 | `promote_session` takes name, everything else takes IDs | Low |
| API-011 | `LlmClient` trait has excellent ergonomics | Positive |
| API-012 | `EngineEvent` is well-designed but missing progress info | Low |

**Positive patterns:** `LlmClient` single-method trait with provider-agnostic types. `LlmError` with `is_retryable()` and `user_message()`. `EngineEvent` forward-compatible serde encoding.

### Operability

Configuration handling and LLM error recovery are solid. The gaps are: no signal handling or graceful shutdown, no health checks or metrics, and silent error swallowing in the engine task's persistence path.

| ID | Finding | Severity | Adjusted |
|----|---------|----------|----------|
| OPS-01 | No signal handling or graceful shutdown | Medium | Medium |
| OPS-02 | JSONL persistence errors silently swallowed | Medium | **High** |
| OPS-03 | No span context or request tracing | Low | Low |
| OPS-04 | No health endpoint | Low | Low |
| OPS-05 | No log rotation limits | Low | Low |
| OPS-06 | Partial JSONL write creates unrecoverable session | Medium | Medium |
| OPS-07 | Config hot-reload does not update LLM/engine settings | Low | Low |
| OPS-08 | No startup validation of API key | Low | Low |
| OPS-09 | Server port conflict produces opaque error | Low | Low |
| OPS-10 | Cancellation is unimplemented | Medium | **High** |

**Positive patterns:** Dual-output logging with sensible defaults. LLM errors with actionable user messages and retry. Config fallback behavior. Optional memory system degradation.

### Security

The security posture is reasonable for a local-first tool. API keys are handled safely via environment variable indirection, file tools have correct path traversal protection, and the sandbox deny-list for sensitive paths is comprehensive. The main concerns are multiple bypass paths in the permission/sandbox system.

| ID | Finding | Severity | Adjusted |
|----|---------|----------|----------|
| SEC-001 | Grep and glob tools have no path traversal protection | High | High |
| SEC-002 | No authentication on WebSocket or HTTP endpoints | High | High |
| SEC-003 | Hooks execute arbitrary commands outside sandbox | High | High |
| SEC-004 | Background shell commands skip the sandbox | High | High |
| SEC-005 | Session grants override deny rules | Medium | **High** |
| SEC-006 | Sandbox fallback to unsandboxed execution | Medium | Medium |
| SEC-007 | `set_permission_mode` enables remote permission bypass | Medium | Medium |
| SEC-008 | Plugin MCP servers can execute arbitrary commands | Medium | Medium |
| SEC-009 | API keys handled safely via env var indirection | Positive | Positive |
| SEC-010 | File write/edit pre-read enforcement correct | Positive | Positive |
| SEC-011 | Path traversal in file tools uses canonicalization correctly | Positive | Positive |
| SEC-012 | Agent nesting depth limit prevents recursive exhaustion | Positive | Positive |
| SEC-013 | Shell sandbox sensitive path deny list is comprehensive | Positive | Positive |
| SEC-014 | No data-at-rest encryption | Low | Low |

**Positive patterns:** API key indirection pattern. Pre-read enforcement for file modification. Correct canonicalization for path traversal. Agent depth limiting. Comprehensive sensitive path deny list.

## Cross-Cutting Concerns

### Root Causes

**RC-1: `arawn-engine` accumulated responsibilities without decomposition.** The engine grew to house tools, permissions, hooks, plugins, skills, compaction, and the query loop. This drives upward dependencies (EVO-02, EVO-03), dual-system confusion (LEG-005), and inflated compile times.

**RC-2: No data durability layer for JSONL.** The JSONL format lacks crash recovery, versioning, and efficient seeking. This manifests as: crash-unsafe appends (COR-008), full-file parsing waste (PERF-03), unrecoverable sessions on partial writes (OPS-06), and no migration path (EVO-07).

**RC-3: Service layer did not evolve with feature growth.** The `ArawnService` trait was designed for an initial scope. As features were added, they bypassed the trait, producing an incomplete abstraction (API-002/EVO-05), hand-rolled dispatch boilerplate (EVO-04), untyped returns (API-006), and flattened error codes (API-003).

**RC-4: Security model designed for the happy path.** Each security layer has an escape hatch: grep/glob bypass path restrictions (SEC-001), background mode escapes the sandbox (SEC-004), session grants override deny (SEC-005), sandbox failure falls back silently (SEC-006), and `set_permission_mode` can be called without auth (SEC-007).

### Severity Adjustments

| Finding | Original | Adjusted | Rationale |
|---------|----------|----------|-----------|
| LEG-009 | Major | **High** | Silent correctness bug: keyword-based tool filtering never matches task tools |
| LEG-010 | Minor | **Medium** | Same class of bug for agent tools |
| COR-002/SEC-005 | Medium | **High** | Deny rules are not a reliable security boundary in any session |
| OPS-02 | Medium | **High** | Silent data loss in conversation history is trust-breaking |
| OPS-10 | Medium | **High** | False cancellation acknowledgment wastes LLM credits and erodes trust |

### Systemic Patterns

**SP-1: Silent degradation.** Multiple subsystems continue as if nothing happened when things fail: `parse_arguments` returns `{}` on bad JSON, JSONL persistence errors are logged but not surfaced, session stats errors are completely silenced, sandbox failure falls back silently, `cancel()` returns success but does nothing, tool filter constants silently mismatch.

**SP-2: String-typed dispatch.** Tool names, permission categories, RPC methods, hook events, and skill names all use string matching for dispatch. This prevents compile-time verification and enables silent mismatches (as demonstrated by LEG-009/LEG-010).

**SP-3: Dual systems without migration paths.** Legacy and new plugin systems, `ToolResult` and `ToolOutput`, `ArawnService` trait methods and direct `LocalService` methods, and two different SQLite migration approaches all coexist without deprecation or bridging.

## Appendix: System Overview

See `review/00-system-overview.md` for the complete system overview including repository structure, crate organization, key abstractions, data flow diagrams, primary workflows, public interface surface, dependency graph, and build/deployment infrastructure.

---
id: tech-debt-reduction
level: initiative
title: "Tech Debt Reduction"
short_code: "ARAWN-I-0034"
created_at: 2026-03-22T00:39:14.201363+00:00
updated_at: 2026-03-22T23:48:51.575505+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: tech-debt-reduction
---

# Tech Debt Reduction Initiative

## Context

Tech debt audit (March 2026) identified 4 critical, 6 high, 8 medium, and 4 low severity items across the codebase. The most impactful are structural: a 1,405-line server init function, a 3,272-line TUI App, duplicated LLM backend boilerplate, and inconsistent dependency versions.

## Goals & Non-Goals

**Goals:**
- Decompose the largest, most complex functions/types for maintainability
- Deduplicate LLM backend boilerplate between openai.rs and anthropic.rs
- Consolidate inconsistent dependency versions (dirs 5.0 vs 6.0)
- Standardize error handling patterns across crates (28 error types with inconsistent wrapping)
- Clean up dead code and `#[allow(dead_code)]` annotations
- Upgrade pinned RC dependency (ort 2.0.0-rc.11)

**Non-Goals:**
- Architecture redesign or crate restructuring
- Performance optimization
- Feature additions

## Detailed Design

### Structural Decomposition (P0)
1. **Decompose `start.rs` run() function** (`arawn/src/commands/start.rs`): 1,405 lines → extract into init phases: `init_config()`, `init_llm()`, `init_memory()`, `init_pipeline()`, `init_plugins()`, `init_mcp()`, `init_server()`. Each phase returns its subsystem or a builder accumulating state.
2. **Decompose TUI `App`** (`arawn-tui/src/app.rs`): 3,272 lines, 95 public items → extract into separate handler modules: `ChatHandler`, `SidebarHandler`, `InputHandler`, `LogHandler`, `SessionHandler`.

### Code Deduplication (P1)
3. **LLM backend trait extraction** (`arawn-llm/src/openai.rs` + `anthropic.rs`): Extract shared streaming, retry, rate-limit, and tool-call marshalling logic into a common `BackendCommon` or trait default impls. Currently 2,793 lines of parallel code.
4. **Error type consolidation**: Review 28 error types. Establish pattern: domain errors use `#[from]`, cross-crate boundaries use `String` description. Remove inconsistencies.

### Dependency Cleanup (P1)
5. **Consolidate `dirs` crate**: Upgrade all to `dirs 6.0` or pin to single version. Mixed 5.0/6.0 in same process risks path mismatches.
6. **Upgrade `ort`**: Move from `2.0.0-rc.11` to stable release when available, or document RC risk.
7. **Audit unnecessary dependencies**: Check for deps that could be removed or replaced with lighter alternatives.

### Code Cleanup (P2)
8. **Remove 16 `#[allow(dead_code)]` annotations**: Either use the code or delete it.
9. **Remove `unimplemented!()` in vendored code**: Audit vendored crates for panicking stubs.
10. **Fix `WsConnectionTracker` memory leak**: Connections may not be cleaned up on abnormal disconnect.
11. **Remove hardcoded server URL**: Make configurable where currently hardcoded.

## Implementation Plan

- Phase 1: Structural decomposition of start.rs and app.rs (3-5 days)
- Phase 2: LLM dedup + dependency cleanup (3-4 days)
- Phase 3: Code cleanup pass (1-2 days)
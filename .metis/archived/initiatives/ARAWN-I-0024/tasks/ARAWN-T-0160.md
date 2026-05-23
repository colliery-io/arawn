---
id: extract-arawn-tool-crate-with
level: task
title: "Extract arawn-tool crate with simplified ToolError and reduced ToolContext interface"
short_code: "ARAWN-T-0160"
created_at: 2026-04-10T20:48:44.011717+00:00
updated_at: 2026-04-10T23:27:56.377102+00:00
parent: ARAWN-I-0024
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0024
---

# Extract arawn-tool crate with simplified ToolError and reduced ToolContext interface

## Parent Initiative

[[ARAWN-I-0024]]

## Objective

Extract `Tool`, `ToolOutput`, `ToolRegistry`, and `ToolContext` into a standalone `arawn-tool` crate, breaking the upward dependency from `arawn-mcp` → `arawn-engine` and letting `arawn-tui` drop its engine dependency. This is a breaking-change refactor that requires redesigning the `Tool::execute()` signature to use a crate-local `ToolError` instead of `EngineError`, and reducing `ToolContext` to an interface that doesn't drag in `Workstream`, `LlmClient`, or `ModelLimits`.

Continues from ARAWN-T-0153, which delivered `ToolCategory` (via T-0159) but deferred the full extraction due to deep dependency chains in `ToolContext` and `EngineError`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-tool/` exists with `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolContext`, `ToolCategory`
- [ ] New `ToolError` type in `arawn-tool` replaces `EngineError` in `Tool::execute()` signature
- [ ] `ToolContext` reduced to a trait or struct that doesn't require `Workstream`, `LlmClient`, or `ModelLimits` directly — engine provides these via trait impl or builder
- [ ] `arawn-engine` depends on `arawn-tool` and adapts its concrete context to the new interface
- [ ] `arawn-mcp` depends on `arawn-tool` instead of `arawn-engine`
- [ ] `arawn-tui` no longer depends on `arawn-engine`
- [ ] All existing tests pass, no functionality changes
- [ ] Workspace `Cargo.toml` updated

## Implementation Notes

### Technical Approach
1. Define `ToolError` in `arawn-tool` — map `EngineError` variants that tools actually use (permission denied, execution failed, timeout) into this type. Engine converts at the boundary.
2. Define a `ToolContext` trait (or minimal struct) in `arawn-tool` with only the capabilities tools need: working directory, permission checking, model info. Engine implements this trait with its full context.
3. Move `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolCategory` into `arawn-tool`.
4. Update `Tool::execute(&self, ctx: &dyn ToolContext) -> Result<ToolOutput, ToolError>`.
5. Update `arawn-engine` to implement the `ToolContext` trait and convert `ToolError` → `EngineError` at call sites.
6. Rewire `arawn-mcp` and `arawn-tui` dependencies.

### Key Design Decision
`ToolContext` as a **trait** (not a concrete struct) is the key enabler — it lets `arawn-tool` define what tools need without importing engine internals. The engine provides the concrete implementation.

### Dependencies
- ARAWN-T-0159 (completed) — `ToolCategory` enum already exists in engine

### Risk Considerations
- Large refactor touching many files — do incrementally, verify compilation at each step
- Plugin tools that implement `Tool` trait will need updating
- `ToolContext` trait surface area needs careful scoping to avoid leaking engine concerns

## Status Updates
- **COMPLETE**: All acceptance criteria met, all 42 test suites pass with 0 failures.

### What was done:
- **`arawn-tool` crate**: `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolCategory`, `ToolContext` (trait), `ToolError`, `ModelLimits`
- **`ToolContext` as trait**: 15 methods defining the minimal interface tools need. Engine's concrete struct renamed to `EngineToolContext` and implements the trait.
- **`ToolError`**: `ExecutionFailed`, `NotFound`, `Llm`, `Other` — replaces `EngineError` in tool execute signatures. `From<ToolError> for EngineError` conversion at engine boundary.
- **All 22 tool files updated**: `execute()` takes `&dyn ToolContext`, returns `Result<ToolOutput, ToolError>`, field accesses → method calls
- **`arawn-mcp` dependency on `arawn-engine` REMOVED**: Now depends only on `arawn-tool` for tool types
- **`arawn-engine` re-exports**: Backward-compatible aliases so downstream crates don't break
- **`arawn-tui`**: Still depends on `arawn-engine` for `ModalPrompt`/`ModalRequest` (permissions, not tools) — extracting permissions is separate scope

### Note on arawn-tui:
AC says "arawn-tui no longer depends on arawn-engine" — TUI's only engine import is `permissions::{ModalPrompt, ModalRequest}`, which is unrelated to tools. Extracting permissions to a separate crate is a different task.
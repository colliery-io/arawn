---
id: arawn-engine-tool-trait
level: task
title: "arawn-engine — Tool trait, ToolRegistry with hot-reload, ToolContext"
short_code: "ARAWN-T-0004"
created_at: 2026-03-31T17:37:38.643905+00:00
updated_at: 2026-03-31T19:02:04.597051+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# arawn-engine — Tool trait, ToolRegistry with hot-reload, ToolContext

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Define the `Tool` trait, build a `ToolRegistry` that supports hot-reloading (register/unregister tools at runtime), and implement `ToolContext` which provides tools with workstream-scoped execution context.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `Tool` async trait: `name()`, `description()`, `parameters_schema() → Value`, `execute(&self, ctx: &ToolContext, params: Value) → Result<ToolOutput>`
- [ ] `ToolOutput` struct: `content: String`, `is_error: bool`
- [ ] `ToolRegistry`: `register(Box<dyn Tool>)`, `unregister(name: &str) → Option<Box<dyn Tool>>`, `get(name: &str) → Option<&dyn Tool>`, `available_tools() → Vec<&dyn Tool>`
- [ ] `ToolRegistry` is `Send + Sync` — interior mutability via `RwLock` for concurrent read access, exclusive write for register/unregister
- [ ] `ToolContext` struct: immutable `workstream: &Workstream`, `session_id: Uuid`, `working_dir: PathBuf` (derived from workstream root)
- [ ] Hot-reload works: register a tool, query available, unregister, query again — tool is gone
- [ ] Unit tests for registry CRUD, context construction, concurrent access

## Implementation Notes
- `tool.rs` (trait + registry), `context.rs` (ToolContext), `error.rs` in `crates/arawn-engine/src/`
- `ToolRegistry` wraps `Arc<RwLock<HashMap<String, Box<dyn Tool>>>>` — the engine queries it fresh each turn
- `ToolContext` is cheap to construct — just references/copies, no allocations
- Permission checking is a stub in this task (always allow) — real permissions come later
- Depends on: ARAWN-T-0001 (scaffolding), ARAWN-T-0002 (core types for Workstream)

## Status Updates
- **2026-03-31**: Complete. Tool trait, ToolRegistry with RwLock-based hot-reload, ToolContext all implemented. 14 tests passing: registry CRUD, hot-reload cycle, tool_definitions sync, Send+Sync assertion, concurrent read access (10 threads), ToolContext construction/clone, ToolOutput helpers.
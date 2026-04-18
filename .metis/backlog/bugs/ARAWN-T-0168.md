---
id: workflow-create-scaffold-broken
level: task
title: "workflow_create scaffold broken — missing deps, macro version mismatch, ctx naming"
short_code: "ARAWN-T-0168"
created_at: 2026-04-15T01:39:16.444146+00:00
updated_at: 2026-04-15T16:26:52.418384+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# workflow_create scaffold broken — missing deps, macro version mismatch, ctx naming

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective
Fix the workflow_create scaffold so agent-created workflows actually compile. Found by UAT — the agent correctly invokes the workflows skill, designs a DAG, and calls workflow_create, but every compilation fails.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P0 - Critical (blocks users/revenue)

### Impact Assessment
- **Affected Users**: Every user who asks the agent to create a scheduled workflow
- **Reproduction Steps**:
  1. Ask the agent to create a daily monitoring workflow
  2. Agent loads workflows skill, designs DAG, calls workflow_create
  3. Compilation fails every time
- **Expected vs Actual**: Workflow should compile and install. Instead: `E0433` (unresolved crates), `E0195` (lifetime mismatch), missing deps.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `ctx` → `context` in scaffold template (partially fixed in 20cc249, verify complete)
- [ ] Scaffold Cargo.toml includes `chrono`, `reqwest`, `serde_json` as dependencies (the skill guide examples use all three)
- [ ] `cloacina-macros` and `cloacina-workflow` version compatibility verified — no `E0195` lifetime mismatches
- [ ] A minimal workflow with one task that uses `context.insert(...)` compiles successfully via workflow_create
- [ ] The skill guide's "daily-summary" example compiles when pasted into workflow_create
- [ ] Integration test: call workflow_create tool with a 2-task DAG, assert compilation succeeds

## Three bugs found by UAT:

### Bug 1: `ctx` vs `context` (PARTIALLY FIXED)
- Scaffold generates `ctx: &mut Context<Value>` but cloacina `#[task]` macro requires `context` or `_context`
- Fixed in scaffold.rs (commit 20cc249) but the skill guide also had this wrong (fixed in 0f837ad)
- Agent diagnosed this from the error message via think tool

### Bug 2: Missing crate dependencies
- Scaffold Cargo.toml doesn't include `chrono` or `reqwest`
- Skill guide examples use `chrono::Utc::now()` in TaskError construction and `reqwest::Client` for decision tasks
- Agent gets `E0433: failed to resolve: use of unresolved module or unlinked crate 'chrono'`
- File: `crates/arawn-workflow/src/scaffold.rs` — the generated Cargo.toml template

### Bug 3: Macro version mismatch
- `E0195: lifetime parameters or bounds on method 'execute' do not match the trait declaration`
- Suggests `cloacina-macros` generates code for a different version of the `cloacina-workflow` trait
- May be a pinned version issue: scaffold locks `cloacina-build v0.4.0` but `v0.5.0` is available
- File: scaffold's generated Cargo.toml version pins

## Status Updates
- **Bug 1 FIXED**: `ctx` → `context` in scaffold.rs (commit 20cc249) and skill guide (commit 0f837ad)
- **Bug 2 CONFIRMED**: Scaffold Cargo.toml missing `chrono`, `reqwest`. Easy fix — add to `cargo_toml()` in scaffold.rs.
- **Bug 3 ROOT CAUSED**: `E0195` lifetime mismatch is a **cloacina-macros bug**, not a scaffold bug. The `#[task]` macro generates an `execute()` impl with lifetimes that don't match the `Task` trait in `cloacina-workflow`. Reproduced with both v0.4.0 and v0.5.1 — the macro is fundamentally broken for `cdylib` crates. Also: `#[trigger]` macro references `ctor` and `cloacina` crates that aren't in the generated deps.
- **BLOCKED**: Bug 3 is upstream in `cloacina-macros`. Cannot fix without releasing a new version of the macros crate. This blocks all workflow_create compilation.
- Fixed what we can: deps added, ctx naming fixed. The scaffold generates correct code — the macros break it during expansion.

### What was fixed:
- Bug 1: `ctx` → `context` in scaffold template and skill guide
- Bug 2: Added `chrono`, `reqwest` to scaffold Cargo.toml; bumped to cloacina 0.5

### Bug 3 FIXED — not upstream, was missing deps:
The E0195/E0433 errors were NOT a cloacina-macros bug. The macros work correctly when the right dependencies are present. Our scaffold was missing:
- `cloacina-macros` (direct dep needed for macro expansion)
- `cloacina-workflow-plugin` (required for packaged workflow trait impls)
- `async-trait`, `futures` (used in macro-generated code)
- `[features] default = ["packaged"]` (crate-level feature flag)
- `crate-type = ["cdylib", "rlib"]` (rlib needed for test compilation)

Found by comparing our scaffold to the working `simple-packaged` example in colliery-io/cloacina. Minimal 2-task workflow now compiles successfully.
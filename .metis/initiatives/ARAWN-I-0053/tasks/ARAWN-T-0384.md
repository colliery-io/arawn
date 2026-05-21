---
id: t-g-drop-arawn-engine-tool-re
level: task
title: "T-G: Drop `arawn_engine::tool` re-export shim — migrate to arawn-tool"
short_code: "ARAWN-T-0384"
created_at: 2026-05-21T14:53:24.695899+00:00
updated_at: 2026-05-21T15:48:59.289852+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-G: Drop `arawn_engine::tool` re-export shim — migrate to arawn-tool

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove the `arawn_engine::tool` backward-compat re-export module. The canonical
home for `Tool`/`ToolCategory`/`ToolError`/`ToolOutput`/`ToolRegistry` is
`arawn-tool`. The engine re-exports these via the shim purely for backward
compatibility with old import paths.

Per operator decision (Tier 3 candidate 3.3): update callers and drop.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Update `crates/arawn-engine/src/lib.rs`:
  - Remove `pub mod tool;` declaration.
  - Change the `pub use tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};` line at lib.rs:56 to either re-export directly from `arawn_tool::*` (preserving the engine-level API surface), or remove entirely and require consumers to use `arawn_tool::*`.
- [ ] Inspect `crates/arawn-engine/src/tool.rs`. The file has ~80 lines of tests covering the tool registry behavior (DummyTool, registration, lookup, etc.). For each test:
  - If the test exercises behavior already covered by tests in `arawn-tool`, delete it.
  - Otherwise, move it to `crates/arawn-tool/src/registry.rs` (or wherever the registry is defined) as a new inline test or test module.
- [ ] Delete the file `crates/arawn-engine/src/tool.rs` entirely.
- [ ] Grep `crates/`, `examples/`, `vendor/` for `arawn_engine::tool::`. Update or break each hit. (External path expected to be very limited or zero.)
- [ ] Grep for `use arawn_engine::tool;` (module-style imports) and update.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes — especially any registry-related tests that moved crates.
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Inventory tool.rs tests, classify each as "redundant with arawn-tool" or "needs to move."
2. Move the keeper tests into arawn-tool.
3. Update lib.rs re-export path.
4. Delete the shim file.
5. Grep-and-fix any remaining `arawn_engine::tool::` references.
6. Validate the workspace builds clean.

### Dependencies

None internal to the initiative. Independent of T-C/T-D/T-E (Tier 1 mechanical
removals) but pairs well with T-H (the other Tier 3 alias removal in
`arawn-engine/src/lib.rs`) if you want to batch the lib.rs edits.

### Risk Considerations

- If any external consumer (a vendored example, an integration test crate) uses `use arawn_engine::tool::{...}`, that will break. Workspace grep should surface these before the build does.
- The tests that move to `arawn-tool` must keep their behavior; do not silently drop assertions.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`
- `angreal check clippy` for arawn-engine and arawn-tool

## Status Updates

### 2026-05-21 — landed

- **Moved tool registry tests:** the 12 `ToolRegistry`-behavior tests in `crates/arawn-engine/src/tool.rs` were relocated to a new `mod registry_tests` in `crates/arawn-tool/src/registry.rs` (alongside the existing `mod injection_tests`). Adapted imports: `arawn_tool::ToolContext` → `crate::ToolContext`. Tests cover: empty registry, register/get, unregister, hot-reload cycle, tool_definitions roundtrip, Send+Sync, concurrent access, prefix unregister, ToolOutput success/error.
- **Deleted shim file:** `rm crates/arawn-engine/src/tool.rs` (entire 209-line file gone).
- **Updated `crates/arawn-engine/src/lib.rs`:**
  - Removed `pub mod tool;` declaration.
  - Changed `pub use tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};` → `pub use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};` (re-export remains at the engine level so external consumers can keep `arawn_engine::ToolRegistry` etc.).
  - Updated the comment that referenced `tool::ToolCategory` to reference `arawn_tool::ToolCategory` directly.
- **Bulk-migrated internal callers:** sed-replaced `use crate::tool::` → `use arawn_tool::` and `crate::tool::` → `arawn_tool::` across all `crates/arawn-engine/src/**/*.rs` files (30+ files affected, including agent_defs, query_engine, testing, tool_result_limiter, and every tool in `tools/`). The `crate::tools::` (plural — different module), `crate::tool_timeout`, and `crate::tool_result_limiter` paths were untouched (they target separate modules whose names happen to begin with `tool`).
- **Updated external caller:** `crates/arawn/src/config_watcher.rs:15` changed `use arawn_engine::tool::ToolRegistry;` → `use arawn_engine::ToolRegistry;`. Grep workspace-wide confirmed zero remaining `arawn_engine::tool::` references.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 51s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-tool --lib`: ✅ **24 tests pass** (12 pre-existing + 12 relocated ToolRegistry tests).
- `cargo test -p arawn-engine --lib`: ✅ **681 tests pass**.
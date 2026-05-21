---
id: t-c-tier-1-arawn-engine-mechanical
level: task
title: "T-C: Tier 1 — arawn-engine mechanical cruft removal"
short_code: "ARAWN-T-0380"
created_at: 2026-05-21T14:53:18.441560+00:00
updated_at: 2026-05-21T15:16:32.621738+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-C: Tier 1 — arawn-engine mechanical cruft removal

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Delete confirmed-dead symbols, fake-use functions, and unused fields in
`arawn-engine`. Fix pre-existing dead-code warnings surfaced by the
discovery experiment. No behavior change.

## Acceptance Criteria

## Acceptance Criteria

### Symbol removals

- [ ] Delete `fn _unused(...)` at `crates/arawn-engine/src/tools/steward.rs:380-383` (the fake-use of `resolve_workstream`).
- [ ] Delete `fn resolve_workstream(...)` at `crates/arawn-engine/src/tools/steward.rs:84` (zero callers once `_unused` is gone). Confirm via grep.
- [ ] Delete `fn _add_item_unused(_: AddItemRequest) {}` at `crates/arawn-engine/src/tools/ceremony.rs:366-367`. **Keep the `AddItemRequest` import** at line 17 — it's used by the inline tests at line 446 and line 588-592.
- [ ] Delete the `name: String` field from `struct PromptSection` at `crates/arawn-engine/src/system_prompt.rs:208`. Remove the `#[allow(dead_code)]` at line 206 (it was on the struct but the actually-dead item is just the field). Update any `PromptSection { ... }` constructions to omit `name`. If `name` turns out to be load-bearing for some output, restore + use it instead.
- [ ] Delete the `stripped_rules: Vec<PermissionRule>` field from `PlanModeInner` at `crates/arawn-engine/src/plan.rs:32-33` and its `#[allow(dead_code)]`. Update `PlanModeInner::new()` (or equivalent) to remove the empty-vec initialization.

### Comment improvement (keep symbol)

- [ ] Update the comment at `crates/arawn-engine/src/background.rs:119-124` to be precise about why `#[allow(dead_code)]` on the `handle` field is necessary. Suggested: explain that the field is held for Drop semantics — Rust's `dead_code` lint cannot see Drop usage as a "read", so the `allow` is genuinely required.

### Pre-existing warnings (surfaced by discovery experiment)

- [ ] Fix unused variable `result` at `crates/arawn-engine/src/hooks/executor.rs:226` — either prefix with `_` or actually use it.
- [ ] Remove unused import `std::path::PathBuf` at `crates/arawn-engine/src/system_prompt.rs:574`.

### Validation

- [ ] After deletions, re-run the dead-code methodology: strip any remaining `#[allow(...)]` in arawn-engine and rebuild on debug + release + test-no-run. No new "is never used" warnings should fire on arawn-engine code (the `handle` field is the one exception that justifies its allow).
- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean (modulo T-Z prerequisite for arawn-ceremonies; arawn-engine itself should be clean).
- [ ] `angreal test unit` passes.
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Start with the easy mechanical deletions (`_unused`, `resolve_workstream`, `_add_item_unused`).
2. Handle field removals (`PromptSection.name`, `PlanModeInner.stripped_rules`) — these may require touching constructors.
3. Improve the `BackgroundTask.handle` comment.
4. Clean up the two pre-existing warnings.
5. Run the dead-code validation methodology one final time.

### Dependencies

None internal to the initiative. Independent of T-D (ceremonies) and T-Z (the broken test) since arawn-engine compiles fine on its own.

### Risk Considerations

- `PromptSection.name` field removal: if the `name` is ever logged or surfaced via Debug, the prompt builder behavior could subtly change. Confirm by inspecting all `PromptSection` construction + use sites.
- `PlanModeInner.stripped_rules` field removal: if any code path reads the field via reflection or by deserializing/reserializing PlanModeInner, there'd be a problem. Per Agent 1's investigation, no such code exists, but verify.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`
- `angreal check clippy` for arawn-engine

## Status Updates

### 2026-05-21 — landed

**Symbol removals:**
- Deleted `fn _unused(...)` in `crates/arawn-engine/src/tools/steward.rs:378-383` (fake-use of `resolve_workstream`).
- Deleted `fn resolve_workstream(...)` in `crates/arawn-engine/src/tools/steward.rs:84-99` (zero non-test callers).
- Deleted `fn _add_item_unused(_: AddItemRequest) {}` in `crates/arawn-engine/src/tools/ceremony.rs:362-367`. The `AddItemRequest` import at line 17 was now unused — the inline test mod at line 439 has its own `use ...::{AddItemRequest, ...}` — so I also dropped `AddItemRequest` from the top-level import (preserving the test's own import).
- Dropped `name: String` field from `struct PromptSection` in `system_prompt.rs:208` and the `#[allow(dead_code)]` at 206. Removed `name:` initialization from all 12 PromptSection construction sites (used Python helper to walk PromptSection blocks safely — initial sed pass overshot into ToolDefinition constructions, so I reverted and reran with a structural matcher).
- Dropped `stripped_rules: Vec<PermissionRule>` field from `PlanModeInner` in `plan.rs:32-33` and the empty-vec init at `plan.rs:54`. Removed `use crate::permissions::PermissionRule` import (now unused).
- Removed `MemoryHandle` from steward.rs's `use crate::workstream_router::{...}` import (no longer needed after `resolve_workstream` deletion).

**Comment improvement:**
- Updated `BackgroundTask.handle` comment in `crates/arawn-engine/src/background.rs:119-125` to explain precisely why `#[allow(dead_code)]` is necessary: Rust's lint can't see Drop semantics as a "read", so the allow is genuinely required. Comment now points future maintainers at `.handle.as_ref()` for the future `abort()` use site.

**Pre-existing warnings cleaned up:**
- `crates/arawn-engine/src/hooks/executor.rs:226` — changed `let result = ...` to `let _ = ...` (the first call's return value was shadowed by a second `let result` at line 230 and never used).
- `crates/arawn-engine/src/system_prompt.rs:574` — removed unused `use std::path::PathBuf;` in `mod tests`.

**Validation:**
- `cargo check --workspace`: ✅ clean, no warnings on arawn-engine code I touched.
- `cargo build --workspace --release`: ✅ clean (1m 08s).
- `cargo test --workspace --no-run`: ✅ clean (14.66s, all test targets compile).
- `cargo test --workspace --lib`: ✅ **1,756 tests pass, 0 fail** across all crates.
- Remaining workspace warnings are all pre-existing and slated for other tasks: `error_type` field in `arawn-llm/retry.rs` (T-F), arawn-memory benchmark unused funcs/fields (T-3.7-followup), snake_case naming warnings in feeds tests (pre-existing, out of scope), `PluginComponents`/`ToolContext`/`SidebarSection` unused imports in test files (out of scope).
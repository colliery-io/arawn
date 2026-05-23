---
id: t-a-restructure-arawn-engine-src
level: task
title: "T-A: Restructure `arawn-engine/src/testing.rs` — inline vs split (direction decided at task start)"
short_code: "ARAWN-T-0397"
created_at: 2026-05-22T01:46:52.952848+00:00
updated_at: 2026-05-22T01:55:35.119797+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-A: Restructure `arawn-engine/src/testing.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction (decided at task start)

**Split into focused pieces**, NOT inline. Rationale:

- `testing.rs` has 7 consumers in `arawn-tests/`: full_pipeline, hooks, memory_tools, skills, permissions, hot_reload, workflows. Inlining would mean ~30-50 lines × 7 callers of duplicated setup. The harness has real reuse value.
- The `feedback_inline_tests` operator memory targets `#[cfg(test)] mod tests` blocks, not test *helpers*. The harness is helper code, not test code.
- The file's 1,904 lines are 83% inline tests (1,586 lines) and 17% harness code (~317 lines). The tests stay inline per `feedback_inline_tests`; the harness code splits.

## Plan

Create `crates/arawn-engine/src/testing/` directory with four files:
- `mod.rs` — thin orchestrator + re-exports for `HarnessResult`, `TestHarness`, `TestHarnessBuilder` (the existing pub API stays identical).
- `result.rs` — `HarnessResult` struct + impl.
- `builder.rs` — `TestHarnessBuilder` struct + fluent methods + `build()` + `Default` impl.
- `harness.rs` — `TestHarness` struct (fields `pub(super)` so builder can construct) + impl + the inline `#[cfg(test)] mod tests` block.

Delete the old `testing.rs`.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-engine/src/testing.rs` deleted.
- [ ] Directory `crates/arawn-engine/src/testing/` with `mod.rs`, `result.rs`, `builder.rs`, `harness.rs`.
- [ ] Public API of `arawn_engine::testing::*` unchanged — all 7 consumer files in `arawn-tests/` compile without modification.
- [ ] `cargo check --workspace` clean, no new warnings.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `cargo test -p arawn-engine --lib testing` — all 44 harness tests pass.

## Status Updates

### 2026-05-22 — landed

**Split executed:**
- `testing/mod.rs` (22 lines): re-exports `HarnessResult`, `TestHarness`, `TestHarnessBuilder` — same pub API as before.
- `testing/result.rs` (35 lines): `HarnessResult` struct + accessors.
- `testing/builder.rs` (181 lines): `TestHarnessBuilder` struct + 11 fluent methods + `build()` + `Default`.
- `testing/harness.rs` (1,705 lines): `TestHarness` struct (with `pub(super)` fields so sibling `builder.rs` can construct via the struct literal in `build()`) + `impl TestHarness` + inline `#[cfg(test)] mod tests` block with 44 harness tests.

**Cross-module visibility:** `TestHarness` fields use `pub(super)` so `builder.rs`'s `build()` can construct the struct directly. Both files live under the `testing/` module, so `super` resolves to `testing` for both.

**Old file deleted:** `crates/arawn-engine/src/testing.rs` removed; rustc auto-discovers the `testing/` directory because `lib.rs` already declares `pub mod testing;`.

**Net effect on the hotspot:**
- Before: `testing.rs` 1,904 lines (single god-file).
- After: operative code split across 3 small focused files (35 + 181 + ~120 in harness.rs before the tests block = ~336 lines of real code). `harness.rs` is still big (1,705 lines) because the 44 inline tests live there — but the *operative harness code* is ~120 lines + tests, exactly as I-0054 intends.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 09s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-engine --lib testing`: ✅ **44 tests pass**, 0 fail.
- Consumer crates compile unchanged: arawn-tests' 7 callers still resolve `arawn_engine::testing::TestHarness` via the preserved `pub use`.
---
id: t-c-tier-1-arawn-engine-mechanical
level: task
title: "T-C: Tier 1 — arawn-engine mechanical cruft removal"
short_code: "ARAWN-T-0380"
created_at: 2026-05-21T14:53:18.441560+00:00
updated_at: 2026-05-21T14:53:18.441560+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*

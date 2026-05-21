---
id: t-a-workspace-dependency-cleanup
level: task
title: "T-A: Workspace dependency cleanup — drop dead `ignore` dep + dedupe arawn-embed"
short_code: "ARAWN-T-0378"
created_at: 2026-05-21T14:53:15.347580+00:00
updated_at: 2026-05-21T15:07:16.386262+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-A: Workspace dependency cleanup — drop dead `ignore` dep + dedupe arawn-embed

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove a workspace dependency that no code imports and deduplicate a redundant
dev-dependency declaration. Pure Cargo.toml cleanup with no Rust code changes.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Remove `ignore = "0.4"` from root `Cargo.toml` `[workspace.dependencies]`.
- [ ] Remove `ignore = { workspace = true }` from `crates/arawn-engine/Cargo.toml`.
- [ ] Remove the duplicate `arawn-embed = { path = "../arawn-embed" }` line from `crates/arawn-memory/Cargo.toml` `[dev-dependencies]` (the entry under `[dependencies]` already covers it).
- [ ] `cargo check --workspace` passes clean.
- [ ] `cargo build --workspace --release` passes clean.
- [ ] `cargo test --workspace --no-run` passes clean (modulo T-Z prerequisite).
- [ ] Grep confirms zero `use ignore::` or `ignore::` references in `crates/`.

## Implementation Notes

### Technical Approach

Both cleanups are pure manifest edits. Verify nothing else breaks by running
the three build profiles.

### Dependencies

None — this is the lowest-risk task in the initiative. Good warm-up.

### Risk Considerations

Negligible. Either the workspace dep is truly unused (verified by Agent 7
discovery) or removing it surfaces a hidden user that wasn't found — in
which case the build will fail loudly.

## Verification

Per the initiative's dead-code methodology, run:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`

## Status Updates

### 2026-05-21 — landed

- Removed `ignore = "0.4"` from root `Cargo.toml:62` and `ignore = { workspace = true }` from `crates/arawn-engine/Cargo.toml:37`.
- Removed duplicate `arawn-embed = { path = "../arawn-embed" }` from `crates/arawn-memory/Cargo.toml:23` `[dev-dependencies]`. Regular `[dependencies]` entry at line 7 carries through to dev/tests.
- Verified zero `use ignore::` / `ignore::` references in `crates/`.
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 07s).
- `cargo test --workspace --no-run`: ❌ pre-existing E0046 in `brief_pipeline.rs` (the T-0390/T-Z prerequisite) — task's acceptance criteria explicitly allow this until T-Z lands.
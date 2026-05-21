---
id: t-h-drop-enginetoolcontext-as
level: task
title: "T-H: Drop `EngineToolContext as ToolContext` backward-compat alias"
short_code: "ARAWN-T-0385"
created_at: 2026-05-21T14:53:26.235890+00:00
updated_at: 2026-05-21T14:53:26.235890+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-H: Drop `EngineToolContext as ToolContext` backward-compat alias

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove the backward-compat alias `pub use context::EngineToolContext as ToolContext;`
in `crates/arawn-engine/src/lib.rs:32-34`. Per operator decision (Tier 3
candidate 3.4): kill backward compats. Agent 1 verified zero internal callers.

## Acceptance Criteria

- [ ] Remove the alias and its surrounding doc comment at `crates/arawn-engine/src/lib.rs:31-34`. The line `pub use context::EngineToolContext;` (or equivalent — the canonical name) should remain so that `arawn_engine::EngineToolContext` continues to work.
- [ ] Grep across `crates/`, `examples/`, `vendor/` for `arawn_engine::ToolContext` (the alias path) — confirm zero hits.
- [ ] If grep finds any hit, decide per-callsite whether to migrate to `arawn_engine::EngineToolContext` (preferred) or `arawn_tool::ToolContext` (also valid, since `EngineToolContext` is an alias for the trait from `arawn-tool`).

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes.
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Grep `arawn_engine::ToolContext` workspace-wide.
2. Remove the alias.
3. Fix any callers.
4. Validate.

### Dependencies

None. Pairs well with T-G (other arawn-engine lib.rs cleanup) if batched.

### Risk Considerations

Low. The discovery agent confirmed zero internal callers; external callers
would surface via the build immediately.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`

## Status Updates

*To be added during implementation*

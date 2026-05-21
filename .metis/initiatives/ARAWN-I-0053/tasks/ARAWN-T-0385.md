---
id: t-h-drop-enginetoolcontext-as
level: task
title: "T-H: Drop `EngineToolContext as ToolContext` backward-compat alias"
short_code: "ARAWN-T-0385"
created_at: 2026-05-21T14:53:26.235890+00:00
updated_at: 2026-05-21T16:16:38.027224+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-21 — landed

- Removed `pub use context::EngineToolContext as ToolContext;` (and its backward-compat doc comment) from `crates/arawn-engine/src/lib.rs:31-33`. The canonical `pub use context::EngineToolContext;` re-export stays.
- Updated all callers (more than Agent 1's initial sweep had identified — the alias had real users):
  - `crates/arawn-tests/tests/tool_artifacts.rs:9` — `use arawn_engine::ToolContext as EngineToolContext;` → `use arawn_engine::EngineToolContext;` (the test was already calling the imported type `EngineToolContext`, so the alias was load-bearing).
  - `crates/arawn-workflow/src/agent_executor.rs:15` + line 114 — replaced `ToolContext` references with `EngineToolContext`.
  - `crates/arawn/src/local_service.rs` — 4 references (import + signature + 2 constructions) updated.
  - `crates/arawn-tests/tests/compaction.rs` — 5 references updated.
  - `crates/arawn-tests/tests/engine_persistence.rs` — 3 references updated.
- Note: this surfaced that Agent 1's "zero internal callers" finding during discovery was wrong — there were 5+ callers using the alias as a shorthand. The alias removal forced explicit naming, which is now consistent across the workspace.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 08s).
- `cargo test --workspace --no-run`: ✅ clean (5.68s).
- `cargo test -p arawn-tests --tests`: ✅ all integration test targets pass (compaction, engine_persistence, full_pipeline, hooks, hot_reload, local_service, memory_stack, memory_tools, permissions, plugin_components, skills, tool_artifacts, websocket, workflows, uat_fixture_smoke, uat_*_seed).
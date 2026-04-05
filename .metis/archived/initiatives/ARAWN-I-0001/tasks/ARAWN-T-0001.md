---
id: workspace-scaffolding-cargo
level: task
title: "Workspace scaffolding — Cargo workspace, crate stubs, CI, angreal tasks"
short_code: "ARAWN-T-0001"
created_at: 2026-03-31T17:37:34.485159+00:00
updated_at: 2026-03-31T18:54:41.567519+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# Workspace scaffolding — Cargo workspace, crate stubs, CI, angreal tasks

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Set up the clean-slate Cargo workspace with all 4 crate stubs compiling, angreal build/test/check tasks wired, and basic CI. This is the foundation everything else builds on.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Workspace `Cargo.toml` at repo root with 4 members: `crates/arawn`, `crates/arawn-core`, `crates/arawn-llm`, `crates/arawn-engine`
- [ ] Each crate has `Cargo.toml` + `src/lib.rs` (or `main.rs` for binary) with placeholder content
- [ ] `angreal build workspace` compiles successfully
- [ ] `angreal test unit` runs (even if no tests yet)
- [ ] `angreal check all` passes (fmt + clippy + check)
- [ ] Shared workspace dependencies in root `Cargo.toml` (`[workspace.dependencies]`)
- [ ] Common deps added: `tokio`, `serde`, `serde_json`, `thiserror`, `async-trait`, `anyhow`
- [ ] `.gitignore` updated for Rust workspace

## Implementation Notes
- Clean out any remnants of old code from the repo root (the analysis markdowns and claude-code reference can stay)
- Use workspace dependency inheritance so versions are managed in one place
- Binary crate (`arawn`) depends on the other three
- `arawn-engine` depends on `arawn-core` and `arawn-llm`
- `arawn-llm` and `arawn-core` are independent of each other

## Status Updates
- **2026-03-31**: Workspace scaffolded. 4 crates created with workspace dep inheritance. All compile, tests pass, fmt+clippy clean. Angreal tasks updated (removed stale runtime refs). Went slightly ahead on stubs — core types and LLM trait are in place since they're needed for cross-crate references.
---
id: t-b-split-arawn-engine-src-tools
level: task
title: "T-B: Split `arawn-engine/src/tools/workstream.rs` — one file per Tool struct"
short_code: "ARAWN-T-0398"
created_at: 2026-05-22T01:46:54.449837+00:00
updated_at: 2026-05-22T02:18:18.253326+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-B: Split `arawn-engine/src/tools/workstream.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-engine/src/tools/workstream.rs` deleted; replaced by `crates/arawn-engine/src/tools/workstream/` directory.
- [ ] One file per Tool struct under `tools/workstream/`.
- [ ] Public API of `arawn_engine::tools::Workstream*Tool` unchanged.
- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed

**Split executed via Python helper.** The original 2,316-line file split into 13 files under `tools/workstream/`:

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 917 | orchestrator + re-exports + inline tests (`#[cfg(test)] mod tests` block) |
| `session.rs` | 38 | `SessionWorkstream` + Default impl |
| `create.rs` | 177 | `WorkstreamCreateTool` |
| `list.rs` | 92 | `WorkstreamListTool` |
| `switch.rs` | 101 | `WorkstreamSwitchTool` |
| `show.rs` | 101 | `WorkstreamShowTool` |
| `describe.rs` | 69 | `WorkstreamDescribeTool` |
| `bind.rs` | 174 | `BindBackfillHook` trait + `WorkstreamBindTool` |
| `unbind.rs` | 144 | `UnbindHook` trait + `WorkstreamUnbindTool` + `collect_child_feed_ids` helper |
| `promote.rs` | 164 | `WorkstreamPromoteTool` |
| `delete.rs` | 72 | `WorkstreamDeleteTool` |
| `propose_ontology.rs` | 180 | `WorkstreamProposeOntologyTool` |
| `util.rs` | 167 | github-scope parsing (`GithubScope`, `parse_github_scope`, `is_github_scope_binding`, `validate_github_scope_scheme`) + feed-binding helpers (`find_workstreams_binding`, `upsert_repo_mirror_feed`, `delete_feed`, `extract_json_block`) |

The private helpers in `util.rs` were promoted to `pub(super)` so siblings can use them. Public functions kept `pub` and re-exported from `mod.rs`.

**Approach:** Python script extracted lines per block, prepended a common import preamble, then trimmed trailing leading-comments-for-next-block. `cargo fix --lib -p arawn-engine` cleaned up unused-import warnings from the over-broad preamble (10 fixes across 4 files).

**Cross-module visibility:**
- `util.rs` private helpers → `pub(super)` so siblings can import.
- Trait imports adjusted in `bind.rs` and `unbind.rs` to avoid colliding with the traits they DEFINE.
- `GithubScope` enum added to the per-tool imports where it's referenced.

**Inline tests in `mod.rs`** needed explicit imports (the file previously had top-level `use std::sync::{Arc, Mutex}`; now those are encapsulated in submodules). Added `use super::util::*; use std::sync::{Arc, Mutex}; use arawn_core::{SCRATCH_NAME, Workstream}; use arawn_storage::Store; use arawn_tool::Tool; use serde_json::json; use uuid::Uuid;` to the test mod.

**Validation:**
- `cargo check --workspace`: ✅ clean (no new warnings).
- `cargo build --workspace --release`: ✅ clean.
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-engine --lib tools::workstream`: ✅ **36 tests pass**, 0 fail.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.

**Net result:** the 2,316-line god-file is gone. Largest remaining file in the cluster is `mod.rs` at 917 lines, but ~800 of those are inline tests; the operative orchestration is ~117 lines.
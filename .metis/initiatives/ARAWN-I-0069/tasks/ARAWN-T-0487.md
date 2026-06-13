---
id: p3-6-directory-glob-scoped
level: task
title: "P3-6: Directory/glob-scoped permission grant shapes (engine)"
short_code: "ARAWN-T-0487"
created_at: 2026-06-13T14:43:30.055625+00:00
updated_at: 2026-06-13T15:05:48.829021+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-6: Directory/glob-scoped permission grant shapes (engine)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — implements P3-6 (MEDIUM, engine-side — benefits TUI and the future GUI alike).

## Objective **[REQUIRED]**

Widen session permission grants from exact-shape-only to **directory/glob-scoped**, so approving an operation on `~/x/foo.rs` covers `~/x/bar.rs` too — users stop re-approving near-identical operations in every client.

**The defect** (`crates/arawn-engine/src/permissions/checker.rs:138-144`): session grants match exact shapes with only a wildcard fallback; a grant for one path doesn't generalize to siblings in the same directory.

### Type
- [x] Feature / Tech Debt — permission ergonomics (engine, all clients)

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A granted operation can be scoped to the parent directory (or a glob), so a subsequent near-identical operation under that scope is auto-allowed without re-prompting.
- [ ] Scope widening is conservative: a file grant covers the file's immediate parent dir (or an explicit glob), never an ancestor — it can't silently become a whole-home grant. Deny rules stay authoritative over grants.
- [ ] Engine-side so TUI + GUI both benefit; the audit trail records the granted scope.
- [ ] Inline permission tests: grant on `dir/a` allows `dir/b`; does NOT allow `other_dir/c`; non-path grants unaffected; a deny is never widened.
- [ ] `angreal check all` + `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In `permissions/checker.rs`, extend the grant representation to carry a scope (exact | dir | glob) and match a candidate operation against held grants by scope. Default new path-bearing grants to directory scope (or surface the choice). Keep deny rules authoritative; record the scope in the audit.

### Dependencies
Engine `permissions` module. Interacts with the permission-audit surface (T-0476 status) — record the scope.

### Risk Considerations
Over-broadening is the danger — immediate parent dir only, never ancestors, never widen a deny. Make the scope explicit in the audit so an over-grant is visible.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**

**Finding first:** directory scoping for `file_write`/`file_edit` *already* existed via T-0276's `ArgShape::file_shape`, which folds the target to its home-folded parent dir — so a grant on `~/x/foo.rs` already covered `~/x/bar.rs` for *those two* tools. The real remaining gap (and a latent over-grant): any *other* path-bearing tool (`Edit`, `Write`, etc.) collapses to the `"<tool>:*"` wildcard, so "Allow Always" granted the **whole tool for every path** — the opposite of conservative. And the scope was implicit in a shape string, never auditable.

**What this task added:**
- `approval::target_parent_dir(raw_input) -> Option<String>` (refactored out of `file_shape`, now shared) — extracts `path`/`file_path`, returns the home-folded **immediate parent** dir, or `None` for non-path calls.
- `permissions::checker::GrantScope { Exact, Directory(String) }`, recorded in `DecisionReason::SessionGrant(GrantScope)` and surfaced in the audit as `session grant (dir: <dir>)`.
- `SessionGrants` gains a `dir_grants: HashSet<(tool, dir)>` set + `grant_dir()` and `granted_scope()` (exact/wildcard first, then directory).
- **Conservative narrowing at grant time** (`prompt_user`, Allow Always): a path-bearing call whose shape is *only* the bare wildcard now records a **directory grant instead of** the whole-tool wildcard — so `Edit` on `dir/a` covers `dir/b` but no longer silently covers every other directory. Tools with an already-specific shape (file_write) keep their shape grant + a dir grant; non-path tools (shell/env) are unchanged.
- Deny rules remain authoritative (deny is evaluated before grants in `check_explained`).

**Tests (4 new):** `dir_grant_covers_siblings_not_other_dirs` (sibling ok, other dir denied, ancestor denied, no whole-tool wildcard recorded), `dir_grant_auto_allows_sibling_end_to_end_and_audits_scope` (audit reason carries `dir: /srv/x`, proving grant short-circuit vs. a fresh prompt), `deny_rule_overrides_directory_grant`, `non_path_grant_records_no_directory_scope`.

`cargo test -p arawn-engine --lib`: 739 passed. Gate clippy clean; fmt clean. Engine-side, so TUI + future GUI both benefit. Glob scope left as a documented extension point (directory satisfies the AC; glob would need a prompter choice — out of this engine-only slice).
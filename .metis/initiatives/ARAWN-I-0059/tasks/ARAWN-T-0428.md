---
id: t-a-code-storage-wire-rename
level: task
title: "T-A: Code + storage + wire rename workstream → lens (atomic)"
short_code: "ARAWN-T-0428"
created_at: 2026-05-27T01:53:54.645872+00:00
updated_at: 2026-05-27T01:55:29.908424+00:00
parent: ARAWN-I-0059
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0059
---

# T-A: Code + storage + wire rename workstream → lens (atomic)

## Parent Initiative

[[ARAWN-I-0059]]

## Objective

Rename `workstream` → `lens` across all Rust code, storage, and wire/API in one
atomic landing (the workspace won't compile while a shared type is half-renamed).
No behavior change; pre-release so the data model breaks freely (no migration).

## Type
Tech Debt / Chore — cross-crate mechanical rename.

## Scope

- **Identifiers** (case-aware, plural-first): `Workstream→Lens`,
  `workstreams→lenses`, `workstream→lens`, `WORKSTREAM→LENS`. Types
  (`WorkstreamStore→LensStore`, DTOs, enums), fields, vars, fn names, comments,
  log/help strings.
- **Module/file renames**: `arawn-core/src/workstream.rs`,
  `arawn-storage/src/workstream_store.rs`, `arawn-engine/src/tools/workstream/`
  (+ `mod`/`use` paths).
- **Agent tools** (`name()` strings + any registry/schemas): `workstream_*` →
  `lens_*` — incl. `workstream_new/list/show/describe/switch/bind/unbind/
  promote/delete/propose_ontology`, steward `workstream_journal/refine/rollback/
  dust/apply/tag`, retro detector `workstream_neglect`, context `workstream_name`.
- **Slash command**: `/workstream` → `/lens` (hard cut, no alias); CLI flags
  (`--workstream` etc.) and TUI sidebar labels.
- **Wire/DTO**: `workstream_id` → `lens_id`, `WorkstreamInfo`, etc., across
  `arawn-service`, ws_server/ws_client, RPC method names if any (`workstream.*`).
- **Storage (breaking, no migration)**: `CREATE TABLE workstreams` → `lenses`
  and all queries; on-disk dir `<data_dir>/workstreams/<slug>/` → `lenses/<slug>/`
  (path-join helpers).
- **Tests/fixtures**: `workstream_id` JSON in websocket/uat tests, `uat_fixture`,
  seed objectives — update so they pass.

Leave unchanged: `tags_ontology` (already correct), the `scratch` default slug,
anything spelled "workstream" only inside unrelated third-party output.

## Acceptance Criteria

## Acceptance Criteria

- [ ] No `workstream` (any case) remains in `crates/**/*.rs` except where it
      legitimately refers to the AWEN project or external strings — verified by
      `rg -i workstream crates`.
- [ ] Module/file renames done with paths updated; `arawn-core`, `arawn-storage`,
      `arawn-engine`, `arawn-service`, `arawn`, `arawn-tui`, ceremonies, steward,
      feeds, projections, memory all compile.
- [ ] Agent tools register under `lens_*`; `/lens` works, `/workstream` is gone.
- [ ] DB schema creates `lenses`; on-disk dir is `lenses/<slug>/`.
- [ ] `angreal check workspace` clean (incl. clippy/fmt via `angreal check all`);
      `angreal test unit` + integration/fixtures green.
- [ ] A fresh run creates a `lenses/` data dir and `/lens list` shows `scratch`.

## Technical Approach

Drive with case-aware `rg`/`sed` sweeps per directory, plural-first
(`workstreams→lenses`) then singular, then `Workstream→Lens`, `WORKSTREAM→LENS`.
`git mv` the module/doc files first, fix `mod`/`use`, then sweep contents.
Compile crate-by-crate in dependency order (core → storage → engine → service →
arawn/tui) to localize breakage. Read each diff hunk class once to catch
false positives (rare for this project-specific term).

## Dependencies
Root of the initiative. Independent of T-B (docs), but ideally lands first so
docs reference the final names.

## Risk Considerations
- Blind sed could touch an unintended occurrence — mitigate by reviewing `rg`
  output classes and compiling incrementally.
- Wire rename means TUI + server must move together (they do, same repo).
- No migration: any existing dev `~/.arawn/workstreams` data is abandoned; that's
  accepted (pre-release).

## Status Updates

**2026-05-27 — Done** (commit `c0f1d6d`, branch `rename/workstream-to-lens`).
- `git mv` of all workstream-named files/dirs (core `lens.rs`, `lens_router.rs`,
  `tools/lens/`, builtin `lens-create.md`, `lens_store.rs`, migrations
  V3/V4/V10, `local_service/lenses.rs`, sidebar snapshot).
- Case-aware sweep (plural-first) over 452 code/sql/skill/snap files + UAT
  fixture JSON. `tags_ontology` + `scratch` left intact.
- 3 sidebar insta snapshots regenerated (border width shrank with the shorter
  word); verified the only diff was padding, then accepted.
- `rg -i workstream crates` → NONE. `angreal check workspace` + fmt clean;
  `angreal test unit` = 2072 passed / 0 failed / 82 suites.

NOTE: Metis MCP index went stale this session (couldn't see I-0059 created this
session), so this status + phase were set by editing the file directly. Truth is
the markdown on disk.
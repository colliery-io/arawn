---
id: t-d-tier-1-arawn-ceremonies
level: task
title: "T-D: Tier 1 — arawn-ceremonies mechanical cruft removal"
short_code: "ARAWN-T-0381"
created_at: 2026-05-21T14:53:19.953836+00:00
updated_at: 2026-05-21T15:33:29.878314+00:00
parent: ARAWN-I-0053
blocked_by: [ARAWN-T-0390]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-D: Tier 1 — arawn-ceremonies mechanical cruft removal

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Delete confirmed-dead symbols, convert test-only helpers to `#[cfg(test)]`,
collapse a backward-compat name alias, and remove an unused test-helper
parameter in `arawn-ceremonies`.

**Blocked by [[ARAWN-T-0390]]**: the broken `brief_pipeline.rs` integration
test must be fixed first so lint analysis runs cleanly on this crate
(see initiative inventory Tier 4 for context).

## Acceptance Criteria

## Acceptance Criteria

### Symbol removals

- [ ] Delete `fn status_str(TabletStatus) -> &'static str` at `crates/arawn-ceremonies/src/service.rs:907-913` and its `#[allow(dead_code)]`. Verified zero callers.
- [ ] Delete `fn rollback(conn: &ConnHandle) -> Result<(), CeremonyError>` at `crates/arawn-ceremonies/src/engine.rs:526-535` and its `#[allow(dead_code)]`. Verified zero callers (inline tests use `begin` and `commit` but not `rollback`).

### Convert to `#[cfg(test)]`

- [ ] Convert `fn begin(...)` at `crates/arawn-ceremonies/src/engine.rs:504-513` from `#[allow(dead_code)]` to `#[cfg(test)]`. Used by the inline test at line 830.
- [ ] Convert `fn commit(...)` at `crates/arawn-ceremonies/src/engine.rs:515-524` the same way. Used by the inline test at line 840.

### Collapse alias

- [ ] Rename `monday_sunday_for_iso_week_public` → `monday_sunday_for_iso_week` in `crates/arawn-ceremonies/src/plugins/retro.rs`. Update every caller (use grep).
- [ ] Remove the `use ... as monday_sunday_for_iso_week;` re-alias at `crates/arawn-ceremonies/src/plugins/retro_detectors.rs:26`.

### Remove unused test-helper parameter

- [ ] Remove the `tablet_id_prefix: &str` parameter from `fn build_service_with_items(...)` at `crates/arawn-ceremonies/src/service.rs:971-975`.
- [ ] Update all 9 call sites in `service.rs` (around lines 1011, 1021, 1033, 1065, 1085, 1141, 1172, 1227, 1540) to pass 2 args instead of 3.
- [ ] Remove the `let _ = tablet_id_prefix;` line at `crates/arawn-ceremonies/src/service.rs:1005`.

### Validation

- [ ] After deletions and conversions, re-run the dead-code methodology: strip remaining `#[allow(...)]` in arawn-ceremonies and rebuild on debug + release + test-no-run.
- [ ] With T-Z landed, `cargo test --workspace --no-run` should now produce CLEAN lint signal for arawn-ceremonies — no false-positive "never used" on `begin`/`commit`.
- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes (specifically the arawn-ceremonies tests, including the ones at engine.rs:790-843 that exercise begin/commit/dispatch_for).
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Wait for T-Z to land (broken test fix).
2. Apply symbol removals (`status_str`, `rollback`).
3. Convert `begin`/`commit` to `#[cfg(test)]`. The simplest form is wrapping the two `fn` definitions individually with `#[cfg(test)]`, since they're at module scope.
4. Collapse the `monday_sunday_for_iso_week_public` alias.
5. Strip `tablet_id_prefix` from the test helper and its 9 call sites.
6. Validate.

### Dependencies

- **Hard:** [[ARAWN-T-0390]] (T-Z) must be merged first for clean lint signal.
- **None internal** to the initiative otherwise.

### Risk Considerations

- The `begin`/`commit` `#[cfg(test)]` conversion is the only non-trivial change — if the file imports them from a path that's compiled in non-test builds, the conversion will fail compilation. Verify by also running `cargo build` (non-test) after the change.
- Renaming `monday_sunday_for_iso_week_public` may surface external callers if any exist outside of the in-crate use sites. Grep workspace-wide.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run` (requires T-Z first)
- `angreal test unit`
- `angreal test integration`
- `angreal check clippy` for arawn-ceremonies

## Status Updates

### 2026-05-21 — landed

- Deleted `fn rollback` in `crates/arawn-ceremonies/src/engine.rs:526` (truly dead — no callers anywhere).
- Converted `fn begin` and `fn commit` in engine.rs:504/515 from `#[allow(dead_code)]` to `#[cfg(test)]`. They're used by the inline test at engine.rs:830/840.
- Deleted `fn status_str` in `crates/arawn-ceremonies/src/service.rs:910` and its `#[allow(dead_code)]`. Removed now-unused `TabletStatus` import from line 30.
- Collapsed the `monday_sunday_for_iso_week_public` alias: deleted the wrapper at `plugins/retro.rs:687-693`; made the underlying `fn monday_sunday_for_iso_week` `pub(crate)` and updated its doc-comment to reference the catalog re-use. Updated the import in `plugins/retro_detectors.rs:26` from `... as monday_sunday_for_iso_week` to a direct import.
- Removed `tablet_id_prefix: &str` parameter from `build_service_with_items` in service.rs:963-967. Updated all 9 call sites via sed (verified `tablet_id_prefix` no longer appears in the file). Removed the `let _ = tablet_id_prefix;` line.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 08s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-ceremonies`: ✅ **149 tests pass** (145 lib + 2 brief_pipeline + 2 retro_uat), 0 fail.
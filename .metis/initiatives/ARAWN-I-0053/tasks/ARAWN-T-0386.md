---
id: t-i-delete-pre-t-0276-wildcard
level: task
title: "T-I: Delete pre-T-0276 wildcard permission API (`grant`, `is_granted`)"
short_code: "ARAWN-T-0386"
created_at: 2026-05-21T14:53:27.878327+00:00
updated_at: 2026-05-21T16:21:57.520885+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-I: Delete pre-T-0276 wildcard permission API (`grant`, `is_granted`)

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove the two wildcard-grant backward-compat methods on `SessionGrants`
that pre-date T-0276's shape-aware permission API. Per operator decision
(Tier 3 candidate 3.2): drop — no deprecation cycle. Replaced by the
shape-aware `grant_shape()` / `is_granted_shape()` API.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Delete `SessionGrants::grant(...)` method (the wildcard variant) at `crates/arawn-engine/src/permissions/checker.rs:130-138`.
- [ ] Delete `SessionGrants::is_granted(...)` method (the wildcard variant) at `crates/arawn-engine/src/permissions/checker.rs:140-149`.
- [ ] Inspect `is_granted_shape()` at `crates/arawn-engine/src/permissions/checker.rs:151-160`. Its doc comment notes a "falls back to wildcard" behavior. After the wildcard API is gone, that fallback may be a self-fallback (a shape-keyed wildcard entry) rather than a call to the deleted methods. Verify the implementation is internally consistent and update the doc if needed.
- [ ] Grep `crates/`, `examples/`, `vendor/` for `.grant(` and `.is_granted(` invocations on `SessionGrants` variables. There should be zero non-test calls. Test code calling the deleted methods should be migrated to `grant_shape`/`is_granted_shape` or deleted if redundant.
- [ ] Update the existing test `permission_mode_legacy_strings_fail` at `crates/arawn-engine/src/permissions/checker.rs:994` if it references the deleted methods.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes — especially permission-related tests.
- [ ] `angreal test integration` passes.
- [ ] `angreal test uat` should be run after this task lands as part of the tier-3 boundary verification (operator may choose to batch with T-J/T-K for one UAT run).

## Implementation Notes

### Technical Approach

1. Grep to confirm zero non-test callers.
2. Delete the two methods.
3. Update or delete tests that exercised the old API.
4. Validate.

### Dependencies

None internal to the initiative. Pairs well with T-J (other permission/storage
deletions) if batched.

### Risk Considerations

- If external code (e.g., a plugin or workflow) calls the wildcard API, the build will fail loudly. Migrate or delete those callers.
- `is_granted_shape()`'s internal fallback may have depended on the wildcard methods' implementation. Read the code carefully to ensure the fallback still works after the methods are gone.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`
- `angreal check clippy` for arawn-engine

Tier-3 boundary UAT (`angreal test uat` + `angreal test uat-judge`) should
run after T-I/T-J/T-K all land.

## Status Updates

### 2026-05-21 — landed

- Deleted `SessionGrants::grant(tool_name: String)` and `SessionGrants::is_granted(tool_name: &str)` from `crates/arawn-engine/src/permissions/checker.rs:132-149` (the pre-T-0276 wildcard variants).
- Updated the `SessionGrants` doc comment to drop the "historical zero-arg ... API survives" wording and instead document the wildcard `"<tool>:*"` shape convention that `is_granted_shape` falls back to.
- `is_granted_shape` was already self-contained (its wildcard fallback constructs an `ArgShape` directly rather than calling the deleted methods) — no logic changes needed there.
- Migrated 5 test call sites in `checker.rs`:
  - `.is_granted("Bash")` → `.is_granted_shape("Bash", &crate::approval::ArgShape("Bash:*".into()))` (lines 645, 658)
  - `.grant("Bash".to_string())` → `.grant_shape("Bash".to_string(), crate::approval::ArgShape("Bash:*".into()))` (lines 805, 879)
  - `.grant("think".to_string())` → `.grant_shape("think".to_string(), crate::approval::ArgShape("think:*".into()))` (line 819)
- Verified zero remaining non-`_shape` callers via grep (the only `.grant(...)` / `.is_granted(...)` calls left are on the inner `SessionAllowlist`, a different type with a 2-arg signature).

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 27s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-engine --lib`: ✅ **681 tests pass**, including the permission checker tests that exercise the migrated wildcard semantic.
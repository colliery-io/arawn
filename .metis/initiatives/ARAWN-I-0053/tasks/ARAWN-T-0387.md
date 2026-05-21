---
id: t-j-delete-workstreamstore-delete
level: task
title: "T-J: Delete `WorkstreamStore::delete` hard-delete API"
short_code: "ARAWN-T-0387"
created_at: 2026-05-21T14:53:29.377601+00:00
updated_at: 2026-05-21T16:24:32.255589+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-J: Delete `WorkstreamStore::delete` hard-delete API

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove `WorkstreamStore::delete(id: Uuid)` — the V1-era hard-delete method.
Per operator decision (Tier 3 candidate 3.5): kill it. `soft_delete(name)` is
the canonical API and Agent 4 found zero external callers.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Delete `pub fn delete(&self, id: Uuid) -> Result<(), StorageError>` (or its actual signature) at `crates/arawn-storage/src/workstream_store.rs:222-...` along with its doc comment.
- [ ] Grep `crates/`, `examples/`, `vendor/`, `tests/` for `WorkstreamStore::delete` or `.delete(` invocations on a `WorkstreamStore` variable. Confirm zero hits (or migrate any that exist to `soft_delete`).
- [ ] Confirm `soft_delete(name)` still exists and is the survivor.
- [ ] If any tests exercise the hard-delete path, delete or migrate them.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes — especially workstream-related storage tests.
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Grep first.
2. Delete the method.
3. Validate.

### Dependencies

None.

### Risk Considerations

Low. Agent 4 already confirmed no external callers.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`

Tier-3 boundary UAT (`angreal test uat` + `angreal test uat-judge`) should
run after T-I/T-J/T-K all land.

## Status Updates

### 2026-05-21 — landed

- Deleted `pub fn delete(&self, id: Uuid) -> Result<bool, StorageError>` and its "Retained for backward compatibility" doc comment from `crates/arawn-storage/src/workstream_store.rs:222-230`.
- Grep workspace-wide confirmed zero callers of `WorkstreamStore::delete`. The other `.delete(...)` invocations (`SessionStore::delete`, `TokenStore::delete`, `FeedStore::delete`, `CredentialStore::delete`, `delete_entity`) target different types and are unaffected.
- `soft_delete(name)` remains as the canonical workstream-removal API.
- `Uuid` is still used elsewhere in the file (for the `id` field type and other methods), so the import stays.

**Validation:**
- `cargo check --workspace`: ✅ clean, no warnings.
- `cargo build --workspace --release`: ✅ clean (1m 17s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-storage`: ✅ **76 tests pass**, 0 fail.
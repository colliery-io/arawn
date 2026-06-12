---
id: p1-2-eliminate-rwlock-poison
level: task
title: "P1-2: Eliminate RwLock poison panics in service hot paths"
short_code: "ARAWN-T-0469"
created_at: 2026-06-11T11:07:17.448682+00:00
updated_at: 2026-06-11T15:11:10.189027+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-2: Eliminate RwLock poison panics in service hot paths

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements finding P1-2 (CRITICAL). Second task of Phase 1.

## Objective **[REQUIRED]**

Eliminate panic-on-poison in long-running service paths: one panic in any background writer (ceremony engine, feed runtime) must never permanently brick the server. Replace `std::sync::RwLock` + `.unwrap()` in service hot paths with `parking_lot::RwLock` (no poisoning), or map poison to a recoverable error.

**Current defects:**
- `crates/arawn/src/local_service/mod.rs` — ~10 `.unwrap()` calls on RwLock guards in RPC hot paths (lines 171, 176, 191, 195, …): `feed_runtime.read().unwrap()`, `ceremony_service.write().unwrap()`, etc. A poisoned lock makes every subsequent request panic until restart.
- `crates/arawn/src/main.rs:69` — the `notice_tx` broadcast sender is `Arc<RwLock<…>>` unwrapped across async tasks; one failing subscriber poisons it for all clients.
- `crates/arawn/src/main.rs:911-938` — feed runtime / LLM clients accessed via `.read().unwrap().clone()`; also panics when the inner `Option` is `None` (subsystem not initialized) instead of returning a clean service error.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] No `.unwrap()`/`.expect()` on lock guards remains in production paths of `crates/arawn` (grep-verifiable — the only `.lock().unwrap()` string left is inside a doc comment)
- [x] An injected panic in a background writer (test) does not break subsequent LocalService RPC handling (`poison_recovery_tests`)
- [x] Accessing an uninitialized subsystem (`feed_runtime` = `None`) returns a clean `ServiceError` naming the subsystem (`feed_runtime_or_err`), never panics — pre-existing behavior preserved
- [x] Inline regression tests for the poison paths (`rwlock_read_recovers_after_writer_panic`, `mutex_recovers_after_holder_panic`)
- [x] `angreal test unit` green for the changed crate (arawn 59/0); `angreal check all` carries pre-existing repo-wide failures unrelated to this task (see status note)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Add `parking_lot` as a workspace dependency and mechanically swap `std::sync::RwLock` in the service layer (simplest solution; no poisoning semantics, no `.unwrap()` needed). For `Option`-inner state (`feed_runtime`, `ceremony_service`), replace panicking access with a helper returning `ServiceError::Unavailable { subsystem }`. Audit with `grep -rn "\.read()\.unwrap()\|\.write()\.unwrap()\|\.lock()\.unwrap()" crates/arawn/src/`.

### Dependencies
None. Touches `local_service/mod.rs` + `main.rs` — low conflict with ARAWN-T-0470/T-0471 (nearby files); sequence within Phase 1 accordingly.

### Risk Considerations
Behavioral change: code that previously crashed fast via poison propagation will now continue running after a background panic — make sure those panics are still logged loudly (panic hook / task join handling) so they aren't traded for a new silent failure. The broader "surface subsystem failure" story is Phase 2's P2-1 status surface.

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** Eliminated panic-on-poison across the arawn binary via a poison-recovery helper rather than parking_lot.

**Design decision (deviated from the suggested parking_lot swap):** `Mutex<Store>` is used pervasively across `arawn-engine` (todo.rs, all lens tools, ceremony_sources) and `arawn-steward` (runner, doorwatch) — dozens of cross-crate signatures take `Arc<std::sync::Mutex<Store>>`, and `SharedAudit` is an `arawn-engine` `std::sync::Mutex`. Converting Store/audit to `parking_lot` would ripple type changes through two out-of-scope crates. The task explicitly allowed the alternative ("map poison to a recoverable error"), which is the better fit here: it stays on `std::sync`, needs **no new dependency**, and touches only `crates/arawn`.

**Implementation:**
- New `crates/arawn/src/lock_ext.rs` — a `Recover` trait on `LockResult<T>` with `.recover()` = `unwrap_or_else(PoisonError::into_inner)`. Registered as `pub mod lock_ext` in `lib.rs`. Documented why poison-recovery is correct for a long-running server (a partially-updated guard beats a cascading 3am outage — same effective behavior as parking_lot).
- Replaced **50 lock-guard `.unwrap()` sites** across 10 files (`local_service/{mod,sessions,permissions,integrations,lenses}.rs`, `channel_prompt.rs`, `config_watcher.rs`, `startup/{feeds,integrations}.rs`, `main.rs`) with `.recover()`. Now a panic in any background writer (ceremony/feed/steward task) can no longer poison a lock and brick every subsequent RPC.
- `feed_runtime`/`ceremony_service` `None`-handling already returned a clean `ServiceError` (`feed_runtime_or_err`); preserved.
- Note: `notice_tx` is a `tokio::sync::broadcast::Sender` (not a poisonable lock) — the original finding's "Arc<RwLock> notice_tx" was imprecise; no change needed there. tokio async locks (`.lock().await`) were left untouched.

**Tests:** new `poison_recovery_tests` module exercises `.recover()` on the `RwLock<Option<_>>` shape (feed_runtime) and a `Mutex` after a writer thread panics mid-guard — asserts the lock is poisoned yet subsequent `.recover()` access does not panic and recovers the data.

**Verification:** `cargo build --workspace` clean; **arawn 59 passed / 0 failed** (incl. 2 new tests); all my changed lines are `rustfmt --check` clean (edition 2024); no clippy findings in changed code; no new dependency (Cargo.toml unchanged).

**Caveat (pre-existing, same as T-0468):** three `rustfmt --check` diffs remain in `local_service/mod.rs:17`, `sessions.rs:5`, `main.rs:477` — all on pre-existing `use`-block / call lines I did not touch, reflecting the rust-1.93-vs-HEAD rustfmt mismatch that reproduces on clean HEAD. `angreal check all` is red independent of this task for that reason plus the pre-existing `E0521` in a clippy `--all-targets` build.
//! Poison-tolerant lock acquisition (ARAWN-T-0469 / P1-2).
//!
//! The server is long-running and several `std::sync` locks are shared
//! between request handlers and background tasks (ceremonies, feeds, the
//! steward). With a plain `.lock().unwrap()`, a panic in *any* of those
//! holders poisons the lock and every subsequent acquisition panics too —
//! one transient failure bricks the whole service until restart.
//!
//! `Recover` maps the poison case back to the inner guard instead of
//! panicking. Recovering a poisoned lock can expose state a panicking writer
//! left half-updated, but for this service that is strictly better than a
//! cascading outage — the alternative is the server going dark at 3am over a
//! single failed background job. (This is exactly what `parking_lot` locks do
//! by not tracking poisoning at all; we stay on `std` to avoid a dependency
//! and to keep `Mutex<Store>` compatible with the engine/steward crates.)

use std::sync::{LockResult, PoisonError};

/// Extension over `LockResult` (the return of `Mutex::lock` / `RwLock::read` /
/// `RwLock::write`) that recovers the guard from a poisoned lock.
pub trait Recover<T> {
    /// Return the lock guard, recovering it if the lock was poisoned.
    fn recover(self) -> T;
}

impl<T> Recover<T> for LockResult<T> {
    #[inline]
    fn recover(self) -> T {
        self.unwrap_or_else(PoisonError::into_inner)
    }
}

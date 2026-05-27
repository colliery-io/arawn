//! Process-wide LLM resource gate.
//!
//! Every subsystem that invokes an LLM funnels through this gate
//! before making the call. The gate exists to bound concurrent
//! local-bound work — Ollama is effectively serial; concurrent
//! requests stack memory and have crashed users' laptops in
//! practice. Cloud-bound calls flow through the same gate today;
//! the original plan to distinguish them via permit type was reverted
//! along with the (premature) routing layer.
//!
//! # API
//!
//! - [`acquire_local`] — `.await` returns a [`LocalPermit`] once a
//!   slot is available. The permit is RAII — dropping it releases
//!   the slot.
//! - [`try_acquire_local`] — synchronous; returns
//!   [`AcquireError::Busy`] instead of waiting when the slot is full.

use std::sync::OnceLock;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Number of concurrent `LocalPermit`s allowed across the process.
/// Set to 1 because Ollama is effectively serial on consumer hardware
/// — concurrent calls stack memory and have OOM-killed laptops.
const LOCAL_SLOTS: usize = 1;

/// Errors returned when an acquire cannot proceed immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireError {
    /// No slots are free. Only returned by [`try_acquire_local`];
    /// `acquire_local` waits instead.
    Busy,
}

/// RAII permit for a local-bound LLM call. Dropping returns the slot
/// to the semaphore.
#[derive(Debug)]
pub struct LocalPermit {
    _inner: OwnedSemaphorePermit,
}

static SEMAPHORE: OnceLock<std::sync::RwLock<std::sync::Arc<Semaphore>>> = OnceLock::new();

fn semaphore() -> &'static std::sync::RwLock<std::sync::Arc<Semaphore>> {
    SEMAPHORE
        .get_or_init(|| std::sync::RwLock::new(std::sync::Arc::new(Semaphore::new(LOCAL_SLOTS))))
}

/// Acquire a `LocalPermit`, waiting if every slot is full. Returns
/// `AcquireError::Busy` only if the underlying semaphore is closed,
/// which doesn't happen in production (the semaphore is never closed).
/// The `Result` return type is kept for caller compatibility.
pub async fn acquire_local() -> Result<LocalPermit, AcquireError> {
    let sem = semaphore().read().unwrap().clone();
    sem.acquire_owned()
        .await
        .map(|permit| LocalPermit { _inner: permit })
        .map_err(|_| AcquireError::Busy)
}

/// Non-blocking variant. Returns immediately with `Busy` if the slot
/// is full.
pub fn try_acquire_local() -> Result<LocalPermit, AcquireError> {
    let sem = semaphore().read().unwrap().clone();
    sem.try_acquire_owned()
        .map(|permit| LocalPermit { _inner: permit })
        .map_err(|_| AcquireError::Busy)
}

/// Test-only: reset the semaphore to a fresh `LOCAL_SLOTS`-permit
/// instance. Production callers never touch this — the gate is sticky
/// for the life of the process.
#[cfg(test)]
pub fn reset_for_test() {
    let sem = semaphore();
    *sem.write().unwrap() = std::sync::Arc::new(Semaphore::new(LOCAL_SLOTS));
}

/// Test-only: a process-wide mutex that gate-mutating tests should
/// hold for their duration. Prevents two parallel tests from stepping
/// on the shared semaphore.
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_and_reset() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_for_test();
        guard
    }

    #[tokio::test]
    async fn local_acquires_serialise_behind_one_slot() {
        let _guard = lock_and_reset();
        let _permit = acquire_local().await.expect("first acquire");
        match try_acquire_local() {
            Err(AcquireError::Busy) => {}
            other => panic!("expected Busy with 1-slot held, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn local_acquire_proceeds_after_first_drops() {
        let _guard = lock_and_reset();
        let permit = acquire_local().await.expect("first acquire");
        drop(permit);
        let _second = try_acquire_local().expect("second acquire after drop");
    }
}

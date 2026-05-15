//! Process-wide LLM resource gate.
//!
//! Every subsystem that invokes an LLM funnels through this gate
//! before making the call. The gate exists to bound concurrent
//! *local-bound* work — Ollama is effectively serial; concurrent
//! requests stack memory and have crashed the user's laptop in
//! practice. Cloud-bound calls bypass the slot budget because the
//! bottleneck there is bandwidth, not local RAM.
//!
//! # API
//!
//! - [`acquire_local`] — `.await` returns a [`LocalPermit`] once a
//!   slot is available. If the policy is paused, returns
//!   [`AcquireError::Paused`] immediately. The permit is RAII —
//!   dropping it releases the slot.
//! - [`try_acquire_local`] — synchronous; returns
//!   [`AcquireError::Busy`] instead of waiting when the slot is full.
//! - [`acquire_remote`] — synchronous, never blocks, never fails.
//!   Returns a [`RemotePermit`] that does not count against the slot
//!   budget but does mark the call for telemetry symmetry.
//! - [`current_policy`] / [`current_signals`] — cheap reads for
//!   diagnostics and tests.
//!
//! # State model
//!
//! State is a single process-wide [`GateState`]. Tests that exercise
//! the gate's mutating surface should:
//! 1. Acquire the [`TEST_LOCK`] mutex first (prevents concurrent test
//!    runs from sharing one semaphore).
//! 2. Call [`reset_for_test`] to restore default policy + signals
//!    and a fresh semaphore.

use std::sync::{Arc, OnceLock, RwLock};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub mod policy;
pub mod signals;

pub use policy::{Capacity, Policy, Signals, decide};

/// Errors returned when an acquire cannot proceed immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireError {
    /// The gate is in `Capacity::Pause`. Reason is the policy's
    /// explanation, suitable for logs.
    Paused(String),
    /// No slots are free (only returned by `try_acquire_local`).
    Busy,
}

/// RAII permit for a local-bound LLM call. Dropping returns the slot
/// to the semaphore.
#[derive(Debug)]
pub struct LocalPermit {
    _inner: OwnedSemaphorePermit,
}

/// Cheap permit for a remote-bound LLM call. Does not count against
/// the slot budget. Today this is a marker type only; future
/// telemetry (T-0278) can read it to attribute calls.
#[derive(Debug)]
pub struct RemotePermit {
    _private: (),
}

struct GateState {
    semaphore: RwLock<Arc<Semaphore>>,
    policy: RwLock<Policy>,
    signals: RwLock<Signals>,
}

impl GateState {
    fn new(policy: Policy) -> Self {
        Self {
            semaphore: RwLock::new(Arc::new(Semaphore::new(policy.local_slots))),
            policy: RwLock::new(policy),
            signals: RwLock::new(Signals::default()),
        }
    }
}

static STATE: OnceLock<GateState> = OnceLock::new();

fn state() -> &'static GateState {
    STATE.get_or_init(|| GateState::new(Policy::default()))
}

/// Replace the active policy. Allocates a fresh semaphore sized to
/// the new `local_slots`; outstanding permits drop naturally and
/// release into the old (now-detached) semaphore.
pub fn set_policy(policy: Policy) {
    let s = state();
    let new_sem = Arc::new(Semaphore::new(policy.local_slots));
    {
        let mut sem_slot = s.semaphore.write().unwrap();
        *sem_slot = new_sem;
    }
    *s.policy.write().unwrap() = policy;
}

/// Replace the in-memory signals snapshot. Used by tests today; in
/// production the sampler task will write here once the real probe
/// lands.
pub fn set_signals(signals: Signals) {
    *state().signals.write().unwrap() = signals;
}

/// Read the active policy (cheap clone).
pub fn current_policy() -> Policy {
    state().policy.read().unwrap().clone()
}

/// Read the most recent signals snapshot (cheap clone).
pub fn current_signals() -> Signals {
    state().signals.read().unwrap().clone()
}

/// Acquire a `LocalPermit`, waiting if every slot is full. Returns
/// [`AcquireError::Paused`] immediately when the policy is paused.
pub async fn acquire_local() -> Result<LocalPermit, AcquireError> {
    let s = state();
    {
        let cap = decide(&s.policy.read().unwrap(), &s.signals.read().unwrap());
        if let Capacity::Pause(reason) = cap {
            return Err(AcquireError::Paused(reason));
        }
    }
    let sem = s.semaphore.read().unwrap().clone();
    match sem.acquire_owned().await {
        Ok(permit) => Ok(LocalPermit { _inner: permit }),
        Err(_) => {
            // The semaphore got replaced under us (set_policy). Retry
            // once with the new semaphore.
            let sem2 = s.semaphore.read().unwrap().clone();
            let permit = sem2
                .acquire_owned()
                .await
                .map_err(|_| AcquireError::Busy)?;
            Ok(LocalPermit { _inner: permit })
        }
    }
}

/// Non-blocking variant. Returns immediately with `Busy` if the slot
/// is full or `Paused` if the policy is paused.
pub fn try_acquire_local() -> Result<LocalPermit, AcquireError> {
    let s = state();
    let cap = decide(&s.policy.read().unwrap(), &s.signals.read().unwrap());
    if let Capacity::Pause(reason) = cap {
        return Err(AcquireError::Paused(reason));
    }
    let sem = s.semaphore.read().unwrap().clone();
    match sem.try_acquire_owned() {
        Ok(permit) => Ok(LocalPermit { _inner: permit }),
        Err(_) => Err(AcquireError::Busy),
    }
}

/// Acquire a `RemotePermit`. Always succeeds, never blocks.
pub fn acquire_remote() -> RemotePermit {
    RemotePermit { _private: () }
}

/// Test-only: reset policy, signals, and semaphore to default.
/// Production callers never touch this — the gate is sticky for the
/// life of the process.
#[cfg(test)]
pub fn reset_for_test() {
    set_policy(Policy::default());
    set_signals(Signals::default());
}

/// Test-only: a process-wide mutex that gate-mutating tests should
/// hold for their duration. Prevents two parallel tests from
/// stepping on the shared semaphore.
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
    async fn remote_permit_never_blocks() {
        let _guard = lock_and_reset();
        let _r1 = acquire_remote();
        let _r2 = acquire_remote();
        let _r3 = acquire_remote();
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

    #[tokio::test]
    async fn pause_blocks_local_acquires() {
        let _guard = lock_and_reset();
        set_policy(Policy {
            local_slots: 1,
            free_ram_pause_bytes: Some(500_000_000),
            on_battery_extra_pause_bytes: None,
        });
        set_signals(Signals {
            free_ram_bytes: Some(100_000_000),
            on_battery: Some(false),
        });
        match try_acquire_local() {
            Err(AcquireError::Paused(reason)) => assert!(reason.contains("RAM")),
            other => panic!("expected Paused, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn pause_does_not_block_remote() {
        let _guard = lock_and_reset();
        set_policy(Policy {
            local_slots: 1,
            free_ram_pause_bytes: Some(500_000_000),
            on_battery_extra_pause_bytes: None,
        });
        set_signals(Signals {
            free_ram_bytes: Some(100_000_000),
            on_battery: Some(false),
        });
        let _remote = acquire_remote();
    }

    #[tokio::test]
    async fn current_policy_round_trips() {
        let _guard = lock_and_reset();
        set_policy(Policy {
            local_slots: 3,
            free_ram_pause_bytes: None,
            on_battery_extra_pause_bytes: None,
        });
        assert_eq!(current_policy().local_slots, 3);
    }

    #[tokio::test]
    async fn set_policy_resizes_semaphore() {
        let _guard = lock_and_reset();
        set_policy(Policy {
            local_slots: 2,
            free_ram_pause_bytes: None,
            on_battery_extra_pause_bytes: None,
        });
        let _p1 = acquire_local().await.expect("first");
        let _p2 = acquire_local().await.expect("second");
        // Third should now be busy.
        match try_acquire_local() {
            Err(AcquireError::Busy) => {}
            other => panic!("expected Busy at slot 3, got {other:?}"),
        }
    }
}

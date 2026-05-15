//! Host-signal sampling for the LLM resource gate.
//!
//! The production sampler is intentionally stubbed in this first cut:
//! the gate ships with permanent `Signals::default()` so the only
//! protection in effect is the 1-slot semaphore. A real RAM/battery
//! probe (likely via `sysinfo`) lands in a follow-up — see the
//! pause-on-RAM-low test for the shape the test harness expects.
//!
//! Test code injects signals directly via the [`crate::gate::set_test_signals`]
//! function; that is the only path that mutates state today.

use super::policy::Signals;

/// Snapshot the current host signals.
///
/// **Stub:** returns `Signals::default()` until a real probe lands.
/// Kept as a function so callers depend on a stable surface — the
/// sampler-task implementation can swap in without touching call
/// sites.
pub fn sample() -> Signals {
    Signals::default()
}

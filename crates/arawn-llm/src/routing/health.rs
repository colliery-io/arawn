//! Cached health of the configured local LLM provider.
//!
//! Production wiring: spawn a tokio task at startup that probes the
//! local provider with a small `warmup` ping every N seconds and
//! flips an `AtomicBool` accordingly. The policy reads the flag
//! lock-free.
//!
//! This first cut ships the API + a constant-`Healthy` default. The
//! polling task lands as a follow-up — the call sites + policy
//! integration are the load-bearing parts.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::policy::LocalHealth;

/// Health probe for the configured local provider.
#[derive(Debug)]
pub struct LocalHealthChecker {
    /// `true` when the last probe (or the default) reported healthy.
    healthy: AtomicBool,
    /// True when a local profile is configured; false otherwise.
    /// Distinguishes "Unhealthy" from "NotConfigured".
    configured: bool,
}

impl LocalHealthChecker {
    /// Build a checker for an environment with a configured local
    /// profile. Starts in the `Healthy` state — the polling task
    /// will demote it once it observes a failure.
    pub fn configured() -> Self {
        Self {
            healthy: AtomicBool::new(true),
            configured: true,
        }
    }

    /// Build a checker for an environment with no local profile.
    /// Always reports `NotConfigured`.
    pub fn not_configured() -> Self {
        Self {
            healthy: AtomicBool::new(false),
            configured: false,
        }
    }

    /// Cheap, lock-free read of the current health snapshot.
    pub fn snapshot(&self) -> LocalHealth {
        if !self.configured {
            return LocalHealth::NotConfigured;
        }
        if self.healthy.load(Ordering::Relaxed) {
            LocalHealth::Healthy
        } else {
            LocalHealth::Unhealthy
        }
    }

    /// Manually mark the local provider as healthy. The polling task
    /// uses this; tests use it directly.
    pub fn mark_healthy(&self) {
        if self.configured {
            self.healthy.store(true, Ordering::Relaxed);
        }
    }

    /// Manually mark the local provider as unhealthy. Tests use this
    /// to drive failure paths.
    pub fn mark_unhealthy(&self) {
        if self.configured {
            self.healthy.store(false, Ordering::Relaxed);
        }
    }
}

/// Shared handle. The pool stores one; the routing provider clones
/// it.
pub type SharedHealth = Arc<LocalHealthChecker>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_configured_is_sticky() {
        let h = LocalHealthChecker::not_configured();
        h.mark_healthy(); // no-op
        assert_eq!(h.snapshot(), LocalHealth::NotConfigured);
    }

    #[test]
    fn configured_starts_healthy() {
        let h = LocalHealthChecker::configured();
        assert_eq!(h.snapshot(), LocalHealth::Healthy);
    }

    #[test]
    fn mark_unhealthy_demotes() {
        let h = LocalHealthChecker::configured();
        h.mark_unhealthy();
        assert_eq!(h.snapshot(), LocalHealth::Unhealthy);
        h.mark_healthy();
        assert_eq!(h.snapshot(), LocalHealth::Healthy);
    }
}

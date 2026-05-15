//! Policy + signals shape for the LLM resource gate.
//!
//! In this first cut the signal sampler is a stub — [`Signals::default`]
//! is always observable and the policy permanently reports
//! [`Capacity::Available`]. The API is shaped so the RAM / on-battery
//! refinement can land later without breaking callers.
//!
//! When that refinement arrives:
//! - The sampler task fills in [`Signals`] from `sysinfo` (or platform
//!   equivalents) every 30s.
//! - [`decide`] maps signals → [`Capacity::Pause`] when free RAM is
//!   below the configured threshold.
//! - Test-only callers can already inject signals via the existing
//!   `Signals` setter on the gate state.

/// Live host signals fed into the policy decision. All fields are
/// `Option` so the absence of a probe (e.g. battery on a desktop)
/// doesn't falsely steer the decision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Signals {
    /// Free system RAM, in bytes. `None` if not yet sampled.
    pub free_ram_bytes: Option<u64>,
    /// Whether the host is currently running on battery. `None` on
    /// desktops or where the probe is unsupported.
    pub on_battery: Option<bool>,
}

/// The decision the policy hands back to the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capacity {
    /// Available — `acquire_local()` may try to grab a slot.
    Available,
    /// Paused — `acquire_local()` should wait until conditions
    /// recover. The string is a human-readable reason for logs and
    /// telemetry.
    Pause(String),
}

/// Policy configuration. Defaults are conservative: 1 local slot,
/// no RAM threshold (until a real sampler ships, the threshold is
/// effectively dormant).
#[derive(Debug, Clone)]
pub struct Policy {
    /// Number of concurrent `LocalPermit`s allowed across the
    /// process. Default 1 (matches Ollama's serial-call reality).
    pub local_slots: usize,
    /// Free-RAM threshold below which `decide` returns
    /// `Capacity::Pause`. Set to `None` to disable the RAM gate.
    pub free_ram_pause_bytes: Option<u64>,
    /// When `Some(threshold)` and `Signals::on_battery == Some(true)`,
    /// the policy tightens to a paused state below `threshold` even if
    /// the always-on threshold above would have allowed it. Disabled
    /// by default (battery-aware throttling is opt-in).
    pub on_battery_extra_pause_bytes: Option<u64>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            local_slots: 1,
            free_ram_pause_bytes: None,
            on_battery_extra_pause_bytes: None,
        }
    }
}

/// Decide whether the policy currently allows local-slot acquires.
pub fn decide(policy: &Policy, signals: &Signals) -> Capacity {
    if let Some(threshold) = policy.free_ram_pause_bytes
        && let Some(free) = signals.free_ram_bytes
        && free < threshold
    {
        return Capacity::Pause(format!(
            "free RAM {free} below pause threshold {threshold}"
        ));
    }
    if let (Some(extra), Some(free), Some(true)) = (
        policy.on_battery_extra_pause_bytes,
        signals.free_ram_bytes,
        signals.on_battery,
    ) && free < extra
    {
        return Capacity::Pause(format!(
            "on battery and free RAM {free} below battery-pause threshold {extra}"
        ));
    }
    Capacity::Available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_always_available_without_signals() {
        let policy = Policy::default();
        let signals = Signals::default();
        assert_eq!(decide(&policy, &signals), Capacity::Available);
    }

    #[test]
    fn pause_when_free_ram_below_threshold() {
        let policy = Policy {
            local_slots: 1,
            free_ram_pause_bytes: Some(1_000_000_000),
            on_battery_extra_pause_bytes: None,
        };
        let signals = Signals {
            free_ram_bytes: Some(500_000_000),
            on_battery: Some(false),
        };
        match decide(&policy, &signals) {
            Capacity::Pause(_) => {}
            other => panic!("expected Pause, got {other:?}"),
        }
    }

    #[test]
    fn no_pause_when_ram_above_threshold() {
        let policy = Policy {
            local_slots: 1,
            free_ram_pause_bytes: Some(1_000_000_000),
            on_battery_extra_pause_bytes: None,
        };
        let signals = Signals {
            free_ram_bytes: Some(2_000_000_000),
            on_battery: Some(true),
        };
        assert_eq!(decide(&policy, &signals), Capacity::Available);
    }

    #[test]
    fn on_battery_tightens_when_configured() {
        let policy = Policy {
            local_slots: 1,
            free_ram_pause_bytes: Some(500_000_000),
            on_battery_extra_pause_bytes: Some(1_500_000_000),
        };
        let plugged_in = Signals {
            free_ram_bytes: Some(1_000_000_000),
            on_battery: Some(false),
        };
        let on_battery = Signals {
            free_ram_bytes: Some(1_000_000_000),
            on_battery: Some(true),
        };
        // Plugged in, 1G free is above the 500M base threshold → allowed.
        assert_eq!(decide(&policy, &plugged_in), Capacity::Available);
        // On battery, 1G free is below the 1.5G battery threshold → paused.
        assert!(matches!(decide(&policy, &on_battery), Capacity::Pause(_)));
    }
}

//! Pure routing policy — given a request shape, decide Local vs Remote.
//!
//! No I/O here. All state (health, recent usage) is supplied by the
//! caller via [`LocalHealth`] and [`RoutingHints`]. The wrapping
//! [`super::provider::IntelligentRoutingProvider`] takes care of
//! actually invoking the chosen client and falling back if Local
//! fails.

use crate::hints::ModelHint;

/// Per-call hints that bias the routing decision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoutingHints {
    /// When true the request must never leave the local runtime. No
    /// fallback is permitted even if Local is unhealthy or errors.
    pub privacy_required: bool,
    /// Latency preference. `Low` biases toward Local; `Normal` is the
    /// default and lets the policy decide on other grounds.
    pub latency_budget: LatencyBudget,
    /// Recent-usage pressure on the *remote* model. `High` biases
    /// toward Local. The tracker that produces this lives in
    /// `crate::usage`; the policy is agnostic to how the threshold
    /// was computed.
    pub usage_pressure: UsagePressure,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LatencyBudget {
    /// Prefer the lowest-latency path (Local).
    Low,
    #[default]
    Normal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UsagePressure {
    #[default]
    Low,
    /// Recent remote usage is above the configured threshold. Bias
    /// toward Local when health permits.
    High,
}

/// Snapshot of local-provider health at decision time. Tri-state so
/// "not configured" is distinct from "currently unhealthy".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHealth {
    /// No local profile configured. Every decision goes Remote.
    NotConfigured,
    /// Local is up and serving traffic.
    Healthy,
    /// Local is configured but currently failing health checks.
    Unhealthy,
}

/// The decision the policy hands back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingTarget {
    /// Use the local profile.
    Local,
    /// Use the remote profile.
    Remote,
}

/// Outcome of [`decide`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub primary: RoutingTarget,
    /// When `Some`, the caller may retry on the named target if the
    /// primary fails. `None` means "no fallback" — e.g. when
    /// `privacy_required` forced Local.
    pub fallback: Option<RoutingTarget>,
    /// Short reason string for telemetry.
    pub reason: &'static str,
}

/// Decide where to route a request.
pub fn decide(hint: ModelHint, hints: &RoutingHints, health: LocalHealth) -> Decision {
    // Privacy is the strongest signal — it disables Remote outright.
    if hints.privacy_required {
        return Decision {
            primary: RoutingTarget::Local,
            fallback: None,
            reason: "privacy_required",
        };
    }

    // Heavy work goes Remote unless Local is configured + healthy AND
    // explicitly preferred. Heavy is otherwise too slow on consumer-
    // grade local models.
    if hint == ModelHint::Heavy {
        return Decision {
            primary: RoutingTarget::Remote,
            fallback: None,
            reason: "heavy_remote",
        };
    }

    match (hint, health) {
        // Lightweight + healthy local → Local, with Remote fallback.
        (ModelHint::Lightweight, LocalHealth::Healthy) => Decision {
            primary: RoutingTarget::Local,
            fallback: Some(RoutingTarget::Remote),
            reason: "lightweight_local",
        },
        (ModelHint::Lightweight, LocalHealth::Unhealthy)
        | (ModelHint::Lightweight, LocalHealth::NotConfigured) => Decision {
            primary: RoutingTarget::Remote,
            fallback: None,
            reason: "lightweight_local_unavailable",
        },

        // Medium is hint-driven by latency + usage signals when Local
        // is healthy; otherwise Remote.
        (ModelHint::Medium, LocalHealth::Healthy) => {
            let prefer_local = hints.latency_budget == LatencyBudget::Low
                || hints.usage_pressure == UsagePressure::High;
            if prefer_local {
                Decision {
                    primary: RoutingTarget::Local,
                    fallback: Some(RoutingTarget::Remote),
                    reason: "medium_local_biased",
                }
            } else {
                Decision {
                    primary: RoutingTarget::Remote,
                    fallback: Some(RoutingTarget::Local),
                    reason: "medium_remote_default",
                }
            }
        }
        (ModelHint::Medium, _) => Decision {
            primary: RoutingTarget::Remote,
            fallback: None,
            reason: "medium_local_unavailable",
        },

        // Heavy handled above.
        (ModelHint::Heavy, _) => unreachable!("heavy short-circuited above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> RoutingHints {
        RoutingHints::default()
    }

    #[test]
    fn privacy_forces_local_no_fallback() {
        let mut hints = h();
        hints.privacy_required = true;
        let d = decide(ModelHint::Heavy, &hints, LocalHealth::Unhealthy);
        assert_eq!(d.primary, RoutingTarget::Local);
        assert_eq!(d.fallback, None);
        assert_eq!(d.reason, "privacy_required");
    }

    #[test]
    fn heavy_goes_remote_no_fallback() {
        let d = decide(ModelHint::Heavy, &h(), LocalHealth::Healthy);
        assert_eq!(d.primary, RoutingTarget::Remote);
        assert_eq!(d.fallback, None);
    }

    #[test]
    fn lightweight_healthy_goes_local_with_fallback() {
        let d = decide(ModelHint::Lightweight, &h(), LocalHealth::Healthy);
        assert_eq!(d.primary, RoutingTarget::Local);
        assert_eq!(d.fallback, Some(RoutingTarget::Remote));
    }

    #[test]
    fn lightweight_unhealthy_goes_remote() {
        let d = decide(ModelHint::Lightweight, &h(), LocalHealth::Unhealthy);
        assert_eq!(d.primary, RoutingTarget::Remote);
        assert_eq!(d.fallback, None);
    }

    #[test]
    fn lightweight_not_configured_goes_remote() {
        let d = decide(ModelHint::Lightweight, &h(), LocalHealth::NotConfigured);
        assert_eq!(d.primary, RoutingTarget::Remote);
    }

    #[test]
    fn medium_default_goes_remote_with_local_fallback() {
        let d = decide(ModelHint::Medium, &h(), LocalHealth::Healthy);
        assert_eq!(d.primary, RoutingTarget::Remote);
        assert_eq!(d.fallback, Some(RoutingTarget::Local));
    }

    #[test]
    fn medium_low_latency_goes_local() {
        let mut hints = h();
        hints.latency_budget = LatencyBudget::Low;
        let d = decide(ModelHint::Medium, &hints, LocalHealth::Healthy);
        assert_eq!(d.primary, RoutingTarget::Local);
        assert_eq!(d.fallback, Some(RoutingTarget::Remote));
    }

    #[test]
    fn medium_high_usage_pressure_goes_local() {
        let mut hints = h();
        hints.usage_pressure = UsagePressure::High;
        let d = decide(ModelHint::Medium, &hints, LocalHealth::Healthy);
        assert_eq!(d.primary, RoutingTarget::Local);
    }

    #[test]
    fn medium_unhealthy_goes_remote_no_fallback() {
        let d = decide(ModelHint::Medium, &h(), LocalHealth::Unhealthy);
        assert_eq!(d.primary, RoutingTarget::Remote);
        assert_eq!(d.fallback, None);
    }
}

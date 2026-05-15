//! Structured record per routing decision.
//!
//! Today the record is emitted via `tracing` at info level. Once the
//! typed event bus lands (ARAWN-S-0004 §B), the same record type
//! will publish onto a `DomainEvent::Routing(_)` channel instead.

use serde::Serialize;

use crate::hints::ModelHint;

use super::policy::{Decision, RoutingTarget};

#[derive(Debug, Clone, Serialize)]
pub struct RoutingRecord {
    pub target: &'static str,
    pub model: String,
    pub category: &'static str,
    pub privacy_required: bool,
    pub latency_budget: &'static str,
    pub usage_pressure: &'static str,
    pub fallback_used: bool,
    pub outcome: RoutingOutcome,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingOutcome {
    /// Primary target succeeded.
    Success,
    /// Primary failed and the fallback was used (the call ultimately
    /// succeeded).
    FellBack,
    /// Both primary and fallback failed.
    Failed,
}

impl RoutingRecord {
    pub(crate) fn from_decision(
        decision: &Decision,
        hint: ModelHint,
        hints: &super::policy::RoutingHints,
        model: String,
        outcome: RoutingOutcome,
        fallback_used: bool,
    ) -> Self {
        Self {
            target: match decision.primary {
                RoutingTarget::Local => "local",
                RoutingTarget::Remote => "remote",
            },
            model,
            category: hint.as_str(),
            privacy_required: hints.privacy_required,
            latency_budget: match hints.latency_budget {
                super::policy::LatencyBudget::Low => "low",
                super::policy::LatencyBudget::Normal => "normal",
            },
            usage_pressure: match hints.usage_pressure {
                super::policy::UsagePressure::Low => "low",
                super::policy::UsagePressure::High => "high",
            },
            fallback_used,
            outcome,
            reason: decision.reason,
        }
    }

    /// Emit through `tracing::info!` so operators see the trail in
    /// their log surface. The line is structured so JSON-format
    /// subscribers get the fields intact.
    pub fn emit(&self) {
        tracing::info!(
            target = self.target,
            model = %self.model,
            category = self.category,
            privacy_required = self.privacy_required,
            latency_budget = self.latency_budget,
            usage_pressure = self.usage_pressure,
            fallback_used = self.fallback_used,
            outcome = ?self.outcome,
            reason = self.reason,
            "routing decision"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::policy::{LatencyBudget, RoutingHints, UsagePressure, decide, LocalHealth};
    use crate::hints::ModelHint;

    #[test]
    fn record_carries_hints_and_decision() {
        let mut hints = RoutingHints::default();
        hints.latency_budget = LatencyBudget::Low;
        hints.usage_pressure = UsagePressure::High;
        let d = decide(ModelHint::Medium, &hints, LocalHealth::Healthy);
        let r = RoutingRecord::from_decision(
            &d,
            ModelHint::Medium,
            &hints,
            "test-model".into(),
            RoutingOutcome::Success,
            false,
        );
        assert_eq!(r.target, "local");
        assert_eq!(r.category, "medium");
        assert_eq!(r.latency_budget, "low");
        assert_eq!(r.usage_pressure, "high");
        assert!(!r.fallback_used);
    }

    #[test]
    fn outcome_serialises_as_snake_case() {
        let serialised = serde_json::to_string(&RoutingOutcome::FellBack).unwrap();
        assert_eq!(serialised, "\"fell_back\"");
    }
}

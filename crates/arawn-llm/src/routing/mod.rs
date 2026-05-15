//! Health-aware routing policy. Picks Local or Remote per request
//! and falls back transparently when the primary fails.
//!
//! Entry points:
//! - [`policy::decide`] — pure decision table, easy to unit-test.
//! - [`provider::IntelligentRoutingProvider`] — `LlmClient` impl
//!   that wraps a `(local, remote)` pair, consults
//!   [`health::LocalHealthChecker`], emits a
//!   [`telemetry::RoutingRecord`] per call, and retries on
//!   fallback when permitted.
//! - [`health::LocalHealthChecker`] — cached health snapshot the
//!   policy reads. The polling task that updates it is a follow-up
//!   to this initial cut; tests mark it manually.

pub mod health;
pub mod policy;
pub mod provider;
pub mod telemetry;

pub use health::{LocalHealthChecker, SharedHealth};
pub use policy::{
    Decision, LatencyBudget, LocalHealth, RoutingHints, RoutingTarget, UsagePressure, decide,
};
pub use provider::{IntelligentRoutingProvider, ProviderHandle};
pub use telemetry::{RoutingOutcome, RoutingRecord};

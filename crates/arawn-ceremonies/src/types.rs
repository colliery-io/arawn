//! Value types used by ceremony plugins and the engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tablet lifecycle status. Matches the `status` column in
/// `ceremony_tablets` (T-0280).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabletStatus {
    /// Generated, awaiting the user.
    Open,
    /// User has interacted with the tablet (toggled a todo, confirmed
    /// priorities, wrote a diary entry).
    Reviewed,
    /// The tablet's review window passed without user interaction.
    Unreviewed,
    /// Archived/legacy — kept for historical query.
    Archived,
}

impl TabletStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TabletStatus::Open => "open",
            TabletStatus::Reviewed => "reviewed",
            TabletStatus::Unreviewed => "unreviewed",
            TabletStatus::Archived => "archived",
        }
    }
}

/// What kind of row a `ceremony_items` entry represents. Mirrors the
/// `kind` column in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    CalendarEvent,
    Attention,
    Proposal,
    Todo,
    Pattern,
    Priority,
    Freeform,
}

/// Output of the deterministic gather phase. Opaque to the engine —
/// each plugin defines its own concrete layout inside `payload`.
/// Carried as JSON so the engine can stash it on a debug surface
/// without knowing every plugin's shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatheredFacts {
    /// Plugin-defined JSON. The compose phase reads this; the engine
    /// only forwards.
    pub payload: serde_json::Value,
    /// When the gather ran. Used by telemetry + dedupe.
    pub gathered_at: DateTime<Utc>,
}

impl GatheredFacts {
    pub fn new(payload: serde_json::Value) -> Self {
        Self {
            payload,
            gathered_at: Utc::now(),
        }
    }
}

/// A pattern row to be written to `ceremony_patterns_detected`.
/// `payload` carries cited source rows in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    /// The ISO week the pattern applies to (e.g. `"2026-W20"`).
    pub iso_week: String,
    /// Stable key identifying the rule (e.g. `"priority_completion_ratio"`).
    pub pattern_key: String,
    /// A numeric measure of the pattern's intensity. Interpretation is
    /// per-rule (a ratio for completion, a count for rollover, etc).
    pub magnitude: f64,
    /// Source rows the rule consulted, serialised for audit.
    pub payload: serde_json::Value,
}

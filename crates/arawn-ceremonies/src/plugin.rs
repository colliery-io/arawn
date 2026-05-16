//! `Ceremony` plugin trait + the supporting items the engine hands
//! plugins at run time.
//!
//! The trait is intentionally narrow: a kind tag, a way to compute
//! the period key for a given clock, a default cron schedule, the
//! gather + compose halves of the pipeline, and an optional pattern
//! detector. Everything else (transactional writes, citation
//! enforcement, RPC, broadcast events) lives in the engine.

use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::CeremonyError;
use crate::types::{DetectedPattern, GatheredFacts, ItemKind};

/// Cron-like schedule. A thin wrapper today so the trait surface is
/// stable; T-0281 will swap this for the concrete cloacina schedule
/// type without touching plugin implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Cron expression string (e.g. `"0 7 * * *"`). Parsed by the
    /// engine when wiring cloacina. Plugins return a literal here
    /// and never parse cron themselves.
    pub expression: String,
    /// IANA timezone name (e.g. `"America/New_York"`). The engine
    /// interprets the expression in this timezone.
    pub timezone: String,
}

impl CronSchedule {
    pub fn new(expression: impl Into<String>, timezone: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            timezone: timezone.into(),
        }
    }

    /// Convenience: a schedule in the user's local timezone. Plugins
    /// typically want this; the engine reads the local timezone from
    /// runtime config.
    pub fn local(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            timezone: "Local".to_string(),
        }
    }
}

impl fmt::Display for CronSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.expression, self.timezone)
    }
}

/// Stable identifier for an interactive action a plugin contributes
/// (e.g. retro's diary-upsert, weekly's confirm-priority). The engine
/// surfaces these in the RPC catalog so clients render the right UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveAction {
    /// Stable key (e.g. `"upsert_diary"`).
    pub key: String,
    /// Human-readable label for the UI.
    pub label: String,
}

/// Item the LLM path produces. Citation is required at construction —
/// the engine refuses writes without one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedItem {
    pub tablet_id: String,
    pub section_key: String,
    pub ordinal: i32,
    pub kind: ItemKind,
    pub body: serde_json::Value,
    /// Required. The `id` of the row this composed item cites:
    /// signal_id, event_id, proposal_id, or `ceremony_patterns_detected.id`.
    pub citation_id: String,
}

/// Item the user-write path produces. No citation — used for
/// freeform diary entries and user-added todos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItem {
    pub tablet_id: String,
    pub section_key: String,
    pub ordinal: i32,
    pub kind: ItemKind,
    pub body: serde_json::Value,
}

/// Items the compose phase returns to the engine. Engine routes each
/// variant to the correct write path; that is how the two-write-path
/// contract is enforced at the type level rather than at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NewItem {
    /// LLM-produced; carries a citation. Engine writes via the
    /// strict path.
    Composed(ComposedItem),
    /// User-produced; no citation. Engine writes via the permissive
    /// path.
    User(UserItem),
}

impl NewItem {
    pub fn composed(item: ComposedItem) -> Self {
        Self::Composed(item)
    }
    pub fn user(item: UserItem) -> Self {
        Self::User(item)
    }
    pub fn tablet_id(&self) -> &str {
        match self {
            NewItem::Composed(c) => &c.tablet_id,
            NewItem::User(u) => &u.tablet_id,
        }
    }
}

/// Context handed to a plugin during gather + compose. The concrete
/// implementation lives in the engine (T-0282) and exposes the
/// write paths + DB query helpers. Plugins see only the trait
/// methods exposed here.
#[async_trait]
pub trait CeremonyCtx: Send + Sync {
    /// Period key the engine generated for this run (e.g.
    /// `"2026-05-15"` for daily, `"2026-W20"` for weekly).
    fn period_key(&self) -> &str;

    /// Tablet id the engine just opened for this run. Plugins use
    /// this on every `NewItem` they construct.
    fn tablet_id(&self) -> &str;

    /// Write the pattern row eagerly during pattern detection so
    /// dependent composed items can cite its returned id. T-0282
    /// implements; this trait method ships now so the surface is
    /// stable.
    async fn write_pattern_row(
        &self,
        pattern: DetectedPattern,
    ) -> Result<String, CeremonyError>;

    /// Capability check — does this ctx have a SQL connection
    /// behind it (i.e. is it the production `EngineCtx`)? Returns
    /// `None` on stub contexts that don't have rollup access.
    /// Default `None`; the engine ctx overrides.
    ///
    /// Used by [`crate::patterns::DetectorRegistry`] to decide
    /// whether per-rule detectors can run. Stub ctxs in tests can
    /// still drive composed-item writes via `write_pattern_row`
    /// but cannot read history.
    fn conn_handle(&self) -> Option<&crate::engine::ConnHandle> {
        None
    }
}

/// Pattern detector framework hook. Plugins that surface
/// comparative patterns (retro is the canonical case) return one of
/// these from `Ceremony::patterns`. The engine runs each detector
/// in order, writes rows via `CeremonyCtx::write_pattern_row`, and
/// passes the resulting ids to the compose phase as candidate
/// citations.
#[async_trait]
pub trait PatternDetector: Send + Sync {
    /// Run every registered rule and return the rows the engine
    /// should persist. Each rule's source data should live inside
    /// the row's `payload`.
    async fn detect(&self, ctx: &dyn CeremonyCtx) -> Result<Vec<DetectedPattern>, CeremonyError>;
}

/// Contract every ceremony plugin implements.
#[async_trait]
pub trait Ceremony: Send + Sync {
    /// Stable kind tag (`"daily"`, `"weekly"`, `"retro"`, ...).
    /// Used as the dispatch key in the engine and as a query column
    /// in the schema.
    fn kind(&self) -> &'static str;

    /// Compute the period key for the given clock. Daily returns the
    /// date; weekly + retro return the ISO week.
    fn period_key(&self, now: DateTime<Utc>) -> String;

    /// Cron schedule the engine should register. Users can override
    /// via the RPC config surface (T-0283).
    fn default_schedule(&self) -> CronSchedule;

    /// Pull every fact needed by the compose phase. Deterministic —
    /// no LLM. Errors are surfaced as `CeremonyError`.
    async fn gather(&self, ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError>;

    /// Produce the items the engine will write. The engine routes
    /// `Composed` variants through the strict citation path and
    /// `User` variants through the permissive path.
    async fn compose(
        &self,
        ctx: &dyn CeremonyCtx,
        facts: GatheredFacts,
    ) -> Result<Vec<NewItem>, CeremonyError>;

    /// Plugin-contributed interactive actions (e.g. retro's
    /// `upsert_diary`). Default: none.
    fn interactive_actions(&self) -> Vec<InteractiveAction> {
        Vec::new()
    }

    /// Optional pattern detector. Retro is the only plugin that
    /// returns `Some` in v1.
    fn patterns(&self) -> Option<&dyn PatternDetector> {
        None
    }
}

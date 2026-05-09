//! `FeedTemplate` trait + `TemplateCtx` (the runtime handle templates
//! use to reach providers and emit logs).

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::clients::{FeedClients, NoopClients};
use crate::error::FeedError;
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

/// Result returned from a single feed run.
///
/// The runtime reads `cursor` and persists it back to `meta.json`. The
/// `summary` flows into cloacina's audit row and the meta's
/// `last_status` field.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// The new cursor to persist. Schemaless — each template owns its
    /// own shape (Slack stores `{"latest_ts": ...}`, Gmail stores
    /// `{"history_id": ...}`, etc).
    pub cursor: Value,
    pub summary: RunSummary,
    /// Free-form short status string for forensics: "ok",
    /// "no-new-items", "rate-limited", etc.
    pub status: String,
}

/// Per-run handle a template uses to reach providers and emit metadata.
///
/// `clients` is mock-friendly: tests build a `TemplateCtx` with a fake
/// `FeedClients` impl that returns canned data; production wires the
/// real `arawn-integrations` clients in.
pub struct TemplateCtx {
    clients: Arc<dyn FeedClients>,
}

impl TemplateCtx {
    pub fn new(clients: Arc<dyn FeedClients>) -> Self {
        Self { clients }
    }

    /// Test-only convenience: a ctx where every provider client returns
    /// "not connected." Useful for stub templates that don't need any
    /// provider access.
    pub fn noop() -> Self {
        Self {
            clients: Arc::new(NoopClients),
        }
    }

    pub fn clients(&self) -> &Arc<dyn FeedClients> {
        &self.clients
    }
}

/// One named, parameterized fetch+write recipe owned by an integration.
///
/// Templates are pure Rust trait impls — no cloacina macros, no
/// `.cloacina` packages, no scaffolding. Adding a new template is one
/// file in `src/templates/<provider>/<name>.rs`.
#[async_trait]
pub trait FeedTemplate: Send + Sync {
    /// Stable identifier "<provider>/<template>" — e.g.
    /// `slack/channel-archive`. Used as the registry key + as part of
    /// the on-disk path layout.
    fn name(&self) -> &'static str;

    /// Validate parameters at registration time. Should reject unknown
    /// params, unresolvable references (e.g. a Slack channel that
    /// doesn't exist), and anything that's guaranteed to fail at run
    /// time.
    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError>;

    /// Sensible default cadence + initial cursor for the given params.
    /// Used when arawn.toml / `/watch` doesn't specify one.
    fn defaults(&self, params: &TemplateParams) -> FeedDefaults;

    /// Run one fetch+write cycle.
    ///
    /// `feed_dir` is the dir the template can write into freely (the
    /// runtime never writes here other than `meta.json`). `cursor` is
    /// the value the template returned on its previous run, or
    /// JSON `null` on first run.
    ///
    /// Templates own their own storage layout — JSONL append, per-record
    /// JSON, native binary mirror, whatever fits the data semantics.
    /// Runtime guarantees the dir exists and is writable.
    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError>;

    /// Optional discovery hook for the `/watch` picker.
    ///
    /// Templates whose required params are enumerable from the
    /// provider API (a Slack channel, a Jira project, a Confluence
    /// space) override this to return a list of `(label, params)`
    /// pairs; the TUI shows them as a selectable list and submits
    /// the chosen `params` directly to `feed_register`.
    ///
    /// Templates whose params are free-form (a Gmail sender pattern,
    /// a Drive folder path, an arbitrary cadence override) leave the
    /// default `Ok(None)` — the TUI then prints a usage message
    /// instead of opening an empty picker.
    async fn discover(
        &self,
        _ctx: &TemplateCtx,
    ) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        Ok(None)
    }
}

/// One pickable choice surfaced by `FeedTemplate::discover`.
///
/// `label` is what the picker shows (e.g. `"#design"`, `"ENG —
/// Engineering"`). `params` is the JSON object the user's choice
/// resolves to — handed straight to `feed_register` without further
/// shaping. `hint` is an optional second line for context (id,
/// privacy marker, member count).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryRow {
    pub label: String,
    #[serde(default)]
    pub hint: Option<String>,
    pub params: Value,
}

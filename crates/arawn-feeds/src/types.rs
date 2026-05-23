//! Shared types passed between the runtime and template impls.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Template-specific parameters from the feed config row. Schemaless
/// JSON so each template can define its own param shape; the runtime
/// just round-trips it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateParams(pub Value);

impl TemplateParams {
    pub fn new(v: Value) -> Self {
        Self(v)
    }

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Convenience getter for a string field on the params object.
    pub fn get_str<'a>(&'a self, key: &str) -> Option<&'a str> {
        self.0.get(key)?.as_str()
    }
}

/// Sensible default cadence + initial cursor a template suggests for a
/// given param set. Returned by `FeedTemplate::defaults` and used when
/// `arawn.toml` / `/watch` doesn't override.
#[derive(Debug, Clone)]
pub struct FeedDefaults {
    /// Cron expression. Must satisfy the 15-minute cadence floor.
    pub cadence: String,
    /// Initial cursor value to write to `meta.json` on first registration.
    /// Schemaless — each template owns its inner shape (Slack stores
    /// `{"latest_ts": ...}`, Gmail stores `{"history_id": ...}`, etc).
    pub initial_cursor: Value,
}

/// Summary metrics from one fetch+write cycle, persisted to cloacina's
/// audit row and to `meta.json.last_status`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunSummary {
    pub items_written: u64,
    pub bytes_written: u64,
    /// Total duration including provider call + disk write.
    pub duration: Duration,
}

/// What the runtime persists to `meta.json` at the feed dir root.
/// Templates read the `cursor` field on entry, return a new cursor in
/// their `RunSummary`, and the runtime writes the updated meta back
/// atomically via [`crate::meta::MetaStore`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMeta {
    pub template: String,
    pub params: TemplateParams,
    /// Schemaless — opaque to the runtime; each template knows its
    /// own shape. Defaults to JSON null on first registration.
    #[serde(default)]
    pub cursor: Value,
    /// ISO-8601 UTC timestamp of the most recent successful run.
    /// `None` until the first successful run completes.
    pub last_run_at: Option<String>,
    /// Free-form short string the template returned on the most recent
    /// run — "ok", "no-new-items", "rate-limited", etc. Diagnostic.
    pub last_status: Option<String>,
    /// Monotonic count of run attempts (success + failure).
    #[serde(default)]
    pub run_count: u64,
}

impl FeedMeta {
    pub fn new(template: impl Into<String>, params: TemplateParams, initial_cursor: Value) -> Self {
        Self {
            template: template.into(),
            params,
            cursor: initial_cursor,
            last_run_at: None,
            last_status: None,
            run_count: 0,
        }
    }
}

/// User-facing snapshot of one feed: the row state, last-run health
/// from `meta.json`, and the size of its data dir.
///
/// Returned by `FeedRuntime::list_summaries` and shown in the
/// `/feeds` modal. Kept Serializable so it round-trips through the
/// service WS without a re-shaping layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSummary {
    pub id: String,
    pub template: String,
    pub cadence: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    /// `meta.json.last_run_at` if any run has completed.
    pub last_run_at: Option<String>,
    /// Free-form short status from the last run ("ok", "no-new-items",
    /// "rate-limited", "auth-error", ...).
    pub last_status: Option<String>,
    /// Monotonic count of run attempts (success + failure).
    pub run_count: u64,
    /// Recursive byte size of the feed's data dir at list time.
    pub data_size_bytes: u64,
    /// Resolved on-disk path of the feed's data dir.
    pub data_dir: String,
}

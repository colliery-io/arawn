//! Public projection types.

use chrono::{DateTime, Utc};

/// A single projection row, type-erased to the common fields every
/// projection table has. Concrete projection variants are owned by
/// per-feed-type modules (e.g. `gmail::GmailMessageProjection`).
///
/// `metadata` carries the normalized per-feed-type fields as a JSON
/// object — convenient for the agent tool's hydration path; the
/// per-feed module owns the canonical typed struct.
#[derive(Debug, Clone)]
pub struct ProjectionRow {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub title: String,
    pub body_text: String,
    pub feed_type: String,
    pub metadata: serde_json::Value,
}

/// Marker trait for type-specific projection structs. Each per-feed
/// module implements this on its own typed struct (e.g. `GmailMessage`).
///
/// Implementations supply the `feed_type` table name, the row identity
/// fields (`feed_id` + `source_id`), and a `ProjectionRow` view for
/// the generic writer.
pub trait Projection {
    /// Stable table name (e.g. `gmail_messages`). Used by the writer
    /// to route inserts.
    fn feed_type(&self) -> &'static str;

    /// Project this typed value into the generic row view used by
    /// the writer + the agent tool.
    fn row(&self) -> ProjectionRow;
}

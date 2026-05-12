//! Projection table schema.
//!
//! One table per feed type, with a paired FTS5 virtual table for text
//! search and an `<table>_embeddings` raw-blob table for semantic
//! search. Tables follow a shared shape so the agent-facing search
//! tool (T-0247) can iterate them uniformly:
//!
//! ```text
//! <feed_type> (
//!   id TEXT PRIMARY KEY,           -- (feed_id, source_id) → hashed
//!   feed_id TEXT,
//!   source_id TEXT,
//!   source_ts TEXT,                -- RFC3339
//!   title TEXT,                    -- synthesized per feed type
//!   body_text TEXT,
//!   metadata TEXT,                 -- JSON; per-type fields
//!   created_at TEXT,
//!   updated_at TEXT,
//!   body_hash TEXT,                -- sha256(body_text) for embed dirty-check
//!   UNIQUE(feed_id, source_id)
//! )
//!
//! <feed_type>_fts USING fts5(
//!   projection_id UNINDEXED, title, body_text, tokenize='unicode61'
//! )
//!
//! <feed_type>_embeddings (
//!   projection_id TEXT PRIMARY KEY,
//!   body_hash TEXT NOT NULL,       -- cached against body_text
//!   embedding BLOB
//! )
//! ```
//!
//! Per-feed-type *additional* normalized columns (sender, channel,
//! thread_id, …) live in `metadata` JSON. Hot-path filtering fields
//! get hoisted to indexed columns in follow-up tasks if needed.

use rusqlite::Connection;

use crate::error::ProjectionError;

/// Idempotently create all schema for a given feed type.
pub fn ensure_feed_type_tables(
    conn: &Connection,
    feed_type: &str,
) -> Result<(), ProjectionError> {
    let table_sql = format!(
        "CREATE TABLE IF NOT EXISTS {feed_type} (
            id TEXT PRIMARY KEY,
            feed_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            source_ts TEXT NOT NULL,
            title TEXT NOT NULL,
            body_text TEXT NOT NULL,
            metadata TEXT NOT NULL DEFAULT '{{}}',
            body_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(feed_id, source_id)
        );
        CREATE INDEX IF NOT EXISTS idx_{feed_type}_source_ts
            ON {feed_type}(source_ts);
        CREATE INDEX IF NOT EXISTS idx_{feed_type}_feed_id
            ON {feed_type}(feed_id);"
    );
    conn.execute_batch(&table_sql)
        .map_err(|e| ProjectionError::Schema(format!("create {feed_type}: {e}")))?;

    let fts_sql = format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {feed_type}_fts USING fts5(
            projection_id UNINDEXED, title, body_text, tokenize = 'unicode61'
        );"
    );
    conn.execute_batch(&fts_sql)
        .map_err(|e| ProjectionError::Schema(format!("create {feed_type}_fts: {e}")))?;

    let embed_sql = format!(
        "CREATE TABLE IF NOT EXISTS {feed_type}_embeddings (
            projection_id TEXT PRIMARY KEY,
            body_hash TEXT NOT NULL,
            embedding BLOB
        );"
    );
    conn.execute_batch(&embed_sql)
        .map_err(|e| ProjectionError::Schema(format!("create {feed_type}_embeddings: {e}")))?;

    Ok(())
}

/// Set basic pragmas for a projection database.
pub fn apply_pragmas(conn: &Connection) -> Result<(), ProjectionError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| ProjectionError::Storage(format!("pragmas: {e}")))?;
    Ok(())
}

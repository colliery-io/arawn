//! Projection table schema.
//!
//! Per feed type, three tables:
//!
//! - **`<feed_type>`** — normalized data rows (id, feed_id, source_id,
//!   source_ts, title, body_text, metadata JSON, body_hash, timestamps).
//! - **`<feed_type>_fts`** — FTS5 virtual table over title + body_text.
//! - **`<feed_type>_embeddings`** — bookkeeping for the embed pass:
//!   `projection_id PRIMARY KEY, body_hash, status TEXT`. Status is one
//!   of `pending` (newly written or body changed), `embedded` (vector
//!   in the vec0 table), or `skipped` (body too short to embed).
//! - **`<feed_type>_vec`** — sqlite-vec `vec0` virtual table holding
//!   the actual `float[EMBEDDING_DIMS]` vectors, keyed by projection_id.
//!
//! Per-feed-type *additional* normalized columns (sender, channel,
//! thread_id, …) live in the data table's `metadata` JSON. Hot-path
//! filtering fields get hoisted to indexed columns in follow-up tasks
//! if needed.

use rusqlite::Connection;

use crate::error::ProjectionError;

/// Embedding dimensionality. Matches `arawn-embed`'s all-MiniLM-L6-v2.
/// All projection feed types share the same dimension because they
/// share an embedder.
pub const EMBEDDING_DIMS: usize = 384;

/// One-shot initialization of the sqlite-vec extension. Must be
/// called before opening any projection database that uses vectors.
/// Idempotent — backed by `sqlite3_auto_extension`.
pub fn init_vector_extension() {
    use rusqlite::ffi::sqlite3_auto_extension;
    use sqlite_vec::sqlite3_vec_init;
    unsafe {
        #[allow(clippy::missing_transmute_annotations)]
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
}

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
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'embedded', 'skipped'))
        );
        CREATE INDEX IF NOT EXISTS idx_{feed_type}_emb_status
            ON {feed_type}_embeddings(status);"
    );
    conn.execute_batch(&embed_sql)
        .map_err(|e| ProjectionError::Schema(format!("create {feed_type}_embeddings: {e}")))?;

    let vec_sql = format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {feed_type}_vec USING vec0(
            projection_id TEXT PRIMARY KEY,
            embedding float[{dims}]
        );",
        dims = EMBEDDING_DIMS
    );
    conn.execute_batch(&vec_sql)
        .map_err(|e| ProjectionError::Schema(format!("create {feed_type}_vec: {e}")))?;

    Ok(())
}

/// Set basic pragmas for a projection database.
pub fn apply_pragmas(conn: &Connection) -> Result<(), ProjectionError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| ProjectionError::Storage(format!("pragmas: {e}")))?;
    Ok(())
}

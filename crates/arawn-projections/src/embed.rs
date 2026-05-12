//! Embedding pass.
//!
//! Walks `<feed_type>_embeddings WHERE embedding IS NULL`, fetches the
//! body_text, runs it through any `Embedder` impl supplied by the
//! caller, and writes the resulting vector back as a raw little-endian
//! f32 BLOB.
//!
//! The pass is callable, idempotent, and bounded by `max_per_pass`. A
//! cron / scheduled tokio loop in arawn main drives the cadence — see
//! `crates/arawn/src/main.rs`.

use std::future::Future;
use std::pin::Pin;

use rusqlite::params;
use tracing::{debug, warn};

use crate::error::ProjectionError;
use crate::store::ProjectionStore;

/// Feed types whose body_text is worth embedding. `jira_history` is
/// intentionally excluded — its body is "<field> <author> <from> to
/// <to>", which is high-noise for semantic similarity.
pub const EMBEDDABLE_FEED_TYPES: &[&str] = &[
    "gmail_messages",
    "slack_messages",
    "slack_thread_messages",
    "drive_files",
    "jira_issues",
    "jira_comments",
    "confluence_pages",
    "calendar_events",
];

/// Minimum body length worth embedding. Anything shorter is mostly
/// noise (single-emoji slack reactions, "ok thx" replies); we'd
/// rather skip than burn the compute.
const MIN_BODY_CHARS: usize = 16;

#[derive(Debug, Default, Clone)]
pub struct EmbedPassOutcome {
    pub embedded: usize,
    pub skipped_empty: usize,
    pub errors: usize,
}

/// Lightweight embedding interface this crate consumes. Implemented
/// for any type that can map a batch of texts to a batch of f32
/// vectors. Keeps `arawn-projections` from depending on `arawn-embed`
/// directly — the caller passes any concrete impl.
pub trait Embedder: Send + Sync {
    fn embed_batch<'a>(
        &'a self,
        texts: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, String>> + Send + 'a>>;
}

/// Run a single embed pass over every embeddable feed type, capped at
/// `max_per_pass` rows total. Returns a per-pass tally.
pub async fn run_embed_pass(
    store: &ProjectionStore,
    embedder: &dyn Embedder,
    batch_size: usize,
    max_per_pass: usize,
) -> Result<EmbedPassOutcome, ProjectionError> {
    let mut outcome = EmbedPassOutcome::default();
    let mut remaining = max_per_pass;

    'feeds: for &feed_type in EMBEDDABLE_FEED_TYPES {
        // Drain this feed type one batch at a time until we either
        // exhaust pending rows or hit `max_per_pass`. Stops gracefully
        // mid-feed if the cap kicks in.
        loop {
            if remaining == 0 {
                break 'feeds;
            }
            let take = remaining.min(batch_size).max(1);
            let rows = store.pending_embedding_rows(feed_type, take)?;
            if rows.is_empty() {
                break;
            }
            let processed_in_batch = embed_batch(
                store,
                feed_type,
                &rows,
                embedder,
                &mut outcome,
            )
            .await?;
            remaining = remaining.saturating_sub(processed_in_batch);
            debug!(
                feed_type = feed_type,
                embedded = outcome.embedded,
                "embed pass progress"
            );
            if processed_in_batch == 0 {
                // Defensive: every row was sentinel-skipped or errored.
                // Avoid an infinite loop on a pathological batch.
                break;
            }
        }
    }
    Ok(outcome)
}

async fn embed_batch(
    store: &ProjectionStore,
    feed_type: &str,
    rows: &[PendingEmbedRow],
    embedder: &dyn Embedder,
    outcome: &mut EmbedPassOutcome,
) -> Result<usize, ProjectionError> {
    let mut processed = 0usize;

    let mut texts: Vec<&str> = Vec::with_capacity(rows.len());
    let mut keep: Vec<&PendingEmbedRow> = Vec::with_capacity(rows.len());
    for r in rows {
        if r.body_text.chars().count() < MIN_BODY_CHARS {
            outcome.skipped_empty += 1;
            processed += 1;
            if let Err(e) = store.write_embedding(feed_type, &r.projection_id, &[]) {
                warn!(
                    feed_type = feed_type,
                    id = %r.projection_id,
                    error = %e,
                    "marking row as skipped failed"
                );
                outcome.errors += 1;
            }
            continue;
        }
        keep.push(r);
        texts.push(r.body_text.as_str());
    }

    if texts.is_empty() {
        return Ok(processed);
    }

    let vectors = match embedder.embed_batch(&texts).await {
        Ok(v) => v,
        Err(e) => {
            warn!(feed_type = feed_type, error = %e, "embed batch failed");
            outcome.errors += 1;
            return Ok(processed);
        }
    };
    if vectors.len() != keep.len() {
        warn!(
            feed_type = feed_type,
            expected = keep.len(),
            got = vectors.len(),
            "embed batch returned wrong shape; skipping"
        );
        outcome.errors += 1;
        return Ok(processed);
    }
    for (row, vec) in keep.iter().zip(vectors.iter()) {
        if let Err(e) = store.write_embedding(feed_type, &row.projection_id, vec) {
            warn!(
                feed_type = feed_type,
                id = %row.projection_id,
                error = %e,
                "write embedding failed"
            );
            outcome.errors += 1;
            continue;
        }
        outcome.embedded += 1;
        processed += 1;
    }
    Ok(processed)
}

/// A row pending embedding: the `<feed_type>` row's projection id +
/// its current body_text.
#[derive(Debug, Clone)]
pub struct PendingEmbedRow {
    pub projection_id: String,
    pub body_text: String,
}

impl ProjectionStore {
    /// Find rows in `<feed_type>` whose embed status is `pending`,
    /// capped at `limit`. Used by the embed pass.
    pub fn pending_embedding_rows(
        &self,
        feed_type: &str,
        limit: usize,
    ) -> Result<Vec<PendingEmbedRow>, ProjectionError> {
        let conn = self
            .conn()
            .lock()
            .map_err(|_| ProjectionError::Storage("conn lock poisoned".into()))?;
        crate::schema::ensure_feed_type_tables(&conn, feed_type)?;
        let sql = format!(
            "SELECT p.id, p.body_text \
             FROM {feed_type} p \
             JOIN {feed_type}_embeddings e ON e.projection_id = p.id \
             WHERE e.status = 'pending' \
             LIMIT ?1"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ProjectionError::Storage(format!("prepare pending: {e}")))?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(PendingEmbedRow {
                    projection_id: r.get::<_, String>(0)?,
                    body_text: r.get::<_, String>(1)?,
                })
            })
            .map_err(|e| ProjectionError::Storage(format!("pending: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| ProjectionError::Storage(e.to_string()))?);
        }
        Ok(out)
    }

    /// Write a freshly computed embedding for a projection row. The
    /// vec0 virtual table holds the actual vector; the bookkeeping
    /// table flips status to `embedded`. An empty `vector` slice
    /// marks the row as intentionally `skipped` (see `MIN_BODY_CHARS`).
    pub fn write_embedding(
        &self,
        feed_type: &str,
        projection_id: &str,
        vector: &[f32],
    ) -> Result<(), ProjectionError> {
        use zerocopy::IntoBytes;
        let conn = self
            .conn()
            .lock()
            .map_err(|_| ProjectionError::Storage("conn lock poisoned".into()))?;
        crate::schema::ensure_feed_type_tables(&conn, feed_type)?;

        if vector.is_empty() {
            // Intentional skip — just flip the status, no vec0 write.
            let sql = format!(
                "UPDATE {feed_type}_embeddings SET status = 'skipped' \
                 WHERE projection_id = ?1"
            );
            conn.execute(&sql, params![projection_id])
                .map_err(|e| ProjectionError::Storage(format!("mark skipped: {e}")))?;
            return Ok(());
        }

        if vector.len() != crate::schema::EMBEDDING_DIMS {
            return Err(ProjectionError::Schema(format!(
                "embedding dim mismatch: expected {}, got {}",
                crate::schema::EMBEDDING_DIMS,
                vector.len()
            )));
        }

        // vec0 doesn't support INSERT OR REPLACE — delete first.
        let del_sql = format!(
            "DELETE FROM {feed_type}_vec WHERE projection_id = ?1"
        );
        conn.execute(&del_sql, params![projection_id])
            .map_err(|e| ProjectionError::Storage(format!("vec delete: {e}")))?;
        let ins_sql = format!(
            "INSERT INTO {feed_type}_vec (projection_id, embedding) VALUES (?1, ?2)"
        );
        conn.execute(&ins_sql, params![projection_id, vector.as_bytes()])
            .map_err(|e| ProjectionError::Storage(format!("vec insert: {e}")))?;

        // Flip status to embedded. INSERT-or-update in case the row
        // arrived via a path that skipped the writer's invalidate
        // (defensive).
        let meta_sql = format!(
            "INSERT INTO {feed_type}_embeddings (projection_id, body_hash, status) \
             VALUES (?1, '', 'embedded') \
             ON CONFLICT(projection_id) DO UPDATE SET status = 'embedded'"
        );
        conn.execute(&meta_sql, params![projection_id])
            .map_err(|e| ProjectionError::Storage(format!("mark embedded: {e}")))?;
        Ok(())
    }
}

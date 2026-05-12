//! `ProjectionStore` — sqlite-backed writer + reader for projection rows.
//!
//! All writes are transactional dual-writes against the per-feed-type
//! table + FTS5 + embedding cache. The embedding column itself is left
//! NULL by the writer; callers wire an embedding pipeline separately
//! (T-0247 / follow-up) and update `*_embeddings.embedding` once the
//! vector is computed. The `body_hash` column lets a re-embedding pass
//! detect stale entries cheaply.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{Connection, params};
use tracing::debug;

use crate::error::ProjectionError;
use crate::schema;
use crate::types::{Projection, ProjectionRow};

/// Sqlite-backed projection store. One file per arawn data root; the
/// per-feed-type tables live alongside each other.
pub struct ProjectionStore {
    conn: Mutex<Connection>,
}

impl ProjectionStore {
    pub fn open(path: &Path) -> Result<Self, ProjectionError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)
            .map_err(|e| ProjectionError::Storage(format!("open db: {e}")))?;
        schema::apply_pragmas(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> Result<Self, ProjectionError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ProjectionError::Storage(format!("open in-memory: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Ensure schema for a feed type exists. Safe to call repeatedly.
    pub fn ensure_feed_type(&self, feed_type: &str) -> Result<(), ProjectionError> {
        let conn = self.conn.lock().unwrap();
        schema::ensure_feed_type_tables(&conn, feed_type)
    }

    /// Write a single projection inside a transaction: row UPSERT,
    /// FTS5 upsert, embedding cache placeholder (NULL embedding until
    /// the embed pass fills it in).
    pub fn write<P: Projection>(&self, projection: &P) -> Result<WriteOutcome, ProjectionError> {
        self.write_batch(std::slice::from_ref(projection))
    }

    /// Write many projections in one transaction.
    pub fn write_batch<P: Projection>(
        &self,
        projections: &[P],
    ) -> Result<WriteOutcome, ProjectionError> {
        if projections.is_empty() {
            return Ok(WriteOutcome::default());
        }
        let mut conn = self.conn.lock().unwrap();
        // Materialize the type table set so we ensure schema once per
        // batch even if all rows share a type.
        let feed_types: HashSet<&'static str> =
            projections.iter().map(|p| p.feed_type()).collect();
        for ft in &feed_types {
            schema::ensure_feed_type_tables(&conn, ft)?;
        }

        let tx = conn
            .transaction()
            .map_err(|e| ProjectionError::Storage(format!("tx begin: {e}")))?;
        let mut outcome = WriteOutcome::default();
        for p in projections {
            let row = p.row();
            let action = write_row(&tx, p.feed_type(), &row)?;
            match action {
                WriteAction::Inserted => outcome.inserted += 1,
                WriteAction::Updated => outcome.updated += 1,
                WriteAction::Unchanged => outcome.unchanged += 1,
            }
        }
        tx.commit()
            .map_err(|e| ProjectionError::Storage(format!("tx commit: {e}")))?;
        debug!(
            inserted = outcome.inserted,
            updated = outcome.updated,
            unchanged = outcome.unchanged,
            "projection batch committed",
        );
        Ok(outcome)
    }

    /// Returns ids that are NOT yet projected for a given feed.
    /// Used by the per-feed backfill to walk on-disk mirrors and skip
    /// already-projected items.
    pub fn missing_source_ids(
        &self,
        feed_type: &str,
        feed_id: &str,
        candidate_source_ids: &[String],
    ) -> Result<Vec<String>, ProjectionError> {
        if candidate_source_ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        schema::ensure_feed_type_tables(&conn, feed_type)?;
        let placeholders = std::iter::repeat_n("?", candidate_source_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT source_id FROM {feed_type} \
             WHERE feed_id = ? AND source_id IN ({placeholders})"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ProjectionError::Storage(format!("prepare missing: {e}")))?;
        let mut params_vec: Vec<&dyn rusqlite::types::ToSql> = Vec::with_capacity(1 + candidate_source_ids.len());
        params_vec.push(&feed_id);
        for s in candidate_source_ids {
            params_vec.push(s);
        }
        let mut rows = stmt
            .query(params_vec.as_slice())
            .map_err(|e| ProjectionError::Storage(format!("query missing: {e}")))?;
        let mut present = HashSet::new();
        while let Some(row) = rows.next().map_err(|e| ProjectionError::Storage(e.to_string()))? {
            let s: String = row.get(0)?;
            present.insert(s);
        }
        Ok(candidate_source_ids
            .iter()
            .filter(|s| !present.contains(s.as_str()))
            .cloned()
            .collect())
    }

    /// Total rows for a feed_type — useful for tests and ops.
    pub fn count(&self, feed_type: &str) -> Result<usize, ProjectionError> {
        let conn = self.conn.lock().unwrap();
        schema::ensure_feed_type_tables(&conn, feed_type)?;
        let cnt: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {feed_type}"), [], |r| r.get(0))
            .map_err(|e| ProjectionError::Storage(format!("count: {e}")))?;
        Ok(cnt as usize)
    }

    /// FTS search over a single feed type. Returns `(projection_id,
    /// rank)`. Caller hydrates the full row via `get_row`.
    pub fn fts_search(
        &self,
        feed_type: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ProjectionError> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT projection_id FROM {feed_type}_fts \
             WHERE {feed_type}_fts MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ProjectionError::Storage(format!("prepare fts: {e}")))?;
        let rows = stmt
            .query_map(params![query, limit as i64], |r| r.get::<_, String>(0))
            .map_err(|e| ProjectionError::Storage(format!("fts: {e}")))?;
        let mut ids = Vec::new();
        for r in rows {
            ids.push(r.map_err(|e| ProjectionError::Storage(e.to_string()))?);
        }
        Ok(ids)
    }

    /// Get a single projection row by primary key.
    pub fn get_row(
        &self,
        feed_type: &str,
        projection_id: &str,
    ) -> Result<Option<ProjectionRow>, ProjectionError> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT id, feed_id, source_id, source_ts, title, body_text, metadata \
             FROM {feed_type} WHERE id = ?1"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ProjectionError::Storage(format!("prepare get: {e}")))?;
        let mut rows = stmt
            .query(params![projection_id])
            .map_err(|e| ProjectionError::Storage(format!("get: {e}")))?;
        let Some(row) = rows
            .next()
            .map_err(|e| ProjectionError::Storage(e.to_string()))?
        else {
            return Ok(None);
        };
        let id: String = row.get(0)?;
        let feed_id: String = row.get(1)?;
        let source_id: String = row.get(2)?;
        let source_ts_str: String = row.get(3)?;
        let title: String = row.get(4)?;
        let body_text: String = row.get(5)?;
        let metadata_str: String = row.get(6)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
        let source_ts = chrono::DateTime::parse_from_rfc3339(&source_ts_str)
            .map_err(|e| ProjectionError::Schema(format!("source_ts: {e}")))?
            .with_timezone(&Utc);
        Ok(Some(ProjectionRow {
            id,
            feed_id,
            source_id,
            source_ts,
            title,
            body_text,
            feed_type: feed_type.to_string(),
            metadata,
        }))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WriteOutcome {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
}

enum WriteAction {
    Inserted,
    Updated,
    Unchanged,
}

fn body_hash(body_text: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    body_text.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn write_row(
    tx: &rusqlite::Transaction<'_>,
    feed_type: &str,
    row: &ProjectionRow,
) -> Result<WriteAction, ProjectionError> {
    let now = Utc::now().to_rfc3339();
    let hash = body_hash(&row.body_text);
    let metadata_str = serde_json::to_string(&row.metadata)?;
    let source_ts = row.source_ts.to_rfc3339();

    // Look up existing row by (feed_id, source_id) and decide
    // insert / update-with-fresh-text / unchanged.
    let lookup_sql = format!(
        "SELECT id, body_hash FROM {feed_type} WHERE feed_id = ?1 AND source_id = ?2"
    );
    let mut stmt = tx
        .prepare(&lookup_sql)
        .map_err(|e| ProjectionError::Storage(format!("prepare lookup: {e}")))?;
    let mut rows = stmt
        .query(params![&row.feed_id, &row.source_id])
        .map_err(|e| ProjectionError::Storage(format!("lookup: {e}")))?;

    if let Some(existing) =
        rows.next().map_err(|e| ProjectionError::Storage(e.to_string()))?
    {
        let existing_id: String = existing.get(0)?;
        let existing_hash: String = existing.get(1)?;
        drop(rows);
        drop(stmt);

        if existing_hash == hash {
            // Refresh metadata/timestamps lazily; skip FTS + embed bump.
            let update_sql = format!(
                "UPDATE {feed_type} SET metadata = ?1, source_ts = ?2, updated_at = ?3 \
                 WHERE id = ?4"
            );
            tx.execute(
                &update_sql,
                params![metadata_str, source_ts, now.clone(), existing_id],
            )
            .map_err(|e| ProjectionError::Storage(format!("update metadata: {e}")))?;
            return Ok(WriteAction::Unchanged);
        }

        // Body changed — refresh row + FTS + invalidate embedding cache.
        let update_sql = format!(
            "UPDATE {feed_type} SET title = ?1, body_text = ?2, metadata = ?3, \
                 source_ts = ?4, body_hash = ?5, updated_at = ?6 \
             WHERE id = ?7"
        );
        tx.execute(
            &update_sql,
            params![
                row.title,
                row.body_text,
                metadata_str,
                source_ts,
                hash.clone(),
                now.clone(),
                existing_id.clone(),
            ],
        )
        .map_err(|e| ProjectionError::Storage(format!("update body: {e}")))?;
        fts_upsert(tx, feed_type, &existing_id, &row.title, &row.body_text)?;
        embedding_invalidate(tx, feed_type, &existing_id, &hash)?;
        Ok(WriteAction::Updated)
    } else {
        drop(rows);
        drop(stmt);
        let insert_sql = format!(
            "INSERT INTO {feed_type} \
                (id, feed_id, source_id, source_ts, title, body_text, metadata, \
                 body_hash, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)"
        );
        tx.execute(
            &insert_sql,
            params![
                row.id,
                row.feed_id,
                row.source_id,
                source_ts,
                row.title,
                row.body_text,
                metadata_str,
                hash.clone(),
                now,
            ],
        )
        .map_err(|e| ProjectionError::Storage(format!("insert: {e}")))?;
        fts_upsert(tx, feed_type, &row.id, &row.title, &row.body_text)?;
        embedding_invalidate(tx, feed_type, &row.id, &hash)?;
        Ok(WriteAction::Inserted)
    }
}

fn fts_upsert(
    tx: &rusqlite::Transaction<'_>,
    feed_type: &str,
    projection_id: &str,
    title: &str,
    body_text: &str,
) -> Result<(), ProjectionError> {
    let delete_sql = format!(
        "DELETE FROM {feed_type}_fts WHERE projection_id = ?1"
    );
    tx.execute(&delete_sql, params![projection_id])
        .map_err(|e| ProjectionError::Storage(format!("fts delete: {e}")))?;
    let insert_sql = format!(
        "INSERT INTO {feed_type}_fts (projection_id, title, body_text) VALUES (?1, ?2, ?3)"
    );
    tx.execute(&insert_sql, params![projection_id, title, body_text])
        .map_err(|e| ProjectionError::Storage(format!("fts insert: {e}")))?;
    Ok(())
}

fn embedding_invalidate(
    tx: &rusqlite::Transaction<'_>,
    feed_type: &str,
    projection_id: &str,
    body_hash: &str,
) -> Result<(), ProjectionError> {
    // Upsert into <feed_type>_embeddings with the new body_hash and
    // NULL embedding. The embed pass detects rows where body_hash
    // differs from the embedded vector's keyed hash and refreshes.
    let sql = format!(
        "INSERT INTO {feed_type}_embeddings (projection_id, body_hash, embedding) \
         VALUES (?1, ?2, NULL) \
         ON CONFLICT(projection_id) DO UPDATE SET body_hash = excluded.body_hash, \
             embedding = NULL"
    );
    tx.execute(&sql, params![projection_id, body_hash])
        .map_err(|e| ProjectionError::Storage(format!("embed cache: {e}")))?;
    Ok(())
}

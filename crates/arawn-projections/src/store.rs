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
    /// Accessor for sibling modules (e.g. `embed`) that need to issue
    /// raw SQL against the same connection.
    /// Access the underlying rusqlite connection mutex. Used by
    /// sibling crates that need to run their own queries against the
    /// projection tables (e.g. arawn-extractor's runner walks
    /// `<feed_type>` rows by source_ts).
    pub fn conn(&self) -> &Mutex<Connection> {
        &self.conn
    }

    pub fn open(path: &Path) -> Result<Self, ProjectionError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Auto-load sqlite-vec for this and subsequent connections.
        schema::init_vector_extension();
        let conn = Connection::open(path)
            .map_err(|e| ProjectionError::Storage(format!("open db: {e}")))?;
        schema::apply_pragmas(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> Result<Self, ProjectionError> {
        schema::init_vector_extension();
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
        let feed_types: HashSet<&'static str> = projections.iter().map(|p| p.feed_type()).collect();
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
        let mut params_vec: Vec<&dyn rusqlite::types::ToSql> =
            Vec::with_capacity(1 + candidate_source_ids.len());
        params_vec.push(&feed_id);
        for s in candidate_source_ids {
            params_vec.push(s);
        }
        let mut rows = stmt
            .query(params_vec.as_slice())
            .map_err(|e| ProjectionError::Storage(format!("query missing: {e}")))?;
        let mut present = HashSet::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| ProjectionError::Storage(e.to_string()))?
        {
            let s: String = row.get(0)?;
            present.insert(s);
        }
        Ok(candidate_source_ids
            .iter()
            .filter(|s| !present.contains(s.as_str()))
            .cloned()
            .collect())
    }

    /// Rows still awaiting an embedding (status `pending`, not yet parked)
    /// across every feed type, for the health surface (ARAWN-I-0068 P2-1).
    /// Parked rows — those that exhausted [`crate::embed::MAX_EMBED_RETRIES`]
    /// — are excluded here and reported by [`Self::errored_embedding_count`]
    /// instead.
    pub fn pending_embedding_count(&self) -> Result<u64, ProjectionError> {
        self.count_embeddings_where(&format!(
            "status = 'pending' AND retry_count < {}",
            crate::embed::MAX_EMBED_RETRIES
        ))
    }

    /// Rows parked after repeatedly failing to embed (ARAWN-T-0481) —
    /// `status = 'pending'` but `retry_count` has hit the cap. Surfaced via
    /// `/status` so a stuck embed backlog is visible.
    pub fn errored_embedding_count(&self) -> Result<u64, ProjectionError> {
        self.count_embeddings_where(&format!(
            "status = 'pending' AND retry_count >= {}",
            crate::embed::MAX_EMBED_RETRIES
        ))
    }

    /// Sum a COUNT over every `<feed_type>_embeddings` table for rows
    /// matching `where_clause`. The clause is built from in-crate constants
    /// (never user input). Returns 0 when no embedding tables exist yet.
    fn count_embeddings_where(&self, where_clause: &str) -> Result<u64, ProjectionError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' AND name LIKE '%\\_embeddings' ESCAPE '\\'",
            )
            .map_err(|e| ProjectionError::Storage(format!("list embedding tables: {e}")))?;
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| ProjectionError::Storage(format!("list embedding tables: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| ProjectionError::Storage(format!("list embedding tables: {e}")))?;
        drop(stmt);

        let mut total: u64 = 0;
        for table in tables {
            // Table names come from sqlite_master (not user input); they're
            // identifiers so they can't be bound as query parameters.
            let cnt: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM \"{table}\" WHERE {where_clause}"),
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| ProjectionError::Storage(format!("count {table}: {e}")))?;
            total += cnt.max(0) as u64;
        }
        Ok(total)
    }

    /// Total rows for a feed_type — useful for tests and ops.
    pub fn count(&self, feed_type: &str) -> Result<usize, ProjectionError> {
        let conn = self.conn.lock().unwrap();
        schema::ensure_feed_type_tables(&conn, feed_type)?;
        let cnt: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {feed_type}"), [], |r| {
                r.get(0)
            })
            .map_err(|e| ProjectionError::Storage(format!("count: {e}")))?;
        Ok(cnt as usize)
    }

    /// Vector similarity search over a single feed type. Returns up
    /// to `limit` projection ids sorted by ascending distance against
    /// `query_vec` (closer first). Skipped / pending rows naturally
    /// drop out because they have no row in `<feed_type>_vec`.
    pub fn vector_search(
        &self,
        feed_type: &str,
        query_vec: &[f32],
        limit: usize,
    ) -> Result<Vec<String>, ProjectionError> {
        use zerocopy::IntoBytes;
        if query_vec.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        schema::ensure_feed_type_tables(&conn, feed_type)?;
        let sql = format!(
            "SELECT projection_id FROM {feed_type}_vec \
             WHERE embedding MATCH ?1 \
             ORDER BY distance \
             LIMIT ?2"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| ProjectionError::Storage(format!("prepare vec: {e}")))?;
        let rows = stmt
            .query_map(params![query_vec.as_bytes(), limit as i64], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| ProjectionError::Storage(format!("vec: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| ProjectionError::Storage(e.to_string()))?);
        }
        Ok(out)
    }

    /// FTS search over a single feed type. Returns `(projection_id,
    /// rank)`. Caller hydrates the full row via `get_row`.
    ///
    /// The user-supplied `query` is passed through [`escape_fts5`]
    /// so natural identifiers like `RFC-0042` or `sign-off` don't
    /// trigger FTS5's column-scoped or operator grammar (ARAWN-T-0370).
    /// Empty / whitespace-only queries return an empty result
    /// without touching SQLite — FTS5 rejects an empty MATCH.
    pub fn fts_search(
        &self,
        feed_type: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ProjectionError> {
        let escaped = escape_fts5(query);
        if escaped.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        // Ensure schema exists — `vector_search` and the other read
        // methods do the same. Without this, searching a feed type
        // that has never been written fails with
        // `no such table: <feed_type>_fts` (ARAWN-T-0371).
        schema::ensure_feed_type_tables(&conn, feed_type)?;
        let sql = format!(
            "SELECT projection_id FROM {feed_type}_fts \
             WHERE {feed_type}_fts MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        let run = |match_expr: &str| -> Result<Vec<String>, ProjectionError> {
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| ProjectionError::Storage(format!("prepare fts: {e}")))?;
            let rows = stmt
                .query_map(params![match_expr, limit as i64], |r| r.get::<_, String>(0))
                .map_err(|e| ProjectionError::Storage(format!("fts: {e}")))?;
            let mut ids = Vec::new();
            for r in rows {
                ids.push(r.map_err(|e| ProjectionError::Storage(e.to_string()))?);
            }
            Ok(ids)
        };
        // Primary query is AND (all tokens) — precise. Recall fallback
        // (ARAWN-T-0500): a multi-token query that ANDs to *nothing* retries
        // as OR (any token), ranked by FTS rank. Without this, a realistic
        // natural-language query like "Project Falcon meeting notes" returns 0
        // whenever a single token ("meeting") is absent, even though the row
        // plainly matches the rest.
        let ids = run(&escaped)?;
        if ids.is_empty() && query.split_whitespace().count() > 1 {
            return run(&escape_fts5_or(query));
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

/// Escape a user-supplied query for safe inclusion in an FTS5
/// `MATCH` expression.
///
/// FTS5 grammar gives special meaning to `:`, `-`, `(`, `)`,
/// double-quotes, and the keywords `AND`/`OR`/`NOT`/`NEAR`. A
/// natural identifier like `RFC-0042` is interpreted as a
/// column-scoped query and fails with `no such column: 0042`.
///
/// Strategy: split on whitespace, wrap each token in
/// double-quotes (FTS5 treats `"…"` as a literal phrase with no
/// internal operator parsing), and escape any embedded `"` by
/// doubling per FTS5 quoting rules.
///
/// Tokens are AND-joined (the FTS5 default), so this is the precise
/// query. For recall, [`ProjectionStore::fts_search`] falls back to the
/// OR form ([`escape_fts5_or`]) when the AND query returns nothing
/// (ARAWN-T-0500).
///
/// Returns an empty string for empty / whitespace-only input;
/// callers should treat that as "no query, no results" rather
/// than passing to FTS5 (an empty MATCH is a syntax error).
pub fn escape_fts5(query: &str) -> String {
    escape_fts5_joined(query, " ")
}

/// Like [`escape_fts5`] but OR-joins the quoted tokens, so a row matching
/// *any* term surfaces (ranked by FTS rank). The recall fallback used by
/// [`ProjectionStore::fts_search`] when the precise AND query finds nothing.
pub fn escape_fts5_or(query: &str) -> String {
    escape_fts5_joined(query, " OR ")
}

fn escape_fts5_joined(query: &str, sep: &str) -> String {
    query
        .split_whitespace()
        .map(|tok| {
            let escaped = tok.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(sep)
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
    let lookup_sql =
        format!("SELECT id, body_hash FROM {feed_type} WHERE feed_id = ?1 AND source_id = ?2");
    let mut stmt = tx
        .prepare(&lookup_sql)
        .map_err(|e| ProjectionError::Storage(format!("prepare lookup: {e}")))?;
    let mut rows = stmt
        .query(params![&row.feed_id, &row.source_id])
        .map_err(|e| ProjectionError::Storage(format!("lookup: {e}")))?;

    if let Some(existing) = rows
        .next()
        .map_err(|e| ProjectionError::Storage(e.to_string()))?
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
    let delete_sql = format!("DELETE FROM {feed_type}_fts WHERE projection_id = ?1");
    tx.execute(&delete_sql, params![projection_id])
        .map_err(|e| ProjectionError::Storage(format!("fts delete: {e}")))?;
    let insert_sql = format!(
        "INSERT INTO {feed_type}_fts (projection_id, title, body_text) VALUES (?1, ?2, ?3)"
    );
    tx.execute(&insert_sql, params![projection_id, title, body_text])
        .map_err(|e| ProjectionError::Storage(format!("fts insert: {e}")))?;
    Ok(())
}

/// Mark a projection row's embedding as pending re-compute. On
/// insert / body-changed update we drop any old vector and stamp the
/// status as `pending`; the embed pass will fill in a fresh vector.
fn embedding_invalidate(
    tx: &rusqlite::Transaction<'_>,
    feed_type: &str,
    projection_id: &str,
    body_hash: &str,
) -> Result<(), ProjectionError> {
    // ARAWN-T-0481: a body change resets `retry_count` so a row that
    // previously parked (its old text kept failing the embedder) gets a
    // fresh set of attempts against the new text.
    let meta_sql = format!(
        "INSERT INTO {feed_type}_embeddings (projection_id, body_hash, status, retry_count) \
         VALUES (?1, ?2, 'pending', 0) \
         ON CONFLICT(projection_id) DO UPDATE SET body_hash = excluded.body_hash, \
             status = 'pending', retry_count = 0"
    );
    tx.execute(&meta_sql, params![projection_id, body_hash])
        .map_err(|e| ProjectionError::Storage(format!("embed meta: {e}")))?;
    // Drop any stale vector — the body changed, so the old vector is
    // no longer valid for this row.
    let vec_sql = format!("DELETE FROM {feed_type}_vec WHERE projection_id = ?1");
    tx.execute(&vec_sql, params![projection_id])
        .map_err(|e| ProjectionError::Storage(format!("embed vec drop: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod fts_escape_tests {
    use super::*;
    use crate::types::{Projection, ProjectionRow};
    use chrono::TimeZone;

    // --- escape_fts5 unit tests ---

    #[test]
    fn escape_empty_returns_empty() {
        assert_eq!(escape_fts5(""), "");
        assert_eq!(escape_fts5("   "), "");
        assert_eq!(escape_fts5("\t\n"), "");
    }

    #[test]
    fn escape_quotes_each_token() {
        assert_eq!(escape_fts5("foo"), r#""foo""#);
        assert_eq!(escape_fts5("foo bar"), r#""foo" "bar""#);
    }

    #[test]
    fn escape_or_joins_tokens_with_or() {
        // ARAWN-T-0500 recall fallback form.
        assert_eq!(escape_fts5_or(""), "");
        assert_eq!(escape_fts5_or("solo"), r#""solo""#);
        assert_eq!(escape_fts5_or("foo bar"), r#""foo" OR "bar""#);
        assert_eq!(
            escape_fts5_or("RFC-0042 sign-off"),
            r#""RFC-0042" OR "sign-off""#
        );
    }

    #[test]
    fn escape_neutralises_hyphen() {
        // The bug-triggering tokens from the UAT failure.
        assert_eq!(escape_fts5("RFC-0042"), r#""RFC-0042""#);
        assert_eq!(escape_fts5("sign-off"), r#""sign-off""#);
        assert_eq!(
            escape_fts5("RFC-0042 sign-off Alice"),
            r#""RFC-0042" "sign-off" "Alice""#
        );
    }

    #[test]
    fn escape_neutralises_colon_and_parens() {
        assert_eq!(escape_fts5("foo:bar"), r#""foo:bar""#);
        assert_eq!(escape_fts5("(foo OR bar)"), r#""(foo" "OR" "bar)""#);
    }

    #[test]
    fn escape_doubles_embedded_quotes() {
        // FTS5 phrase quoting: " inside "..." is "" (two doubles).
        assert_eq!(escape_fts5(r#"say "hi""#), r#""say" """hi""""#);
    }

    // --- fts_search end-to-end tests against real SQLite FTS5 ---

    /// Test-only projection that targets the `slack_messages` table
    /// (one of the schemas registered by `ensure_feed_type_tables`).
    struct TestProj {
        id: String,
        feed_id: String,
        source_id: String,
        ts: chrono::DateTime<chrono::Utc>,
        title: String,
        body: String,
    }
    impl Projection for TestProj {
        fn feed_type(&self) -> &'static str {
            "slack_messages"
        }
        fn row(&self) -> ProjectionRow {
            ProjectionRow {
                id: self.id.clone(),
                feed_id: self.feed_id.clone(),
                source_id: self.source_id.clone(),
                source_ts: self.ts,
                title: self.title.clone(),
                body_text: self.body.clone(),
                feed_type: "slack_messages".into(),
                metadata: serde_json::json!({}),
            }
        }
    }

    fn open_store() -> ProjectionStore {
        let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let store = ProjectionStore::open(&path).expect("open store");
        // Ensure feed type table exists so subsequent searches don't
        // race the lazy creation in write_batch.
        store.ensure_feed_type("slack_messages").unwrap();
        store
    }

    fn seed(store: &ProjectionStore, id: &str, title: &str, body: &str) {
        let p = TestProj {
            id: id.into(),
            feed_id: "feed-1".into(),
            source_id: id.into(),
            ts: chrono::Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap(),
            title: title.into(),
            body: body.into(),
        };
        store.write_batch(&[p]).expect("write");
    }

    #[test]
    fn pending_embedding_count_sums_pending_rows() {
        let store = open_store();
        // No rows yet → zero backlog (and no panic on the empty-table set).
        assert_eq!(store.pending_embedding_count().unwrap(), 0);
        // Each freshly-written projection row is marked embedding-pending.
        seed(&store, "m1", "one", "body one");
        seed(&store, "m2", "two", "body two");
        assert_eq!(store.pending_embedding_count().unwrap(), 2);
    }

    #[test]
    fn write_embedding_rejects_stale_body_hash() {
        // ARAWN-T-0481: a vector computed for text that has since changed must
        // NOT be committed — the row stays pending and re-embeds later.
        let store = open_store();
        let body = "this is a sufficiently long body for embedding";
        seed(&store, "m1", "title", body);
        let vector = vec![0.1f32; crate::schema::EMBEDDING_DIMS];

        // Wrong hash → no-op, row stays pending.
        store
            .write_embedding("slack_messages", "m1", &vector, "STALEHASH")
            .unwrap();
        assert_eq!(
            store.pending_embedding_count().unwrap(),
            1,
            "a stale-hash write must leave the row pending"
        );

        // Correct hash → the row embeds.
        let correct = body_hash(body);
        store
            .write_embedding("slack_messages", "m1", &vector, &correct)
            .unwrap();
        assert_eq!(
            store.pending_embedding_count().unwrap(),
            0,
            "a matching-hash write embeds the row"
        );
    }

    #[test]
    fn parked_rows_drop_out_of_pending_and_count_as_errored() {
        // ARAWN-T-0481: a row whose batch keeps failing parks after the retry
        // cap — it leaves the pending fetch (so it stops blocking the queue)
        // and is reported as errored instead.
        let store = open_store();
        seed(&store, "m1", "t", "a long enough body for embedding work");
        assert_eq!(store.pending_embedding_count().unwrap(), 1);

        for _ in 0..crate::embed::MAX_EMBED_RETRIES {
            store
                .bump_embedding_retries("slack_messages", &["m1"])
                .unwrap();
        }

        assert_eq!(
            store.pending_embedding_count().unwrap(),
            0,
            "a parked row is no longer counted pending"
        );
        assert_eq!(
            store.errored_embedding_count().unwrap(),
            1,
            "a parked row is counted errored"
        );
        assert!(
            store
                .pending_embedding_rows("slack_messages", 10)
                .unwrap()
                .is_empty(),
            "a parked row is excluded from the pending fetch"
        );
    }

    #[test]
    fn hyphenated_identifier_matches_post_fix() {
        let store = open_store();
        seed(
            &store,
            "m1",
            "Re: RFC-0042 sign-off",
            "Alice asked about the doc",
        );
        let hits = store
            .fts_search("slack_messages", "RFC-0042", 10)
            .expect("search must not error");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn hyphenated_phrase_matches() {
        let store = open_store();
        seed(&store, "m1", "Sign-off needed", "review-comments-pending");
        let hits = store
            .fts_search("slack_messages", "sign-off", 10)
            .expect("search");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn multi_token_is_implicit_and() {
        let store = open_store();
        seed(&store, "m1", "RFC-0042 by Alice", "sign-off requested");
        seed(&store, "m2", "Random other thread", "no RFC here");
        let hits = store
            .fts_search("slack_messages", "RFC-0042 Alice", 10)
            .expect("search");
        // m1 contains both tokens; m2 contains neither.
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn verbose_query_falls_back_to_or_for_recall() {
        // ARAWN-T-0500: a realistic multi-word query where one token is absent
        // must still surface the matching row (recall), via the OR fallback.
        let store = open_store();
        seed(
            &store,
            "m1",
            "Falcon standup",
            "decision: migrate the ledger to Postgres 16. Codename Operation Bluefin.",
        );
        seed(&store, "m2", "Unrelated thread", "lunch plans");
        // "meeting"/"notes" are absent from m1, so strict AND → 0; OR surfaces m1.
        let hits = store
            .fts_search("slack_messages", "Falcon meeting notes migration", 10)
            .expect("search");
        assert_eq!(
            hits,
            vec!["m1".to_string()],
            "OR fallback must surface the partial match"
        );
    }

    #[test]
    fn or_fallback_does_not_fire_when_and_matches() {
        // When the precise AND query already matches, keep it — don't broaden
        // to OR (which would also pull in m2's lone "Falcon").
        let store = open_store();
        seed(&store, "m1", "Falcon migration", "Postgres 16 cutover");
        seed(&store, "m2", "Falcon lunch", "tacos");
        let hits = store
            .fts_search("slack_messages", "Falcon migration", 10)
            .expect("search");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn colon_in_query_does_not_trigger_column_lookup() {
        let store = open_store();
        seed(&store, "m1", "label:work-mode", "tagging convention");
        let hits = store
            .fts_search("slack_messages", "label:work-mode", 10)
            .expect("search");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn empty_query_returns_empty_without_error() {
        let store = open_store();
        seed(&store, "m1", "anything", "anywhere");
        let hits = store.fts_search("slack_messages", "", 10).expect("ok");
        assert!(hits.is_empty());
        let hits2 = store.fts_search("slack_messages", "   ", 10).expect("ok");
        assert!(hits2.is_empty());
    }

    /// T-0371: searching a feed type that has never been written
    /// must not error with `no such table`. The schema is lazily
    /// created on first search just like every other read path.
    #[test]
    fn search_unwritten_feed_type_returns_empty_not_error() {
        // Open store, but do NOT seed anything for the queried feed
        // type. `open_store` ensures `slack_messages`, so pick a
        // different never-touched feed type for this test.
        let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let store = ProjectionStore::open(&path).expect("open store");
        // No ensure_feed_type, no writes — exactly the UAT failure
        // mode where jira_history had no rows.
        let hits = store
            .fts_search("jira_history", "RFC-0042", 10)
            .expect("must not error on unwritten feed type");
        assert!(hits.is_empty());
    }
}

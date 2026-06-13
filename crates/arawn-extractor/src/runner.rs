//! `ExtractorRunner` — drives the extraction chain across new
//! projection rows for a given (lens, feed_type).
//!
//! Public surface:
//! - `run_for_lens(lens, feed_type)` — process one batch
//!   of new rows since the cursor and advance.
//! - `run_for_all_lenses(feed_type)` — iterate active lenses
//!   (skipping archived) and run each. Called from the feeds dispatch
//!   hook after a projection write.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::params;
use tracing::{debug, info, warn};

use arawn_core::Lens;
use arawn_projections::{ProjectionRow, ProjectionStore};
use arawn_storage::{ExtractionLogStore, ExtractionOutcome, ExtractorCursorStore, Store};
use uuid::Uuid;

use crate::chain::ExtractionChain;
use crate::error::ExtractionError;

/// Stats for one `run_for_lens` invocation.
#[derive(Debug, Default, Clone)]
pub struct RunStats {
    pub processed: usize,
    pub kept: usize,
    pub skipped: usize,
    pub errors: usize,
    pub entities_written: usize,
    pub relations_written: usize,
}

/// Default cap on rows per `run_for_lens` invocation. Backfill
/// loops this until exhausted; the dispatch hook fires single bursts.
pub const DEFAULT_BATCH_SIZE: usize = 50;

/// Function that materializes the `MemoryManager` for a lens
/// name. In production this is the same router T-0250 wired into the
/// memory tools; tests pass an in-memory fixture.
pub type MemoryResolver =
    Arc<dyn Fn(&str) -> Result<Arc<arawn_memory::MemoryManager>, ExtractionError> + Send + Sync>;

/// The runner owns the bits that survive across calls — store handles,
/// the chain, the memory resolver. Per-call args are the lens +
/// feed_type and any batch-size overrides.
pub struct ExtractorRunner {
    store: Arc<std::sync::Mutex<Store>>,
    projections: Arc<ProjectionStore>,
    memory: MemoryResolver,
    chain: Arc<dyn ExtractionChain>,
    batch_size: usize,
    /// In-flight backfill gate. Prevents two simultaneous backfills
    /// for the same (lens, feed_type) from racing.
    in_flight: Arc<std::sync::Mutex<std::collections::HashSet<(String, String)>>>,
}

impl ExtractorRunner {
    pub fn new(
        store: Arc<std::sync::Mutex<Store>>,
        projections: Arc<ProjectionStore>,
        memory: MemoryResolver,
        chain: Arc<dyn ExtractionChain>,
    ) -> Self {
        Self {
            store,
            projections,
            memory,
            chain,
            batch_size: DEFAULT_BATCH_SIZE,
            in_flight: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub fn with_batch_size(mut self, n: usize) -> Self {
        self.batch_size = n.max(1);
        self
    }

    /// Process one batch of new projection rows for `lens`. Reads
    /// the cursor, queries `<feed_type> WHERE source_ts > cursor
    /// ORDER BY source_ts ASC LIMIT batch`, dispatches each through the
    /// chain, and advances the cursor on success.
    pub async fn run_for_lens(
        &self,
        lens: &Lens,
        feed_type: &str,
    ) -> Result<RunStats, ExtractionError> {
        let cursor_ts = {
            let store = self.store.lock().unwrap();
            let cursor_store = ExtractorCursorStore::new(store.database());
            cursor_store
                .get(&lens.name, feed_type)?
                .and_then(|c| c.last_source_ts)
        };

        let rows = fetch_projection_rows(&self.projections, feed_type, cursor_ts, self.batch_size)?;
        if rows.is_empty() {
            return Ok(RunStats::default());
        }

        let kb = (self.memory)(&lens.name)?;

        // One provenance id per pass (ARAWN-T-0484) — stamped on every
        // extraction_log row this pass writes.
        let run_id = Uuid::new_v4().to_string();

        let mut stats = RunStats::default();
        let mut latest_processed_ts: Option<DateTime<Utc>> = cursor_ts;
        for row in &rows {
            stats.processed += 1;

            // ARAWN-T-0484: a dismissed row is skipped without re-extracting,
            // so a signal the user threw away doesn't reappear on the next
            // pass. The cursor still advances past it.
            let dismissed = {
                let store = self.store.lock().unwrap();
                ExtractionLogStore::new(store.database())
                    .is_dismissed(&lens.name, &row.id)
                    .unwrap_or(false)
            };
            if dismissed {
                stats.skipped += 1;
                if latest_processed_ts
                    .map(|p| row.source_ts > p)
                    .unwrap_or(true)
                {
                    latest_processed_ts = Some(row.source_ts);
                }
                continue;
            }

            match self.chain.run(lens, row, &kb).await {
                Ok(outcome) => {
                    let log_outcome = if outcome.skipped {
                        stats.skipped += 1;
                        ExtractionOutcome::Skipped
                    } else if outcome.entities_written.is_empty() {
                        // In scope but produced nothing — "empty", not "skipped".
                        stats.kept += 1;
                        ExtractionOutcome::Empty
                    } else {
                        stats.kept += 1;
                        stats.entities_written += outcome.entities_written.len();
                        stats.relations_written += outcome.relations_written;
                        ExtractionOutcome::Ok
                    };
                    // Record the decision (best-effort — a log failure must
                    // not abort extraction).
                    {
                        let store = self.store.lock().unwrap();
                        if let Err(e) = ExtractionLogStore::new(store.database()).record(
                            &lens.name,
                            &row.id,
                            &run_id,
                            log_outcome,
                            outcome.reason.as_deref(),
                        ) {
                            warn!(lens = %lens.name, row_id = %row.id, error = %e,
                                  "failed to record extraction_log row");
                        }
                    }
                    if latest_processed_ts
                        .map(|p| row.source_ts > p)
                        .unwrap_or(true)
                    {
                        latest_processed_ts = Some(row.source_ts);
                    }
                }
                Err(e) => {
                    warn!(
                        lens = %lens.name,
                        feed_type = feed_type,
                        row_id = %row.id,
                        error = %e,
                        "extractor chain failed; cursor stays put for retry"
                    );
                    stats.errors += 1;
                    // On error we stop advancing so the next run retries this row.
                    break;
                }
            }
        }

        if let Some(ts) = latest_processed_ts {
            let store = self.store.lock().unwrap();
            let cursor_store = ExtractorCursorStore::new(store.database());
            cursor_store.advance(&lens.name, feed_type, ts)?;
        }

        if stats.processed > 0 {
            debug!(
                lens = %lens.name,
                feed_type = feed_type,
                processed = stats.processed,
                kept = stats.kept,
                skipped = stats.skipped,
                errors = stats.errors,
                "extractor batch done"
            );
        }
        Ok(stats)
    }

    /// Run `run_for_lens` in a loop until either the projection
    /// stream is exhausted, an error halts the cursor, or `max_duration`
    /// elapses. Used when a lens binds a new source so existing
    /// projection rows get walked through the chain.
    ///
    /// Returns aggregated stats across all loop iterations.
    pub async fn run_for_lens_until_exhausted(
        &self,
        lens: &Lens,
        feed_type: &str,
        max_duration: std::time::Duration,
    ) -> Result<RunStats, ExtractionError> {
        let deadline = std::time::Instant::now() + max_duration;
        let mut total = RunStats::default();
        loop {
            if std::time::Instant::now() >= deadline {
                debug!(
                    lens = %lens.name,
                    feed_type = feed_type,
                    "backfill wall-clock cap hit; next trigger will resume"
                );
                break;
            }
            let stats = self.run_for_lens(lens, feed_type).await?;
            if stats.processed == 0 {
                break;
            }
            total.processed += stats.processed;
            total.kept += stats.kept;
            total.skipped += stats.skipped;
            total.errors += stats.errors;
            total.entities_written += stats.entities_written;
            total.relations_written += stats.relations_written;
            if stats.errors > 0 {
                // Chain returned an error; cursor stayed put. Bail so
                // we don't spin forever on a poison row.
                break;
            }
        }
        Ok(total)
    }

    /// Spawn a backfill task for `(lens_name, feed_types)`. Each
    /// `feed_type` gets its own `run_for_lens_until_exhausted`
    /// invocation under the 10-minute wall-clock cap. Idempotent: if a
    /// backfill for the same `(lens, feed_type)` is already in
    /// flight the new request is dropped.
    ///
    /// Fire-and-forget — the function returns immediately and the
    /// task runs in the tokio runtime.
    pub fn spawn_backfill(self: Arc<Self>, lens_name: String, feed_types: Vec<String>) {
        const MAX: std::time::Duration = std::time::Duration::from_secs(10 * 60);
        for feed_type in feed_types {
            let key = (lens_name.clone(), feed_type.clone());
            {
                let mut guard = self.in_flight.lock().unwrap();
                if !guard.insert(key.clone()) {
                    debug!(
                        lens = %lens_name,
                        feed_type = %feed_type,
                        "backfill already in flight; skipping"
                    );
                    continue;
                }
            }
            let runner = Arc::clone(&self);
            let ws_name = lens_name.clone();
            let ft = feed_type.clone();
            tokio::spawn(async move {
                // Look up the lens record for the run.
                let lens = {
                    let store = runner.store.lock().unwrap();
                    match store.find_lens_by_name(&ws_name) {
                        Ok(Some(ws)) => ws,
                        Ok(None) => {
                            warn!(lens = %ws_name, "backfill: lens not found");
                            runner.in_flight.lock().unwrap().remove(&key);
                            return;
                        }
                        Err(e) => {
                            warn!(error = %e, "backfill: lens lookup failed");
                            runner.in_flight.lock().unwrap().remove(&key);
                            return;
                        }
                    }
                };
                match runner.run_for_lens_until_exhausted(&lens, &ft, MAX).await {
                    Ok(stats) => info!(
                        lens = %ws_name,
                        feed_type = %ft,
                        processed = stats.processed,
                        kept = stats.kept,
                        skipped = stats.skipped,
                        errors = stats.errors,
                        "backfill complete"
                    ),
                    Err(e) => warn!(
                        lens = %ws_name,
                        feed_type = %ft,
                        error = %e,
                        "backfill failed"
                    ),
                }
                runner.in_flight.lock().unwrap().remove(&key);
            });
        }
    }

    /// Iterate every active (non-archived) lens and run extraction
    /// for `feed_type`. Soft-fails per-lens so one bad lens
    /// can't block the rest. Used by the feeds dispatch hook after
    /// projection writes.
    pub async fn run_for_all_lenses(
        &self,
        feed_type: &str,
    ) -> Result<Vec<(String, RunStats)>, ExtractionError> {
        let lenses: Vec<Lens> = {
            let store = self.store.lock().unwrap();
            store.list_lenses()?
        };
        let mut out = Vec::with_capacity(lenses.len());
        for ws in lenses {
            match self.run_for_lens(&ws, feed_type).await {
                Ok(stats) => out.push((ws.name, stats)),
                Err(e) => {
                    warn!(
                        lens = %ws.name,
                        feed_type = feed_type,
                        error = %e,
                        "lens extractor failed; continuing with next lens"
                    );
                    out.push((
                        ws.name,
                        RunStats {
                            errors: 1,
                            ..Default::default()
                        },
                    ));
                }
            }
        }
        if !out.is_empty() {
            info!(
                feed_type = feed_type,
                lenses = out.len(),
                "extractor fan-out complete"
            );
        }
        Ok(out)
    }
}

/// Page projection rows of a given feed_type whose `source_ts` is
/// strictly greater than `cursor_ts` (or every row when cursor is None).
fn fetch_projection_rows(
    store: &ProjectionStore,
    feed_type: &str,
    cursor_ts: Option<DateTime<Utc>>,
    limit: usize,
) -> Result<Vec<ProjectionRow>, ExtractionError> {
    let conn_lock = store.conn().lock().unwrap();
    let conn: &rusqlite::Connection = &conn_lock;
    // Ensure the feed_type's tables exist — protects against a fresh
    // db where the projection writer hasn't run yet for this type.
    arawn_projections::schema::ensure_feed_type_tables(conn, feed_type)?;

    let cursor_str = cursor_ts.map(|d| d.to_rfc3339()).unwrap_or_default();
    let sql = format!(
        "SELECT id, feed_id, source_id, source_ts, title, body_text, metadata \
         FROM {feed_type} \
         WHERE source_ts > ?1 \
         ORDER BY source_ts ASC \
         LIMIT ?2"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| ExtractionError::Storage(format!("prepare fetch: {e}")))?;
    let rows = stmt
        .query_map(
            params![cursor_str, limit as i64],
            |row| -> rusqlite::Result<(String, String, String, String, String, String, String)> {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|e| ExtractionError::Storage(format!("query rows: {e}")))?;

    let mut out = Vec::new();
    for r in rows {
        let (id, feed_id, source_id, source_ts_str, title, body_text, metadata_str) =
            r.map_err(|e: rusqlite::Error| ExtractionError::Storage(e.to_string()))?;
        let source_ts = DateTime::parse_from_rfc3339(&source_ts_str)
            .map_err(|e| ExtractionError::Parse(format!("source_ts: {e}")))?
            .with_timezone(&Utc);
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
        out.push(ProjectionRow {
            id,
            feed_id,
            source_id,
            source_ts,
            title,
            body_text,
            feed_type: feed_type.to_string(),
            metadata,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::StubChain;
    use arawn_memory::MemoryManager;
    use arawn_projections::gmail::GmailMessageProjection;

    fn ws(name: &str) -> Lens {
        let mut w = Lens::new(name, std::env::temp_dir().join(name));
        w.description = format!("test ws {name}").to_string();
        w
    }

    fn fixture_proj(id: &str, body: &str, ts_offset: i64) -> GmailMessageProjection {
        GmailMessageProjection {
            id: arawn_projections::gmail::projection_id("feed-1", id),
            feed_id: "feed-1".into(),
            source_id: id.into(),
            source_ts: chrono::Utc::now() + chrono::Duration::seconds(ts_offset),
            sender: Some("a@e.com".into()),
            recipients: vec![],
            subject: format!("subj-{id}"),
            body_text: body.into(),
            thread_id: None,
            labels: vec![],
        }
    }

    fn setup() -> (
        tempfile::TempDir,
        Arc<std::sync::Mutex<Store>>,
        Arc<ProjectionStore>,
        MemoryResolver,
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_lens().unwrap();
        let store = Arc::new(std::sync::Mutex::new(store));

        let proj_path = tmp.path().join("projections.db");
        let proj = Arc::new(ProjectionStore::open(&proj_path).unwrap());

        let data_dir = tmp.path().to_path_buf();
        let resolver: MemoryResolver = Arc::new(move |name: &str| {
            MemoryManager::for_lens(&data_dir, name, None)
                .map(Arc::new)
                .map_err(|e| ExtractionError::Memory(e.to_string()))
        });

        (tmp, store, proj, resolver)
    }

    #[tokio::test]
    async fn empty_projection_table_is_a_noop() {
        let (_tmp, store, proj, resolver) = setup();
        let runner = ExtractorRunner::new(store, proj, resolver, Arc::new(StubChain));
        let stats = runner
            .run_for_lens(&ws("pat"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 0);
    }

    #[tokio::test]
    async fn stub_chain_advances_cursor_and_marks_skipped() {
        let (_tmp, store, proj, resolver) = setup();
        // Seed three projection rows.
        let p1 = fixture_proj("m1", "alpha body", 0);
        let p2 = fixture_proj("m2", "beta body", 10);
        let p3 = fixture_proj("m3", "gamma body", 20);
        proj.write_batch(&[p1, p2, p3.clone()]).unwrap();

        let runner = ExtractorRunner::new(Arc::clone(&store), proj, resolver, Arc::new(StubChain));
        let stats = runner
            .run_for_lens(&ws("pat"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 3);
        assert_eq!(stats.skipped, 3);
        assert_eq!(stats.kept, 0);

        // Cursor advanced to p3's source_ts.
        let s = store.lock().unwrap();
        let cs = ExtractorCursorStore::new(s.database());
        let c = cs.get("pat", "gmail_messages").unwrap().unwrap();
        assert_eq!(c.last_source_ts, Some(p3.source_ts));
    }

    #[tokio::test]
    async fn rerun_with_no_new_rows_is_a_noop() {
        let (_tmp, store, proj, resolver) = setup();
        proj.write_batch(&[fixture_proj("m1", "body", 0)]).unwrap();
        let runner = ExtractorRunner::new(store, proj, resolver, Arc::new(StubChain));
        let _ = runner
            .run_for_lens(&ws("pat"), "gmail_messages")
            .await
            .unwrap();
        let stats = runner
            .run_for_lens(&ws("pat"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 0);
    }

    #[tokio::test]
    async fn run_until_exhausted_walks_all_pages() {
        let (_tmp, store, proj, resolver) = setup();
        // Seed 7 rows; batch size of 3 forces multiple iterations.
        let rows: Vec<_> = (0..7)
            .map(|i| fixture_proj(&format!("m{i}"), "body", i as i64))
            .collect();
        proj.write_batch(&rows).unwrap();

        let runner =
            ExtractorRunner::new(store, proj, resolver, Arc::new(StubChain)).with_batch_size(3);
        let stats = runner
            .run_for_lens_until_exhausted(
                &ws("pat"),
                "gmail_messages",
                std::time::Duration::from_secs(30),
            )
            .await
            .unwrap();
        assert_eq!(stats.processed, 7);
    }

    #[tokio::test]
    async fn spawn_backfill_is_idempotent_for_in_flight_key() {
        let (_tmp, store, proj, resolver) = setup();
        proj.write_batch(&[fixture_proj("m1", "body", 0)]).unwrap();
        // Pre-mark the key as in-flight so the spawn becomes a no-op.
        let runner = Arc::new(ExtractorRunner::new(
            Arc::clone(&store),
            proj,
            resolver,
            Arc::new(StubChain),
        ));
        runner
            .in_flight
            .lock()
            .unwrap()
            .insert(("pat".into(), "gmail_messages".into()));
        // create the lens so the spawn doesn't bail on lookup.
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", std::env::temp_dir().join("pat")))
            .unwrap();
        // spawn_backfill returns immediately; the second call is the
        // one we're asserting is dropped.
        Arc::clone(&runner).spawn_backfill("pat".into(), vec!["gmail_messages".into()]);
        // Cursor should NOT have advanced because the second spawn was
        // a no-op.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let s = store.lock().unwrap();
        let cs = ExtractorCursorStore::new(s.database());
        let c = cs.get("pat", "gmail_messages").unwrap();
        assert!(c.is_none(), "cursor should not have advanced");
    }

    #[tokio::test]
    async fn run_for_all_lenses_iterates_active_only() {
        let (_tmp, store, proj, resolver) = setup();
        proj.write_batch(&[fixture_proj("m1", "body", 0)]).unwrap();
        // Add a real lens + archive a third.
        {
            let s = store.lock().unwrap();
            s.create_lens(&Lens::new("pat", std::env::temp_dir().join("pat")))
                .unwrap();
            s.create_lens(&Lens::new("old", std::env::temp_dir().join("old")))
                .unwrap();
            s.soft_delete_lens("old").unwrap();
        }
        let runner = ExtractorRunner::new(store, proj, resolver, Arc::new(StubChain));
        let results = runner.run_for_all_lenses("gmail_messages").await.unwrap();
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"scratch"));
        assert!(names.contains(&"pat"));
        assert!(!names.contains(&"old"), "archived lens should be excluded");
    }
}

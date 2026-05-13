//! `FeedDispatchTask` — the single cloacina `Task` impl every feed
//! routes through.
//!
//! Per I-0039: feeds are *not* compiled `.cloacina` packages and are
//! *not* shared between feeds. Each configured feed registers its own
//! `Workflow` (named `feed::<feed_id>`) wrapping one `FeedDispatchTask`
//! that captures the feed's id. At fire time the task:
//!
//! 1. Looks up the feed row by id from the `feeds` table.
//! 2. Resolves the template from the registry.
//! 3. Reads the prior cursor from `meta.json`.
//! 4. Calls `template.run(ctx, params, feed_dir, &cursor)`.
//! 5. Atomically writes the new cursor + last_run + status to `meta.json`.
//!
//! Failures inside the template surface as `cloacina::TaskError`; cloacina's
//! retry/audit machinery handles the rest.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use cloacina::{Context, Task, TaskError, TaskNamespace};
use rusqlite::Connection;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::clients::FeedClients;
use crate::error::FeedError;
use crate::layout::DataLayout;
use crate::meta::MetaStore;
use crate::registry::FeedTemplateRegistry;
use crate::store::FeedStore;
use crate::template::TemplateCtx;
use crate::types::FeedMeta;

/// Shared handles the dispatch task needs to actually run. Cloned (Arc)
/// into every feed's task; cheap because everything inside is an Arc /
/// Mutex / cloneable.
#[derive(Clone)]
pub struct FeedRuntimeContext {
    pub conn: Arc<Mutex<Connection>>,
    pub layout: Arc<DataLayout>,
    pub registry: Arc<FeedTemplateRegistry>,
    pub clients: Arc<dyn FeedClients>,
    /// Optional projection store. When set, every successful feed run
    /// fans out to project items into the per-feed-type tables. Left
    /// optional so feeds can run standalone (e.g. tests) without a
    /// projection backend.
    pub projections: Option<Arc<arawn_projections::ProjectionStore>>,
    /// Optional per-workstream extractor. When set (and projections is
    /// also set), each successful feed run fans out to the extractor
    /// after projection writes — every active workstream evaluates the
    /// new projection rows against its scope.
    pub extractor: Option<Arc<arawn_extractor::ExtractorRunner>>,
}

/// One cloacina-compatible task per feed. Captures the feed's id so we
/// can look up everything else we need at fire time without smuggling
/// state through cloacina's `Context`.
pub struct FeedDispatchTask {
    feed_id: String,
    runtime: FeedRuntimeContext,
    /// Empty — this task has no upstream deps inside its single-task
    /// workflow.
    deps: Vec<TaskNamespace>,
}

impl FeedDispatchTask {
    pub fn new(feed_id: impl Into<String>, runtime: FeedRuntimeContext) -> Self {
        Self {
            feed_id: feed_id.into(),
            runtime,
            deps: Vec::new(),
        }
    }
}

#[async_trait]
impl Task for FeedDispatchTask {
    fn id(&self) -> &str {
        &self.feed_id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.deps
    }

    async fn execute(
        &self,
        context: Context<Value>,
    ) -> Result<Context<Value>, TaskError> {
        run_feed(&self.feed_id, &self.runtime)
            .await
            .map_err(|e| TaskError::ExecutionFailed {
                message: format!("feed '{}' failed: {e}", self.feed_id),
                task_id: self.feed_id.clone(),
                timestamp: Utc::now(),
            })?;
        Ok(context)
    }
}

/// The actual fetch+write cycle. Pulled out of the trait impl so it's
/// also callable from tests without standing up a cloacina runtime.
///
/// Honors the row's `enabled` flag: disabled feeds short-circuit
/// with `status="skipped-disabled"`. The backfill loop in
/// `runtime::spawn_backfill_task` needs to bypass that check (it
/// runs against an enabled=0 row by design); use [`run_feed_force`]
/// there.
pub async fn run_feed(
    feed_id: &str,
    runtime: &FeedRuntimeContext,
) -> Result<crate::template::RunOutcome, FeedError> {
    run_feed_inner(feed_id, runtime, false).await
}

/// Variant that ignores the `enabled` flag — used by the backfill
/// loop, which runs against rows it deliberately set to `enabled=0`
/// to prevent cron firings during the loop.
pub async fn run_feed_force(
    feed_id: &str,
    runtime: &FeedRuntimeContext,
) -> Result<crate::template::RunOutcome, FeedError> {
    run_feed_inner(feed_id, runtime, true).await
}

async fn run_feed_inner(
    feed_id: &str,
    runtime: &FeedRuntimeContext,
    force: bool,
) -> Result<crate::template::RunOutcome, FeedError> {
    // 1. Look up the feed row.
    let record = {
        let conn = runtime.conn.lock().await;
        let store = FeedStore::new(&conn);
        store
            .get(feed_id)?
            .ok_or_else(|| FeedError::Storage(format!("feed '{feed_id}' not in DB")))?
    };

    if !record.enabled && !force {
        // Disabled feeds shouldn't have an active cron schedule, but
        // if one fires anyway (e.g. mid-pause race) we no-op cleanly.
        return Ok(crate::template::RunOutcome {
            cursor: Value::Null,
            summary: Default::default(),
            status: "skipped-disabled".into(),
        });
    }

    // 2. Resolve the template.
    let template = runtime.registry.require(&record.template)?;

    // 3. Build TemplateCtx + ensure feed dir.
    let feed_dir: PathBuf = runtime.layout.ensure_feed_dir(&record.template, &record.id)?;
    let ctx = TemplateCtx::new(runtime.clients.clone());

    // 4. Read prior cursor (or default to JSON null on first run).
    let prior_meta = MetaStore::read(&feed_dir)?;
    let cursor = prior_meta
        .as_ref()
        .map(|m| m.cursor.clone())
        .unwrap_or(Value::Null);

    // 5. Run the template.
    let outcome = match template
        .run(&ctx, &record.params, &feed_dir, &cursor)
        .await
    {
        Ok(o) => o,
        Err(e) => {
            // Update meta to surface the failure status without
            // advancing the cursor. This way operators see the last
            // failure even if cloacina's audit row is mined later.
            persist_meta_failure(&feed_dir, &record.template, &record.params, &cursor, &e)
                .unwrap_or_else(|persist_err| {
                    warn!(feed_id = %record.id, error = %persist_err,
                          "failed to persist failure-status meta.json");
                });
            error!(feed_id = %record.id, error = %e, "feed run failed");
            return Err(e);
        }
    };

    // 6. Atomically persist the new cursor + last_run.
    let mut meta = prior_meta.unwrap_or_else(|| {
        FeedMeta::new(record.template.clone(), record.params.clone(), cursor.clone())
    });
    meta.template = record.template.clone();
    meta.params = record.params.clone();
    meta.cursor = outcome.cursor.clone();
    meta.last_run_at = Some(Utc::now().to_rfc3339());
    meta.last_status = Some(outcome.status.clone());
    meta.run_count += 1;
    MetaStore::write(&feed_dir, &meta)?;

    info!(
        feed_id = %record.id,
        template = %record.template,
        items = outcome.summary.items_written,
        bytes = outcome.summary.bytes_written,
        status = %outcome.status,
        "feed run complete"
    );

    // 7. Fan out into the projection layer if configured. Projection
    // failures must not fail the feed run — they're a downstream view.
    let projections_touched_types = if let Some(projections) = runtime.projections.as_ref() {
        match arawn_projections::project_feed_dir(
            projections,
            &record.template,
            &record.id,
            &feed_dir,
        ) {
            Ok(out) => {
                if out.inserted > 0 || out.updated > 0 {
                    info!(
                        feed_id = %record.id,
                        template = %record.template,
                        inserted = out.inserted,
                        updated = out.updated,
                        "projection sync"
                    );
                }
                projection_feed_types_for(&record.template)
            }
            Err(e) => {
                warn!(
                    feed_id = %record.id,
                    template = %record.template,
                    error = %e,
                    "projection sync failed; feed run still considered successful"
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // 8. Fan out into the per-workstream extractor for each
    // projection feed_type this template touched. Reactive trigger
    // (downstream of capture) per the I-0040 phase 4 design.
    // Soft-fails — extractor errors must not fail the feed run.
    if let Some(extractor) = runtime.extractor.as_ref() {
        for feed_type in projections_touched_types {
            match extractor.run_for_all_workstreams(&feed_type).await {
                Ok(per_ws) => {
                    let total_kept: usize = per_ws.iter().map(|(_, s)| s.kept).sum();
                    if total_kept > 0 {
                        info!(
                            feed_id = %record.id,
                            feed_type = %feed_type,
                            workstreams = per_ws.len(),
                            kept = total_kept,
                            "extractor fan-out"
                        );
                    }
                }
                Err(e) => warn!(
                    feed_id = %record.id,
                    feed_type = %feed_type,
                    error = %e,
                    "extractor fan-out failed; feed run still successful"
                ),
            }
        }
    }

    Ok(outcome)
}

/// Map a feed template name to the projection feed_types it produces.
/// Slack writes to two tables (top-level + thread replies); other
/// templates write to one each. Used to decide which extractor
/// invocations to fan out after a feed run.
fn projection_feed_types_for(template_name: &str) -> Vec<String> {
    let provider = template_name.split('/').next().unwrap_or(template_name);
    match provider {
        "gmail" => vec!["gmail_messages".into()],
        "slack" => vec!["slack_messages".into(), "slack_thread_messages".into()],
        "drive" => vec!["drive_files".into()],
        "jira" => vec![
            "jira_issues".into(),
            "jira_comments".into(),
            "jira_history".into(),
        ],
        "confluence" => vec!["confluence_pages".into()],
        "calendar" => vec!["calendar_events".into()],
        _ => Vec::new(),
    }
}

fn persist_meta_failure(
    feed_dir: &std::path::Path,
    template: &str,
    params: &crate::types::TemplateParams,
    cursor: &Value,
    err: &FeedError,
) -> Result<(), FeedError> {
    let prior = MetaStore::read(feed_dir)?;
    let mut meta = prior.unwrap_or_else(|| {
        FeedMeta::new(template, params.clone(), cursor.clone())
    });
    meta.last_run_at = Some(Utc::now().to_rfc3339());
    meta.last_status = Some(format!("error: {err}"));
    meta.run_count += 1;
    MetaStore::write(feed_dir, &meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::NoopClients;
    use crate::store::new_record;
    use crate::templates::default_registry;
    use crate::types::TemplateParams;
    use serde_json::json;
    use tempfile::tempdir;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE feeds (
                id TEXT PRIMARY KEY,
                template TEXT NOT NULL,
                params TEXT NOT NULL,
                cadence TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    fn build_runtime(tmp_root: &std::path::Path, conn: Connection) -> FeedRuntimeContext {
        FeedRuntimeContext {
            conn: Arc::new(Mutex::new(conn)),
            layout: Arc::new(DataLayout::new(tmp_root)),
            registry: Arc::new(default_registry()),
            clients: Arc::new(NoopClients),
            projections: None,
            extractor: None,
        }
    }

    #[tokio::test]
    async fn run_feed_executes_stub_template_and_persists_meta() {
        let tmp = tempdir().unwrap();
        let conn = open_test_db();
        {
            let store = FeedStore::new(&conn);
            store
                .insert(&new_record(
                    "test-stub",
                    "stub/echo",
                    TemplateParams::new(json!({ "message": "hi" })),
                    "*/15 * * * *",
                ))
                .unwrap();
        }
        let runtime = build_runtime(tmp.path(), conn);

        let outcome = run_feed("test-stub", &runtime).await.unwrap();
        assert_eq!(outcome.status, "ok");
        assert_eq!(outcome.summary.items_written, 1);

        let feed_dir = runtime
            .layout
            .feed_dir("stub/echo", "test-stub")
            .unwrap();
        let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
        assert_eq!(meta.run_count, 1);
        assert_eq!(meta.last_status.as_deref(), Some("ok"));
        assert_eq!(meta.cursor["run_count"], 1);
        assert!(feed_dir.join("log.jsonl").exists());
    }

    #[tokio::test]
    async fn run_feed_increments_cursor_across_invocations() {
        let tmp = tempdir().unwrap();
        let conn = open_test_db();
        {
            let store = FeedStore::new(&conn);
            store
                .insert(&new_record(
                    "iter",
                    "stub/echo",
                    TemplateParams::default(),
                    "*/15 * * * *",
                ))
                .unwrap();
        }
        let runtime = build_runtime(tmp.path(), conn);

        for expected in 1..=3 {
            let outcome = run_feed("iter", &runtime).await.unwrap();
            assert_eq!(outcome.cursor["run_count"], expected);
        }

        let feed_dir = runtime.layout.feed_dir("stub/echo", "iter").unwrap();
        let lines: Vec<_> = std::fs::read_to_string(feed_dir.join("log.jsonl"))
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["run"], 1);
        assert_eq!(lines[2]["run"], 3);
    }

    #[tokio::test]
    async fn run_feed_skips_disabled_feed() {
        let tmp = tempdir().unwrap();
        let conn = open_test_db();
        {
            let store = FeedStore::new(&conn);
            let mut rec = new_record(
                "off",
                "stub/echo",
                TemplateParams::default(),
                "*/15 * * * *",
            );
            rec.enabled = false;
            store.insert(&rec).unwrap();
        }
        let runtime = build_runtime(tmp.path(), conn);
        let outcome = run_feed("off", &runtime).await.unwrap();
        assert_eq!(outcome.status, "skipped-disabled");
        // Disabled feeds don't write meta.json
        let feed_dir = runtime.layout.feed_dir("stub/echo", "off").unwrap();
        assert!(MetaStore::read(&feed_dir).unwrap().is_none());
    }

    #[tokio::test]
    async fn run_feed_returns_storage_error_for_missing_id() {
        let tmp = tempdir().unwrap();
        let conn = open_test_db();
        let runtime = build_runtime(tmp.path(), conn);
        let err = run_feed("missing", &runtime).await.unwrap_err();
        match err {
            FeedError::Storage(msg) => assert!(msg.contains("missing")),
            other => panic!("expected Storage, got {other:?}"),
        }
    }
}

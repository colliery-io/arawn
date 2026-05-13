//! Boot-time wiring: read every enabled feed from the DB and register
//! each as a cloacina workflow + cron schedule.
//!
//! Per I-0039: one workflow per feed (`workflow_name = feed::<feed_id>`)
//! containing a single `FeedDispatchTask`. cloacina's
//! `register_cron_workflow` schedules the workflow to fire on the
//! feed's cadence. Catchup, retry, single-instance enforcement, and
//! audit are all inherited from cloacina.

use std::sync::Arc;

use cloacina::workflow::WorkflowBuilder;
use cloacina::{DefaultRunner, Runtime, TaskNamespace};
use rusqlite::Connection;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::cadence::validate_cadence;
use crate::clients::FeedClients;
use crate::dispatch::{FeedDispatchTask, FeedRuntimeContext};
use crate::error::FeedError;
use crate::layout::DataLayout;
use crate::meta::MetaStore;
use crate::registry::FeedTemplateRegistry;
use crate::store::{FeedRecord, FeedStore, new_record};
use crate::template::{DiscoveryRow, TemplateCtx};
use crate::types::{FeedMeta, FeedSummary, TemplateParams};

/// arawn-feeds doesn't depend on arawn-workflow directly to avoid a
/// dependency cycle; instead we take an `Arc<DefaultRunner>` (the
/// concrete cloacina runner). The arawn-workflow `WorkflowRunner`
/// exposes its inner `DefaultRunner` via `inner()` and the server boot
/// passes that handle in.
pub type CloacinaRunner = DefaultRunner;

/// Format the cloacina workflow name for a feed. One feed = one
/// workflow_name = one cron schedule.
///
/// The separator is a single `_` rather than `::` because cloacina's
/// `TaskNamespace` Display format is `tenant::package::workflow::task`
/// — putting `::` inside the workflow name confuses the namespace
/// parser (it sees more than 4 parts).
pub fn feed_workflow_name(feed_id: &str) -> String {
    format!("feed_{feed_id}")
}

/// One-stop entry the server boot calls after the workflow runner is
/// up. Reads every enabled feed, registers a workflow + cron schedule
/// for each. Returns the live `FeedRuntime` handle so dynamic adds
/// (`/watch`) can route through the same code path.
pub async fn start(
    runner: Arc<CloacinaRunner>,
    conn: Arc<Mutex<Connection>>,
    layout: Arc<DataLayout>,
    registry: Arc<FeedTemplateRegistry>,
    clients: Arc<dyn FeedClients>,
    projections: Option<Arc<arawn_projections::ProjectionStore>>,
    extractor: Option<Arc<arawn_extractor::ExtractorRunner>>,
) -> Result<FeedRuntime, FeedError> {
    let runtime_ctx = FeedRuntimeContext {
        conn: conn.clone(),
        layout: layout.clone(),
        registry: registry.clone(),
        clients: clients.clone(),
        projections: projections.clone(),
        extractor: extractor.clone(),
    };

    let feeds = {
        let c = conn.lock().await;
        FeedStore::new(&c).list_enabled()?
    };

    let mut registered = 0usize;
    let mut skipped = 0usize;
    for record in &feeds {
        match register_one(&runner, &runtime_ctx, record).await {
            Ok(()) => registered += 1,
            Err(e) => {
                warn!(
                    feed_id = %record.id,
                    template = %record.template,
                    error = %e,
                    "failed to register feed; continuing"
                );
                skipped += 1;
            }
        }
    }

    // Resume in-progress backfills (rows with enabled=0 +
    // last_status="backfilling"). These are the leftovers of a
    // backfill loop interrupted by a crash or restart.
    let all_feeds = {
        let c = conn.lock().await;
        FeedStore::new(&c).list_all()?
    };
    let resumed = resume_pending_backfills(runner.clone(), runtime_ctx.clone(), &all_feeds);
    if resumed > 0 {
        info!(resumed, "resumed in-progress backfill loops");
    }

    info!(registered, skipped, "feed runtime started");

    Ok(FeedRuntime {
        runner,
        runtime_ctx,
    })
}

/// Live handle for dynamic feed registration (Phase 6: `/watch`).
pub struct FeedRuntime {
    runner: Arc<CloacinaRunner>,
    runtime_ctx: FeedRuntimeContext,
}

impl FeedRuntime {
    /// Register an additional feed without a server restart. Inserts
    /// the row and registers the workflow + cron schedule.
    pub async fn register_feed_runtime(
        &self,
        record: &FeedRecord,
    ) -> Result<(), FeedError> {
        register_one(&self.runner, &self.runtime_ctx, record).await
    }

    pub fn runtime_ctx(&self) -> &FeedRuntimeContext {
        &self.runtime_ctx
    }

    /// Full dynamic-registration flow used by the `/watch` command.
    ///
    /// 1. Validate template exists + params shape.
    /// 2. Build the FeedRecord (using template-supplied cadence
    ///    default if `cadence_override` is None).
    /// 3. Persist row.
    /// 4. Write initial `meta.json` with the template's initial cursor.
    /// 5. Register cloacina workflow + cron via `register_one`.
    ///
    /// Failures roll back: if cron registration fails, the DB row is
    /// removed so the next boot doesn't try to register a half-baked
    /// feed.
    pub async fn register_feed_dynamic(
        &self,
        template: &str,
        feed_id: &str,
        params: TemplateParams,
        cadence_override: Option<String>,
    ) -> Result<FeedRecord, FeedError> {
        // Step 1 — template + params validation up front so a bad
        // call never touches disk or DB.
        let tmpl = self.runtime_ctx.registry.require(template)?;
        tmpl.validate(&params)?;
        let defaults = tmpl.defaults(&params);
        let cadence = cadence_override.unwrap_or(defaults.cadence);
        validate_cadence(&cadence)?;

        // Detect the `since=` first-run-seed mode. When present, we
        // insert the row as `enabled=0` and run a backfill loop in a
        // spawned task; cron registration only happens after the
        // loop completes successfully. See ARAWN-T-0227.
        let has_since = params
            .as_value()
            .get("since")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty());

        // Step 2 — record. `enabled` starts false in backfill mode so
        // a fresh cron registration can't fire while the loop is
        // walking pages.
        let mut record = new_record(feed_id, template, params.clone(), cadence);
        if has_since {
            record.enabled = false;
        }

        // Step 3 — persist row.
        {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).insert(&record)?;
        }

        // Step 4 — initial meta.json. In backfill mode we tag
        // `last_status="backfilling"` so a server restart mid-loop
        // can resume from the persisted cursor.
        let feed_dir = self
            .runtime_ctx
            .layout
            .ensure_feed_dir(template, feed_id)?;
        let mut meta = FeedMeta::new(template, params, defaults.initial_cursor);
        if has_since {
            meta.last_status = Some("backfilling".into());
        }
        if let Err(e) = MetaStore::write(&feed_dir, &meta) {
            // Rollback the DB row — meta + row stay consistent.
            let c = self.runtime_ctx.conn.lock().await;
            let _ = FeedStore::new(&c).delete(feed_id);
            return Err(e);
        }

        if has_since {
            // Backfill mode: spawn the loop, leave cron unregistered
            // until the loop completes. The slash-command response
            // returns immediately; the user sees a follow-up notice
            // when the backfill finishes.
            spawn_backfill_task(
                Arc::clone(&self.runner),
                self.runtime_ctx.clone(),
                feed_id.to_string(),
            );
            return Ok(record);
        }

        // Step 5 — cron registration (steady-state path).
        if let Err(e) = register_one(&self.runner, &self.runtime_ctx, &record).await {
            // Rollback the DB row so a re-run of /watch isn't blocked
            // by a UNIQUE constraint on a half-registered feed.
            let c = self.runtime_ctx.conn.lock().await;
            let _ = FeedStore::new(&c).delete(feed_id);
            return Err(e);
        }

        Ok(record)
    }

    /// Trigger a one-off run of an enabled feed, outside the cron
    /// schedule. Backs `/feeds run <id>` — useful for "test now,
    /// don't wait 30 min" debugging plus ops scenarios where the
    /// schedule slipped.
    ///
    /// Goes through the same `dispatch::run_feed` code the cloacina
    /// cron path uses, so cursor advancement and meta.json writes
    /// behave identically. Disabled feeds short-circuit with
    /// `status="skipped-disabled"` (existing dispatch behavior).
    pub async fn run_feed_once(
        &self,
        feed_id: &str,
    ) -> Result<crate::template::RunOutcome, FeedError> {
        crate::dispatch::run_feed(feed_id, &self.runtime_ctx).await
    }

    /// Pause a feed: drop its cloacina cron schedule and flip the row
    /// to `enabled=0`. The data dir is left intact so a future
    /// `resume_feed` can pick up where the cursor left off.
    ///
    /// Idempotent: calling pause on an already-paused feed is a no-op
    /// from the user's perspective (returns Ok).
    pub async fn pause_feed(&self, feed_id: &str) -> Result<FeedRecord, FeedError> {
        let record = {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c)
                .get(feed_id)?
                .ok_or_else(|| FeedError::InvalidParams(format!("no feed '{feed_id}'")))?
        };
        // Delete the cron schedule first — it's the load-bearing step.
        // If it fails we leave the DB row alone so we're not surprised
        // by a "paused" feed that's still firing.
        delete_schedule_for(&self.runner, &feed_workflow_name(feed_id)).await?;
        {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).set_enabled(feed_id, false)?;
        }
        info!(%feed_id, "feed paused");
        Ok(FeedRecord { enabled: false, ..record })
    }

    /// Resume a previously-paused feed: re-register the cloacina
    /// cron schedule using the row's persisted cadence, then flip
    /// `enabled=1`.
    pub async fn resume_feed(&self, feed_id: &str) -> Result<FeedRecord, FeedError> {
        let mut record = {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c)
                .get(feed_id)?
                .ok_or_else(|| FeedError::InvalidParams(format!("no feed '{feed_id}'")))?
        };
        record.enabled = true;
        // Re-register first; only flip the DB if cron registration
        // succeeds (otherwise the row would say "active" but nothing
        // would fire).
        register_one(&self.runner, &self.runtime_ctx, &record).await?;
        {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).set_enabled(feed_id, true)?;
        }
        info!(%feed_id, "feed resumed");
        Ok(record)
    }

    /// Decommission: drop the cloacina cron schedule, delete the DB
    /// row, and recursively delete the feed's data dir.
    ///
    /// Order is deliberately cron→fs→row: if cron deletion fails we
    /// haven't lost any data, and if fs deletion fails the row stays
    /// so the user can retry. Returns the now-deleted record + the
    /// number of bytes wiped from disk.
    pub async fn remove_feed(
        &self,
        feed_id: &str,
    ) -> Result<RemoveOutcome, FeedError> {
        let record = {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c)
                .get(feed_id)?
                .ok_or_else(|| FeedError::InvalidParams(format!("no feed '{feed_id}'")))?
        };
        delete_schedule_for(&self.runner, &feed_workflow_name(feed_id)).await?;

        let feed_dir = self
            .runtime_ctx
            .layout
            .feed_dir(&record.template, feed_id)?;
        let bytes_wiped = dir_size_bytes(&feed_dir);
        if feed_dir.exists() {
            std::fs::remove_dir_all(&feed_dir).map_err(|e| {
                FeedError::Storage(format!("rm -rf {}: {e}", feed_dir.display()))
            })?;
        }

        {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).delete(feed_id)?;
        }
        info!(%feed_id, bytes_wiped, "feed decommissioned");
        Ok(RemoveOutcome { record, bytes_wiped })
    }

    /// Run the template's discovery hook. Returns:
    /// - `Some(rows)` when the template implements `discover` and the
    ///   provider call succeeded. `rows` may still be empty (no
    ///   channels accessible, no projects, etc.).
    /// - `None` when the template doesn't support discovery (free-form
    ///   params) — callers should print a usage message.
    pub async fn discover_template(
        &self,
        template_name: &str,
    ) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        let template = self.runtime_ctx.registry.require(template_name)?;
        let ctx = TemplateCtx::new(self.runtime_ctx.clients.clone());
        template.discover(&ctx).await
    }

    /// List every feed in the DB (enabled or paused) with on-disk
    /// status info.
    pub async fn list_summaries(&self) -> Result<Vec<FeedSummary>, FeedError> {
        let records = {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).list_all()?
        };
        let mut out = Vec::with_capacity(records.len());
        for r in records {
            // Bad template names shouldn't happen for rows the runtime
            // created, but skip defensively rather than poison the
            // whole list call.
            let feed_dir = match self.runtime_ctx.layout.feed_dir(&r.template, &r.id) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let meta = MetaStore::read(&feed_dir).ok().flatten();
            let data_size_bytes = dir_size_bytes(&feed_dir);
            out.push(FeedSummary {
                id: r.id,
                template: r.template,
                cadence: r.cadence,
                enabled: r.enabled,
                created_at: r.created_at.to_rfc3339(),
                updated_at: r.updated_at.to_rfc3339(),
                last_run_at: meta.as_ref().and_then(|m| m.last_run_at.clone()),
                last_status: meta.as_ref().and_then(|m| m.last_status.clone()),
                run_count: meta.as_ref().map(|m| m.run_count).unwrap_or(0),
                data_size_bytes,
                data_dir: feed_dir.to_string_lossy().to_string(),
            });
        }
        Ok(out)
    }
}

/// Hard cap on backfill loop iterations. ~10k pages × 200 messages =
/// 2M Slack messages — well past any realistic single-channel scale.
/// Reaching this exits cleanly with `last_status="backfill-failed:
/// page-cap exceeded"`.
const BACKFILL_PAGE_CAP: u32 = 10_000;

/// Base delay used when a provider rate-limits us without a Retry-After
/// header, and the starting unit for transient-error exponential backoff.
const BASE_BACKOFF: std::time::Duration = std::time::Duration::from_secs(2);

/// Wall-clock cap on cumulative rate-limit waits inside a single
/// backfill run. Past this, we defer the rest to the next cron tick
/// (cursor is already persisted per page).
const MAX_RATE_LIMIT_WAIT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// How many consecutive transient errors (Provider/Storage) we'll
/// retry before bailing. Each retry waits BASE_BACKOFF * 2^(attempt-1).
const TRANSIENT_MAX_ATTEMPTS: u32 = 3;

/// Pure helper: backoff for the Nth consecutive transient retry
/// (1-indexed). Capped to avoid silly waits if anyone bumps the
/// attempt counter. No jitter here — jitter is added at the call site
/// so this stays unit-testable.
fn transient_backoff(attempt: u32) -> std::time::Duration {
    let shift = attempt.saturating_sub(1).min(6);
    BASE_BACKOFF * 2u32.pow(shift)
}

/// How a backfill ended. Both variants are "good enough" outcomes that
/// flip the row to enabled=1; the difference is the meta status we
/// record. Hard failures return `Err` instead and stay enabled=0.
enum BackfillExit {
    /// Drained — no more items.
    Complete(BackfillStats),
    /// Hit the rate-limit wall-clock cap mid-run. Cursor is persisted;
    /// the next cron tick resumes from there.
    RateLimitDeferred(BackfillStats),
}

/// Spawn the backfill loop as a detached tokio task. Repeatedly calls
/// `dispatch::run_feed` until either the run reports zero items
/// (caught up), the cursor stops advancing (template/provider bug
/// guard), or the page cap is hit. On clean completion, flips the
/// row to `enabled=1` and registers the cron schedule.
///
/// Resilience covered in this v1: cursor-stalled detection, page cap,
/// and the underlying `run_feed` already persists the cursor
/// atomically per page so a crash mid-loop is safe (boot resumption
/// picks up via `resume_pending_backfills`). Network-blip retry,
/// rate-limit Retry-After parsing, and per-page Schema-skip remain
/// follow-ups (see ARAWN-T-0227).
fn spawn_backfill_task(
    runner: Arc<CloacinaRunner>,
    runtime_ctx: FeedRuntimeContext,
    feed_id: String,
) {
    tokio::spawn(async move {
        let outcome = run_backfill_loop(&runner, &runtime_ctx, &feed_id).await;
        match outcome {
            Ok(BackfillExit::Complete(stats)) => {
                info!(
                    feed_id = %feed_id,
                    pages = stats.pages,
                    items = stats.items,
                    "backfill complete"
                );
                if let Err(e) = finalize_backfill_success(&runner, &runtime_ctx, &feed_id, None).await
                {
                    warn!(
                        feed_id = %feed_id,
                        error = %e,
                        "backfill: failed to flip enabled=1 / register cron"
                    );
                }
            }
            Ok(BackfillExit::RateLimitDeferred(stats)) => {
                info!(
                    feed_id = %feed_id,
                    pages = stats.pages,
                    items = stats.items,
                    "backfill deferred (rate-limited) — cron will resume from cursor"
                );
                if let Err(e) = finalize_backfill_success(
                    &runner,
                    &runtime_ctx,
                    &feed_id,
                    Some("backfill-rate-limited"),
                )
                .await
                {
                    warn!(
                        feed_id = %feed_id,
                        error = %e,
                        "backfill: failed to flip enabled=1 / register cron after rate-limit defer"
                    );
                }
            }
            Err(e) => {
                warn!(feed_id = %feed_id, error = %e, "backfill failed");
                let _ = mark_backfill_failed(&runtime_ctx, &feed_id, &e.to_string()).await;
            }
        }
    });
}

#[derive(Default)]
struct BackfillStats {
    pages: u32,
    items: u64,
}

async fn run_backfill_loop(
    _runner: &Arc<CloacinaRunner>,
    runtime_ctx: &FeedRuntimeContext,
    feed_id: &str,
) -> Result<BackfillExit, FeedError> {
    let mut stats = BackfillStats::default();
    let mut prev_cursor: Option<serde_json::Value> = None;
    let mut rate_limit_wait_total = std::time::Duration::ZERO;
    let mut transient_attempts: u32 = 0;

    while stats.pages < BACKFILL_PAGE_CAP {
        let result = crate::dispatch::run_feed_force(feed_id, runtime_ctx).await;
        let outcome = match result {
            Ok(o) => {
                transient_attempts = 0;
                o
            }
            Err(FeedError::RateLimited { retry_after }) => {
                let wait = retry_after.unwrap_or(BASE_BACKOFF);
                if rate_limit_wait_total + wait > MAX_RATE_LIMIT_WAIT {
                    info!(
                        feed_id = %feed_id,
                        waited_secs = rate_limit_wait_total.as_secs(),
                        "backfill rate-limit cap reached — deferring to next cron tick"
                    );
                    return Ok(BackfillExit::RateLimitDeferred(stats));
                }
                rate_limit_wait_total += wait;
                info!(
                    feed_id = %feed_id,
                    wait_secs = wait.as_secs(),
                    total_waited_secs = rate_limit_wait_total.as_secs(),
                    "backfill rate-limited — sleeping then retrying"
                );
                tokio::time::sleep(wait).await;
                continue;
            }
            Err(e @ (FeedError::Provider(_) | FeedError::Storage(_))) => {
                transient_attempts += 1;
                if transient_attempts > TRANSIENT_MAX_ATTEMPTS {
                    return Err(e);
                }
                let wait = transient_backoff(transient_attempts);
                warn!(
                    feed_id = %feed_id,
                    attempt = transient_attempts,
                    wait_secs = wait.as_secs(),
                    error = %e,
                    "backfill transient error — retrying with backoff"
                );
                tokio::time::sleep(wait).await;
                continue;
            }
            Err(e) => return Err(e),
        };

        stats.pages += 1;
        stats.items += outcome.summary.items_written;

        // Caught up: empty page → done.
        if outcome.summary.items_written == 0 {
            return Ok(BackfillExit::Complete(stats));
        }

        // Cursor-stalled guard: items came back but cursor didn't
        // advance → template / provider bug. Bail rather than spin.
        if let Some(prior) = &prev_cursor
            && prior == &outcome.cursor
        {
            return Err(FeedError::Provider(
                "backfill cursor stalled — template returned items but cursor unchanged".into(),
            ));
        }
        prev_cursor = Some(outcome.cursor);
    }

    Err(FeedError::Provider(format!(
        "backfill page cap of {BACKFILL_PAGE_CAP} exceeded"
    )))
}

async fn finalize_backfill_success(
    runner: &Arc<CloacinaRunner>,
    runtime_ctx: &FeedRuntimeContext,
    feed_id: &str,
    meta_status: Option<&str>,
) -> Result<(), FeedError> {
    // Re-load the record to register cron with current state.
    let record = {
        let c = runtime_ctx.conn.lock().await;
        FeedStore::new(&c)
            .get(feed_id)?
            .ok_or_else(|| FeedError::Storage(format!("feed '{feed_id}' missing post-backfill")))?
    };

    // Register cron first — only flip enabled=1 if cron actually
    // takes, so we never leave a row that says "active" but has no
    // schedule.
    let mut to_register = record.clone();
    to_register.enabled = true;
    register_one(runner, runtime_ctx, &to_register).await?;

    {
        let c = runtime_ctx.conn.lock().await;
        FeedStore::new(&c).set_enabled(feed_id, true)?;
    }

    // Surface a non-default meta status when the backfill ended in a
    // soft-defer (e.g. rate-limit cap). Cron will overwrite this on the
    // next successful run.
    if let Some(status) = meta_status {
        let feed_dir = runtime_ctx
            .layout
            .feed_dir(&record.template, &record.id)?;
        if let Some(mut meta) = MetaStore::read(&feed_dir)? {
            meta.last_status = Some(status.to_string());
            MetaStore::write(&feed_dir, &meta)?;
        }
    }
    Ok(())
}

async fn mark_backfill_failed(
    runtime_ctx: &FeedRuntimeContext,
    feed_id: &str,
    err: &str,
) -> Result<(), FeedError> {
    let record = {
        let c = runtime_ctx.conn.lock().await;
        FeedStore::new(&c).get(feed_id)?
    };
    let Some(record) = record else {
        // Feed got removed mid-backfill; nothing to update.
        return Ok(());
    };
    let feed_dir = runtime_ctx
        .layout
        .feed_dir(&record.template, &record.id)?;
    if let Some(mut meta) = MetaStore::read(&feed_dir)? {
        meta.last_status = Some(format!("backfill-failed: {err}"));
        MetaStore::write(&feed_dir, &meta)?;
    }
    Ok(())
}

/// On boot, find feeds whose `meta.json.last_status == "backfilling"`
/// and re-spawn the loop. Their DB row is `enabled=0` so cron won't
/// fire them; only the spawn re-runs the loop.
pub fn resume_pending_backfills(
    runner: Arc<CloacinaRunner>,
    runtime_ctx: FeedRuntimeContext,
    records: &[FeedRecord],
) -> usize {
    let mut resumed = 0;
    for record in records {
        if record.enabled {
            continue;
        }
        let feed_dir = match runtime_ctx.layout.feed_dir(&record.template, &record.id) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let last_status = MetaStore::read(&feed_dir)
            .ok()
            .flatten()
            .and_then(|m| m.last_status);
        if last_status.as_deref() == Some("backfilling") {
            info!(feed_id = %record.id, "resuming backfill from persisted cursor");
            spawn_backfill_task(
                Arc::clone(&runner),
                runtime_ctx.clone(),
                record.id.clone(),
            );
            resumed += 1;
        }
    }
    resumed
}

/// Outcome of a successful `remove_feed` — the row that was deleted
/// plus how many bytes the data dir contained. Useful for the
/// confirm-modal "this deletes <N> bytes" message and audit logs.
#[derive(Debug, Clone)]
pub struct RemoveOutcome {
    pub record: FeedRecord,
    pub bytes_wiped: u64,
}

/// Look up cloacina's cron schedule by workflow name and delete it
/// if present. Idempotent: returns Ok even if no schedule matches.
async fn delete_schedule_for(
    runner: &CloacinaRunner,
    workflow_name: &str,
) -> Result<(), FeedError> {
    // 1000 is well above any realistic feed count and avoids
    // pagination — cloacina itself caps server-side at 100k.
    let schedules = runner
        .list_cron_schedules(false, 1000, 0)
        .await
        .map_err(|e| FeedError::Provider(format!("list_cron_schedules: {e}")))?;
    if let Some(s) = schedules
        .into_iter()
        .find(|s| s.workflow_name == workflow_name)
    {
        runner
            .delete_cron_schedule(s.id)
            .await
            .map_err(|e| FeedError::Provider(format!("delete_cron_schedule: {e}")))?;
    }
    Ok(())
}

fn dir_size_bytes(path: &std::path::Path) -> u64 {
    fn walk(p: &std::path::Path, acc: &mut u64) {
        let entries = match std::fs::read_dir(p) {
            Ok(it) => it,
            Err(_) => return,
        };
        for e in entries.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let path = e.path();
            if ft.is_dir() {
                walk(&path, acc);
            } else if ft.is_file()
                && let Ok(md) = path.metadata() {
                    *acc += md.len();
                }
        }
    }
    let mut total = 0u64;
    walk(path, &mut total);
    total
}

async fn register_one(
    runner: &CloacinaRunner,
    ctx: &FeedRuntimeContext,
    record: &FeedRecord,
) -> Result<(), FeedError> {
    // 1. Validate the cadence honors the 15-minute floor.
    validate_cadence(&record.cadence)?;

    // 2. Resolve template — fail fast with a clear error if the binary
    // doesn't ship that template.
    let template = ctx.registry.require(&record.template)?;
    template.validate(&record.params)?;

    // 3. Build a workflow constructor that captures the feed_id +
    // shared runtime ctx; cloacina calls this when it needs to
    // instantiate the workflow (could be many times; cheap clone).
    let workflow_name = feed_workflow_name(&record.id);
    let feed_id = record.id.clone();
    let runtime_ctx = ctx.clone();
    let template_name = record.template.clone();

    let _ = template_name; // captured below if we ever annotate the Workflow

    let constructor_name = workflow_name.clone();
    let ctor_runtime_ctx = runtime_ctx.clone();
    let ctor_feed_id = feed_id.clone();
    let constructor = move || -> cloacina::Workflow {
        let task: Arc<dyn cloacina::Task> = Arc::new(FeedDispatchTask::new(
            ctor_feed_id.clone(),
            ctor_runtime_ctx.clone(),
        ));
        WorkflowBuilder::new(&constructor_name)
            .add_task(task)
            .expect("single-task workflow construction cannot fail")
            .build()
            .expect("single-task workflow validation cannot fail")
    };

    let runtime: Arc<Runtime> = runner.runtime();

    // Register the task at the namespace cloacina's executor uses
    // when looking up runnable tasks at fire time. `Workflow::add_task`
    // constructs the namespace as ("public", "embedded", workflow_name,
    // task.id()); we mirror that here.
    let task_namespace = TaskNamespace::new(
        "public",
        "embedded",
        &workflow_name,
        &feed_id,
    );
    let task_runtime_ctx = runtime_ctx.clone();
    let task_feed_id = feed_id.clone();
    runtime.register_task(task_namespace, move || -> Arc<dyn cloacina::Task> {
        Arc::new(FeedDispatchTask::new(
            task_feed_id.clone(),
            task_runtime_ctx.clone(),
        ))
    });

    runtime.register_workflow(workflow_name.clone(), constructor);

    // 4. Schedule it. UTC for now — feed cadences are absolute, the
    // user expresses cron in UTC. (A future task may wire workstream
    // / user timezone in.)
    //
    // Idempotency: cloacina's `register_cron_workflow` always inserts
    // a new row — there's no upsert. So every call to register_one
    // (boot-time scan + every /watch + every resume) would accumulate
    // a duplicate cron schedule for the same workflow_name, and each
    // cron tick fires ALL of them. Surfaced during T-0218 UAT —
    // 7 schedules per feed after a few server restarts. Clean it up
    // here by deleting any pre-existing schedule for this
    // workflow_name before we register the new one.
    delete_schedule_for(runner, &workflow_name).await?;
    runner
        .register_cron_workflow(&workflow_name, &record.cadence, "UTC")
        .await
        .map_err(|e| FeedError::Provider(format!("register_cron_workflow: {e}")))?;

    info!(
        feed_id = %record.id,
        template = %record.template,
        cadence = %record.cadence,
        "feed registered"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn transient_backoff_doubles_per_attempt() {
        assert_eq!(transient_backoff(1), Duration::from_secs(2));
        assert_eq!(transient_backoff(2), Duration::from_secs(4));
        assert_eq!(transient_backoff(3), Duration::from_secs(8));
    }

    #[test]
    fn transient_backoff_clamps() {
        // Past the clamp the value stops growing exponentially; we just
        // care that it doesn't overflow / return an absurd sleep.
        let big = transient_backoff(20);
        assert!(big <= Duration::from_secs(2 * 64));
    }
}


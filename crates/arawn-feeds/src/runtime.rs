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
) -> Result<FeedRuntime, FeedError> {
    let runtime_ctx = FeedRuntimeContext {
        conn: conn.clone(),
        layout: layout.clone(),
        registry: registry.clone(),
        clients: clients.clone(),
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

        // Step 2 — record.
        let record = new_record(feed_id, template, params.clone(), cadence);

        // Step 3 — persist row.
        {
            let c = self.runtime_ctx.conn.lock().await;
            FeedStore::new(&c).insert(&record)?;
        }

        // Step 4 — initial meta.json. Best-effort — if this fails the
        // row exists but no meta yet; the dispatcher will recreate it
        // on first run. We still surface the error since the user
        // would rather see "couldn't write meta" now than silently
        // drift.
        let feed_dir = self
            .runtime_ctx
            .layout
            .ensure_feed_dir(template, feed_id)?;
        let meta = FeedMeta::new(template, params, defaults.initial_cursor);
        if let Err(e) = MetaStore::write(&feed_dir, &meta) {
            // Rollback the DB row — meta + row stay consistent.
            let c = self.runtime_ctx.conn.lock().await;
            let _ = FeedStore::new(&c).delete(feed_id);
            return Err(e);
        }

        // Step 5 — cron registration.
        if let Err(e) = register_one(&self.runner, &self.runtime_ctx, &record).await {
            // Rollback the DB row so a re-run of /watch isn't blocked
            // by a UNIQUE constraint on a half-registered feed.
            let c = self.runtime_ctx.conn.lock().await;
            let _ = FeedStore::new(&c).delete(feed_id);
            return Err(e);
        }

        Ok(record)
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

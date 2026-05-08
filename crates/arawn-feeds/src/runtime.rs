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
use cloacina::{DefaultRunner, Runtime};
use rusqlite::Connection;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::cadence::validate_cadence;
use crate::clients::FeedClients;
use crate::dispatch::{FeedDispatchTask, FeedRuntimeContext};
use crate::error::FeedError;
use crate::layout::DataLayout;
use crate::registry::FeedTemplateRegistry;
use crate::store::{FeedRecord, FeedStore};

/// arawn-feeds doesn't depend on arawn-workflow directly to avoid a
/// dependency cycle; instead we take an `Arc<DefaultRunner>` (the
/// concrete cloacina runner). The arawn-workflow `WorkflowRunner`
/// exposes its inner `DefaultRunner` via `inner()` and the server boot
/// passes that handle in.
pub type CloacinaRunner = DefaultRunner;

/// Format the cloacina workflow name for a feed. One feed = one
/// workflow_name = one cron schedule.
pub fn feed_workflow_name(feed_id: &str) -> String {
    format!("feed::{feed_id}")
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

//! Continual data feeds (I-0039). Extracted from `main.rs` as part
//! of I-0054 T-E. Registers per-feed cloacina cron schedules that
//! route through arawn-feeds' template dispatcher.

use std::sync::Arc;
use std::sync::RwLock;

use tracing::{debug, info, warn};

use crate::LocalService;
use super::integrations::IntegrationsForFeeds;

/// Wire the continual data-feed runtime. Returns silently (with log)
/// if the workflow runner is unavailable.
pub async fn wire_continual_feeds(
    workflow_runner_handle: Option<&Arc<arawn_workflow::WorkflowRunner>>,
    data_dir: &str,
    integrations: &IntegrationsForFeeds,
    projections: Option<&Arc<arawn_projections::ProjectionStore>>,
    extractor_runner: Option<&Arc<arawn_extractor::ExtractorRunner>>,
    service: &mut LocalService,
    feed_runtime_for_hooks: &Arc<RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>>,
) {
    // Continual data feeds (I-0039). Registers per-feed cloacina
    // cron schedules that route through arawn-feeds' template
    // dispatcher. Skipped if the workflow runner failed to start —
    // feeds need cloacina to schedule them.
    if let Some(workflow_runner) = workflow_runner_handle {
        let feeds_db_path = std::path::PathBuf::from(&data_dir).join("arawn.db");
        match rusqlite::Connection::open(&feeds_db_path) {
            Ok(conn) => {
                // arawn-feeds expects the schema to already be in
                // place (V2 feeds migration is owned by
                // arawn-storage and was applied when `Store::open`
                // ran above).
                let feeds_conn = Arc::new(tokio::sync::Mutex::new(conn));
                let feeds_layout = Arc::new(arawn_feeds::DataLayout::new(&data_dir));
                let feeds_registry = Arc::new(arawn_feeds::default_registry());

                let mut clients = arawn_feeds::RealClients::new();
                if let Some(slack) = integrations.slack.as_ref() {
                    clients = clients.with_slack(Arc::clone(slack));
                }
                if let Some(cal) = integrations.calendar.as_ref() {
                    clients = clients.with_calendar(Arc::clone(cal));
                }
                if let Some(gm) = integrations.gmail.as_ref() {
                    clients = clients.with_gmail(Arc::clone(gm));
                }
                if let Some(dr) = integrations.drive.as_ref() {
                    clients = clients.with_drive(Arc::clone(dr));
                }
                if let Some(at) = integrations.atlassian.as_ref() {
                    clients = clients.with_atlassian(Arc::clone(at));
                }
                if let Some(gh) = integrations.github.as_ref() {
                    clients = clients.with_github(Arc::clone(gh));
                }
                let clients: Arc<dyn arawn_feeds::FeedClients> = Arc::new(clients);

                // Reuse the outer projection store + extractor runner
                // built earlier so the bind hook, feed_search, embed
                // pass, and feed dispatch all share one instance.
                let feeds_projections = projections.cloned();
                let feeds_extractor = extractor_runner.cloned();

                match arawn_feeds::start(
                    workflow_runner.cloacina_runner(),
                    feeds_conn,
                    feeds_layout,
                    feeds_registry,
                    clients,
                    feeds_projections,
                    feeds_extractor,
                )
                .await
                {
                    Ok(runtime) => {
                        // Hand the live runtime to the service so
                        // `/watch` and `/feeds` route through it.
                        let runtime = Arc::new(runtime);
                        service.set_feed_runtime(Arc::clone(&runtime));
                        // T-0329 — also expose the runtime to the
                        // bind/unbind hooks so they can hot-add or
                        // hot-remove cron schedules.
                        *feed_runtime_for_hooks.write().unwrap() =
                            Some(Arc::clone(&runtime));
                        info!("feed runtime started");
                    }
                    Err(e) => warn!(error = %e, "feed runtime failed to start"),
                }
            }
            Err(e) => warn!(error = %e, db = %feeds_db_path.display(),
                "feed runtime unavailable — could not open arawn.db"),
        }
    } else {
        debug!("feed runtime skipped — workflow runner not available");
    }

}

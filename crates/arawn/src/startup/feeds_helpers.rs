//! Per-feed helpers: org-expand and single-feed cron registration.

use std::sync::Arc;

use tracing::{debug, info, warn};

/// I-0050 T-0327 — list every repo under `owner` (via the github
/// integration's authenticated client) and register one
/// `github/repo-mirror` feed per repo against `workstream`.
/// Idempotent: existing feeds are skipped.
pub async fn expand_github_org(
    github: Arc<arawn_integrations::github::GithubIntegration>,
    store: Arc<std::sync::Mutex<arawn_storage::Store>>,
    feed_runtime: Option<Arc<arawn_feeds::FeedRuntime>>,
    workstream: String,
    owner: String,
) {
    use arawn_feeds::GithubFeedClient;
    let real_client = arawn_feeds::clients::RealGithubClient::new(github);
    let repos = match real_client.list_org_repos(&owner, 10).await {
        Ok(v) => v,
        Err(e) => {
            warn!(owner = %owner, error = %e, "org-expand: list_org_repos failed");
            return;
        }
    };
    info!(owner = %owner, count = repos.len(), workstream = %workstream,
          "org-expand: registering per-repo feeds");
    let now = chrono::Utc::now().to_rfc3339();
    let mut new_feed_ids: Vec<String> = Vec::new();
    {
        let store_guard = match store.lock() {
            Ok(g) => g,
            Err(_) => {
                warn!("org-expand: store mutex poisoned");
                return;
            }
        };
        let conn = store_guard.database().conn();
        for repo in repos {
            let name = repo
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let feed_id = format!("github-repo:{owner}/{name}");
            let params_json =
                serde_json::json!({"owner": &owner, "name": &name}).to_string();
            // Idempotent insert — skip if already exists.
            match conn.execute(
                "INSERT OR IGNORE INTO feeds \
                 (id, template, params, cadence, enabled, created_at, updated_at) \
                 VALUES (?1, 'github/repo-mirror', ?2, '*/30 * * * *', 1, ?3, ?3)",
                rusqlite::params![&feed_id, &params_json, &now],
            ) {
                Ok(rows) if rows > 0 => new_feed_ids.push(feed_id),
                Ok(_) => {} // already existed → no cron register
                Err(e) => warn!(feed_id = %feed_id, error = %e,
                                "org-expand: insert failed"),
            }
        }
    }
    // T-0329 — hot-register cron for each newly-inserted feed so the
    // org expand becomes live without a restart.
    if let Some(frt) = feed_runtime {
        for id in new_feed_ids {
            register_one_feed(Arc::clone(&frt), Arc::clone(&store), &id).await;
        }
    }
    let _ = workstream; // future: persist which workstream owns these
}

/// T-0329 — fetch a feed record by id and register its cron schedule
/// with the live runtime. Idempotent: cloacina's `register_one`
/// deletes any pre-existing schedule for the same workflow_name
/// before inserting a new one.
pub async fn register_one_feed(
    feed_runtime: Arc<arawn_feeds::FeedRuntime>,
    store: Arc<std::sync::Mutex<arawn_storage::Store>>,
    feed_id: &str,
) {
    let record = {
        let s = match store.lock() {
            Ok(g) => g,
            Err(_) => {
                warn!(feed_id = %feed_id, "register_one_feed: store mutex poisoned");
                return;
            }
        };
        let feed_store = arawn_feeds::FeedStore::new(s.database().conn());
        match feed_store.get(feed_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                debug!(feed_id = %feed_id,
                       "register_one_feed: row not found; skipping");
                return;
            }
            Err(e) => {
                warn!(feed_id = %feed_id, error = %e,
                      "register_one_feed: lookup failed");
                return;
            }
        }
    };
    if let Err(e) = feed_runtime.register_feed_runtime(&record).await {
        warn!(feed_id = %feed_id, error = %e,
              "register_one_feed: cron registration failed");
    }
}

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::util::{GithubScope, delete_feed, parse_github_scope};

pub trait UnbindHook: Send + Sync {
    fn on_unbind(&self, removed_feed_ids: &[String]);
}

pub struct LensUnbindTool {
    store: Arc<Mutex<Store>>,
    hook: Option<Arc<dyn UnbindHook>>,
}

impl LensUnbindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store, hook: None }
    }

    pub fn with_unbind_hook(mut self, hook: Arc<dyn UnbindHook>) -> Self {
        self.hook = Some(hook);
        self
    }
}

#[async_trait]
impl Tool for LensUnbindTool {
    fn name(&self) -> &str {
        "lens_unbind"
    }

    fn description(&self) -> &str {
        "Remove a feed binding from a lens. Silent no-op if not bound."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "feed_id": {"type": "string"}
            },
            "required": ["name", "feed_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let feed_id = params
            .get("feed_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || feed_id.is_empty() {
            return Ok(ToolOutput::error(
                "name and feed_id are required".to_string(),
            ));
        }
        let removed_feed_ids = {
            let store = self.store.lock().unwrap();
            match store.remove_lens_binding(&name, &feed_id) {
                Ok(()) => {
                    // I-0050 T-0326/0327 — sweep child feeds for github
                    // scope unbinds. Repo unbind drops a single feed;
                    // org unbind drops every `github-repo:owner/*`
                    // feed registered via the org expand. T-0329 fires
                    // the hook so live cron schedules go too.
                    match parse_github_scope(&feed_id) {
                        Some(GithubScope::Repo { owner, name: repo }) => {
                            let feed_id_full = format!("github-repo:{owner}/{repo}");
                            let _ = delete_feed(&store, &feed_id_full);
                            vec![feed_id_full]
                        }
                        Some(GithubScope::Org { owner }) => {
                            // Discover the child feed_ids before
                            // deleting them so the hook can unregister
                            // each cron.
                            let pattern = format!("github-repo:{owner}/%");
                            let conn = store.database().conn();
                            let removed = collect_child_feed_ids(conn, &pattern);
                            let _ = conn.execute(
                                "DELETE FROM feeds WHERE id LIKE ?1",
                                rusqlite::params![&pattern],
                            );
                            removed
                        }
                        None => Vec::new(),
                    }
                }
                Err(e) => {
                    return Ok(ToolOutput::error(format!("failed: {e}")));
                }
            }
        };
        // Fire the hook outside the store lock — it spawns its own
        // async work and shouldn't hold our mutex.
        if !removed_feed_ids.is_empty()
            && let Some(hook) = self.hook.as_ref()
        {
            hook.on_unbind(&removed_feed_ids);
        }
        Ok(ToolOutput::success(
            json!({
                "name": name,
                "feed_id": feed_id,
                "removed_feed_ids": removed_feed_ids,
            })
            .to_string(),
        ))
    }
}

fn collect_child_feed_ids(conn: &rusqlite::Connection, pattern: &str) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT id FROM feeds WHERE id LIKE ?1") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map(rusqlite::params![pattern], |r| r.get::<_, String>(0)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rows.flatten().collect()
}

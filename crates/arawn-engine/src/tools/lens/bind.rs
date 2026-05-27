use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::util::{
    GithubScope, delete_feed, find_lenses_binding, parse_github_scope, upsert_repo_mirror_feed,
    validate_github_scope_scheme,
};

pub trait BindBackfillHook: Send + Sync {
    fn on_bind(&self, lens_name: &str, feed_id: &str);
}

pub struct LensBindTool {
    store: Arc<Mutex<Store>>,
    hook: Option<Arc<dyn BindBackfillHook>>,
}

impl LensBindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store, hook: None }
    }

    pub fn with_backfill_hook(mut self, hook: Arc<dyn BindBackfillHook>) -> Self {
        self.hook = Some(hook);
        self
    }
}

#[async_trait]
impl Tool for LensBindTool {
    fn name(&self) -> &str {
        "lens_bind"
    }

    fn description(&self) -> &str {
        "Bind a feed to a lens. Bindings hint to the Phase 4 extractor \
         which feed items should land in this lens's KB. Idempotent. \
         \n\n\
         `feed_id` accepts either a real feed id (e.g. `gh-notifs-pat`) \
         OR a GitHub scope scheme (I-0045 T-0322):\n\
         - `github:repo:owner/name` — bind a specific repo (e.g. \
           `github:repo:openai/codex`). Repo-bindings take priority over \
           org-bindings on conflict.\n\
         - `github:org:owner` — bind every repo under an org (e.g. \
           `github:org:openai`).\n\
         \n\
         Scope bindings let one feed (e.g. `gh-notifs-personal`) fan out \
         to multiple lenses based on which repo/org each row touches."
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
        // I-0045 T-0322 — validate github scope-binding schemes
        // before they hit the store. Catches common typos at the
        // tool boundary (the store accepts any string).
        if let Err(msg) = validate_github_scope_scheme(&feed_id) {
            return Ok(ToolOutput::error(msg));
        }
        // I-0050 T-0326 — enforce org-supersedes-repo semantics and
        // register/unregister `github-repo:` feeds for scope bindings.
        let mut superseded: Vec<String> = Vec::new();
        if let Some(scope) = parse_github_scope(&feed_id) {
            let store = self.store.lock().unwrap();
            match &scope {
                GithubScope::Repo { owner, .. } => {
                    let org_key = format!("github:org:{owner}");
                    let already = find_lenses_binding(&store, |b| b == org_key);
                    if let Some((other_ws, _)) = already.first() {
                        return Ok(ToolOutput::error(format!(
                            "binding rejected: covered by `{org_key}` already bound to lens `{other_ws}`"
                        )));
                    }
                }
                GithubScope::Org { owner } => {
                    let prefix = format!("github:repo:{owner}/");
                    let to_drop: Vec<(String, String)> =
                        find_lenses_binding(&store, |b| b.starts_with(&prefix));
                    for (ws, binding) in &to_drop {
                        let _ = store.remove_lens_binding(ws, binding);
                        if let Some(GithubScope::Repo { owner, name }) = parse_github_scope(binding)
                        {
                            let feed_id = format!("github-repo:{owner}/{name}");
                            let _ = delete_feed(&store, &feed_id);
                        }
                    }
                    superseded.extend(to_drop.into_iter().map(|(ws, b)| format!("{b} (ws={ws})")));
                }
            }
        }
        let result = {
            let store = self.store.lock().unwrap();
            store.add_lens_binding(&name, &feed_id)
        };
        match result {
            Ok(()) => {
                // I-0050 T-0326 — register the `github-repo:owner/name`
                // feed for repo binds. Org binds are handled in T-0327
                // (list_org_repos fan-out happens via the bind hook).
                if let Some(GithubScope::Repo { owner, name: repo }) = parse_github_scope(&feed_id)
                {
                    let feed_id_full = format!("github-repo:{owner}/{repo}");
                    let store = self.store.lock().unwrap();
                    if let Err(e) = upsert_repo_mirror_feed(&store, &feed_id_full, &owner, &repo) {
                        // Persistence of the binding succeeded; feed
                        // registration is best-effort here. Log via
                        // tool output rather than failing the bind.
                        return Ok(ToolOutput::success(
                            json!({
                                "name": name,
                                "feed_id": feed_id,
                                "warning": format!("binding stored, feed registration failed: {e}")
                            })
                            .to_string(),
                        ));
                    }
                }
                // Fire backfill hook if wired. Drop store lock first
                // — the hook spawns its own task and shouldn't hold
                // our lock.
                if let Some(hook) = self.hook.as_ref() {
                    hook.on_bind(&name, &feed_id);
                }
                let mut body = json!({"name": name, "feed_id": feed_id});
                if !superseded.is_empty() {
                    body["superseded"] = json!(superseded);
                }
                Ok(ToolOutput::success(body.to_string()))
            }
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

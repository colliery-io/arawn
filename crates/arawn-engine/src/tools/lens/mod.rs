//! Lens slash commands.
//!
//! Tools that cover the lens lifecycle: new, list, show, describe, bind,
//! unbind, delete. Each is a `Tool` impl with a `lens_*` name so the slash
//! dispatcher routes naturally.
//!
//! ARAWN-I-0061: there is no `lens_switch` — lenses are standing, memory-aware
//! signal extractors, not a place you switch into. `SessionLens` remains only as
//! an internal default-context shim (pinned to `scratch`), not a user-switchable
//! write target.

mod bind;
mod create;
mod delete;
mod describe;
mod list;
mod propose_ontology;
mod session;
mod show;
mod unbind;
pub(crate) mod util;

pub use bind::{BindBackfillHook, LensBindTool};
pub use create::LensCreateTool;
pub use delete::LensDeleteTool;
pub use describe::LensDescribeTool;
pub use list::LensListTool;
pub use propose_ontology::LensProposeOntologyTool;
pub use session::SessionLens;
pub use show::LensShowTool;
pub use unbind::{LensUnbindTool, UnbindHook};
pub use util::{
    GithubScope, is_github_scope_binding, parse_github_scope, validate_github_scope_scheme,
};

#[cfg(test)]
mod tests {
    use super::util::*;
    use super::*;
    use arawn_core::Lens;
    use arawn_storage::Store;
    use arawn_tool::Tool;
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn setup() -> (tempfile::TempDir, Arc<Mutex<Store>>, SessionLens) {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_lens().unwrap();
        (tmp, Arc::new(Mutex::new(store)), SessionLens::scratch())
    }

    fn test_ctx(tmp: &tempfile::TempDir) -> crate::context::EngineToolContext {
        let ws = Lens::scratch(tmp.path());
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
            .with_data_dir(tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn create_succeeds_with_valid_slug_description_and_ontology() {
        let (tmp, store, _) = setup();
        let tool = LensCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "Pat's day job",
                    "tags_ontology": ["postgres", "ledger"]
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        assert!(result.content.contains("pat"));
        // Ontology is materialized — both seed tags should be present.
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        assert_eq!(ont.count().unwrap(), 2);
        assert!(ont.contains("postgres").unwrap());
        assert!(ont.contains("ledger").unwrap());
    }

    #[tokio::test]
    async fn create_refuses_scratch() {
        let (tmp, store, _) = setup();
        let tool = LensCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "scratch",
                    "description": "x",
                    "tags_ontology": ["x"]
                }),
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn create_refuses_missing_description() {
        let (tmp, store, _) = setup();
        let tool = LensCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "tags_ontology": ["x"]}),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("description"));
    }

    #[tokio::test]
    async fn create_refuses_empty_ontology() {
        let (tmp, store, _) = setup();
        let tool = LensCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "x",
                    "tags_ontology": []
                }),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("tags_ontology"));
    }

    #[tokio::test]
    async fn create_dedupes_and_normalizes_ontology() {
        let (tmp, store, _) = setup();
        let tool = LensCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "x",
                    "tags_ontology": ["Falcon", "falcon ", "FALCON", "ledger"]
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        assert_eq!(ont.count().unwrap(), 2);
        assert!(ont.contains("falcon").unwrap());
        assert!(ont.contains("ledger").unwrap());
    }


    #[tokio::test]
    async fn show_defaults_to_active() {
        let (tmp, store, active) = setup();
        let tool = LensShowTool::new(store.clone(), active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn describe_updates_description() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let tool = LensDescribeTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "description": "skip-level for pat"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let fetched = store
            .lock()
            .unwrap()
            .find_lens_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.description, "skip-level for pat");
    }

    // I-0045 T-0322 — github scope-binding validation.

    #[test]
    fn github_repo_scheme_accepts_owner_slash_name() {
        validate_github_scope_scheme("github:repo:openai/codex").unwrap();
    }

    #[test]
    fn github_repo_scheme_rejects_missing_slash() {
        let err = validate_github_scope_scheme("github:repo:openai").unwrap_err();
        assert!(err.contains("github:repo"), "msg: {err}");
    }

    #[test]
    fn github_repo_scheme_rejects_empty_owner_or_name() {
        assert!(validate_github_scope_scheme("github:repo:/codex").is_err());
        assert!(validate_github_scope_scheme("github:repo:openai/").is_err());
    }

    #[test]
    fn github_org_scheme_accepts_owner() {
        validate_github_scope_scheme("github:org:openai").unwrap();
    }

    #[test]
    fn github_org_scheme_rejects_empty_or_with_slash() {
        assert!(validate_github_scope_scheme("github:org:").is_err());
        assert!(validate_github_scope_scheme("github:org:openai/codex").is_err());
    }

    #[test]
    fn non_github_feed_ids_pass_through_unchanged() {
        // Regular feed_ids must NOT trip the github validator.
        validate_github_scope_scheme("gh-notifs-personal").unwrap();
        validate_github_scope_scheme("slack-design").unwrap();
        validate_github_scope_scheme("").unwrap();
    }

    #[test]
    fn is_github_scope_binding_recognises_both_schemes() {
        assert!(is_github_scope_binding("github:repo:openai/codex"));
        assert!(is_github_scope_binding("github:org:openai"));
        assert!(!is_github_scope_binding("gh-notifs-personal"));
        assert!(!is_github_scope_binding("github_notifications"));
    }

    #[tokio::test]
    async fn bind_accepts_github_repo_scheme_and_stores_it() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        let out = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        let fetched = store
            .lock()
            .unwrap()
            .find_lens_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.bindings, vec!["github:repo:openai/codex"]);
    }

    // I-0050 T-0326 — feed registration + org-supersedes-repo.

    #[test]
    fn parse_github_scope_handles_both_schemes() {
        assert_eq!(
            parse_github_scope("github:repo:openai/codex"),
            Some(GithubScope::Repo {
                owner: "openai".into(),
                name: "codex".into()
            })
        );
        assert_eq!(
            parse_github_scope("github:org:openai"),
            Some(GithubScope::Org {
                owner: "openai".into()
            })
        );
        assert_eq!(parse_github_scope("github:repo:openai"), None);
        assert_eq!(parse_github_scope("github:org:openai/codex"), None);
        assert_eq!(parse_github_scope("github:repo:openai/"), None);
        assert_eq!(parse_github_scope("gh-feed"), None);
    }

    fn count_feeds(store: &Arc<Mutex<Store>>, feed_id: &str) -> i64 {
        let s = store.lock().unwrap();
        let conn = s.database().conn();
        conn.query_row(
            "SELECT COUNT(*) FROM feeds WHERE id = ?1",
            rusqlite::params![feed_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn repo_bind_registers_feed_record() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        let out = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        // Feed record exists with the expected id + template + params.
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 1);
        let s = store.lock().unwrap();
        let conn = s.database().conn();
        let (template, params, cadence): (String, String, String) = conn
            .query_row(
                "SELECT template, params, cadence FROM feeds WHERE id = 'github-repo:openai/codex'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(template, "github/repo-mirror");
        assert!(params.contains("\"openai\""));
        assert!(params.contains("\"codex\""));
        assert_eq!(cadence, "*/30 * * * *");
    }

    #[tokio::test]
    async fn repo_bind_is_idempotent_no_duplicate_feed() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        let _ = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        let _ = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 1);
    }

    #[tokio::test]
    async fn repo_bind_rejected_when_org_already_bound() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-a", tmp.path().join("ws/a")))
            .unwrap();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-b", tmp.path().join("ws/b")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        // ws-a binds the org.
        bind.execute(
            &test_ctx(&tmp),
            json!({"name": "ws-a", "feed_id": "github:org:openai"}),
        )
        .await
        .unwrap();
        // ws-b tries to bind a repo under openai.
        let out = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "ws-b", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        assert!(out.is_error, "should reject");
        assert!(out.content.contains("github:org:openai"));
        assert!(out.content.contains("ws-a"));
        // ws-b has no binding stored.
        let ws_b = store
            .lock()
            .unwrap()
            .find_lens_by_name("ws-b")
            .unwrap()
            .unwrap();
        assert!(ws_b.bindings.is_empty());
        // And no repo feed was registered.
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 0);
    }

    #[tokio::test]
    async fn org_bind_supersedes_existing_repo_binds() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-a", tmp.path().join("ws/a")))
            .unwrap();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-b", tmp.path().join("ws/b")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        // ws-a binds two repos under openai.
        bind.execute(
            &test_ctx(&tmp),
            json!({"name": "ws-a", "feed_id": "github:repo:openai/codex"}),
        )
        .await
        .unwrap();
        bind.execute(
            &test_ctx(&tmp),
            json!({"name": "ws-a", "feed_id": "github:repo:openai/tinker"}),
        )
        .await
        .unwrap();
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 1);
        assert_eq!(count_feeds(&store, "github-repo:openai/tinker"), 1);
        // ws-b binds the org → both repo schedules dropped.
        let out = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "ws-b", "feed_id": "github:org:openai"}),
            )
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        // The two repo feeds are gone.
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 0);
        assert_eq!(count_feeds(&store, "github-repo:openai/tinker"), 0);
        // ws-a's repo bindings are removed too.
        let ws_a = store
            .lock()
            .unwrap()
            .find_lens_by_name("ws-a")
            .unwrap()
            .unwrap();
        assert!(
            ws_a.bindings.is_empty(),
            "ws-a should have no bindings, got {:?}",
            ws_a.bindings
        );
        // Output mentions the supersession.
        let body: serde_json::Value = serde_json::from_str(&out.content).unwrap();
        let sup = body["superseded"].as_array().unwrap();
        assert_eq!(sup.len(), 2);
    }

    // T-0329 — unbind hook fires with the right list of feed_ids.

    struct CapturingUnbindHook {
        captured: Mutex<Vec<Vec<String>>>,
    }
    impl UnbindHook for CapturingUnbindHook {
        fn on_unbind(&self, removed_feed_ids: &[String]) {
            self.captured
                .lock()
                .unwrap()
                .push(removed_feed_ids.to_vec());
        }
    }

    #[tokio::test]
    async fn unbind_hook_fires_with_repo_feed_id() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        // Hand-seed: binding + feed row.
        {
            let s = store.lock().unwrap();
            s.add_lens_binding("pat", "github:repo:openai/codex")
                .unwrap();
            s.database()
                .conn()
                .execute(
                    "INSERT INTO feeds (id, template, params, cadence, enabled, created_at, updated_at) \
                     VALUES ('github-repo:openai/codex', 'github/repo-mirror', '{}', '*/30 * * * *', 1, ?1, ?1)",
                    rusqlite::params!["2026-05-18T00:00:00Z"],
                )
                .unwrap();
        }
        let hook = Arc::new(CapturingUnbindHook {
            captured: Mutex::new(Vec::new()),
        });
        let unbind = LensUnbindTool::new(store.clone())
            .with_unbind_hook(hook.clone() as Arc<dyn UnbindHook>);
        let _ = unbind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        let captured = hook.captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0], vec!["github-repo:openai/codex".to_string()]);
    }

    #[tokio::test]
    async fn unbind_hook_fires_with_all_org_child_ids() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-org", tmp.path().join("ws/o")))
            .unwrap();
        {
            let s = store.lock().unwrap();
            s.add_lens_binding("ws-org", "github:org:openai").unwrap();
            let now = "2026-05-18T00:00:00Z";
            for feed_id in ["github-repo:openai/codex", "github-repo:openai/tinker"] {
                s.database()
                    .conn()
                    .execute(
                        "INSERT INTO feeds (id, template, params, cadence, enabled, created_at, updated_at) \
                         VALUES (?1, 'github/repo-mirror', '{}', '*/30 * * * *', 1, ?2, ?2)",
                        rusqlite::params![feed_id, now],
                    )
                    .unwrap();
            }
        }
        let hook = Arc::new(CapturingUnbindHook {
            captured: Mutex::new(Vec::new()),
        });
        let unbind = LensUnbindTool::new(store.clone())
            .with_unbind_hook(hook.clone() as Arc<dyn UnbindHook>);
        let _ = unbind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "ws-org", "feed_id": "github:org:openai"}),
            )
            .await
            .unwrap();
        let captured = hook.captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        let mut got = captured[0].clone();
        got.sort();
        assert_eq!(
            got,
            vec![
                "github-repo:openai/codex".to_string(),
                "github-repo:openai/tinker".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn unbind_hook_not_fired_for_plain_feed_id() {
        // Non-github bindings shouldn't trigger the hook with random
        // ids — `removed_feed_ids` is empty and the hook is skipped.
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        store
            .lock()
            .unwrap()
            .add_lens_binding("pat", "gh-notifs-personal")
            .unwrap();
        let hook = Arc::new(CapturingUnbindHook {
            captured: Mutex::new(Vec::new()),
        });
        let unbind = LensUnbindTool::new(store.clone())
            .with_unbind_hook(hook.clone() as Arc<dyn UnbindHook>);
        let _ = unbind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "gh-notifs-personal"}),
            )
            .await
            .unwrap();
        assert!(hook.captured.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unbind_org_scope_sweeps_all_child_feeds() {
        // I-0050 T-0327 — when an org binding is removed, every
        // github-repo:owner/* feed should be deleted (those are the
        // children registered by the org-expand step).
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("ws-org", tmp.path().join("ws/o")))
            .unwrap();
        // Hand-seed two child feeds + an unrelated repo feed.
        {
            let s = store.lock().unwrap();
            let conn = s.database().conn();
            let now = "2026-05-18T00:00:00Z";
            for feed_id in [
                "github-repo:openai/codex",
                "github-repo:openai/tinker",
                "github-repo:microsoft/foo",
            ] {
                conn.execute(
                    "INSERT INTO feeds (id, template, params, cadence, enabled, created_at, updated_at) \
                     VALUES (?1, 'github/repo-mirror', '{}', '*/30 * * * *', 1, ?2, ?2)",
                    rusqlite::params![feed_id, now],
                )
                .unwrap();
            }
            // The org binding itself on the lens.
            s.add_lens_binding("ws-org", "github:org:openai").unwrap();
        }
        let unbind = LensUnbindTool::new(store.clone());
        let out = unbind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "ws-org", "feed_id": "github:org:openai"}),
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        // Both openai child feeds gone; microsoft feed unaffected.
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 0);
        assert_eq!(count_feeds(&store, "github-repo:openai/tinker"), 0);
        assert_eq!(count_feeds(&store, "github-repo:microsoft/foo"), 1);
    }

    #[tokio::test]
    async fn unbind_repo_scope_drops_feed() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        bind.execute(
            &test_ctx(&tmp),
            json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
        )
        .await
        .unwrap();
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 1);
        let unbind = LensUnbindTool::new(store.clone());
        let out = unbind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai/codex"}),
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        assert_eq!(count_feeds(&store, "github-repo:openai/codex"), 0);
    }

    #[tokio::test]
    async fn bind_rejects_malformed_github_repo_scheme() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        let out = bind
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "feed_id": "github:repo:openai"}),
            )
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("github:repo"));
    }

    #[tokio::test]
    async fn bind_and_unbind_round_trip() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = LensBindTool::new(store.clone());
        bind.execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_lens_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.bindings, vec!["f1"]);
        let unbind = LensUnbindTool::new(store.clone());
        unbind
            .execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_lens_by_name("pat")
            .unwrap()
            .unwrap();
        assert!(fetched.bindings.is_empty());
    }

    #[tokio::test]
    async fn delete_refuses_scratch() {
        let (tmp, store, active) = setup();
        let tool = LensDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "scratch"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active") || result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn delete_refuses_currently_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = LensDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active"));
    }

    #[tokio::test]
    async fn delete_soft_marks_archived() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("temp", tmp.path().join("ws/temp")))
            .unwrap();
        let tool = LensDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "temp"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        // listed via list_all_lenses should still show it as archived.
        let all = store.lock().unwrap().list_all_lenses().unwrap();
        let found = all.iter().find(|w| w.name == "temp").unwrap();
        assert!(found.archived);
    }


    #[tokio::test]
    async fn show_includes_ontology() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("lenses/pat")))
            .unwrap();
        // Seed the ontology table directly.
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        ont.add("falcon", arawn_memory::AddedVia::Manual).unwrap();
        ont.add("ledger", arawn_memory::AddedVia::Manual).unwrap();

        active.set("pat");
        let tool = LensShowTool::new(store.clone(), active);
        let r = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!r.is_error, "got: {}", r.content);
        let v: serde_json::Value = serde_json::from_str(&r.content).unwrap();
        let tags: Vec<String> = v["tags_ontology"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        assert!(tags.contains(&"falcon".to_string()));
        assert!(tags.contains(&"ledger".to_string()));
    }

    #[tokio::test]
    async fn list_marks_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_lens(&Lens::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = LensListTool::new(store.clone()).with_active(active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        // Active lens should be flagged.
        assert!(result.content.contains("\"active\":true"));
    }
}

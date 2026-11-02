//! Integration tests for the two Jira templates. Mock-only.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use arawn_feeds::templates::jira::{AssigneeTrackerTemplate, ProjectTrackerTemplate};
use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, ConfluencePageBody, ConfluencePageMeta, DataLayout,
    DriveFeedClient, FeedClients, FeedError, FeedMeta, FeedTemplate, GithubFeedClient,
    GmailFeedClient, JiraIssueDetail, JiraIssueMeta, MetaStore, SlackFeedClient, TemplateCtx,
    TemplateParams,
};

/// In-memory atlassian emulator.
#[derive(Default)]
struct MockAtlassian {
    /// FIFO list of (search-jql, list) pairs.
    jql_pages: Mutex<Vec<Vec<JiraIssueMeta>>>,
    /// key -> queued JiraIssueDetail responses (FIFO).
    issue_details: Mutex<HashMap<String, Vec<JiraIssueDetail>>>,
    /// keys whose `issue_full` should fail with Provider.
    fail_full: Mutex<std::collections::HashSet<String>>,
    /// keys whose `issue_full` is a 404 (deleted, or not visible).
    not_found: Mutex<std::collections::HashSet<String>>,
    /// A JQL naming this text fails with a 400, as Jira does for a key
    /// that no longer exists.
    reject_jql_naming: Mutex<Option<String>>,
    /// Recorded calls.
    jql_calls: Mutex<Vec<(String, u32)>>,
    full_calls: Mutex<Vec<(String, bool, bool)>>,
    project_resolved: Mutex<Vec<String>>,
}

impl MockAtlassian {
    fn queue_search(&self, list: Vec<JiraIssueMeta>) {
        self.jql_pages.lock().unwrap().push(list);
    }
    fn queue_detail(&self, key: &str, detail: JiraIssueDetail) {
        self.issue_details
            .lock()
            .unwrap()
            .entry(key.into())
            .or_default()
            .push(detail);
    }
    fn fail_full(&self, key: &str) {
        self.fail_full.lock().unwrap().insert(key.into());
    }
    fn not_found(&self, key: &str) {
        self.not_found.lock().unwrap().insert(key.into());
    }
    fn jql_calls(&self) -> Vec<(String, u32)> {
        self.jql_calls.lock().unwrap().clone()
    }
    fn full_calls(&self) -> Vec<(String, bool, bool)> {
        self.full_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl AtlassianFeedClient for MockAtlassian {
    async fn space_pages_modified_since(
        &self,
        _: &str,
        _: Option<DateTime<Utc>>,
    ) -> Result<Vec<ConfluencePageMeta>, FeedError> {
        unreachable!("jira tests don't touch confluence")
    }
    async fn page_body_storage(&self, _: &str) -> Result<ConfluencePageBody, FeedError> {
        unreachable!("jira tests don't touch confluence")
    }

    async fn jql_search(
        &self,
        jql: &str,
        max_results: u32,
    ) -> Result<Vec<JiraIssueMeta>, FeedError> {
        self.jql_calls
            .lock()
            .unwrap()
            .push((jql.into(), max_results));
        if let Some(bad) = self.reject_jql_naming.lock().unwrap().as_deref()
            && jql.contains(bad)
        {
            return Err(FeedError::Provider(format!(
                "jira 400: {bad} does not exist"
            )));
        }
        let mut q = self.jql_pages.lock().unwrap();
        Ok(if q.is_empty() { vec![] } else { q.remove(0) })
    }

    async fn issue_full(
        &self,
        key: &str,
        want_changelog: bool,
        want_comments: bool,
    ) -> Result<JiraIssueDetail, FeedError> {
        self.full_calls
            .lock()
            .unwrap()
            .push((key.into(), want_changelog, want_comments));
        if self.fail_full.lock().unwrap().contains(key) {
            return Err(FeedError::Provider(format!(
                "simulated full fail for {key}"
            )));
        }
        if self.not_found.lock().unwrap().contains(key) {
            return Err(FeedError::InvalidParams(format!("jira not found: {key}")));
        }
        let mut all = self.issue_details.lock().unwrap();
        let queue = all
            .get_mut(key)
            .ok_or_else(|| FeedError::Provider(format!("no detail queued for {key}")))?;
        if queue.is_empty() {
            return Err(FeedError::Provider(format!("queue empty for {key}")));
        }
        Ok(queue.remove(0))
    }

    async fn resolve_project(&self, key_or_id: &str) -> Result<String, FeedError> {
        self.project_resolved.lock().unwrap().push(key_or_id.into());
        Ok(format!("id-{key_or_id}"))
    }

    async fn list_jira_projects(&self) -> Result<Vec<arawn_feeds::JiraProjectMeta>, FeedError> {
        unreachable!("jira tests don't use list_jira_projects");
    }

    async fn list_confluence_spaces(
        &self,
    ) -> Result<Vec<arawn_feeds::ConfluenceSpaceMeta>, FeedError> {
        unreachable!("jira tests don't use list_confluence_spaces");
    }
}

struct MockClients {
    atlassian: Arc<MockAtlassian>,
}

impl FeedClients for MockClients {
    fn slack(&self, _workspace: &str) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        None
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        None
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        Some(self.atlassian.clone())
    }
    fn github(&self) -> Option<Arc<dyn GithubFeedClient>> {
        None
    }
    fn linear(&self) -> Option<Arc<dyn arawn_feeds::LinearFeedClient>> {
        None
    }
}

fn issue_meta(key: &str, updated: &str) -> JiraIssueMeta {
    JiraIssueMeta {
        key: key.into(),
        id: format!("id-{key}"),
        updated: Some(updated.into()),
        summary: Some(format!("{key} summary")),
    }
}

fn issue_detail(
    key: &str,
    updated: &str,
    comments: Option<Vec<Value>>,
    changelog: Option<Vec<Value>>,
) -> JiraIssueDetail {
    JiraIssueDetail {
        meta: issue_meta(key, updated),
        fields: json!({
            "summary": format!("{key} summary"),
            "updated": updated,
            "status": { "name": "To Do" },
        }),
        comments,
        changelog,
    }
}

fn comment(id: &str, body: &str) -> Value {
    json!({
        "id": id,
        "author": { "displayName": "Alice" },
        "body": body,
        "created": "2026-05-08T10:00:00.000+0000",
    })
}

fn history(id: &str, field: &str, to: &str) -> Value {
    json!({
        "id": id,
        "created": "2026-05-08T10:00:00.000+0000",
        "items": [{ "field": field, "to": to }],
    })
}

async fn run_once(
    template: &dyn FeedTemplate,
    ctx: &TemplateCtx,
    params: &TemplateParams,
    feed_dir: &PathBuf,
) -> arawn_feeds::RunOutcome {
    let cursor = MetaStore::read(feed_dir)
        .unwrap()
        .map(|m| m.cursor)
        .unwrap_or(json!({"latest_updated_iso": null, "issues": {}}));
    let outcome = template
        .run(ctx, params, feed_dir, &cursor)
        .await
        .expect("template run failed");
    let mut meta = MetaStore::read(feed_dir)
        .unwrap()
        .unwrap_or_else(|| FeedMeta::new(template.name(), params.clone(), Value::Null));
    meta.cursor = outcome.cursor.clone();
    meta.last_run_at = Some(Utc::now().to_rfc3339());
    meta.last_status = Some(outcome.status.clone());
    meta.run_count += 1;
    MetaStore::write(feed_dir, &meta).unwrap();
    outcome
}

fn read_jsonl(path: &PathBuf) -> Vec<Value> {
    if !path.exists() {
        return vec![];
    }
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

// ── project-tracker ─────────────────────────────────────────────────

#[tokio::test]
async fn project_tracker_appends_new_comments_overwrites_issue_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/project-tracker", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![issue_meta("ENG-1", "2026-05-08T09:00:00.000+0000")]);
    mock.queue_detail(
        "ENG-1",
        issue_detail(
            "ENG-1",
            "2026-05-08T09:00:00.000+0000",
            Some(vec![comment("100", "first"), comment("101", "second")]),
            Some(vec![history("200", "status", "In Progress")]),
        ),
    );

    // Run 2: same issue updated — new comment + old comments dedupe.
    mock.queue_search(vec![issue_meta("ENG-1", "2026-05-08T11:00:00.000+0000")]);
    mock.queue_detail(
        "ENG-1",
        issue_detail(
            "ENG-1",
            "2026-05-08T11:00:00.000+0000",
            Some(vec![
                comment("100", "first"),
                comment("101", "second"),
                comment("102", "third"),
            ]),
            Some(vec![history("200", "status", "In Progress")]),
        ),
    );

    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));
    let params = TemplateParams(json!({ "project": "ENG" }));
    run_once(&ProjectTrackerTemplate, &ctx, &params, &feed_dir).await;
    run_once(&ProjectTrackerTemplate, &ctx, &params, &feed_dir).await;

    let issue_path = feed_dir.join("ENG-1").join("issue.json");
    let snap: Value = serde_json::from_str(&std::fs::read_to_string(&issue_path).unwrap()).unwrap();
    assert_eq!(snap["updated"], "2026-05-08T11:00:00.000+0000");

    let comments = read_jsonl(&feed_dir.join("ENG-1").join("comments.jsonl"));
    assert_eq!(comments.len(), 3, "all three comments present, no dupes");

    let history = read_jsonl(&feed_dir.join("ENG-1").join("history.jsonl"));
    assert_eq!(history.len(), 1, "history entry deduped on second run");

    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(
        meta.cursor["latest_updated_iso"],
        "2026-05-08T11:00:00.000+0000"
    );
    assert_eq!(meta.cursor["issues"]["ENG-1"]["last_comment_id"], "102");
    assert_eq!(meta.cursor["issues"]["ENG-1"]["last_history_id"], "200");
}

#[tokio::test]
async fn project_tracker_history_advances_independently_of_comments() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/project-tracker", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![issue_meta("ENG-7", "2026-05-08T09:00:00.000+0000")]);
    mock.queue_detail(
        "ENG-7",
        issue_detail(
            "ENG-7",
            "2026-05-08T09:00:00.000+0000",
            Some(vec![comment("500", "c1")]),
            Some(vec![history("900", "status", "In Progress")]),
        ),
    );
    // Run 2: new history but same comment.
    mock.queue_search(vec![issue_meta("ENG-7", "2026-05-08T11:00:00.000+0000")]);
    mock.queue_detail(
        "ENG-7",
        issue_detail(
            "ENG-7",
            "2026-05-08T11:00:00.000+0000",
            Some(vec![comment("500", "c1")]),
            Some(vec![
                history("900", "status", "In Progress"),
                history("901", "priority", "High"),
            ]),
        ),
    );

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let params = TemplateParams(json!({ "project": "ENG" }));
    run_once(&ProjectTrackerTemplate, &ctx, &params, &feed_dir).await;
    run_once(&ProjectTrackerTemplate, &ctx, &params, &feed_dir).await;

    let comments = read_jsonl(&feed_dir.join("ENG-7").join("comments.jsonl"));
    assert_eq!(comments.len(), 1);
    let history_log = read_jsonl(&feed_dir.join("ENG-7").join("history.jsonl"));
    assert_eq!(history_log.len(), 2, "both history entries present");
}

#[tokio::test]
async fn project_tracker_partial_failure_doesnt_block_other_issues() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/project-tracker", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![
        issue_meta("ENG-A", "2026-05-08T09:00:00.000+0000"),
        issue_meta("ENG-B", "2026-05-08T10:00:00.000+0000"),
    ]);
    mock.fail_full("ENG-A");
    mock.queue_detail(
        "ENG-B",
        issue_detail(
            "ENG-B",
            "2026-05-08T10:00:00.000+0000",
            Some(vec![]),
            Some(vec![]),
        ),
    );

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let params = TemplateParams(json!({ "project": "ENG" }));
    let outcome = run_once(&ProjectTrackerTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 1);

    assert!(!feed_dir.join("ENG-A").exists(), "ENG-A skipped");
    assert!(feed_dir.join("ENG-B").join("issue.json").exists());
}

#[tokio::test]
async fn project_tracker_validates_project() {
    assert!(
        ProjectTrackerTemplate
            .validate(&TemplateParams::default())
            .is_err()
    );
    let p = TemplateParams(json!({ "project": "" }));
    assert!(ProjectTrackerTemplate.validate(&p).is_err());
    let p = TemplateParams(json!({ "project": "ENG" }));
    ProjectTrackerTemplate.validate(&p).unwrap();
}

// ── assignee-tracker ────────────────────────────────────────────────

#[tokio::test]
async fn assignee_tracker_writes_only_issue_json_no_logs() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![issue_meta("ENG-3", "2026-05-08T09:00:00.000+0000")]);
    mock.queue_detail(
        "ENG-3",
        issue_detail("ENG-3", "2026-05-08T09:00:00.000+0000", None, None),
    );
    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));

    let outcome = run_once(
        &AssigneeTrackerTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 1);

    let issue_dir = feed_dir.join("ENG-3");
    assert!(issue_dir.join("issue.json").exists());
    assert!(
        !issue_dir.join("comments.jsonl").exists(),
        "no comments log on personal feed"
    );
    assert!(
        !issue_dir.join("history.jsonl").exists(),
        "no history log on personal feed"
    );

    // assignee-tracker requested issue_full with both flags off.
    let calls = mock.full_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], ("ENG-3".into(), false, false));
}

#[tokio::test]
async fn assignee_tracker_uses_currentUser_jql_and_advances_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![issue_meta("X-1", "2026-05-08T09:00:00.000+0000")]);
    mock.queue_detail(
        "X-1",
        issue_detail("X-1", "2026-05-08T09:00:00.000+0000", None, None),
    );
    mock.queue_search(vec![]); // run 2 returns nothing
    // run 2 rechecks X-1: still mine.
    mock.queue_search(vec![issue_meta("X-1", "2026-05-08T09:00:00.000+0000")]);

    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));
    run_once(
        &AssigneeTrackerTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    run_once(
        &AssigneeTrackerTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;

    let calls = mock.jql_calls();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].0.contains("currentUser()"));
    assert_eq!(calls[2].0, "key in (\"X-1\") AND assignee = currentUser()");
    assert!(
        calls[1].0.contains("updated >="),
        "second run carries `since` clause"
    );
}

#[tokio::test]
async fn returns_auth_when_atlassian_not_connected() {
    struct NoAtlassian;
    impl FeedClients for NoAtlassian {
        fn slack(&self, _workspace: &str) -> Option<Arc<dyn SlackFeedClient>> {
            None
        }
        fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
            None
        }
        fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
            None
        }
        fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
            None
        }
        fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
            None
        }
        fn github(&self) -> Option<Arc<dyn GithubFeedClient>> {
            None
        }
        fn linear(&self) -> Option<Arc<dyn arawn_feeds::LinearFeedClient>> {
            None
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/project-tracker", "ENG")
        .unwrap();
    let ctx = TemplateCtx::new(Arc::new(NoAtlassian));
    let params = TemplateParams(json!({ "project": "ENG" }));
    let err = ProjectTrackerTemplate
        .run(&ctx, &params, &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn assignee_tracker_partial_failure_doesnt_block_other_issues() {
    // T-0237: parity with project_tracker_partial_failure — one issue's
    // detail fetch fails, the rest of the batch still writes.
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![
        issue_meta("ENG-X", "2026-05-08T09:00:00.000+0000"),
        issue_meta("ENG-Y", "2026-05-08T10:00:00.000+0000"),
    ]);
    mock.fail_full("ENG-X");
    mock.queue_detail(
        "ENG-Y",
        issue_detail("ENG-Y", "2026-05-08T10:00:00.000+0000", None, None),
    );

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let outcome = run_once(
        &AssigneeTrackerTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 1);
    assert!(!feed_dir.join("ENG-X").exists(), "ENG-X skipped");
    assert!(feed_dir.join("ENG-Y").join("issue.json").exists());
}

// ── assignee-tracker recheck ────────────────────────────────────────

fn snapshot(feed_dir: &std::path::Path, key: &str) -> Value {
    let bytes = std::fs::read(feed_dir.join(key).join("issue.json")).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Run 1 brings A, B and C as mine. Run 2 returns none of them: B is
/// still mine and kept as it is, A was reassigned and is rewritten as not
/// mine, C is gone and its snapshot is removed.
#[tokio::test]
async fn assignee_tracker_recheck_rewrites_reassigned_and_removes_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();
    let t = "2026-05-08T09:00:00.000+0000";
    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![
        issue_meta("ENG-A", t),
        issue_meta("ENG-B", t),
        issue_meta("ENG-C", t),
    ]);
    for k in ["ENG-A", "ENG-B", "ENG-C"] {
        mock.queue_detail(k, issue_detail(k, t, None, None));
    }
    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));
    let params = TemplateParams::default();
    run_once(&AssigneeTrackerTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(snapshot(&feed_dir, "ENG-A")["assigned_to_me"], true);

    // Run 2: the query returns nothing; the recheck finds only B mine.
    mock.queue_search(vec![]);
    mock.queue_search(vec![issue_meta("ENG-B", t)]);
    mock.queue_detail(
        "ENG-A",
        issue_detail("ENG-A", "2026-05-09T09:00:00.000+0000", None, None),
    );
    mock.not_found("ENG-C");
    let outcome = run_once(&AssigneeTrackerTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 1);

    let calls = mock.jql_calls();
    assert_eq!(
        calls[2].0,
        "key in (\"ENG-A\", \"ENG-B\", \"ENG-C\") AND assignee = currentUser()"
    );
    let a = snapshot(&feed_dir, "ENG-A");
    assert_eq!(a["assigned_to_me"], false);
    assert_eq!(a["updated"], "2026-05-09T09:00:00.000+0000");
    assert_eq!(snapshot(&feed_dir, "ENG-B")["assigned_to_me"], true);
    assert!(!feed_dir.join("ENG-C").exists(), "gone issue removed");
    // B was still mine: not fetched again.
    let fulls: Vec<String> = mock.full_calls().into_iter().map(|c| c.0).collect();
    assert_eq!(fulls, ["ENG-A", "ENG-B", "ENG-C", "ENG-A", "ENG-C"]);
}

/// 120 open "mine" snapshots the query did not return are rechecked 50,
/// 50 and 20 keys per request. Done snapshots and ones already marked
/// not mine are not rechecked.
#[tokio::test]
async fn assignee_tracker_recheck_batches_fifty_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();
    let write = |key: &str, category: &str, mine: Option<bool>| {
        let dir = feed_dir.join(key);
        std::fs::create_dir_all(&dir).unwrap();
        let mut snap = json!({
            "key": key,
            "fields": { "status": { "name": "x", "statusCategory": { "key": category } } },
        });
        if let Some(m) = mine {
            snap["assigned_to_me"] = json!(m);
        }
        std::fs::write(dir.join("issue.json"), snap.to_string()).unwrap();
    };
    let keys: Vec<String> = (0..120).map(|n| format!("ENG-{n:03}")).collect();
    for (i, k) in keys.iter().enumerate() {
        // A snapshot from before `assigned_to_me` was written counts as mine.
        write(
            k,
            "indeterminate",
            if i % 2 == 0 { Some(true) } else { None },
        );
    }
    write("ENG-DONE", "done", Some(true));
    write("ENG-AWAY", "new", Some(false));

    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![]);
    let t = "2026-05-08T09:00:00.000+0000";
    for chunk in keys.chunks(50) {
        // All still mine.
        mock.queue_search(chunk.iter().map(|k| issue_meta(k, t)).collect());
    }
    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));
    run_once(
        &AssigneeTrackerTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;

    let calls = mock.jql_calls();
    let sizes: Vec<usize> = calls[1..]
        .iter()
        .map(|(jql, _)| jql.matches("\"ENG-").count())
        .collect();
    assert_eq!(sizes, [50, 50, 20]);
    assert!(
        calls
            .iter()
            .all(|(jql, _)| !jql.contains("ENG-DONE") && !jql.contains("ENG-AWAY"))
    );
    assert!(calls[1..].iter().all(|(_, max)| *max == 50));
    assert!(mock.full_calls().is_empty());
    assert!(feed_dir.join("ENG-000").exists());
}

/// Jira rejects a `key in (...)` naming a deleted key. The recheck then
/// looks up each key, removes the gone one, and retries on the rest.
#[tokio::test]
async fn assignee_tracker_recheck_survives_a_rejected_key() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("jira/assignee-tracker", "me")
        .unwrap();
    let t = "2026-05-08T09:00:00.000+0000";
    let mock = Arc::new(MockAtlassian::default());
    mock.queue_search(vec![issue_meta("ENG-A", t), issue_meta("ENG-C", t)]);
    mock.queue_detail("ENG-A", issue_detail("ENG-A", t, None, None));
    mock.queue_detail("ENG-C", issue_detail("ENG-C", t, None, None));
    let ctx = TemplateCtx::new(Arc::new(MockClients {
        atlassian: mock.clone(),
    }));
    let params = TemplateParams::default();
    run_once(&AssigneeTrackerTemplate, &ctx, &params, &feed_dir).await;

    mock.queue_search(vec![]);
    *mock.reject_jql_naming.lock().unwrap() = Some("ENG-C".into());
    mock.not_found("ENG-C");
    mock.queue_detail("ENG-A", issue_detail("ENG-A", t, None, None));
    mock.queue_search(vec![issue_meta("ENG-A", t)]); // the retry: still mine
    run_once(&AssigneeTrackerTemplate, &ctx, &params, &feed_dir).await;

    let calls = mock.jql_calls();
    assert_eq!(
        calls[3].0,
        "key in (\"ENG-A\") AND assignee = currentUser()"
    );
    assert!(!feed_dir.join("ENG-C").exists(), "gone issue removed");
    assert_eq!(snapshot(&feed_dir, "ENG-A")["assigned_to_me"], true);
}

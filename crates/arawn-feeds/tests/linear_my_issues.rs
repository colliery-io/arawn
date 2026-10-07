//! `linear/my-issues` against a scripted Linear client. Mock-only.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_feeds::templates::linear::MyIssuesTemplate;
use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, DriveFeedClient, FeedClients, FeedError, FeedTemplate,
    GithubFeedClient, GmailFeedClient, LinearFeedClient, LinearIssue, LinearIssuePage,
    SlackFeedClient, TemplateCtx, TemplateParams,
};

/// Pages served in order; records `(since, after)` of every call.
#[derive(Default)]
struct MockLinear {
    pages: Mutex<VecDeque<Result<LinearIssuePage, FeedError>>>,
    calls: Mutex<Vec<(Option<String>, Option<String>)>>,
}

impl MockLinear {
    fn queue(&self, issues: Vec<LinearIssue>, next: Option<&str>) {
        self.pages.lock().unwrap().push_back(Ok(LinearIssuePage {
            issues,
            next: next.map(String::from),
        }));
    }
    fn queue_err(&self, e: FeedError) {
        self.pages.lock().unwrap().push_back(Err(e));
    }
    fn calls(&self) -> Vec<(Option<String>, Option<String>)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl LinearFeedClient for MockLinear {
    async fn my_issues_page(
        &self,
        since: Option<&str>,
        after: Option<&str>,
    ) -> Result<(String, LinearIssuePage), FeedError> {
        self.calls
            .lock()
            .unwrap()
            .push((since.map(String::from), after.map(String::from)));
        let page = self
            .pages
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(LinearIssuePage::default()))?;
        Ok(("me".into(), page))
    }
}

struct MockClients {
    linear: Arc<MockLinear>,
}

impl FeedClients for MockClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
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
    fn linear(&self) -> Option<Arc<dyn LinearFeedClient>> {
        Some(self.linear.clone())
    }
}

fn issue(n: u32, updated: &str, assignee: &str, state_type: &str) -> LinearIssue {
    serde_json::from_value(json!({
        "id": format!("uuid-{n}"),
        "identifier": format!("ENG-{n}"),
        "title": format!("Issue {n}"),
        "description": "details",
        "url": format!("https://linear.app/acme/issue/ENG-{n}"),
        "priority": 2,
        "priorityLabel": "High",
        "dueDate": null,
        "createdAt": "2026-10-01T00:00:00.000Z",
        "updatedAt": updated,
        "state": {"id": "s", "name": "Todo", "type": state_type},
        "team": {"id": "t", "key": "ENG", "name": "Engineering"},
        "assignee": {"id": assignee, "name": "Someone", "email": null},
        "creator": {"id": "me", "name": "Me", "email": "me@example.com"},
        "labels": {"nodes": [{"name": "bug"}]},
        "comments": {"nodes": [
            {"id": "c1", "body": "first", "createdAt": "2026-10-02T00:00:00.000Z",
             "user": {"id": "me", "name": "Me"}}
        ]}
    }))
    .unwrap()
}

fn setup() -> (Arc<MockLinear>, TemplateCtx, tempfile::TempDir) {
    let linear = Arc::new(MockLinear::default());
    let ctx = TemplateCtx::new(Arc::new(MockClients {
        linear: linear.clone(),
    }));
    (linear, ctx, tempfile::tempdir().unwrap())
}

fn read_snapshot(dir: &std::path::Path, id: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join(id).join("issue.json")).unwrap()).unwrap()
}

#[tokio::test]
async fn follows_every_page_and_advances_the_cursor() {
    let (linear, ctx, dir) = setup();
    linear.queue(
        vec![
            issue(1, "2026-10-05T10:00:00.000Z", "me", "started"),
            issue(2, "2026-10-06T10:00:00.000Z", "other", "unstarted"),
        ],
        Some("page-2"),
    );
    linear.queue(
        vec![issue(3, "2026-10-04T10:00:00.000Z", "me", "completed")],
        None,
    );

    let t = MyIssuesTemplate;
    let params = TemplateParams::default();
    let cursor = t.defaults(&params).initial_cursor;
    let out = t.run(&ctx, &params, dir.path(), &cursor).await.unwrap();

    assert_eq!(out.summary.items_written, 3);
    assert_eq!(
        linear.calls(),
        [(None, None), (None, Some("page-2".to_string()))]
    );
    assert_eq!(out.cursor["latest_updated_iso"], "2026-10-06T10:00:00.000Z");

    let s1 = read_snapshot(dir.path(), "uuid-1");
    assert_eq!(s1["identifier"], "ENG-1");
    assert_eq!(s1["state"]["type"], "started");
    assert_eq!(s1["priority"], 2);
    assert_eq!(s1["labels"], json!(["bug"]));
    assert_eq!(s1["assigned_to_me"], true);
    assert_eq!(s1["comments"][0]["author"], "Me");
    assert_eq!(read_snapshot(dir.path(), "uuid-2")["assigned_to_me"], false);

    // The next run asks only for what changed after the cursor.
    let out2 = t.run(&ctx, &params, dir.path(), &out.cursor).await.unwrap();
    assert_eq!(out2.status, "no-new-items");
    assert_eq!(
        linear.calls()[2],
        (Some("2026-10-06T10:00:00.000Z".to_string()), None)
    );
    assert_eq!(out2.cursor, out.cursor);
}

#[tokio::test]
async fn first_run_starts_from_since_and_later_runs_ignore_it() {
    let (linear, ctx, dir) = setup();
    linear.queue(
        vec![issue(1, "2026-10-05T10:00:00.000Z", "me", "started")],
        None,
    );
    let t = MyIssuesTemplate;
    let params = TemplateParams(json!({"since": "2026-09-01T00:00:00Z"}));
    let cursor = t.defaults(&params).initial_cursor;
    let out = t.run(&ctx, &params, dir.path(), &cursor).await.unwrap();
    assert_eq!(
        linear.calls()[0].0.as_deref(),
        Some("2026-09-01T00:00:00+00:00")
    );

    t.run(&ctx, &params, dir.path(), &out.cursor).await.unwrap();
    assert_eq!(
        linear.calls()[1].0.as_deref(),
        Some("2026-10-05T10:00:00.000Z")
    );
}

#[tokio::test]
async fn rate_limit_is_given_back_as_feed_rate_limited() {
    let (linear, ctx, dir) = setup();
    linear.queue(
        vec![issue(1, "2026-10-05T10:00:00.000Z", "me", "started")],
        Some("page-2"),
    );
    linear.queue_err(FeedError::RateLimited {
        retry_after: Some(Duration::from_secs(120)),
    });
    let t = MyIssuesTemplate;
    let params = TemplateParams::default();
    let err = t
        .run(
            &ctx,
            &params,
            dir.path(),
            &t.defaults(&params).initial_cursor,
        )
        .await
        .unwrap_err();
    match err {
        FeedError::RateLimited { retry_after } => {
            assert_eq!(retry_after, Some(Duration::from_secs(120)))
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn not_connected_is_an_auth_error() {
    let t = MyIssuesTemplate;
    let dir = tempfile::tempdir().unwrap();
    let params = TemplateParams::default();
    let err = t
        .run(
            &TemplateCtx::noop(),
            &params,
            dir.path(),
            &t.defaults(&params).initial_cursor,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)), "{err:?}");
}

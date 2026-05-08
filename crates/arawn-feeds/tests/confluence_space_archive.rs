//! Integration tests for `confluence/space-archive`. Mock-only.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, ConfluencePageBody, ConfluencePageMeta, DataLayout,
    DriveFeedClient, FeedClients, FeedError, FeedMeta, FeedTemplate, GmailFeedClient, MetaStore,
    SlackFeedClient, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::confluence::SpaceArchiveTemplate;

#[derive(Default)]
struct MockAtlassianClient {
    /// FIFO queue of canned `space_pages_modified_since` responses.
    page_lists: Mutex<Vec<Vec<ConfluencePageMeta>>>,
    /// page_id -> body XML for `page_body_storage`.
    bodies: Mutex<std::collections::HashMap<String, Option<String>>>,
    /// Recorded `(space_key, since)` per list call.
    list_calls: Mutex<Vec<(String, Option<DateTime<Utc>>)>>,
    /// Recorded body fetches.
    body_calls: Mutex<Vec<String>>,
    /// page_ids that should error on body fetch.
    fail_body: Mutex<std::collections::HashSet<String>>,
}

impl MockAtlassianClient {
    fn queue_pages(&self, pages: Vec<ConfluencePageMeta>) {
        self.page_lists.lock().unwrap().push(pages);
    }
    fn set_body(&self, page_id: &str, xml: Option<String>) {
        self.bodies.lock().unwrap().insert(page_id.into(), xml);
    }
    fn fail_body_for(&self, page_id: &str) {
        self.fail_body.lock().unwrap().insert(page_id.into());
    }
    fn list_calls(&self) -> Vec<(String, Option<DateTime<Utc>>)> {
        self.list_calls.lock().unwrap().clone()
    }
    fn body_calls(&self) -> Vec<String> {
        self.body_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl AtlassianFeedClient for MockAtlassianClient {
    async fn space_pages_modified_since(
        &self,
        space_key: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ConfluencePageMeta>, FeedError> {
        self.list_calls
            .lock()
            .unwrap()
            .push((space_key.into(), since));
        let mut q = self.page_lists.lock().unwrap();
        Ok(if q.is_empty() { vec![] } else { q.remove(0) })
    }

    async fn page_body_storage(
        &self,
        page_id: &str,
    ) -> Result<ConfluencePageBody, FeedError> {
        self.body_calls.lock().unwrap().push(page_id.into());
        if self.fail_body.lock().unwrap().contains(page_id) {
            return Err(FeedError::Provider(format!("simulated body fetch fail for {page_id}")));
        }
        let xml = self.bodies.lock().unwrap().get(page_id).cloned().flatten();
        Ok(ConfluencePageBody {
            id: page_id.into(),
            storage_xml: xml,
            version: Some(1),
        })
    }
}

struct MockClients {
    atlassian: Arc<MockAtlassianClient>,
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
        Some(self.atlassian.clone())
    }
}

fn page(id: &str, title: &str, modified: &str, version: i64) -> ConfluencePageMeta {
    ConfluencePageMeta {
        id: id.into(),
        title: title.into(),
        space_key: "ENG".into(),
        version: Some(version),
        modified_time: Some(modified.into()),
        url: Some(format!("https://example.atlassian.net/wiki/{id}")),
    }
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
        .unwrap_or(Value::Null);
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

#[tokio::test]
async fn writes_per_page_metadata_and_body() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    mock.queue_pages(vec![
        page("p1", "Project plan", "2026-05-08T09:00:00Z", 3),
        page("p2", "Meeting notes", "2026-05-07T14:00:00Z", 1),
    ]);
    mock.set_body("p1", Some("<p>plan body</p>".into()));
    mock.set_body("p2", Some("<p>notes body</p>".into()));

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock.clone() }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    let outcome = run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 2);
    assert_eq!(outcome.status, "ok");

    // p1/page.json + body.storage.xml
    let p1_meta_path = feed_dir.join("p1").join("page.json");
    assert!(p1_meta_path.exists());
    let p1_meta: Value =
        serde_json::from_str(&std::fs::read_to_string(&p1_meta_path).unwrap()).unwrap();
    assert_eq!(p1_meta["title"], "Project plan");
    assert_eq!(p1_meta["version"], 3);

    let p1_body =
        std::fs::read_to_string(feed_dir.join("p1").join("body.storage.xml")).unwrap();
    assert_eq!(p1_body, "<p>plan body</p>");

    // Cursor advances to highest modified_time seen.
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["last_modified_iso"], "2026-05-08T09:00:00Z");
}

#[tokio::test]
async fn second_run_passes_cursor_as_since() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    mock.queue_pages(vec![page("p1", "First", "2026-05-08T09:00:00Z", 1)]);
    mock.set_body("p1", Some("<p>b1</p>".into()));
    mock.queue_pages(vec![]); // run 2 returns nothing

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock.clone() }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;

    let calls = mock.list_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1, None, "first run has no `since`");
    let expected_since: DateTime<Utc> = "2026-05-08T09:00:00Z".parse().unwrap();
    assert_eq!(
        calls[1].1,
        Some(expected_since),
        "second run uses prior latest as `since`"
    );
}

#[tokio::test]
async fn body_fetch_failure_skips_page_without_aborting_run() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    mock.queue_pages(vec![
        page("p1", "Bad", "2026-05-08T09:00:00Z", 1),
        page("p2", "Good", "2026-05-08T10:00:00Z", 1),
    ]);
    mock.fail_body_for("p1");
    mock.set_body("p2", Some("<p>good body</p>".into()));

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock.clone() }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    let outcome = run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(
        outcome.summary.items_written, 1,
        "only the good page is persisted"
    );

    assert!(!feed_dir.join("p1").exists(), "p1 not written");
    assert!(feed_dir.join("p2").join("page.json").exists());
    assert_eq!(mock.body_calls(), vec!["p1", "p2"]);
}

#[tokio::test]
async fn body_overwritten_on_re_fetch() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    mock.queue_pages(vec![page("p", "Title v1", "2026-05-08T09:00:00Z", 1)]);
    mock.set_body("p", Some("<p>v1</p>".into()));
    mock.queue_pages(vec![page("p", "Title v2", "2026-05-08T11:00:00Z", 2)]);
    mock.set_body("p", Some("<p>v2</p>".into()));

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;

    let body =
        std::fs::read_to_string(feed_dir.join("p").join("body.storage.xml")).unwrap();
    assert_eq!(body, "<p>v2</p>", "body overwrites with latest version");
    let meta_value: Value =
        serde_json::from_str(&std::fs::read_to_string(feed_dir.join("p").join("page.json")).unwrap())
            .unwrap();
    assert_eq!(meta_value["title"], "Title v2");
    assert_eq!(meta_value["version"], 2);
}

#[tokio::test]
async fn page_with_no_body_writes_empty_xml() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    mock.queue_pages(vec![page("placeholder", "TBD", "2026-05-08T09:00:00Z", 1)]);
    mock.set_body("placeholder", None);

    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    let outcome = run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 1);
    let body =
        std::fs::read_to_string(feed_dir.join("placeholder").join("body.storage.xml")).unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn empty_run_is_no_op_with_status() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();

    let mock = Arc::new(MockAtlassianClient::default());
    let ctx = TemplateCtx::new(Arc::new(MockClients { atlassian: mock }));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    let outcome = run_once(&SpaceArchiveTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.status, "no-new-items");
    assert_eq!(outcome.summary.items_written, 0);
}

#[tokio::test]
async fn returns_auth_when_atlassian_not_connected() {
    struct NoAtlassian;
    impl FeedClients for NoAtlassian {
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
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("confluence/space-archive", "ENG")
        .unwrap();
    let ctx = TemplateCtx::new(Arc::new(NoAtlassian));
    let params = TemplateParams(json!({ "space_key": "ENG" }));
    let err = SpaceArchiveTemplate
        .run(&ctx, &params, &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn validate_rejects_missing_space_key() {
    assert!(SpaceArchiveTemplate
        .validate(&TemplateParams::default())
        .is_err());
    let p = TemplateParams(json!({ "space_key": "" }));
    assert!(SpaceArchiveTemplate.validate(&p).is_err());
    let p = TemplateParams(json!({ "space_key": "ENG" }));
    SpaceArchiveTemplate.validate(&p).unwrap();
}

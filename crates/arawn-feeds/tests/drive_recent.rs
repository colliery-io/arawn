//! Integration tests for `drive/recent`. Mock-only.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use arawn_feeds::{
    CalendarFeedClient, DataLayout, DriveFeedClient, DriveFile, FeedClients, FeedError, FeedMeta,
    FeedTemplate, GmailFeedClient, MetaStore, SlackFeedClient, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::drive::RecentTemplate;

#[derive(Default)]
struct MockDriveClient {
    /// FIFO of (since-cutoff-recorded, files-returned) per call.
    pages: Mutex<Vec<Vec<DriveFile>>>,
    calls: Mutex<Vec<DateTime<Utc>>>,
}

impl MockDriveClient {
    fn queue(&self, files: Vec<DriveFile>) {
        self.pages.lock().unwrap().push(files);
    }
    fn last_since(&self) -> Option<DateTime<Utc>> {
        self.calls.lock().unwrap().last().copied()
    }
}

#[async_trait]
impl DriveFeedClient for MockDriveClient {
    async fn resolve_folder(&self, _: &str) -> Result<String, FeedError> {
        unreachable!("recent doesn't resolve folders")
    }
    async fn list_folder_children(&self, _: &str) -> Result<Vec<DriveFile>, FeedError> {
        unreachable!("recent doesn't list folder children")
    }
    async fn list_modified_since(
        &self,
        since: DateTime<Utc>,
        _max_results: u32,
    ) -> Result<Vec<DriveFile>, FeedError> {
        self.calls.lock().unwrap().push(since);
        let mut q = self.pages.lock().unwrap();
        Ok(if q.is_empty() { vec![] } else { q.remove(0) })
    }
    async fn download(&self, _: &str, _: Option<&str>) -> Result<Vec<u8>, FeedError> {
        unreachable!("recent doesn't download bodies")
    }
}

struct MockClients {
    drive: Arc<MockDriveClient>,
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
        Some(self.drive.clone())
    }
}

fn file(id: &str, name: &str, mime: &str, modified: &str) -> DriveFile {
    DriveFile {
        id: id.into(),
        name: name.into(),
        mime_type: mime.into(),
        modified_time: Some(modified.into()),
        md5_checksum: Some("md5stub".into()),
        parents: vec![],
        size: Some(123),
        is_folder: false,
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
async fn writes_per_file_metadata_partitioned_by_modified_date() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout.ensure_feed_dir("drive/recent", "me").unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.queue(vec![
        file("f1", "Q3 Plan.gdoc", "application/vnd.google-apps.document",
             "2026-05-08T10:00:00Z"),
        file("f2", "notes.md", "text/markdown", "2026-05-07T22:30:00Z"),
    ]);

    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let outcome = run_once(
        &RecentTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 2);
    assert_eq!(outcome.status, "ok");

    assert!(feed_dir.join("2026-05-08").join("f1.json").exists());
    assert!(feed_dir.join("2026-05-07").join("f2.json").exists());

    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(
        meta.cursor["latest_modified_iso"], "2026-05-08T10:00:00Z"
    );
}

#[tokio::test]
async fn second_run_uses_cursor_as_since() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout.ensure_feed_dir("drive/recent", "me").unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.queue(vec![file("a", "a.txt", "text/plain", "2026-05-08T10:00:00Z")]);
    mock.queue(vec![]); // run 2 returns nothing
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));

    run_once(&RecentTemplate, &ctx, &TemplateParams::default(), &feed_dir).await;
    run_once(&RecentTemplate, &ctx, &TemplateParams::default(), &feed_dir).await;

    let since = mock.last_since().unwrap();
    let expected: DateTime<Utc> = "2026-05-08T10:00:00Z".parse().unwrap();
    assert_eq!(since, expected, "second run uses prior latest as `since`");
}

#[tokio::test]
async fn empty_run_is_no_op_with_status() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout.ensure_feed_dir("drive/recent", "me").unwrap();
    let mock = Arc::new(MockDriveClient::default());
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock }));
    let outcome = run_once(
        &RecentTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.status, "no-new-items");
    assert_eq!(outcome.summary.items_written, 0);
}

#[tokio::test]
async fn returns_auth_when_drive_not_connected() {
    struct NoDrive;
    impl FeedClients for NoDrive {
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
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout.ensure_feed_dir("drive/recent", "me").unwrap();
    let ctx = TemplateCtx::new(Arc::new(NoDrive));
    let err = RecentTemplate
        .run(&ctx, &TemplateParams::default(), &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn validate_rejects_bad_days_back() {
    let p = TemplateParams(json!({ "days_back": 0 }));
    assert!(RecentTemplate.validate(&p).is_err());
    let p = TemplateParams(json!({ "days_back": 1000 }));
    assert!(RecentTemplate.validate(&p).is_err());
}

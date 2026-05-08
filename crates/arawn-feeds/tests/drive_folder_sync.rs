//! Integration tests for `drive/folder-sync`. Mock-only.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::json;

use arawn_feeds::{
    CalendarFeedClient, DataLayout, DriveFeedClient, DriveFile, FeedClients, FeedError, FeedMeta,
    FeedTemplate, GmailFeedClient, MetaStore, SlackFeedClient, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::drive::FolderSyncTemplate;

/// In-memory Drive emulator. Tests build a tree (folder_id ->
/// children, file_id -> bytes/export-bytes) and the mock serves it.
#[derive(Default)]
struct MockDriveClient {
    /// folder_id -> list of immediate children.
    children: Mutex<HashMap<String, Vec<DriveFile>>>,
    /// file_id -> raw bytes (for non-native types).
    raw_bodies: Mutex<HashMap<String, Vec<u8>>>,
    /// (file_id, export_mime) -> bytes.
    exports: Mutex<HashMap<(String, String), Vec<u8>>>,
    /// Recorded download calls.
    downloads: Mutex<Vec<(String, Option<String>)>>,
}

impl MockDriveClient {
    fn add_folder(&self, id: &str, children: Vec<DriveFile>) {
        self.children.lock().unwrap().insert(id.into(), children);
    }
    fn add_raw(&self, file_id: &str, body: &[u8]) {
        self.raw_bodies
            .lock()
            .unwrap()
            .insert(file_id.into(), body.to_vec());
    }
    fn add_export(&self, file_id: &str, export_mime: &str, body: &[u8]) {
        self.exports
            .lock()
            .unwrap()
            .insert((file_id.into(), export_mime.into()), body.to_vec());
    }
    fn download_calls(&self) -> Vec<(String, Option<String>)> {
        self.downloads.lock().unwrap().clone()
    }
}

#[async_trait]
impl DriveFeedClient for MockDriveClient {
    async fn resolve_folder(&self, path_or_id: &str) -> Result<String, FeedError> {
        Ok(path_or_id.into())
    }
    async fn list_folder_children(&self, folder_id: &str) -> Result<Vec<DriveFile>, FeedError> {
        Ok(self
            .children
            .lock()
            .unwrap()
            .get(folder_id)
            .cloned()
            .unwrap_or_default())
    }
    async fn list_modified_since(
        &self,
        _since: DateTime<Utc>,
        _max_results: u32,
    ) -> Result<Vec<DriveFile>, FeedError> {
        unreachable!("folder-sync doesn't list modified")
    }
    async fn download(
        &self,
        file_id: &str,
        export_mime: Option<&str>,
    ) -> Result<Vec<u8>, FeedError> {
        self.downloads
            .lock()
            .unwrap()
            .push((file_id.into(), export_mime.map(str::to_string)));
        if let Some(mime) = export_mime {
            self.exports
                .lock()
                .unwrap()
                .get(&(file_id.into(), mime.into()))
                .cloned()
                .ok_or_else(|| FeedError::Provider(format!("no export queued for {file_id}")))
        } else {
            self.raw_bodies
                .lock()
                .unwrap()
                .get(file_id)
                .cloned()
                .ok_or_else(|| FeedError::Provider(format!("no raw body queued for {file_id}")))
        }
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

fn raw_file(id: &str, name: &str, mime: &str, md5: &str) -> DriveFile {
    DriveFile {
        id: id.into(),
        name: name.into(),
        mime_type: mime.into(),
        modified_time: Some("2026-05-08T10:00:00Z".into()),
        md5_checksum: Some(md5.into()),
        parents: vec![],
        size: Some(0),
        is_folder: false,
    }
}

fn folder(id: &str, name: &str) -> DriveFile {
    DriveFile {
        id: id.into(),
        name: name.into(),
        mime_type: "application/vnd.google-apps.folder".into(),
        modified_time: None,
        md5_checksum: None,
        parents: vec![],
        size: None,
        is_folder: true,
    }
}

fn google_doc(id: &str, name: &str, modified: &str) -> DriveFile {
    DriveFile {
        id: id.into(),
        name: name.into(),
        mime_type: "application/vnd.google-apps.document".into(),
        modified_time: Some(modified.into()),
        md5_checksum: None,
        parents: vec![],
        size: None,
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
        .unwrap_or(json!({ "files": {} }));
    let outcome = template
        .run(ctx, params, feed_dir, &cursor)
        .await
        .expect("template run failed");
    let mut meta = MetaStore::read(feed_dir)
        .unwrap()
        .unwrap_or_else(|| FeedMeta::new(template.name(), params.clone(), json!({"files": {}})));
    meta.cursor = outcome.cursor.clone();
    meta.last_run_at = Some(Utc::now().to_rfc3339());
    meta.last_status = Some(outcome.status.clone());
    meta.run_count += 1;
    MetaStore::write(feed_dir, &meta).unwrap();
    outcome
}

#[tokio::test]
async fn mirrors_native_files_and_exports_google_natives() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();

    let mock = Arc::new(MockDriveClient::default());
    // folder root contains: README.md, sub/, plan.gdoc
    mock.add_folder(
        "folder123",
        vec![
            raw_file("readme1", "README.md", "text/markdown", "md1"),
            folder("sub1", "sub"),
            google_doc("doc1", "plan", "2026-05-08T10:00:00Z"),
        ],
    );
    mock.add_folder("sub1", vec![raw_file("nested", "deep.txt", "text/plain", "md2")]);
    mock.add_raw("readme1", b"# Readme body");
    mock.add_raw("nested", b"deep content");
    mock.add_export("doc1", "text/markdown", b"# Plan body");

    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let params = TemplateParams(json!({ "folder": "folder123" }));
    let outcome = run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 3);
    assert_eq!(outcome.status, "ok");

    assert_eq!(
        std::fs::read(feed_dir.join("README.md")).unwrap(),
        b"# Readme body"
    );
    assert_eq!(
        std::fs::read(feed_dir.join("sub/deep.txt")).unwrap(),
        b"deep content"
    );
    // Google Doc → exported as markdown, with .md suffix.
    assert_eq!(
        std::fs::read(feed_dir.join("plan.md")).unwrap(),
        b"# Plan body"
    );

    // Cursor records all three.
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["files"]["readme1"]["path"], "README.md");
    assert_eq!(meta.cursor["files"]["nested"]["path"], "sub/deep.txt");
    assert_eq!(meta.cursor["files"]["doc1"]["path"], "plan.md");
}

#[tokio::test]
async fn skips_unchanged_via_change_token_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.add_folder(
        "f",
        vec![raw_file("a", "a.txt", "text/plain", "checksum-v1")],
    );
    mock.add_raw("a", b"hello");
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let params = TemplateParams(json!({ "folder": "f" }));

    run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(mock.download_calls().len(), 1);

    // Run 2: same checksum → should NOT re-download.
    let outcome = run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(mock.download_calls().len(), 1, "no new download on re-run");
    assert_eq!(outcome.summary.items_written, 0);
    assert_eq!(outcome.status, "no-new-items");
}

#[tokio::test]
async fn deletes_local_when_remote_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.add_folder(
        "f",
        vec![
            raw_file("a", "a.txt", "text/plain", "v1"),
            raw_file("b", "b.txt", "text/plain", "v1"),
        ],
    );
    mock.add_raw("a", b"a");
    mock.add_raw("b", b"b");
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let params = TemplateParams(json!({ "folder": "f" }));
    run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert!(feed_dir.join("a.txt").exists());
    assert!(feed_dir.join("b.txt").exists());

    // Remove `b` from remote on next run.
    mock.add_folder("f", vec![raw_file("a", "a.txt", "text/plain", "v1")]);

    run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert!(feed_dir.join("a.txt").exists(), "a survives");
    assert!(!feed_dir.join("b.txt").exists(), "b is removed");
}

#[tokio::test]
async fn moved_file_cleans_up_old_path() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.add_folder(
        "f",
        vec![
            folder("sub1", "sub"),
            raw_file("a", "a.txt", "text/plain", "v1"),
        ],
    );
    mock.add_folder("sub1", vec![]);
    mock.add_raw("a", b"a");
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let params = TemplateParams(json!({ "folder": "f" }));
    run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert!(feed_dir.join("a.txt").exists());

    // Move `a` into `sub/`.
    mock.add_folder("f", vec![folder("sub1", "sub")]);
    mock.add_folder(
        "sub1",
        vec![raw_file("a", "a.txt", "text/plain", "v1")],
    );

    run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert!(
        !feed_dir.join("a.txt").exists(),
        "old path is cleaned up"
    );
    assert!(feed_dir.join("sub").join("a.txt").exists());
}

#[tokio::test]
async fn unsupported_google_native_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();

    let mock = Arc::new(MockDriveClient::default());
    mock.add_folder(
        "f",
        vec![DriveFile {
            id: "form1".into(),
            name: "Survey".into(),
            mime_type: "application/vnd.google-apps.form".into(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
            is_folder: false,
        }],
    );
    let ctx = TemplateCtx::new(Arc::new(MockClients { drive: mock.clone() }));
    let params = TemplateParams(json!({ "folder": "f" }));
    let outcome = run_once(&FolderSyncTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(
        outcome.summary.items_written, 0,
        "form is skipped; nothing written"
    );
    assert_eq!(mock.download_calls().len(), 0);
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
    let feed_dir = layout
        .ensure_feed_dir("drive/folder-sync", "Reports")
        .unwrap();
    let ctx = TemplateCtx::new(Arc::new(NoDrive));
    let params = TemplateParams(json!({ "folder": "f" }));
    let err = FolderSyncTemplate
        .run(&ctx, &params, &feed_dir, &json!({"files": {}}))
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn validate_rejects_missing_folder() {
    assert!(FolderSyncTemplate
        .validate(&TemplateParams::default())
        .is_err());
    let p = TemplateParams(json!({ "folder": "" }));
    assert!(FolderSyncTemplate.validate(&p).is_err());
    let p = TemplateParams(json!({ "folder": "abc" }));
    FolderSyncTemplate.validate(&p).unwrap();
}

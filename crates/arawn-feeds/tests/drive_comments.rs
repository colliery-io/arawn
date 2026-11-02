//! `drive/comments` against a scripted Drive client. Mock-only.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use arawn_feeds::templates::drive::CommentsTemplate;
use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, DocComment, DocMeta, DocReply, DriveFeedClient,
    DriveFile, FeedClients, FeedError, FeedTemplate, GithubFeedClient, GmailFeedClient,
    LinearFeedClient, SlackFeedClient, TemplateCtx, TemplateParams,
};

const ME: &str = "me@example.com";

/// `touched` is what the lookback query returns; `by_id` answers
/// `get_doc` (a missing id is a 404); `comments` is what Drive holds per
/// file. Records every
/// `list_comments` call as `(file id, start)`.
#[derive(Default)]
struct FakeDrive {
    touched: Mutex<Vec<DocMeta>>,
    by_id: Mutex<HashMap<String, DocMeta>>,
    comments: Mutex<HashMap<String, Vec<DocComment>>>,
    calls: Mutex<Vec<(String, Option<String>)>>,
    /// Files whose comments.list answers 403 (no longer shared with me).
    forbidden: Mutex<Vec<String>>,
}

impl FakeDrive {
    fn touch(&self, docs: Vec<DocMeta>) {
        *self.touched.lock().unwrap() = docs;
    }
    fn set_comments(&self, file_id: &str, cs: Vec<DocComment>) {
        self.comments.lock().unwrap().insert(file_id.into(), cs);
    }
    fn calls(&self) -> Vec<(String, Option<String>)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl DriveFeedClient for FakeDrive {
    async fn resolve_folder(&self, _: &str) -> Result<String, FeedError> {
        unreachable!()
    }
    async fn list_folder_children(&self, _: &str) -> Result<Vec<DriveFile>, FeedError> {
        unreachable!()
    }
    async fn list_modified_since(
        &self,
        _: DateTime<Utc>,
        _: u32,
    ) -> Result<Vec<DriveFile>, FeedError> {
        unreachable!()
    }
    async fn download(&self, _: &str, _: Option<&str>) -> Result<Vec<u8>, FeedError> {
        unreachable!()
    }
    async fn about_me(&self) -> Result<String, FeedError> {
        Ok(ME.into())
    }
    async fn list_touched_docs(&self, _: DateTime<Utc>) -> Result<Vec<DocMeta>, FeedError> {
        Ok(self.touched.lock().unwrap().clone())
    }
    async fn get_doc(&self, file_id: &str) -> Result<Option<DocMeta>, FeedError> {
        Ok(self.by_id.lock().unwrap().get(file_id).cloned())
    }
    async fn list_comments(
        &self,
        file_id: &str,
        start: Option<&str>,
    ) -> Result<Vec<DocComment>, FeedError> {
        self.calls
            .lock()
            .unwrap()
            .push((file_id.into(), start.map(String::from)));
        if self.forbidden.lock().unwrap().iter().any(|f| f == file_id) {
            return Err(FeedError::Provider(format!(
                "drive comments: comments.list: HTTP 403: insufficientFilePermissions {file_id}"
            )));
        }
        // startModifiedTime is a minimum (inclusive).
        Ok(self
            .comments
            .lock()
            .unwrap()
            .get(file_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| start.is_none_or(|s| c.modified_time.as_deref().unwrap_or("") >= s))
            .collect())
    }
}

struct Clients(Arc<FakeDrive>);

impl FeedClients for Clients {
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
        Some(self.0.clone())
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        None
    }
    fn github(&self) -> Option<Arc<dyn GithubFeedClient>> {
        None
    }
    fn linear(&self) -> Option<Arc<dyn LinearFeedClient>> {
        None
    }
}

fn doc(id: &str, owned_by_me: bool) -> DocMeta {
    DocMeta {
        id: id.into(),
        name: format!("Doc {id}"),
        mime_type: "application/vnd.google-apps.document".into(),
        web_view_link: Some(format!("https://docs.google.com/document/d/{id}/edit")),
        owned_by_me,
    }
}

fn comment(id: &str, modified: &str) -> DocComment {
    DocComment {
        id: id.into(),
        content: format!("Please look at {id}"),
        author: Some("Robin".into()),
        modified_time: Some(modified.into()),
        ..Default::default()
    }
}

fn mentioning(id: &str, modified: &str) -> DocComment {
    DocComment {
        mentioned_emails: vec![ME.into()],
        ..comment(id, modified)
    }
}

fn my_reply() -> DocReply {
    DocReply {
        id: "r1".into(),
        content: "Done".into(),
        author: Some("Me".into()),
        author_is_me: true,
        ..Default::default()
    }
}

fn setup() -> (Arc<FakeDrive>, TemplateCtx, tempfile::TempDir) {
    let drive = Arc::new(FakeDrive::default());
    let ctx = TemplateCtx::new(Arc::new(Clients(drive.clone())));
    (drive, ctx, tempfile::tempdir().unwrap())
}

async fn run(ctx: &TemplateCtx, params: &Value, dir: &Path, cursor: &Value) -> (Value, String) {
    let out = CommentsTemplate
        .run(ctx, &TemplateParams(params.clone()), dir, cursor)
        .await
        .expect("run");
    (out.cursor, out.status)
}

fn snapshot(dir: &Path, file: &str, comment: &str) -> Option<Value> {
    let bytes = std::fs::read(dir.join(file).join(format!("{comment}.json"))).ok()?;
    Some(serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn the_filter_keeps_only_threads_for_me() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("mine", true), doc("theirs", false)]);
    let t = "2026-10-05T10:00:00+00:00";
    drive.set_comments(
        "mine",
        vec![
            mentioning("mentioned", t),
            DocComment {
                assignee_email: Some("ME@example.com".into()),
                ..comment("assigned", t)
            },
            comment("unanswered", t),
            DocComment {
                replies: vec![my_reply()],
                ..comment("answered", t)
            },
            DocComment {
                resolved: true,
                ..mentioning("resolved", t)
            },
            DocComment {
                deleted: true,
                assignee_email: Some(ME.into()),
                ..comment("deleted", t)
            },
        ],
    );
    // Not my doc: an unanswered comment that does not name me is noise.
    drive.set_comments("theirs", vec![comment("other", t)]);

    let (_, status) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert_eq!(status, "ok");

    let reason = |id| snapshot(tmp.path(), "mine", id).map(|s| s["reason"].clone());
    assert_eq!(reason("mentioned"), Some(json!("mentioned")));
    assert_eq!(reason("assigned"), Some(json!("assigned")));
    assert_eq!(reason("unanswered"), Some(json!("owner_unanswered")));
    assert_eq!(reason("answered"), None, "my reply is the last message");
    assert_eq!(reason("resolved"), None);
    assert_eq!(reason("deleted"), None);
    assert!(snapshot(tmp.path(), "theirs", "other").is_none());

    let s = snapshot(tmp.path(), "mine", "mentioned").unwrap();
    assert_eq!(s["doc_title"], "Doc mine");
    assert_eq!(s["author"], "Robin");
    assert_eq!(
        s["link"],
        "https://docs.google.com/document/d/mine/edit?disco=mentioned"
    );
}

#[tokio::test]
async fn the_per_doc_cursor_advances_and_skips_unchanged_threads() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("f1", false), doc("f2", false)]);
    drive.set_comments(
        "f1",
        vec![
            mentioning("a", "2026-10-05T10:00:00+00:00"),
            mentioning("b", "2026-10-06T09:00:00+00:00"),
        ],
    );

    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert_eq!(cursor["docs"]["f1"], "2026-10-06T09:00:00+00:00");
    assert_eq!(cursor["docs"]["f2"], Value::Null, "no comments yet");
    assert!(cursor["lookback_floor"].is_string());

    let (cursor2, status) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert_eq!(
        drive.calls()[2..],
        [
            (
                "f1".to_string(),
                Some("2026-10-06T09:00:00+00:00".to_string())
            ),
            ("f2".to_string(), None),
        ]
    );
    // The boundary thread came back unchanged: nothing written.
    assert_eq!(status, "no-new-items");
    assert_eq!(cursor2["docs"]["f1"], "2026-10-06T09:00:00+00:00");
}

/// A doc without a snapshot for me drops out of the cursor when it
/// leaves the window, and is not polled any more.
#[tokio::test]
async fn a_doc_that_leaves_the_window_is_dropped_from_the_cursor() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("f1", false), doc("f2", false)]);
    // Not for me: no snapshot.
    drive.set_comments("f2", vec![comment("a", "2026-10-05T10:00:00+00:00")]);
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert_eq!(cursor["docs"]["f2"], "2026-10-05T10:00:00+00:00");

    drive.touch(vec![doc("f1", false)]);
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert!(cursor["docs"].get("f2").is_none());
    assert!(cursor["docs"].get("f1").is_some());
    let polled: Vec<String> = drive.calls()[2..].iter().map(|(f, _)| f.clone()).collect();
    assert_eq!(polled, ["f1"]);
}

#[tokio::test]
async fn a_resolved_thread_removes_its_snapshot() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("f1", false)]);
    drive.set_comments(
        "f1",
        vec![
            mentioning("a", "2026-10-05T10:00:00+00:00"),
            mentioning("b", "2026-10-05T11:00:00+00:00"),
        ],
    );
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert!(snapshot(tmp.path(), "f1", "a").is_some());

    drive.set_comments(
        "f1",
        vec![
            DocComment {
                resolved: true,
                ..mentioning("a", "2026-10-06T08:00:00+00:00")
            },
            mentioning("b", "2026-10-05T11:00:00+00:00"),
        ],
    );
    let (_, status) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert_eq!(status, "ok");
    assert!(snapshot(tmp.path(), "f1", "a").is_none());
    assert!(snapshot(tmp.path(), "f1", "b").is_some());

    // The last thread of a doc going takes the doc's folder with it.
    drive.set_comments(
        "f1",
        vec![DocComment {
            deleted: true,
            ..mentioning("b", "2026-10-07T08:00:00+00:00")
        }],
    );
    run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert!(!tmp.path().join("f1").exists());
}

#[tokio::test]
async fn a_doc_outside_the_window_is_polled_while_it_holds_a_snapshot() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("f1", false)]);
    drive
        .by_id
        .lock()
        .unwrap()
        .insert("f1".into(), doc("f1", false));
    drive.set_comments(
        "f1",
        vec![
            mentioning("a", "2026-10-05T10:00:00+00:00"),
            mentioning("b", "2026-10-05T11:00:00+00:00"),
        ],
    );
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;

    // f1 leaves the window; a is resolved.
    drive.touch(vec![]);
    drive.set_comments(
        "f1",
        vec![
            DocComment {
                resolved: true,
                ..mentioning("a", "2026-10-06T08:00:00+00:00")
            },
            mentioning("b", "2026-10-05T11:00:00+00:00"),
        ],
    );
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert!(
        snapshot(tmp.path(), "f1", "a").is_none(),
        "resolve still seen"
    );
    assert!(snapshot(tmp.path(), "f1", "b").is_some());
    assert_eq!(
        drive.calls().last().unwrap(),
        &(
            "f1".to_string(),
            Some("2026-10-05T11:00:00+00:00".to_string())
        ),
        "polled from its cursor"
    );
    assert_eq!(cursor["docs"]["f1"], "2026-10-06T08:00:00+00:00");

    // Its last thread is resolved: no snapshot left, out of the cursor.
    drive.set_comments(
        "f1",
        vec![DocComment {
            resolved: true,
            ..mentioning("b", "2026-10-07T08:00:00+00:00")
        }],
    );
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert!(!tmp.path().join("f1").exists());
    assert!(cursor["docs"].get("f1").is_none());

    // And it is not polled any more.
    let n = drive.calls().len();
    run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert_eq!(drive.calls().len(), n);
}

#[tokio::test]
async fn a_gone_or_unshared_doc_has_its_snapshots_removed() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("deleted", false), doc("unshared", false)]);
    drive
        .by_id
        .lock()
        .unwrap()
        .insert("unshared".into(), doc("unshared", false));
    for f in ["deleted", "unshared"] {
        drive.set_comments(f, vec![mentioning("a", "2026-10-05T10:00:00+00:00")]);
    }
    let (cursor, _) = run(&ctx, &json!({}), tmp.path(), &Value::Null).await;
    assert!(snapshot(tmp.path(), "deleted", "a").is_some());
    assert!(snapshot(tmp.path(), "unshared", "a").is_some());

    // Both leave the window: "deleted" is a 404 on files.get, "unshared"
    // a 403 on comments.list.
    drive.touch(vec![]);
    drive.forbidden.lock().unwrap().push("unshared".into());
    let (cursor, status) = run(&ctx, &json!({}), tmp.path(), &cursor).await;
    assert_eq!(status, "ok");
    assert!(!tmp.path().join("deleted").exists());
    assert!(!tmp.path().join("unshared").exists());
    assert_eq!(cursor["docs"], json!({}));
}

#[tokio::test]
async fn extra_file_ids_are_polled_as_well() {
    let (drive, ctx, tmp) = setup();
    drive.touch(vec![doc("f1", false)]);
    drive
        .by_id
        .lock()
        .unwrap()
        .insert("hint".into(), doc("hint", false));
    drive.set_comments("hint", vec![mentioning("a", "2026-10-05T10:00:00+00:00")]);

    let params = json!({ "extra_file_ids": ["hint", "f1", "gone"] });
    let (cursor, _) = run(&ctx, &params, tmp.path(), &Value::Null).await;
    assert!(snapshot(tmp.path(), "hint", "a").is_some());
    let polled: Vec<String> = drive.calls().into_iter().map(|(f, _)| f).collect();
    assert_eq!(polled, ["f1", "hint"], "f1 once; an unknown id is skipped");
    assert!(cursor["docs"].get("hint").is_some());
}

#[tokio::test]
async fn returns_auth_when_drive_not_connected() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = TemplateCtx::new(Arc::new(arawn_feeds::NoopClients));
    let err = CommentsTemplate
        .run(&ctx, &TemplateParams::default(), tmp.path(), &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

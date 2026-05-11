//! Integration tests for the three Gmail archive templates. Mock-only;
//! no real Gmail. Covers the shared `archive_query` helper and the
//! per-template query construction.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, DataLayout, DriveFeedClient, FeedClients, FeedError, FeedMeta,
    FeedTemplate, GmailFeedClient, MetaStore, SlackFeedClient, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::gmail::{
    InboxArchiveTemplate, LabelArchiveTemplate, SenderFilterTemplate,
};

/// Minimal Gmail message JSON for tests. Only the fields the template
/// actually reads (`id`, `internalDate`) need to be canonical.
fn message(id: &str, internal_date_ms: i64, subject: &str) -> Value {
    json!({
        "id": id,
        "threadId": format!("t-{id}"),
        "internalDate": internal_date_ms.to_string(),
        "snippet": subject,
        "payload": {
            "headers": [
                {"name": "Subject", "value": subject},
                {"name": "From", "value": "alice@example.com"},
            ],
        },
    })
}

#[derive(Default)]
struct MockGmailClient {
    /// FIFO queue of `(id-list, messages-by-id)` tuples — one per
    /// call to `list_message_ids`.
    pages: Mutex<Vec<(Vec<String>, std::collections::HashMap<String, Value>)>>,
    /// Recorded `(query, max_results)` pairs.
    list_calls: Mutex<Vec<(String, u32)>>,
    /// Recorded ids passed to `get_message`.
    get_calls: Mutex<Vec<String>>,
}

impl MockGmailClient {
    fn queue_messages(&self, msgs: Vec<Value>) {
        let ids: Vec<String> = msgs
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_string())
            .collect();
        let by_id = msgs
            .into_iter()
            .map(|m| (m["id"].as_str().unwrap().to_string(), m))
            .collect();
        self.pages.lock().unwrap().push((ids, by_id));
    }
    fn list_calls(&self) -> Vec<(String, u32)> {
        self.list_calls.lock().unwrap().clone()
    }
    fn get_call_count(&self) -> usize {
        self.get_calls.lock().unwrap().len()
    }
}

#[async_trait]
impl GmailFeedClient for MockGmailClient {
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
    ) -> Result<Vec<String>, FeedError> {
        self.list_calls
            .lock()
            .unwrap()
            .push((query.into(), max_results));
        let mut q = self.pages.lock().unwrap();
        Ok(if q.is_empty() {
            vec![]
        } else {
            q[0].0.clone()
        })
    }

    async fn get_message(&self, id: &str) -> Result<Value, FeedError> {
        self.get_calls.lock().unwrap().push(id.into());
        let q = self.pages.lock().unwrap();
        let (_ids, by_id) = q
            .first()
            .ok_or_else(|| FeedError::Provider("no page queued".into()))?;
        by_id
            .get(id)
            .cloned()
            .ok_or_else(|| FeedError::Provider(format!("unknown id {id}")))
    }
}

struct MockClients {
    gmail: Arc<MockGmailClient>,
}

impl FeedClients for MockClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        Some(self.gmail.clone())
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        None
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        None
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
    meta.last_run_at = Some(chrono::Utc::now().to_rfc3339());
    meta.last_status = Some(outcome.status.clone());
    meta.run_count += 1;
    MetaStore::write(feed_dir, &meta).unwrap();
    outcome
}

fn ymd_ms(y: i32, m: u32, d: u32) -> i64 {
    use chrono::TimeZone;
    chrono::Utc
        .with_ymd_and_hms(y, m, d, 12, 0, 0)
        .unwrap()
        .timestamp_millis()
}

fn read_msg(feed_dir: &PathBuf, day: &str, id: &str) -> Option<Value> {
    let p = feed_dir.join(day).join(format!("{id}.json"));
    if !p.exists() {
        return None;
    }
    Some(serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap())
}

#[tokio::test]
async fn inbox_archive_writes_per_message_partitioned_by_internal_date() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("gmail/inbox-archive", "me")
        .unwrap();

    let mock = Arc::new(MockGmailClient::default());
    mock.queue_messages(vec![
        message("m1", ymd_ms(2026, 5, 8), "Standup notes"),
        message("m2", ymd_ms(2026, 5, 7), "Yesterday's email"),
        message("m3", ymd_ms(2026, 5, 7), "Older but same day"),
    ]);

    let ctx = TemplateCtx::new(Arc::new(MockClients { gmail: mock.clone() }));
    let outcome = run_once(
        &InboxArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 3);
    assert_eq!(outcome.status, "ok");

    assert!(read_msg(&feed_dir, "2026-05-08", "m1").is_some());
    assert!(read_msg(&feed_dir, "2026-05-07", "m2").is_some());
    assert!(read_msg(&feed_dir, "2026-05-07", "m3").is_some());

    // Default query shape — defaults reach the client.
    let calls = mock.list_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "in:inbox newer_than:7d");

    // Cursor advances to highest internalDate seen.
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(
        meta.cursor["latest_internal_date"].as_i64().unwrap(),
        ymd_ms(2026, 5, 8)
    );
}

#[tokio::test]
async fn second_run_skips_already_archived_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("gmail/inbox-archive", "me")
        .unwrap();

    let mock = Arc::new(MockGmailClient::default());
    mock.queue_messages(vec![message("m1", ymd_ms(2026, 5, 8), "First")]);
    let ctx = TemplateCtx::new(Arc::new(MockClients { gmail: mock.clone() }));

    run_once(
        &InboxArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(mock.get_call_count(), 1);

    // Second run: same id is back in the list (e.g., user re-ran a
    // wider window). Helper should skip the get_message call entirely.
    mock.pages.lock().unwrap().clear();
    mock.queue_messages(vec![
        message("m1", ymd_ms(2026, 5, 8), "First"),
        message("m2", ymd_ms(2026, 5, 9), "Second"),
    ]);

    let outcome = run_once(
        &InboxArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 1);
    // m1 already on disk → skipped; only m2 fetched (one new get).
    assert_eq!(mock.get_call_count(), 2);
}

#[tokio::test]
async fn sender_filter_query_uses_from_operator() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("gmail/sender-filter", "alerts")
        .unwrap();

    let mock = Arc::new(MockGmailClient::default());
    mock.queue_messages(vec![message("a1", ymd_ms(2026, 5, 8), "PagerDuty")]);
    let ctx = TemplateCtx::new(Arc::new(MockClients { gmail: mock.clone() }));

    let params = TemplateParams(json!({
        "sender_pattern": "alerts@pagerduty.com",
        "days_back": 3,
    }));
    run_once(&SenderFilterTemplate, &ctx, &params, &feed_dir).await;

    let calls = mock.list_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        "from:\"alerts@pagerduty.com\" newer_than:3d"
    );
}

#[tokio::test]
async fn label_archive_query_uses_label_operator() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("gmail/label-archive", "starred")
        .unwrap();

    let mock = Arc::new(MockGmailClient::default());
    mock.queue_messages(vec![]);
    let ctx = TemplateCtx::new(Arc::new(MockClients { gmail: mock.clone() }));

    let params = TemplateParams(json!({ "label": "Projects/Arawn" }));
    let outcome = run_once(&LabelArchiveTemplate, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.status, "no-new-items");

    let calls = mock.list_calls();
    assert_eq!(
        calls[0].0,
        "label:\"Projects/Arawn\" newer_than:30d"
    );
}

#[tokio::test]
async fn returns_auth_when_gmail_not_connected() {
    struct NoGmail;
    impl FeedClients for NoGmail {
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
        .ensure_feed_dir("gmail/inbox-archive", "me")
        .unwrap();

    let ctx = TemplateCtx::new(Arc::new(NoGmail));
    let err = InboxArchiveTemplate
        .run(&ctx, &TemplateParams::default(), &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn malformed_message_skipped_without_aborting_batch() {
    // T-0237: a message with missing/unparseable internalDate is treated
    // as a malformed item and skipped — the rest of the batch still
    // writes. Previously this returned FeedError::Schema and poisoned
    // the whole run.
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("gmail/inbox-archive", "me")
        .unwrap();

    let mock = Arc::new(MockGmailClient::default());
    mock.queue_messages(vec![
        json!({
            "id": "good1",
            "threadId": "t1",
            "internalDate": "1778414400000",
            "snippet": "ok",
        }),
        json!({
            // Malformed — no internalDate.
            "id": "broken",
            "threadId": "t-broken",
            "snippet": "no internalDate field",
        }),
        json!({
            "id": "good2",
            "threadId": "t2",
            "internalDate": "1778500800000",
            "snippet": "also ok",
        }),
    ]);
    let ctx = TemplateCtx::new(Arc::new(MockClients { gmail: mock }));

    let outcome = InboxArchiveTemplate
        .run(&ctx, &TemplateParams::default(), &feed_dir, &Value::Null)
        .await
        .expect("malformed item should skip, not fail");
    assert_eq!(outcome.summary.items_written, 2, "good messages still written");
    assert_eq!(outcome.status, "ok");
}

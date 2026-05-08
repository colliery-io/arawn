//! End-to-end test of `slack/channel-archive` against a mock Slack
//! client. Exercises:
//! - the `FeedTemplate` trait
//! - cursor advancement across runs
//! - JSONL append + day-partitioning
//! - empty-result no-op behavior
//! - `meta.json` round-trip via the runtime's MetaStore
//!
//! No real Slack involved. The `MockSlackClient` is the harness pattern
//! every Slack-touching template test will reuse.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_feeds::{
    DataLayout, FeedClients, FeedError, FeedMeta, FeedTemplate, MetaStore, SlackFeedClient,
    SlackHistoryPage, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::slack::ChannelArchiveTemplate;

#[derive(Default)]
struct MockSlackClient {
    /// FIFO of canned responses; each call to `channel_history` pops
    /// the front. Lets a test prime several runs in a row.
    history_responses: Mutex<Vec<SlackHistoryPage>>,
    /// What `resolve_channel` returns. Defaults to "CMOCK".
    resolved_id: Mutex<String>,
    /// Recorded calls for assertion.
    history_calls: Mutex<Vec<(String, Option<String>)>>,
}

impl MockSlackClient {
    fn new() -> Self {
        Self {
            history_responses: Mutex::new(vec![]),
            resolved_id: Mutex::new("CMOCK".into()),
            history_calls: Mutex::new(vec![]),
        }
    }
    fn queue(&self, page: SlackHistoryPage) {
        self.history_responses.lock().unwrap().push(page);
    }
    fn calls(&self) -> Vec<(String, Option<String>)> {
        self.history_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl SlackFeedClient for MockSlackClient {
    async fn resolve_channel(&self, _name_or_id: &str) -> Result<String, FeedError> {
        Ok(self.resolved_id.lock().unwrap().clone())
    }

    async fn channel_history(
        &self,
        channel_id: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        self.history_calls
            .lock()
            .unwrap()
            .push((channel_id.into(), oldest_ts.map(str::to_string)));
        let mut responses = self.history_responses.lock().unwrap();
        if responses.is_empty() {
            Ok(SlackHistoryPage {
                messages: vec![],
                next_cursor_ts: oldest_ts.map(str::to_string),
            })
        } else {
            Ok(responses.remove(0))
        }
    }
}

struct MockClients {
    slack: Arc<MockSlackClient>,
}

impl FeedClients for MockClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        Some(self.slack.clone())
    }
}

fn slack_msg(ts: &str, text: &str) -> Value {
    json!({
        "type": "message",
        "ts": ts,
        "user": "UALICE",
        "text": text,
    })
}

/// Walk a YYYY-MM-DD.jsonl file in `feed_dir` and return all parsed
/// JSON lines.
fn read_jsonl(feed_dir: &PathBuf, day: &str) -> Vec<Value> {
    let path = feed_dir.join(format!("{day}.jsonl"));
    if !path.exists() {
        return vec![];
    }
    let body = std::fs::read_to_string(&path).unwrap();
    body.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
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

    // Persist meta the way the runtime would.
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

#[tokio::test]
async fn first_run_writes_messages_and_advances_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    // Build a few messages on 2026-05-08 (prefer a real ts so day
    // partition is deterministic).
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    mock.queue(SlackHistoryPage {
        messages: vec![
            slack_msg(&format!("{day0}.000001"), "hello"),
            slack_msg(&format!("{}.000002", day0 + 60), "second"),
        ],
        next_cursor_ts: Some(format!("{}.000002", day0 + 60)),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 2);
    assert_eq!(outcome.status, "ok");

    // JSONL on disk
    let lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["text"], "hello");
    assert_eq!(lines[1]["text"], "second");

    // meta.json shows the new cursor
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], format!("{}.000002", day0 + 60));
    assert_eq!(meta.run_count, 1);

    // mock saw the call with no cursor (first run)
    let calls = mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "CMOCK");
    assert_eq!(calls[0].1, None);
}

#[tokio::test]
async fn second_run_passes_cursor_and_only_writes_new() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());

    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();

    // Run 1: two messages
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg(&format!("{day0}.0001"), "first")],
        next_cursor_ts: Some(format!("{day0}.0001")),
    });
    // Run 2: one new message
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg(&format!("{}.0001", day0 + 600), "next")],
        next_cursor_ts: Some(format!("{}.0001", day0 + 600)),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let _ = run_once(&template, &ctx, &params, &feed_dir).await;
    let _ = run_once(&template, &ctx, &params, &feed_dir).await;

    let lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(lines.len(), 2, "second run should append, not overwrite");

    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], format!("{}.0001", day0 + 600));
    assert_eq!(meta.run_count, 2);

    // Mock saw both calls; second carried the first's cursor as `oldest_ts`.
    let calls = mock.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1, None);
    assert_eq!(calls[1].1, Some(format!("{day0}.0001")));
}

#[tokio::test]
async fn empty_run_is_a_no_op_with_status() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    // No queue -> default empty page

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 0);
    assert_eq!(outcome.status, "no-new-items");

    // No JSONL files written
    let entries: Vec<_> = std::fs::read_dir(&feed_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jsonl"))
        .collect();
    assert!(entries.is_empty());

    // Cursor still null/preserved
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor, Value::Null);
    assert_eq!(meta.last_status.as_deref(), Some("no-new-items"));
}

#[tokio::test]
async fn messages_partition_across_days() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let d1 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 23, 59, 0)
        .unwrap()
        .timestamp();
    let d2 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 9, 0, 1, 0)
        .unwrap()
        .timestamp();
    mock.queue(SlackHistoryPage {
        messages: vec![
            slack_msg(&format!("{d1}.0001"), "before midnight"),
            slack_msg(&format!("{d2}.0001"), "after midnight"),
        ],
        next_cursor_ts: Some(format!("{d2}.0001")),
    });

    let clients = Arc::new(MockClients { slack: mock });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let _ = run_once(&template, &ctx, &params, &feed_dir).await;

    let day1 = read_jsonl(&feed_dir, "2026-05-08");
    let day2 = read_jsonl(&feed_dir, "2026-05-09");
    assert_eq!(day1.len(), 1);
    assert_eq!(day2.len(), 1);
    assert_eq!(day1[0]["text"], "before midnight");
    assert_eq!(day2[0]["text"], "after midnight");
}

#[tokio::test]
async fn run_returns_auth_when_slack_not_connected() {
    struct NoSlack;
    impl FeedClients for NoSlack {
        fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
            None
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let ctx = TemplateCtx::new(Arc::new(NoSlack));
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let err = template
        .run(&ctx, &params, &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

//! Integration tests for `slack/my-mentions`.
//!
//! Mock-driven so we don't hit Slack. Covers:
//! - First run resolves user_id via auth_test, caches in cursor.
//! - Second run skips auth_test (cursor already has user_id).
//! - search.messages query is `<@USER>`.
//! - Day-grained `after:` overlap is deduped via exact ts comparison.
//! - Empty result writes nothing and reports `no-new-items`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_feeds::{
    CalendarFeedClient, DataLayout, FeedClients, FeedError, FeedMeta, FeedTemplate,
    GmailFeedClient, MetaStore, SlackAuthInfo, SlackFeedClient, SlackHistoryPage, TemplateCtx,
    TemplateParams,
};
use arawn_feeds::templates::slack::MyMentionsTemplate;

#[derive(Default)]
struct MockSlackClient {
    auth_info: Mutex<SlackAuthInfo>,
    auth_test_calls: Mutex<u32>,
    search_responses: Mutex<Vec<SlackHistoryPage>>,
    search_calls: Mutex<Vec<(String, Option<String>)>>,
}

impl MockSlackClient {
    fn new() -> Self {
        Self {
            auth_info: Mutex::new(SlackAuthInfo {
                user_id: "U01ALICE".into(),
                team_id: "T01TEAM".into(),
            }),
            ..Default::default()
        }
    }
    fn queue_search(&self, page: SlackHistoryPage) {
        self.search_responses.lock().unwrap().push(page);
    }
    fn auth_test_count(&self) -> u32 {
        *self.auth_test_calls.lock().unwrap()
    }
    fn search_calls(&self) -> Vec<(String, Option<String>)> {
        self.search_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl SlackFeedClient for MockSlackClient {
    async fn resolve_channel(&self, _: &str) -> Result<String, FeedError> {
        unreachable!("my-mentions doesn't call resolve_channel")
    }
    async fn channel_history(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!("my-mentions doesn't call channel_history")
    }
    async fn thread_replies(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!("my-mentions doesn't call thread_replies")
    }
    async fn open_dm(&self, _: &str) -> Result<String, FeedError> {
        unreachable!("my-mentions doesn't call open_dm")
    }

    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError> {
        *self.auth_test_calls.lock().unwrap() += 1;
        Ok(self.auth_info.lock().unwrap().clone())
    }

    async fn search_messages(
        &self,
        query: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        self.search_calls
            .lock()
            .unwrap()
            .push((query.into(), oldest_ts.map(str::to_string)));
        let mut responses = self.search_responses.lock().unwrap();
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
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        None
    }
}

fn mention_msg(ts: &str, channel: &str, text: &str) -> Value {
    json!({
        "type": "message",
        "ts": ts,
        "user": "U02BOB",
        "channel": { "id": channel },
        "text": text,
        "permalink": format!("https://example.slack.com/archives/{channel}/p{ts}"),
    })
}

fn read_jsonl(feed_dir: &PathBuf, day: &str) -> Vec<Value> {
    let path = feed_dir.join(format!("{day}.jsonl"));
    if !path.exists() {
        return vec![];
    }
    std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
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
async fn first_run_resolves_user_id_and_writes_mentions() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/my-mentions", "me")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    mock.queue_search(SlackHistoryPage {
        messages: vec![
            mention_msg(&format!("{day0}.0001"), "CDESIGN", "<@U01ALICE> ?"),
            mention_msg(&format!("{}.0002", day0 + 60), "CENG", "<@U01ALICE> hi"),
        ],
        next_cursor_ts: Some(format!("{}.0002", day0 + 60)),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = MyMentionsTemplate;
    let outcome = run_once(&template, &ctx, &TemplateParams::default(), &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 2);
    assert_eq!(outcome.status, "ok");

    // auth_test called exactly once for cache miss
    assert_eq!(mock.auth_test_count(), 1);

    // search query was the literal mention token for our user_id
    let calls = mock.search_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "<@U01ALICE>");
    assert_eq!(calls[0].1, None);

    // JSONL written
    let lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(lines.len(), 2);

    // Cursor caches user_id + advances latest_ts
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["my_user_id"], "U01ALICE");
    assert_eq!(meta.cursor["latest_ts"], format!("{}.0002", day0 + 60));
}

#[tokio::test]
async fn second_run_uses_cached_user_id_and_dedupes_overlap() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/my-mentions", "me")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    let early_ts = format!("{day0}.0001");
    let mid_ts = format!("{}.0001", day0 + 60);
    let late_ts = format!("{}.0001", day0 + 120);

    // Run 1: 2 mentions
    mock.queue_search(SlackHistoryPage {
        messages: vec![
            mention_msg(&early_ts, "CX", "first"),
            mention_msg(&mid_ts, "CX", "second"),
        ],
        next_cursor_ts: Some(mid_ts.clone()),
    });
    // Run 2: search returns the same day's results because `after:` is
    // day-grained (Slack's API). Template must dedupe to write only the
    // new one.
    mock.queue_search(SlackHistoryPage {
        messages: vec![
            mention_msg(&early_ts, "CX", "first"),  // duplicate
            mention_msg(&mid_ts, "CX", "second"),    // duplicate
            mention_msg(&late_ts, "CX", "third"),    // new
        ],
        next_cursor_ts: Some(late_ts.clone()),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = MyMentionsTemplate;

    run_once(&template, &ctx, &TemplateParams::default(), &feed_dir).await;
    let outcome2 = run_once(&template, &ctx, &TemplateParams::default(), &feed_dir).await;
    assert_eq!(
        outcome2.summary.items_written, 1,
        "second run should only write the new message, deduping the earlier two"
    );

    // auth_test called exactly once total — second run uses cached id
    assert_eq!(mock.auth_test_count(), 1);

    // Disk has all 3 (run 1 wrote 2, run 2 wrote 1)
    let lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(lines.len(), 3);

    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], late_ts);
}

#[tokio::test]
async fn empty_run_is_a_no_op() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/my-mentions", "me")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    // No search queued → empty response

    let clients = Arc::new(MockClients { slack: mock });
    let ctx = TemplateCtx::new(clients);
    let template = MyMentionsTemplate;

    let outcome = run_once(&template, &ctx, &TemplateParams::default(), &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 0);
    assert_eq!(outcome.status, "no-new-items");

    let entries: Vec<_> = std::fs::read_dir(&feed_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jsonl"))
        .collect();
    assert!(entries.is_empty(), "no JSONL files written");

    // Cursor still caches user_id for next run
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["my_user_id"], "U01ALICE");
    assert!(meta.cursor["latest_ts"].is_null());
}

#[tokio::test]
async fn returns_auth_when_slack_not_connected() {
    struct NoSlack;
    impl FeedClients for NoSlack {
        fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
            None
        }
        fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
            None
        }
        fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
            None
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/my-mentions", "me")
        .unwrap();

    let ctx = TemplateCtx::new(Arc::new(NoSlack));
    let template = MyMentionsTemplate;
    let err = template
        .run(&ctx, &TemplateParams::default(), &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

//! Integration tests for `slack/dm-archive`.
//!
//! Reuses the same mock harness pattern as slack_channel_archive.rs.
//! Focuses on the DM-specific bits: `open_dm` resolution, the rest
//! delegates to the shared `archive_channel_with_threads` helper that
//! channel-archive already exercises.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_feeds::{
    DataLayout, FeedClients, FeedError, FeedMeta, FeedTemplate, MetaStore, SlackAuthInfo,
    SlackFeedClient, SlackHistoryPage, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::slack::DmArchiveTemplate;

#[derive(Default)]
struct MockSlackClient {
    history_responses: Mutex<Vec<SlackHistoryPage>>,
    /// What `open_dm` returns. Defaults to "DMOCK".
    dm_channel_id: Mutex<String>,
    /// Recorded calls.
    open_dm_calls: Mutex<Vec<String>>,
    history_calls: Mutex<Vec<(String, Option<String>)>>,
}

impl MockSlackClient {
    fn new() -> Self {
        Self {
            dm_channel_id: Mutex::new("DMOCK".into()),
            ..Default::default()
        }
    }
    fn queue(&self, page: SlackHistoryPage) {
        self.history_responses.lock().unwrap().push(page);
    }
    fn open_dm_calls(&self) -> Vec<String> {
        self.open_dm_calls.lock().unwrap().clone()
    }
    fn history_calls(&self) -> Vec<(String, Option<String>)> {
        self.history_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl SlackFeedClient for MockSlackClient {
    async fn resolve_channel(&self, _name_or_id: &str) -> Result<String, FeedError> {
        unreachable!("dm-archive shouldn't call resolve_channel")
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

    async fn thread_replies(
        &self,
        _channel_id: &str,
        _parent_ts: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        Ok(SlackHistoryPage {
            messages: vec![],
            next_cursor_ts: oldest_ts.map(str::to_string),
        })
    }

    async fn open_dm(&self, user_id_or_name: &str) -> Result<String, FeedError> {
        self.open_dm_calls
            .lock()
            .unwrap()
            .push(user_id_or_name.to_string());
        Ok(self.dm_channel_id.lock().unwrap().clone())
    }

    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError> {
        unreachable!("dm-archive tests don't use auth_test");
    }

    async fn search_messages(
        &self,
        _query: &str,
        _oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!("dm-archive tests don't use search_messages");
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

fn dm_msg(ts: &str, text: &str) -> Value {
    json!({
        "type": "message",
        "ts": ts,
        "user": "UALICE",
        "text": text,
    })
}

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
async fn dm_archive_opens_dm_then_writes_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/dm-archive", "alice")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    mock.queue(SlackHistoryPage {
        messages: vec![
            dm_msg(&format!("{day0}.0001"), "hey"),
            dm_msg(&format!("{}.0002", day0 + 60), "got a sec?"),
        ],
        next_cursor_ts: Some(format!("{}.0002", day0 + 60)),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = DmArchiveTemplate;
    let params = TemplateParams::new(json!({ "user": "@alice" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 2);

    // open_dm was called with the raw user param
    let opens = mock.open_dm_calls();
    assert_eq!(opens, vec!["@alice".to_string()]);

    // history was called with the resolved DM channel id
    let calls = mock.history_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "DMOCK");

    // JSONL on disk under the dm-archive layout
    let lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["text"], "hey");

    // Cursor advanced
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], format!("{}.0002", day0 + 60));
}

#[tokio::test]
async fn dm_archive_returns_auth_when_slack_not_connected() {
    struct NoSlack;
    impl FeedClients for NoSlack {
        fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
            None
        }
    }

    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/dm-archive", "alice")
        .unwrap();

    let ctx = TemplateCtx::new(Arc::new(NoSlack));
    let template = DmArchiveTemplate;
    let params = TemplateParams::new(json!({ "user": "alice" }));

    let err = template
        .run(&ctx, &params, &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

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
    DataLayout, FeedClients, FeedError, FeedMeta, FeedTemplate, MetaStore, SlackAuthInfo,
    SlackFeedClient, SlackHistoryPage, TemplateCtx, TemplateParams,
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
    /// Per-parent_ts FIFO of replies pages.
    thread_responses: Mutex<std::collections::HashMap<String, Vec<SlackHistoryPage>>>,
    /// Recorded thread calls for assertion: (channel, parent_ts, oldest).
    thread_calls: Mutex<Vec<(String, String, Option<String>)>>,
    /// Per-parent_ts forced error queue. If a parent has an error
    /// queued, the next thread_replies call returns that error instead
    /// of the next page.
    thread_errors: Mutex<std::collections::HashMap<String, Vec<FeedError>>>,
}

impl MockSlackClient {
    fn new() -> Self {
        Self {
            resolved_id: Mutex::new("CMOCK".into()),
            ..Default::default()
        }
    }
    fn queue(&self, page: SlackHistoryPage) {
        self.history_responses.lock().unwrap().push(page);
    }
    fn queue_thread(&self, parent_ts: &str, page: SlackHistoryPage) {
        self.thread_responses
            .lock()
            .unwrap()
            .entry(parent_ts.into())
            .or_default()
            .push(page);
    }
    fn queue_thread_error(&self, parent_ts: &str, err: FeedError) {
        self.thread_errors
            .lock()
            .unwrap()
            .entry(parent_ts.into())
            .or_default()
            .push(err);
    }
    fn calls(&self) -> Vec<(String, Option<String>)> {
        self.history_calls.lock().unwrap().clone()
    }
    fn thread_calls(&self) -> Vec<(String, String, Option<String>)> {
        self.thread_calls.lock().unwrap().clone()
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

    async fn open_dm(&self, _user_id_or_name: &str) -> Result<String, FeedError> {
        Ok(self.resolved_id.lock().unwrap().clone())
    }

    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError> {
        unreachable!("channel-archive tests don't use auth_test");
    }

    async fn search_messages(
        &self,
        _query: &str,
        _oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!("channel-archive tests don't use search_messages");
    }

    async fn thread_replies(
        &self,
        channel_id: &str,
        parent_ts: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        self.thread_calls.lock().unwrap().push((
            channel_id.into(),
            parent_ts.into(),
            oldest_ts.map(str::to_string),
        ));
        // Errors take precedence so tests can prime "this thread fails
        // this run" without unbalancing the response queue.
        let mut errs = self.thread_errors.lock().unwrap();
        if let Some(q) = errs.get_mut(parent_ts) {
            if !q.is_empty() {
                return Err(q.remove(0));
            }
        }
        let mut responses = self.thread_responses.lock().unwrap();
        let page = responses
            .get_mut(parent_ts)
            .and_then(|q| if q.is_empty() { None } else { Some(q.remove(0)) });
        Ok(page.unwrap_or_else(|| SlackHistoryPage {
            messages: vec![],
            next_cursor_ts: oldest_ts.map(str::to_string),
        }))
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

    // Cursor still null/preserved (top-level + empty threads map)
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], Value::Null);
    assert_eq!(meta.cursor["threads"], json!({}));
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

// ─── Thread tests ───────────────────────────────────────────────────

fn slack_msg_with_replies(ts: &str, text: &str, reply_count: u64) -> Value {
    json!({
        "type": "message",
        "ts": ts,
        "user": "UALICE",
        "text": text,
        "reply_count": reply_count,
    })
}

#[tokio::test]
async fn parent_with_replies_seeds_thread_file_and_advances_thread_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let parent_ts_secs = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    let parent_ts = format!("{parent_ts_secs}.000001");
    let reply1_ts = format!("{}.000001", parent_ts_secs + 60);
    let reply2_ts = format!("{}.000001", parent_ts_secs + 120);

    // history returns one parent with replies
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg_with_replies(&parent_ts, "who broke prod?", 2)],
        next_cursor_ts: Some(parent_ts.clone()),
    });
    // thread_replies returns parent + 2 replies (Slack's pattern)
    mock.queue_thread(
        &parent_ts,
        SlackHistoryPage {
            messages: vec![
                slack_msg(&parent_ts, "who broke prod?"),
                slack_msg(&reply1_ts, "investigating"),
                slack_msg(&reply2_ts, "fixed in #1234"),
            ],
            next_cursor_ts: Some(reply2_ts.clone()),
        },
    );

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.status, "ok");

    // Day file: just the parent (top-level message)
    let day_lines = read_jsonl(&feed_dir, "2026-05-08");
    assert_eq!(day_lines.len(), 1);
    assert_eq!(day_lines[0]["text"], "who broke prod?");

    // Thread file: parent + 2 replies (parent seeded once via the
    // history pass, then skipped during replies dedup)
    let thread_path = feed_dir.join("threads").join(format!("{parent_ts}.jsonl"));
    let thread_body = std::fs::read_to_string(&thread_path).unwrap();
    let thread_lines: Vec<Value> = thread_body
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(thread_lines.len(), 3, "parent + 2 replies in thread file");
    assert_eq!(thread_lines[0]["text"], "who broke prod?");
    assert_eq!(thread_lines[1]["text"], "investigating");
    assert_eq!(thread_lines[2]["text"], "fixed in #1234");

    // Cursors
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], parent_ts);
    assert_eq!(meta.cursor["threads"][&parent_ts], reply2_ts);

    // The thread call carried `oldest=None` (first time)
    let tc = mock.thread_calls();
    assert_eq!(tc.len(), 1);
    assert_eq!(tc[0].1, parent_ts);
    assert_eq!(tc[0].2, None);
}

#[tokio::test]
async fn second_run_advances_thread_cursor_independently() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let parent_ts_secs = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    let parent_ts = format!("{parent_ts_secs}.0001");
    let r1 = format!("{}.0001", parent_ts_secs + 60);
    let r2 = format!("{}.0001", parent_ts_secs + 120);

    // Run 1
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg_with_replies(&parent_ts, "?", 1)],
        next_cursor_ts: Some(parent_ts.clone()),
    });
    mock.queue_thread(
        &parent_ts,
        SlackHistoryPage {
            messages: vec![slack_msg(&parent_ts, "?"), slack_msg(&r1, "first reply")],
            next_cursor_ts: Some(r1.clone()),
        },
    );
    // Run 2: no new top-level, one new reply
    mock.queue_thread(
        &parent_ts,
        SlackHistoryPage {
            messages: vec![slack_msg(&r2, "later reply")],
            next_cursor_ts: Some(r2.clone()),
        },
    );

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    run_once(&template, &ctx, &params, &feed_dir).await;
    let outcome2 = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome2.summary.items_written, 1, "only the new reply");

    let thread_path = feed_dir.join("threads").join(format!("{parent_ts}.jsonl"));
    let lines: Vec<Value> = std::fs::read_to_string(&thread_path)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3, "parent + first reply + later reply");
    assert_eq!(lines[2]["text"], "later reply");

    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["threads"][&parent_ts], r2);

    // Second run's thread call carried `oldest=r1`
    let tc = mock.thread_calls();
    assert_eq!(tc.len(), 2);
    assert_eq!(tc[0].2, None);
    assert_eq!(tc[1].2, Some(r1));
}

#[tokio::test]
async fn channel_archive_works_for_dm_id_passthrough() {
    // A user who already knows the DM channel id (D-prefix) can point
    // channel-archive at it directly — no resolve_channel call needed.
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "alice-dm")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    *mock.resolved_id.lock().unwrap() = "D123ABC".into();
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg(&format!("{day0}.0001"), "dm message")],
        next_cursor_ts: Some(format!("{day0}.0001")),
    });

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "D123ABC" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 1);

    // Channel id passed verbatim into history (no name lookup)
    let calls = mock.calls();
    assert_eq!(calls[0].0, "D123ABC");
}

#[tokio::test]
async fn channel_archive_works_for_mpim_id_passthrough() {
    // mpims have no human-friendly name; user must pass the M-id.
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "team-mpim")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    *mock.resolved_id.lock().unwrap() = "MXYZ789".into();
    use chrono::TimeZone;
    let day0 = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    mock.queue(SlackHistoryPage {
        messages: vec![slack_msg(&format!("{day0}.0001"), "group dm message")],
        next_cursor_ts: Some(format!("{day0}.0001")),
    });

    let clients = Arc::new(MockClients { slack: mock });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "MXYZ789" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.summary.items_written, 1);
}

#[test]
fn classify_helper_resolves_kinds_for_picker_use() {
    use arawn_feeds::{ChannelKind, classify_channel_id};
    assert_eq!(classify_channel_id("CABCDEF"), Some(ChannelKind::Public));
    assert_eq!(
        classify_channel_id("GABCDEF"),
        Some(ChannelKind::Private)
    );
    assert_eq!(
        classify_channel_id("DABCDEF"),
        Some(ChannelKind::DirectMessage)
    );
    assert_eq!(classify_channel_id("MABCDEF"), Some(ChannelKind::GroupDm));
    // Names → None so /watch picker falls through to name resolution
    assert_eq!(classify_channel_id("#design"), None);
}

#[tokio::test]
async fn thread_failure_does_not_block_channel_or_other_threads() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("slack/channel-archive", "design")
        .unwrap();

    let mock = Arc::new(MockSlackClient::new());
    use chrono::TimeZone;
    let parent_ts_secs = chrono::Utc
        .with_ymd_and_hms(2026, 5, 8, 9, 0, 0)
        .unwrap()
        .timestamp();
    let bad_parent = format!("{parent_ts_secs}.0001");
    let good_parent = format!("{}.0001", parent_ts_secs + 30);
    let good_reply = format!("{}.0001", parent_ts_secs + 90);

    mock.queue(SlackHistoryPage {
        messages: vec![
            slack_msg_with_replies(&bad_parent, "thread that errors", 5),
            slack_msg_with_replies(&good_parent, "thread that succeeds", 1),
        ],
        next_cursor_ts: Some(good_parent.clone()),
    });
    mock.queue_thread_error(
        &bad_parent,
        FeedError::RateLimited { retry_after: None },
    );
    mock.queue_thread(
        &good_parent,
        SlackHistoryPage {
            messages: vec![
                slack_msg(&good_parent, "thread that succeeds"),
                slack_msg(&good_reply, "ok"),
            ],
            next_cursor_ts: Some(good_reply.clone()),
        },
    );

    let clients = Arc::new(MockClients { slack: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let template = ChannelArchiveTemplate;
    let params = TemplateParams::new(json!({ "channel": "#design" }));

    let outcome = run_once(&template, &ctx, &params, &feed_dir).await;
    assert_eq!(outcome.status, "ok"); // run did NOT fail overall

    // Channel cursor advanced
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.cursor["latest_ts"], good_parent);

    // Bad thread: cursor stayed None (will retry next run)
    assert_eq!(meta.cursor["threads"][&bad_parent], Value::Null);
    // Good thread: cursor advanced
    assert_eq!(meta.cursor["threads"][&good_parent], good_reply);

    // Bad thread file has only the parent (seeded from history); no
    // replies because the call errored.
    let bad_path = feed_dir
        .join("threads")
        .join(format!("{bad_parent}.jsonl"));
    let bad_lines = std::fs::read_to_string(&bad_path).unwrap();
    let bad_count = bad_lines.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(bad_count, 1, "only the parent — replies failed to fetch");
}

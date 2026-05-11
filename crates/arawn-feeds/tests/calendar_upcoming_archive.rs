//! Integration tests for `calendar/upcoming-archive`.
//!
//! Mock-driven; no real Google Calendar API calls. Covers:
//! - First run writes one file per event under `events/`.
//! - Update path: same event id → file is overwritten with new payload.
//! - Cancelled events are preserved (status field round-trips).
//! - `window_days` and `calendar_id` params reach the client.
//! - Auth error when calendar integration not connected.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, DataLayout, DriveFeedClient, FeedClients, FeedError, FeedMeta,
    FeedTemplate, GmailFeedClient, MetaStore, SlackFeedClient, TemplateCtx, TemplateParams,
};
use arawn_feeds::templates::calendar::UpcomingArchiveTemplate;

#[derive(Default)]
struct MockCalendarClient {
    /// Queued responses; FIFO. Each `list_events` call pops one.
    responses: Mutex<Vec<Vec<Value>>>,
    /// Recorded `(calendar_id, time_min, time_max)` per call.
    calls: Mutex<Vec<(String, DateTime<Utc>, DateTime<Utc>)>>,
}

impl MockCalendarClient {
    fn queue(&self, events: Vec<Value>) {
        self.responses.lock().unwrap().push(events);
    }
    fn calls(&self) -> Vec<(String, DateTime<Utc>, DateTime<Utc>)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl CalendarFeedClient for MockCalendarClient {
    async fn list_events(
        &self,
        calendar_id: &str,
        time_min: DateTime<Utc>,
        time_max: DateTime<Utc>,
    ) -> Result<Vec<Value>, FeedError> {
        self.calls
            .lock()
            .unwrap()
            .push((calendar_id.into(), time_min, time_max));
        let mut q = self.responses.lock().unwrap();
        Ok(if q.is_empty() { vec![] } else { q.remove(0) })
    }
}

struct MockClients {
    calendar: Arc<MockCalendarClient>,
}

impl FeedClients for MockClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        Some(self.calendar.clone())
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

fn event(id: &str, summary: &str, start: &str) -> Value {
    json!({
        "id": id,
        "summary": summary,
        "status": "confirmed",
        "start": { "dateTime": start },
        "end":   { "dateTime": start },
    })
}

fn read_event_file(feed_dir: &PathBuf, safe_id: &str) -> Option<Value> {
    let p = feed_dir.join("events").join(format!("{safe_id}.json"));
    if !p.exists() {
        return None;
    }
    let body = std::fs::read_to_string(&p).unwrap();
    Some(serde_json::from_str(&body).unwrap())
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
async fn first_run_writes_one_file_per_event() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    mock.queue(vec![
        event("evt-001", "Standup", "2026-05-09T15:00:00Z"),
        event("evt-002", "1:1 with Bob", "2026-05-10T18:30:00Z"),
        event("evt-003", "Planning", "2026-05-11T13:00:00Z"),
    ]);

    let clients = Arc::new(MockClients { calendar: mock.clone() });
    let ctx = TemplateCtx::new(clients);
    let outcome = run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 3);
    assert_eq!(outcome.status, "ok");

    // One JSON per event id, no day-partitioned dir.
    let entries: Vec<_> = std::fs::read_dir(feed_dir.join("events"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries.len(), 3);

    let file = read_event_file(&feed_dir, "evt-001").expect("evt-001 missing");
    assert_eq!(file["summary"], "Standup");

    // Defaults reached the client: primary calendar, ~7 days window.
    let calls = mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "primary");
    let span = calls[0].2 - calls[0].1;
    assert_eq!(span.num_days(), 7);
}

#[tokio::test]
async fn second_run_overwrites_changed_events() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    mock.queue(vec![event("evt-1", "Old title", "2026-05-09T15:00:00Z")]);
    mock.queue(vec![event("evt-1", "New title", "2026-05-09T16:00:00Z")]);

    let clients = Arc::new(MockClients { calendar: mock });
    let ctx = TemplateCtx::new(clients);

    run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    let first = read_event_file(&feed_dir, "evt-1").unwrap();
    assert_eq!(first["summary"], "Old title");

    run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    let second = read_event_file(&feed_dir, "evt-1").unwrap();
    assert_eq!(
        second["summary"], "New title",
        "the file should be overwritten with the latest payload"
    );

    // Still exactly one file, not two.
    let entries: Vec<_> = std::fs::read_dir(feed_dir.join("events"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn cancelled_events_are_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    mock.queue(vec![json!({
        "id": "evt-cancel",
        "summary": "Was happening",
        "status": "cancelled",
        "start": { "dateTime": "2026-05-09T15:00:00Z" },
        "end":   { "dateTime": "2026-05-09T16:00:00Z" },
    })]);

    let clients = Arc::new(MockClients { calendar: mock });
    let ctx = TemplateCtx::new(clients);
    run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;

    let f = read_event_file(&feed_dir, "evt-cancel").expect("cancelled event missing");
    assert_eq!(f["status"], "cancelled");
}

#[tokio::test]
async fn params_reach_the_client() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "team")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    mock.queue(vec![]);
    let clients = Arc::new(MockClients { calendar: mock.clone() });
    let ctx = TemplateCtx::new(clients);

    let params = TemplateParams(json!({
        "calendar_id": "team@example.com",
        "window_days": 14,
    }));
    run_once(&UpcomingArchiveTemplate, &ctx, &params, &feed_dir).await;

    let calls = mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "team@example.com");
    assert_eq!((calls[0].2 - calls[0].1).num_days(), 14);
}

#[tokio::test]
async fn returns_auth_when_calendar_not_connected() {
    struct NoCal;
    impl FeedClients for NoCal {
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
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let ctx = TemplateCtx::new(Arc::new(NoCal));
    let err = UpcomingArchiveTemplate
        .run(&ctx, &TemplateParams::default(), &feed_dir, &Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, FeedError::Auth(_)));
}

#[tokio::test]
async fn empty_window_writes_nothing_and_status_no_new_items() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    let clients = Arc::new(MockClients { calendar: mock });
    let ctx = TemplateCtx::new(clients);
    let outcome = run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 0);
    assert_eq!(outcome.status, "no-new-items");
}

#[tokio::test]
async fn malformed_event_without_id_is_skipped() {
    // T-0237: a Calendar event with no `id` field is treated as
    // malformed and skipped — the other events in the batch still
    // write.
    let tmp = tempfile::tempdir().unwrap();
    let layout = DataLayout::new(tmp.path());
    let feed_dir = layout
        .ensure_feed_dir("calendar/upcoming-archive", "primary")
        .unwrap();

    let mock = Arc::new(MockCalendarClient::default());
    mock.queue(vec![
        event("good1", "First", "2026-05-09T15:00:00Z"),
        // Malformed — no `id`.
        json!({
            "summary": "no-id event",
            "status": "confirmed",
            "start": { "dateTime": "2026-05-10T15:00:00Z" },
            "end":   { "dateTime": "2026-05-10T16:00:00Z" },
        }),
        event("good2", "Second", "2026-05-11T15:00:00Z"),
    ]);

    let clients = Arc::new(MockClients { calendar: mock });
    let ctx = TemplateCtx::new(clients);
    let outcome = run_once(
        &UpcomingArchiveTemplate,
        &ctx,
        &TemplateParams::default(),
        &feed_dir,
    )
    .await;
    assert_eq!(outcome.summary.items_written, 2, "good events still written");
    assert!(read_event_file(&feed_dir, "good1").is_some());
    assert!(read_event_file(&feed_dir, "good2").is_some());
}

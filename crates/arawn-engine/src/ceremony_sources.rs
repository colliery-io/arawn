//! Real projection-backed impls of the daily ceremony plugin's
//! `CalendarSource` and `AttentionSource` traits (see ARAWN-T-0297).
//!
//! These adapters keep `arawn-ceremonies` itself free of hard deps on
//! `arawn-projections` / `arawn-storage`: the plugin sees only the
//! trait objects, and the binary wires the concrete sources in.
//!
//! Data lives in the per-feed-type projection tables managed by
//! `arawn-projections::ProjectionStore`. We hit the raw rusqlite
//! connection via `ProjectionStore::conn()` because there's no
//! day-range / time-range read API on the store yet (one would be a
//! reasonable follow-up, but we don't need it for the daily plugin).
//!
//! Lens-tagging for attention signals routes each row's
//! `feed_id` through `arawn_storage::Store::find_lens_for_feed`
//! (a thin wrapper over the `lenses.bindings` registry). Results
//! are cached per adapter instance — lenses change rarely and
//! re-querying per row would be wasteful. Cached `None` (no owner)
//! and stale name values are acceptable until process restart.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use arawn_ceremonies::CeremonyError;
use arawn_ceremonies::plugins::{AttentionSource, CalEvent, CalendarSource, SignalRow};
use arawn_projections::ProjectionStore;
use arawn_storage::Store;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::params;

const EXCERPT_CHARS: usize = 300;

fn storage_err(msg: impl Into<String>) -> CeremonyError {
    CeremonyError::Storage(msg.into())
}

fn truncate_excerpt(s: &str) -> String {
    let mut out: String = s.chars().take(EXCERPT_CHARS).collect();
    if s.chars().count() > EXCERPT_CHARS {
        out.push('…');
    }
    out
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, CeremonyError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| storage_err(format!("parse rfc3339 '{s}': {e}")))
}

/// Production `CalendarSource` backed by the `calendar_events`
/// projection table.
///
/// `tz` is the timezone the day-window is bracketed in before
/// converting to UTC for the BETWEEN query. Defaults to UTC (matching
/// pre-T-0306 behaviour and "Local" config fallback).
pub struct ProjectionsCalendarSource {
    projections: Arc<ProjectionStore>,
    tz: Tz,
}

impl ProjectionsCalendarSource {
    pub fn new(projections: Arc<ProjectionStore>) -> Self {
        Self {
            projections,
            tz: chrono_tz::UTC,
        }
    }

    /// Builder: set the timezone used to bracket day windows in
    /// `events_for`. Pass `chrono_tz::UTC` (the default) to keep the
    /// pre-T-0306 UTC semantics.
    pub fn with_tz(mut self, tz: Tz) -> Self {
        self.tz = tz;
        self
    }
}

#[async_trait]
impl CalendarSource for ProjectionsCalendarSource {
    async fn events_for(&self, date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError> {
        // Day window: bracket `date` as [00:00:00, 23:59:59] in
        // `self.tz`, then convert both bounds to UTC for the BETWEEN
        // query against the projection's UTC `source_ts`. When
        // `tz == chrono_tz::UTC` this matches the pre-T-0306 behaviour
        // exactly. DST ambiguity is resolved by picking the earliest
        // candidate (single() falls back to from_local_datetime's
        // earliest variant).
        let day_start_naive = date.and_hms_opt(0, 0, 0).unwrap();
        let day_end_naive = date.and_hms_opt(23, 59, 59).unwrap();
        let start_local = self
            .tz
            .from_local_datetime(&day_start_naive)
            .earliest()
            .or_else(|| self.tz.from_local_datetime(&day_start_naive).latest())
            .ok_or_else(|| storage_err("day_start localisation failed"))?;
        let end_local = self
            .tz
            .from_local_datetime(&day_end_naive)
            .latest()
            .or_else(|| self.tz.from_local_datetime(&day_end_naive).earliest())
            .ok_or_else(|| storage_err("day_end localisation failed"))?;
        let start = start_local.with_timezone(&Utc).to_rfc3339();
        let end = end_local.with_timezone(&Utc).to_rfc3339();

        // Make sure the table exists (no-op if it does).
        self.projections
            .ensure_feed_type("calendar_events")
            .map_err(|e| storage_err(format!("ensure calendar_events: {e}")))?;

        let conn = self
            .projections
            .conn()
            .lock()
            .map_err(|_| storage_err("projection connection mutex poisoned"))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, title, body_text, metadata, source_ts \
                 FROM calendar_events \
                 WHERE source_ts BETWEEN ?1 AND ?2 \
                 ORDER BY source_ts ASC",
            )
            .map_err(|e| storage_err(format!("prepare calendar query: {e}")))?;

        let mut rows = stmt
            .query(params![start, end])
            .map_err(|e| storage_err(format!("query calendar: {e}")))?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| storage_err(format!("calendar row: {e}")))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| storage_err(format!("col id: {e}")))?;
            let title: String = row
                .get(1)
                .map_err(|e| storage_err(format!("col title: {e}")))?;
            let body_text: String = row
                .get(2)
                .map_err(|e| storage_err(format!("col body_text: {e}")))?;
            let metadata_str: String = row
                .get(3)
                .map_err(|e| storage_err(format!("col metadata: {e}")))?;
            let source_ts_str: String = row
                .get(4)
                .map_err(|e| storage_err(format!("col source_ts: {e}")))?;

            let start_dt = parse_rfc3339(&source_ts_str)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
                .map_err(|e| storage_err(format!("metadata json: {e}")))?;
            let end_dt = match metadata.get("end_ts").and_then(|v| v.as_str()) {
                Some(s) => parse_rfc3339(s)?,
                None => start_dt,
            };
            let attendees: Vec<String> = metadata
                .get("attendees")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let body_excerpt = if body_text.is_empty() {
                None
            } else {
                Some(truncate_excerpt(&body_text))
            };

            out.push(CalEvent {
                id,
                title,
                start: start_dt,
                end: end_dt,
                attendees,
                body_excerpt,
            });
        }
        Ok(out)
    }
}

/// Production `AttentionSource` backed by the `gmail_messages`,
/// `slack_messages`, `linear_issues`, `drive_comments` and `jira_issues`
/// projection tables.
///
/// `store` resolves each row's `feed_id` to a lens name (via
/// `Store::find_lens_for_feed`). Results are cached in
/// `feed_lens_cache` keyed by feed_id, with both `Some(name)`
/// and `None` (no owner) memoised — see module docs on the cache
/// invalidation policy.
pub struct ProjectionsAttentionSource {
    projections: Arc<ProjectionStore>,
    store: Arc<Mutex<Store>>,
    feed_lens_cache: Arc<Mutex<HashMap<String, Option<String>>>>,
}

impl ProjectionsAttentionSource {
    pub fn new(projections: Arc<ProjectionStore>, store: Arc<Mutex<Store>>) -> Self {
        Self {
            projections,
            store,
            feed_lens_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Cached `feed_id → lens name` lookup. Cache misses query
    /// `Store::find_lens_for_feed` and memoise both `Some` and
    /// `None` results. Storage errors surface to the caller; the
    /// attention adapter then maps them into a `CeremonyError`.
    fn lens_for_feed(&self, feed_id: &str) -> Result<Option<String>, CeremonyError> {
        {
            let cache = self
                .feed_lens_cache
                .lock()
                .map_err(|_| storage_err("feed_lens_cache mutex poisoned"))?;
            if let Some(hit) = cache.get(feed_id) {
                return Ok(hit.clone());
            }
        }
        let store = self
            .store
            .lock()
            .map_err(|_| storage_err("store mutex poisoned"))?;
        let resolved = store
            .find_lens_for_feed(feed_id)
            .map_err(|e| storage_err(format!("find_lens_for_feed({feed_id}): {e}")))?;
        drop(store);
        let mut cache = self
            .feed_lens_cache
            .lock()
            .map_err(|_| storage_err("feed_lens_cache mutex poisoned"))?;
        cache.insert(feed_id.to_string(), resolved.clone());
        Ok(resolved)
    }
}

/// Linear issues on the user's plate at `end`: open (state type not
/// completed / canceled), assigned to the user, and urgent / high
/// priority or due within [`LINEAR_DUE_DAYS`] (overdue included).
/// Standing signals, not windowed by `start`: an urgent issue matters
/// whether or not it changed today. Due date first, then priority.
const LINEAR_ATTENTION_SQL: &str = "SELECT id, source_id, source_ts, title, body_text, \
         'linear' AS kind, feed_id, \
         json_extract(metadata, '$.priority_label'), json_extract(metadata, '$.due_date') \
       FROM linear_issues \
      WHERE source_ts < ?1 \
        AND json_extract(metadata, '$.assigned_to_me') = 1 \
        AND COALESCE(json_extract(metadata, '$.state_type'), '') \
            NOT IN ('completed', 'canceled') \
        AND (json_extract(metadata, '$.priority') IN (1, 2) \
             OR json_extract(metadata, '$.due_date') <= ?2) \
      ORDER BY json_extract(metadata, '$.due_date') IS NULL, \
               json_extract(metadata, '$.due_date'), \
               json_extract(metadata, '$.priority'), source_ts DESC \
      LIMIT ?3";

/// Open doc comment threads for the user (COLLIERY-I-0533), newest
/// first. The feed keeps only threads that are open and for the user, so
/// every row is a standing signal, not windowed by `start`. Columns 7
/// and 8 carry the reason and the link.
const DOC_COMMENT_ATTENTION_SQL: &str = "SELECT id, source_id, source_ts, title, body_text, \
         'drive_comment' AS kind, feed_id, \
         json_extract(metadata, '$.reason'), json_extract(metadata, '$.link') \
       FROM drive_comments WHERE source_ts < ?1 \
      ORDER BY source_ts DESC LIMIT ?2";

/// Jira issues on the user's plate at `end`, in the same shape as Linear:
/// from the assignee feed, status category not done, and high priority
/// or due within [`LINEAR_DUE_DAYS`] (overdue included). A row written
/// before these fields were kept has none of them: not urgent, no due
/// date, so it stays out. Due date first, then priority.
const JIRA_ATTENTION_SQL: &str = "SELECT id, source_id, source_ts, title, body_text, \
         'jira' AS kind, feed_id, \
         json_extract(metadata, '$.priority'), json_extract(metadata, '$.due_date') \
       FROM jira_issues \
      WHERE source_ts < ?1 \
        AND json_extract(metadata, '$.assigned_to_me') = 1 \
        AND COALESCE(json_extract(metadata, '$.status_category'), '') != 'done' \
        AND (json_extract(metadata, '$.high_priority') = 1 \
             OR json_extract(metadata, '$.due_date') <= ?2) \
      ORDER BY json_extract(metadata, '$.due_date') IS NULL, \
               json_extract(metadata, '$.due_date'), \
               json_extract(metadata, '$.high_priority') IS NOT 1, source_ts DESC \
      LIMIT ?3";

/// Sender of Google's comment notification emails.
const DOCS_NOTIFY_SENDER: &str = "comments-noreply@docs.google.com";

/// Gmail and Slack signals inside `[start, end)`, newest first. Once the
/// `drive_comments` feed has rows, Google's comment notification emails
/// are left out: the doc comment row says the same and knows whether the
/// thread is resolved. Before that, they stay, so nothing is lost. A
/// Slack row's note is its workspace (`slack@acme`, COLLIERY-I-0520).
const WINDOW_ATTENTION_SQL: &str = "SELECT id, source_id, source_ts, title, body_text, kind, feed_id, \
         note, NULL FROM ( \
           SELECT id, source_id, source_ts, title, body_text, 'gmail' AS kind, feed_id, \
                  NULL AS note \
             FROM gmail_messages WHERE source_ts >= ?1 AND source_ts < ?2 \
              AND NOT (instr(lower(COALESCE(json_extract(metadata, '$.sender'), '')), ?4) > 0 \
                       AND EXISTS (SELECT 1 FROM drive_comments)) \
           UNION ALL \
           SELECT id, source_id, source_ts, title, body_text, 'slack' AS kind, feed_id, \
                  'slack@' || workspace AS note \
             FROM slack_messages WHERE source_ts >= ?1 AND source_ts < ?2 \
       ) ORDER BY source_ts DESC LIMIT ?3";

/// How far ahead a Linear or Jira due date pulls an issue into the brief.
const LINEAR_DUE_DAYS: i64 = 7;

struct RawSignal {
    id: String,
    source_id: String,
    ts: DateTime<Utc>,
    summary: String,
    kind: String,
    feed_id: String,
}

/// Run one attention query (9 columns: the 7 signal columns, then two
/// optional note columns for the summary: priority label and due date
/// for Linear and Jira, reason and link for doc comments, the workspace for
/// Slack).
fn query_signals(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<RawSignal>, CeremonyError> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| storage_err(format!("prepare attention between: {e}")))?;
    let mut rows = stmt
        .query(params)
        .map_err(|e| storage_err(format!("query attention between: {e}")))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| storage_err(format!("attention between row: {e}")))?
    {
        let col = |i: usize, name: &str| -> Result<String, CeremonyError> {
            row.get(i)
                .map_err(|e| storage_err(format!("col {name}: {e}")))
        };
        let title = col(3, "title")?;
        let body_text = col(4, "body_text")?;
        let label: Option<String> = row
            .get(7)
            .map_err(|e| storage_err(format!("col priority_label: {e}")))?;
        let due: Option<String> = row
            .get(8)
            .map_err(|e| storage_err(format!("col due_date: {e}")))?;
        let kind = col(5, "kind")?;
        let (mut summary, note) = if kind == "drive_comment" {
            // "Comment on Q3 plan: Robin: Can you … (mentioned, <link>)";
            // the label column is the reason, the due column the link.
            let summary = truncate_excerpt(&format!("Comment on {title}: {body_text}"));
            let note: Vec<String> = label
                .map(|r| r.replace('_', " "))
                .into_iter()
                .chain(due)
                .collect();
            (summary, note)
        } else {
            let summary = if !title.is_empty() {
                truncate_excerpt(&title)
            } else {
                truncate_excerpt(&body_text)
            };
            // "(Urgent, due 2026-10-09)": the word "due" also makes the
            // weekly plugin read it as a deadline.
            let note: Vec<String> = label
                .filter(|l| !l.is_empty() && l != "No priority")
                .into_iter()
                .chain(due.map(|d| format!("due {d}")))
                .collect();
            (summary, note)
        };
        if !note.is_empty() {
            summary.push_str(&format!(" ({})", note.join(", ")));
        }
        out.push(RawSignal {
            id: col(0, "id")?,
            source_id: col(1, "source_id")?,
            ts: parse_rfc3339(&col(2, "source_ts")?)?,
            summary,
            kind,
            feed_id: col(6, "feed_id")?,
        });
    }
    Ok(out)
}

#[async_trait]
impl AttentionSource for ProjectionsAttentionSource {
    /// Linear issues on the user's plate come first (they are few and
    /// must not be crowded out), then open doc comments for the user,
    /// then Jira issues on the user's plate, then Gmail and Slack signals
    /// of the window fill the rest of `cap`.
    async fn between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        cap: usize,
    ) -> Result<Vec<SignalRow>, CeremonyError> {
        if cap == 0 || end <= start {
            return Ok(Vec::new());
        }
        for feed_type in [
            "gmail_messages",
            "slack_messages",
            "linear_issues",
            "drive_comments",
            "jira_issues",
        ] {
            self.projections
                .ensure_feed_type(feed_type)
                .map_err(|e| storage_err(format!("ensure {feed_type}: {e}")))?;
        }

        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();
        let due_by = (end + chrono::Duration::days(LINEAR_DUE_DAYS))
            .date_naive()
            .to_string();

        let raws = {
            let conn = self
                .projections
                .conn()
                .lock()
                .map_err(|_| storage_err("projection connection mutex poisoned"))?;
            let mut raws = query_signals(
                &conn,
                LINEAR_ATTENTION_SQL,
                params![end_str, due_by, cap as i64],
            )?;
            let rest = cap - raws.len();
            if rest > 0 {
                raws.extend(query_signals(
                    &conn,
                    DOC_COMMENT_ATTENTION_SQL,
                    params![end_str, rest as i64],
                )?);
            }
            let rest = cap - raws.len();
            if rest > 0 {
                raws.extend(query_signals(
                    &conn,
                    JIRA_ATTENTION_SQL,
                    params![end_str, due_by, rest as i64],
                )?);
            }
            let rest = cap - raws.len();
            if rest > 0 {
                raws.extend(query_signals(
                    &conn,
                    WINDOW_ATTENTION_SQL,
                    params![start_str, end_str, rest as i64, DOCS_NOTIFY_SENDER],
                )?);
            }
            raws
        };

        let mut out = Vec::with_capacity(raws.len());
        for r in raws {
            let lens = self.lens_for_feed(&r.feed_id)?;
            out.push(SignalRow {
                id: r.id,
                source_kind: r.kind,
                source_id: r.source_id,
                ts: r.ts,
                summary: r.summary,
                lens,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arawn_projections::atlassian::JiraIssueProjection;
    use arawn_projections::calendar::CalendarEventProjection;
    use arawn_projections::drive_comments::DriveCommentProjection;
    use arawn_projections::gmail::GmailMessageProjection;
    use arawn_projections::linear::LinearIssueProjection;
    use arawn_projections::slack::SlackMessageProjection;
    use chrono::{Duration, TimeZone, Utc};
    use tempfile::tempdir;

    fn make_store_pair() -> (Arc<ProjectionStore>, Arc<Mutex<Store>>, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let proj_path = tmp.path().join("projections.db");
        let projections = Arc::new(ProjectionStore::open(&proj_path).unwrap());
        // The attention source carries a Store for future lens
        // joins. We don't exercise it in v1 tests, but the type has to
        // be valid. Use a fresh data root rooted in the same tmp dir.
        let store = Arc::new(Mutex::new(Store::open(tmp.path().join("data")).unwrap()));
        (projections, store, tmp)
    }

    fn cal_event(
        id_seed: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CalendarEventProjection {
        CalendarEventProjection {
            id: format!("ce-{id_seed}"),
            feed_id: "cal-feed".into(),
            source_id: id_seed.into(),
            source_ts: start,
            calendar_id: None,
            summary: format!("Event {id_seed}"),
            description: format!("Body for {id_seed}"),
            location: None,
            start_ts: start,
            end_ts: Some(end),
            all_day: false,
            organizer: Some("alice@example.com".into()),
            attendees: vec!["bob@example.com".into(), "carol@example.com".into()],
            status: Some("confirmed".into()),
            recurring_event_id: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn calendar_returns_events_for_day() {
        let (projections, _store, _tmp) = make_store_pair();

        // Today (2026-05-15 UTC). Two events today, one yesterday.
        let today = NaiveDate::from_ymd_opt(2026, 5, 15).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 5, 14).unwrap();

        let make_dt = |d: NaiveDate, h: u32| -> DateTime<Utc> {
            Utc.from_utc_datetime(&d.and_hms_opt(h, 0, 0).unwrap())
        };

        let ev_y = cal_event("y1", make_dt(yesterday, 10), make_dt(yesterday, 11));
        let ev_t1 = cal_event("t1", make_dt(today, 9), make_dt(today, 10));
        let ev_t2 = cal_event("t2", make_dt(today, 14), make_dt(today, 15));
        projections
            .write_batch(&[ev_y, ev_t1.clone(), ev_t2.clone()])
            .unwrap();

        let src = ProjectionsCalendarSource::new(projections);
        let out = src.events_for(today).await.unwrap();
        assert_eq!(out.len(), 2, "expected 2 today events, got {:?}", out);
        let ids: Vec<&str> = out.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&ev_t1.id.as_str()));
        assert!(ids.contains(&ev_t2.id.as_str()));
        // Ends parse out of metadata.
        assert!(out.iter().all(|e| e.end > e.start));
        // Attendees flowed through.
        assert!(out.iter().all(|e| e.attendees.len() == 2));
    }

    fn gmail_signal(id_seed: &str, ts: DateTime<Utc>) -> GmailMessageProjection {
        GmailMessageProjection {
            id: format!("gm-{id_seed}"),
            feed_id: "gmail-feed".into(),
            source_id: id_seed.into(),
            source_ts: ts,
            sender: Some("alice@example.com".into()),
            recipients: vec!["me@example.com".into()],
            subject: format!("Subject {id_seed}"),
            body_text: format!("Body for {id_seed}"),
            thread_id: None,
            labels: vec![],
        }
    }

    fn slack_signal(id_seed: &str, ts: DateTime<Utc>) -> SlackMessageProjection {
        SlackMessageProjection {
            id: format!("sm-{id_seed}"),
            feed_id: "slack-feed".into(),
            source_id: id_seed.into(),
            source_ts: ts,
            channel_id: Some("C123".into()),
            sender_id: Some("U456".into()),
            text: format!("hello {id_seed}"),
            thread_ts: None,
            reactions: vec![],
            is_thread_reply: false,
            workspace: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn attention_returns_recent_capped() {
        let (projections, store, _tmp) = make_store_pair();

        let now = Utc::now();
        let yesterday = now - Duration::hours(25);
        let an_hour_ago = now - Duration::minutes(30);
        let two_min_ago = now - Duration::minutes(2);

        // 3 gmail rows across the time window.
        let g1 = gmail_signal("g-old", yesterday);
        let g2 = gmail_signal("g-mid", an_hour_ago);
        let g3 = gmail_signal("g-new", two_min_ago);
        projections
            .write_batch(&[g1, g2.clone(), g3.clone()])
            .unwrap();

        // 2 slack rows.
        let s1 = slack_signal("s-old", yesterday);
        let s2 = slack_signal("s-new", two_min_ago - Duration::seconds(30));
        projections.write_batch(&[s1, s2.clone()]).unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let cursor = now - Duration::hours(1);
        let future = now + Duration::days(1);
        let out = src.between(cursor, future, 10).await.unwrap();

        // Only rows strictly newer than `cursor`: g2 (an_hour_ago is
        // ~30m old in this fixture, so > 1h-ago cursor), g3, s2 → 3.
        assert_eq!(out.len(), 3, "got {out:?}");
        // DESC order by ts.
        for w in out.windows(2) {
            assert!(w[0].ts >= w[1].ts, "not DESC: {out:?}");
        }
        // Cap is respected.
        let capped = src.between(cursor, future, 2).await.unwrap();
        assert_eq!(capped.len(), 2);
        // source_kind values are tagged correctly.
        let kinds: Vec<&str> = out.iter().map(|s| s.source_kind.as_str()).collect();
        assert!(kinds.contains(&"gmail"));
        assert!(kinds.contains(&"slack"));
        // A Slack row with no workspace yet has no note.
        let slack = out.iter().find(|s| s.source_kind == "slack").unwrap();
        assert!(!slack.summary.contains("slack@"), "{}", slack.summary);
        // Stable ids flow through.
        let ids: Vec<&str> = out.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&g3.id.as_str()));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_slack_signal_names_its_workspace() {
        let (projections, store, _tmp) = make_store_pair();
        let now = Utc::now();
        let s = slack_signal("s-ws", now - Duration::minutes(5));
        projections.write_batch(&[s]).unwrap();
        arawn_projections::slack::set_workspace(&projections, "slack-feed", "acme").unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let out = src
            .between(now - Duration::hours(1), now + Duration::hours(1), 10)
            .await
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source_kind, "slack");
        assert!(
            out[0].summary.ends_with("(slack@acme)"),
            "{}",
            out[0].summary
        );
    }

    fn linear_issue(
        n: u32,
        updated: DateTime<Utc>,
        priority: u8,
        due: Option<&str>,
        state_type: &str,
        mine: bool,
    ) -> LinearIssueProjection {
        LinearIssueProjection {
            id: format!("li-{n}"),
            feed_id: "linear-mine".into(),
            source_id: format!("uuid-{n}"),
            source_ts: updated,
            identifier: format!("ENG-{n}"),
            title: format!("Issue {n}"),
            description: String::new(),
            url: None,
            state: Some("Todo".into()),
            state_type: Some(state_type.into()),
            priority,
            priority_label: Some(
                ["No priority", "Urgent", "High", "Medium", "Low"][priority as usize].into(),
            ),
            due_date: due.map(String::from),
            team_key: Some("ENG".into()),
            labels: vec![],
            assignee: Some("Me".into()),
            creator: None,
            assigned_to_me: mine,
            comments: vec![],
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn attention_brings_my_urgent_or_due_soon_open_linear_issues() {
        let (projections, store, _tmp) = make_store_pair();
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        // Last changed weeks ago: still on the plate.
        let old = end - Duration::days(20);
        projections
            .write_batch(&[
                linear_issue(1, old, 1, None, "started", true), // urgent: in
                linear_issue(2, old, 3, Some("2026-10-10"), "unstarted", true), // due in 3d: in
                linear_issue(3, old, 1, None, "completed", true), // done: out
                linear_issue(4, old, 2, None, "canceled", true), // canceled: out
                linear_issue(5, old, 3, Some("2026-10-30"), "started", true), // not due soon: out
                linear_issue(6, old, 1, None, "started", false), // not mine: out
                linear_issue(7, old, 4, Some("2026-10-01"), "started", true), // overdue: in
            ])
            .unwrap();
        projections
            .write_batch(&[gmail_signal("g", end - Duration::hours(1))])
            .unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let out = src.between(start, end, 10).await.unwrap();
        let linear: Vec<&str> = out
            .iter()
            .filter(|r| r.source_kind == "linear")
            .map(|r| r.source_id.as_str())
            .collect();
        // Due date first (overdue, then soon), then priority.
        assert_eq!(linear, ["uuid-7", "uuid-2", "uuid-1"], "{out:?}");
        let due_soon = out.iter().find(|r| r.source_id == "uuid-2").unwrap();
        assert_eq!(due_soon.summary, "ENG-2: Issue 2 (Medium, due 2026-10-10)");
        assert!(out.iter().any(|r| r.source_kind == "gmail"));

        // Linear is not crowded out by the window rows, and the cap holds.
        let capped = src.between(start, end, 2).await.unwrap();
        assert_eq!(capped.len(), 2);
        assert!(capped.iter().all(|r| r.source_kind == "linear"));
    }

    /// End to end from the feed's snapshots: an issue reassigned away
    /// (the feed rewrites it with `assigned_to_me: false`) leaves the
    /// brief, and so does one whose snapshot the feed removed.
    #[tokio::test(flavor = "current_thread")]
    async fn a_reassigned_or_gone_linear_issue_leaves_the_brief() {
        let (projections, store, tmp) = make_store_pair();
        let feed_dir = tmp.path().join("linear-mine");
        let write = |id: &str, mine: bool| {
            let dir = feed_dir.join(id);
            std::fs::create_dir_all(&dir).unwrap();
            let snap = serde_json::json!({
                "id": id, "identifier": format!("ENG-{id}"), "title": "Urgent thing",
                "state": {"name": "Todo", "type": "unstarted"},
                "priority": 1, "priority_label": "Urgent",
                "assigned_to_me": mine,
                "updated_at": "2026-10-01T00:00:00.000Z",
            });
            std::fs::write(dir.join("issue.json"), snap.to_string()).unwrap();
        };
        let project = || {
            arawn_projections::project_feed_dir(
                &projections,
                "linear/my-issues",
                "linear-mine",
                &feed_dir,
            )
            .unwrap();
        };
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let src = ProjectionsAttentionSource::new(Arc::clone(&projections), store);
        let linear_ids = |out: Vec<SignalRow>| -> Vec<String> {
            out.into_iter()
                .filter(|r| r.source_kind == "linear")
                .map(|r| r.source_id)
                .collect()
        };

        write("a", true);
        write("b", true);
        project();
        let start = end - Duration::hours(24);
        assert_eq!(
            linear_ids(src.between(start, end, 10).await.unwrap()),
            ["a", "b"]
        );

        write("a", false); // reassigned away
        std::fs::remove_dir_all(feed_dir.join("b")).unwrap(); // gone
        project();
        assert!(linear_ids(src.between(start, end, 10).await.unwrap()).is_empty());
    }

    fn doc_comment(n: u32, modified: DateTime<Utc>) -> DriveCommentProjection {
        DriveCommentProjection {
            id: format!("dc-{n}"),
            feed_id: "drive-comments".into(),
            source_id: format!("F{n}/C{n}"),
            source_ts: modified,
            file_id: format!("F{n}"),
            comment_id: format!("C{n}"),
            doc_title: "Q3 plan".into(),
            link: Some(format!(
                "https://docs.google.com/document/d/F{n}/edit?disco=C{n}"
            )),
            reason: "owner_unanswered".into(),
            author: Some("Robin".into()),
            content: "Can you confirm the budget?".into(),
            quoted: None,
            reply_count: 0,
            last_reply_author: None,
            last_reply: None,
        }
    }

    fn docs_notification(ts: DateTime<Utc>) -> GmailMessageProjection {
        GmailMessageProjection {
            sender: Some("\"Robin (Google Docs)\" <Comments-NoReply@docs.google.com>".into()),
            subject: "Q3 plan - Robin mentioned you".into(),
            ..gmail_signal("docs-mail", ts)
        }
    }

    /// An open comment for me is a standing signal: in even when it last
    /// changed before the window.
    #[tokio::test(flavor = "current_thread")]
    async fn attention_brings_open_doc_comments_for_me() {
        let (projections, store, _tmp) = make_store_pair();
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        projections
            .write_batch(&[doc_comment(1, end - Duration::days(3))])
            .unwrap();
        projections
            .write_batch(&[gmail_signal("g", end - Duration::hours(1))])
            .unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let out = src.between(start, end, 10).await.unwrap();
        let c = out
            .iter()
            .find(|r| r.source_kind == "drive_comment")
            .expect("doc comment in the brief");
        assert_eq!(c.source_id, "F1/C1");
        assert_eq!(
            c.summary,
            "Comment on Q3 plan: Robin: Can you confirm the budget? \
             (owner unanswered, https://docs.google.com/document/d/F1/edit?disco=C1)"
        );
        assert!(out.iter().any(|r| r.source_kind == "gmail"));
    }

    /// End to end from the feed's snapshots: when the feed removes a
    /// thread's snapshot (resolved, or its doc gone), it leaves the brief.
    #[tokio::test(flavor = "current_thread")]
    async fn a_removed_doc_comment_snapshot_leaves_the_brief() {
        let (projections, store, tmp) = make_store_pair();
        let feed_dir = tmp.path().join("drive-comments");
        let dir = feed_dir.join("F1");
        std::fs::create_dir_all(&dir).unwrap();
        let snap = serde_json::json!({
            "file_id": "F1", "comment_id": "C1", "doc_title": "Q3 plan",
            "reason": "mentioned", "author": "Robin", "content": "Check this",
            "modified_time": "2026-09-01T00:00:00+00:00",
        });
        std::fs::write(dir.join("C1.json"), snap.to_string()).unwrap();
        let project = || {
            arawn_projections::project_feed_dir(
                &projections,
                "drive/comments",
                "drive-comments",
                &feed_dir,
            )
            .unwrap();
        };
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        let src = ProjectionsAttentionSource::new(Arc::clone(&projections), store);
        let comments = |out: Vec<SignalRow>| {
            out.into_iter()
                .filter(|r| r.source_kind == "drive_comment")
                .count()
        };

        project();
        assert_eq!(comments(src.between(start, end, 10).await.unwrap()), 1);

        std::fs::remove_dir_all(&dir).unwrap();
        project();
        assert_eq!(comments(src.between(start, end, 10).await.unwrap()), 0);
    }

    /// Google's comment notification email is left out once the comments
    /// feed has rows, and kept before that.
    #[tokio::test(flavor = "current_thread")]
    async fn docs_notification_email_is_dropped_only_once_comments_flow() {
        let (projections, store, _tmp) = make_store_pair();
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        projections
            .write_batch(&[
                docs_notification(end - Duration::hours(2)),
                gmail_signal("other", end - Duration::hours(1)),
            ])
            .unwrap();
        let src = ProjectionsAttentionSource::new(Arc::clone(&projections), store);
        let gmail_ids = |out: Vec<SignalRow>| -> Vec<String> {
            out.into_iter()
                .filter(|r| r.source_kind == "gmail")
                .map(|r| r.source_id)
                .collect()
        };

        // No drive_comments rows yet: the email is the only signal.
        assert_eq!(
            gmail_ids(src.between(start, end, 10).await.unwrap()),
            ["other", "docs-mail"]
        );

        projections
            .write_batch(&[doc_comment(1, end - Duration::hours(2))])
            .unwrap();
        assert_eq!(
            gmail_ids(src.between(start, end, 10).await.unwrap()),
            ["other"]
        );
    }

    fn jira_issue(
        n: u32,
        updated: DateTime<Utc>,
        priority: &str,
        due: Option<&str>,
        category: Option<&str>,
        mine: bool,
    ) -> JiraIssueProjection {
        JiraIssueProjection {
            id: format!("ji-{n}"),
            feed_id: "jira-mine".into(),
            source_id: format!("OPS-{n}"),
            source_ts: updated,
            project_key: Some("OPS".into()),
            summary: format!("Issue {n}"),
            status: Some("In Progress".into()),
            assignee: Some("Me".into()),
            reporter: None,
            priority: Some(priority.into()),
            high_priority: matches!(priority, "Highest" | "High"),
            due_date: due.map(String::from),
            status_category: category.map(String::from),
            assigned_to_me: mine,
            labels: vec![],
            body_text: String::new(),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn attention_brings_my_high_or_due_soon_open_jira_issues() {
        let (projections, store, _tmp) = make_store_pair();
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        let old = end - Duration::days(20);
        let open = Some("indeterminate");
        projections
            .write_batch(&[
                jira_issue(1, old, "High", None, open, true), // high: in
                jira_issue(2, old, "Medium", Some("2026-10-10"), Some("new"), true), // due in 3d: in
                jira_issue(3, old, "Highest", None, Some("done"), true),             // done: out
                jira_issue(4, old, "Medium", Some("2026-10-30"), open, true), // not due soon: out
                jira_issue(5, old, "High", None, open, false),                // not mine: out
                jira_issue(6, old, "Low", Some("2026-10-01"), open, true),    // overdue: in
            ])
            .unwrap();
        // A row written before priority, due date and category were kept.
        let mut legacy = jira_issue(7, old, "Medium", None, None, true);
        legacy.priority = None;
        projections.write_batch(&[legacy]).unwrap();
        projections
            .write_batch(&[linear_issue(1, old, 1, None, "started", true)])
            .unwrap();
        projections
            .write_batch(&[doc_comment(1, end - Duration::days(3))])
            .unwrap();
        projections
            .write_batch(&[gmail_signal("g", end - Duration::hours(1))])
            .unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let out = src.between(start, end, 10).await.unwrap();
        let jira: Vec<&str> = out
            .iter()
            .filter(|r| r.source_kind == "jira")
            .map(|r| r.source_id.as_str())
            .collect();
        // Due date first (overdue, then soon), then priority.
        assert_eq!(jira, ["OPS-6", "OPS-2", "OPS-1"], "{out:?}");
        let due_soon = out.iter().find(|r| r.source_id == "OPS-2").unwrap();
        assert_eq!(due_soon.summary, "OPS-2: Issue 2 (Medium, due 2026-10-10)");

        // Linear, then doc comments, then Jira, then the window rows.
        let kinds: Vec<&str> = out.iter().map(|r| r.source_kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["linear", "drive_comment", "jira", "jira", "jira", "gmail"]
        );
    }

    /// End to end from the feed's snapshots: only the assignee feed's
    /// issues are mine, and an issue that moves to Done leaves the brief
    /// on the next run (issue rows are upserted, not only inserted).
    #[tokio::test(flavor = "current_thread")]
    async fn a_jira_issue_moved_to_done_leaves_the_brief() {
        let (projections, store, tmp) = make_store_pair();
        let write = |feed_dir: &std::path::Path, key: &str, category: &str| {
            let dir = feed_dir.join(key);
            std::fs::create_dir_all(&dir).unwrap();
            let snap = serde_json::json!({
                "key": key,
                "fields": {
                    "summary": "Urgent thing",
                    "status": {"name": "Doing", "statusCategory": {"key": category}},
                    "priority": {"id": "2", "name": "High"},
                    "updated": "2026-10-01T00:00:00.000Z",
                },
            });
            std::fs::write(dir.join("issue.json"), snap.to_string()).unwrap();
        };
        let mine_dir = tmp.path().join("jira-mine");
        let project_dir = tmp.path().join("jira-ops");
        let project = || {
            arawn_projections::project_feed_dir(
                &projections,
                "jira/assignee-tracker",
                "jira-mine",
                &mine_dir,
            )
            .unwrap();
            arawn_projections::project_feed_dir(
                &projections,
                "jira/project-tracker",
                "jira-ops",
                &project_dir,
            )
            .unwrap();
        };
        let end = DateTime::parse_from_rfc3339("2026-10-07T07:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        let src = ProjectionsAttentionSource::new(Arc::clone(&projections), store);
        let jira_ids = |out: Vec<SignalRow>| -> Vec<String> {
            out.into_iter()
                .filter(|r| r.source_kind == "jira")
                .map(|r| r.source_id)
                .collect()
        };

        write(&mine_dir, "OPS-1", "indeterminate");
        write(&project_dir, "OPS-2", "indeterminate"); // someone else's
        project();
        assert_eq!(
            jira_ids(src.between(start, end, 10).await.unwrap()),
            ["OPS-1"]
        );

        write(&mine_dir, "OPS-1", "done");
        project();
        assert!(jira_ids(src.between(start, end, 10).await.unwrap()).is_empty());
    }

    /// 2026-05-17T01:30:00Z is "May 17" in UTC but "May 16 18:30" in
    /// US/Pacific (PDT, UTC-7). `events_for(2026-05-16)` should pick
    /// up the event under US/Pacific and miss it under UTC.
    #[tokio::test(flavor = "current_thread")]
    async fn calendar_respects_configured_timezone() {
        let (projections, _store, _tmp) = make_store_pair();

        let start_utc = DateTime::parse_from_rfc3339("2026-05-17T01:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end_utc = start_utc + Duration::hours(1);
        let ev = cal_event("tz1", start_utc, end_utc);
        projections.write_batch(&[ev.clone()]).unwrap();

        let may16 = NaiveDate::from_ymd_opt(2026, 5, 16).unwrap();

        // UTC bracket of May 16 is [00:00Z, 23:59:59Z] — misses 01:30Z May 17.
        let utc_src = ProjectionsCalendarSource::new(Arc::clone(&projections));
        let utc_hits = utc_src.events_for(may16).await.unwrap();
        assert!(
            utc_hits.iter().all(|e| e.id != ev.id),
            "expected UTC May 16 to miss the May-17-UTC event; got {utc_hits:?}"
        );

        // US/Pacific bracket of May 16 covers up to ~07:00Z May 17.
        let pst_src = ProjectionsCalendarSource::new(Arc::clone(&projections))
            .with_tz(chrono_tz::US::Pacific);
        let pst_hits = pst_src.events_for(may16).await.unwrap();
        let pst_ids: Vec<&str> = pst_hits.iter().map(|e| e.id.as_str()).collect();
        assert!(
            pst_ids.contains(&ev.id.as_str()),
            "expected US/Pacific May 16 to include the May-17-UTC event; got {pst_hits:?}"
        );
    }

    /// Rows whose `feed_id` is bound to a lens tag with that
    /// lens's name; rows with an unregistered `feed_id` stay
    /// `None`.
    #[tokio::test(flavor = "current_thread")]
    async fn attention_tags_lens_from_feed_registry() {
        let (projections, store, tmp) = make_store_pair();

        // Register a lens that owns "gmail-feed" but not
        // "slack-feed".
        {
            let s = store.lock().unwrap();
            let ws_root = tmp.path().join("data/lenses/work");
            std::fs::create_dir_all(&ws_root).unwrap();
            let ws = arawn_core::Lens::new("work", &ws_root);
            s.create_lens(&ws).unwrap();
            s.add_lens_binding("work", "gmail-feed").unwrap();
        }

        let now = Utc::now();
        // Bound feed: gmail (feed_id = "gmail-feed" per gmail_signal).
        let g = gmail_signal("g-bound", now - Duration::minutes(5));
        // Unbound feed: slack (feed_id = "slack-feed", not registered).
        let s = slack_signal("s-unbound", now - Duration::minutes(10));
        projections.write_batch(&[g.clone()]).unwrap();
        projections.write_batch(&[s.clone()]).unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let cursor = now - Duration::hours(1);
        let future = now + Duration::days(1);
        let out = src.between(cursor, future, 10).await.unwrap();
        let g_row = out
            .iter()
            .find(|r| r.id == g.id)
            .expect("gmail row present");
        assert_eq!(
            g_row.lens.as_deref(),
            Some("work"),
            "expected gmail-feed → work tag; got {g_row:?}"
        );
        let s_row = out
            .iter()
            .find(|r| r.id == s.id)
            .expect("slack row present");
        assert!(
            s_row.lens.is_none(),
            "expected unbound slack-feed → None; got {s_row:?}"
        );
    }
}

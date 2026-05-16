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
//! Workstream-tagging for attention signals is left as `None` for
//! v1: the projection tables don't carry a workstream column, and
//! the central feeds registry that owns the mapping lives in
//! `arawn-storage`. The trait DTO field exists so we can light it up
//! later without churning the plugin contract.

use std::sync::{Arc, Mutex};

use arawn_ceremonies::plugins::{
    AttentionSource, CalEvent, CalendarSource, SignalRow,
};
use arawn_ceremonies::CeremonyError;
use arawn_projections::ProjectionStore;
use arawn_storage::Store;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
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
pub struct ProjectionsCalendarSource {
    projections: Arc<ProjectionStore>,
}

impl ProjectionsCalendarSource {
    pub fn new(projections: Arc<ProjectionStore>) -> Self {
        Self { projections }
    }
}

#[async_trait]
impl CalendarSource for ProjectionsCalendarSource {
    async fn events_for(&self, date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError> {
        // Day window: [00:00:00Z, 23:59:59.999Z] of `date` (UTC).
        // The trait docs note that the contract is "events whose
        // local-time start falls on `date`"; v1 we treat date as UTC
        // since the projection only stores UTC source_ts. Timezone
        // handling is a follow-up — flagged on the task.
        let start = Utc
            .from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .to_rfc3339();
        let end = Utc
            .from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
            .to_rfc3339();

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

/// Production `AttentionSource` backed by `gmail_messages` +
/// `slack_messages` projection tables.
///
/// `store` is held for future workstream-tag joins (currently
/// unused — see module docs on the v1 limitation).
pub struct ProjectionsAttentionSource {
    projections: Arc<ProjectionStore>,
    #[allow(dead_code)]
    store: Arc<Mutex<Store>>,
}

impl ProjectionsAttentionSource {
    pub fn new(projections: Arc<ProjectionStore>, store: Arc<Mutex<Store>>) -> Self {
        Self { projections, store }
    }
}

#[async_trait]
impl AttentionSource for ProjectionsAttentionSource {
    async fn since(
        &self,
        cursor: DateTime<Utc>,
        cap: usize,
    ) -> Result<Vec<SignalRow>, CeremonyError> {
        if cap == 0 {
            return Ok(Vec::new());
        }
        // Make sure both tables exist before we UNION over them.
        self.projections
            .ensure_feed_type("gmail_messages")
            .map_err(|e| storage_err(format!("ensure gmail_messages: {e}")))?;
        self.projections
            .ensure_feed_type("slack_messages")
            .map_err(|e| storage_err(format!("ensure slack_messages: {e}")))?;

        let cursor_str = cursor.to_rfc3339();

        let conn = self
            .projections
            .conn()
            .lock()
            .map_err(|_| storage_err("projection connection mutex poisoned"))?;

        let sql = "SELECT id, source_id, source_ts, title, body_text, kind FROM ( \
                       SELECT id, source_id, source_ts, title, body_text, 'gmail' AS kind \
                         FROM gmail_messages WHERE source_ts > ?1 \
                       UNION ALL \
                       SELECT id, source_id, source_ts, title, body_text, 'slack' AS kind \
                         FROM slack_messages WHERE source_ts > ?1 \
                   ) ORDER BY source_ts DESC LIMIT ?2";

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| storage_err(format!("prepare attention query: {e}")))?;

        let mut rows = stmt
            .query(params![cursor_str, cap as i64])
            .map_err(|e| storage_err(format!("query attention: {e}")))?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| storage_err(format!("attention row: {e}")))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| storage_err(format!("col id: {e}")))?;
            let source_id: String = row
                .get(1)
                .map_err(|e| storage_err(format!("col source_id: {e}")))?;
            let source_ts_str: String = row
                .get(2)
                .map_err(|e| storage_err(format!("col source_ts: {e}")))?;
            let title: String = row
                .get(3)
                .map_err(|e| storage_err(format!("col title: {e}")))?;
            let body_text: String = row
                .get(4)
                .map_err(|e| storage_err(format!("col body_text: {e}")))?;
            let kind: String = row
                .get(5)
                .map_err(|e| storage_err(format!("col kind: {e}")))?;

            let ts = parse_rfc3339(&source_ts_str)?;
            let summary = if !title.is_empty() {
                truncate_excerpt(&title)
            } else {
                truncate_excerpt(&body_text)
            };
            out.push(SignalRow {
                id,
                source_kind: kind,
                source_id,
                ts,
                summary,
                // v1: no workstream tagging — see module docs.
                workstream: None,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use arawn_projections::calendar::CalendarEventProjection;
    use arawn_projections::gmail::GmailMessageProjection;
    use arawn_projections::slack::SlackMessageProjection;
    use chrono::{Duration, TimeZone, Utc};
    use tempfile::tempdir;

    fn make_store_pair() -> (Arc<ProjectionStore>, Arc<Mutex<Store>>, tempfile::TempDir) {
        let tmp = tempdir().unwrap();
        let proj_path = tmp.path().join("projections.db");
        let projections = Arc::new(ProjectionStore::open(&proj_path).unwrap());
        // The attention source carries a Store for future workstream
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
        projections.write_batch(&[g1, g2.clone(), g3.clone()]).unwrap();

        // 2 slack rows.
        let s1 = slack_signal("s-old", yesterday);
        let s2 = slack_signal("s-new", two_min_ago - Duration::seconds(30));
        projections.write_batch(&[s1, s2.clone()]).unwrap();

        let src = ProjectionsAttentionSource::new(projections, store);
        let cursor = now - Duration::hours(1);
        let out = src.since(cursor, 10).await.unwrap();

        // Only rows strictly newer than `cursor`: g2 (an_hour_ago is
        // ~30m old in this fixture, so > 1h-ago cursor), g3, s2 → 3.
        assert_eq!(out.len(), 3, "got {out:?}");
        // DESC order by ts.
        for w in out.windows(2) {
            assert!(w[0].ts >= w[1].ts, "not DESC: {out:?}");
        }
        // Cap is respected.
        let capped = src.since(cursor, 2).await.unwrap();
        assert_eq!(capped.len(), 2);
        // source_kind values are tagged correctly.
        let kinds: Vec<&str> = out.iter().map(|s| s.source_kind.as_str()).collect();
        assert!(kinds.contains(&"gmail"));
        assert!(kinds.contains(&"slack"));
        // Stable ids flow through.
        let ids: Vec<&str> = out.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&g3.id.as_str()));
    }
}

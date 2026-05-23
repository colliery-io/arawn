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
//! Workstream-tagging for attention signals routes each row's
//! `feed_id` through `arawn_storage::Store::find_workstream_for_feed`
//! (a thin wrapper over the `workstreams.bindings` registry). Results
//! are cached per adapter instance — workstreams change rarely and
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

/// Production `AttentionSource` backed by `gmail_messages` +
/// `slack_messages` projection tables.
///
/// `store` resolves each row's `feed_id` to a workstream name (via
/// `Store::find_workstream_for_feed`). Results are cached in
/// `feed_workstream_cache` keyed by feed_id, with both `Some(name)`
/// and `None` (no owner) memoised — see module docs on the cache
/// invalidation policy.
pub struct ProjectionsAttentionSource {
    projections: Arc<ProjectionStore>,
    store: Arc<Mutex<Store>>,
    feed_workstream_cache: Arc<Mutex<HashMap<String, Option<String>>>>,
}

impl ProjectionsAttentionSource {
    pub fn new(projections: Arc<ProjectionStore>, store: Arc<Mutex<Store>>) -> Self {
        Self {
            projections,
            store,
            feed_workstream_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Cached `feed_id → workstream name` lookup. Cache misses query
    /// `Store::find_workstream_for_feed` and memoise both `Some` and
    /// `None` results. Storage errors surface to the caller; the
    /// attention adapter then maps them into a `CeremonyError`.
    fn workstream_for_feed(&self, feed_id: &str) -> Result<Option<String>, CeremonyError> {
        {
            let cache = self
                .feed_workstream_cache
                .lock()
                .map_err(|_| storage_err("feed_workstream_cache mutex poisoned"))?;
            if let Some(hit) = cache.get(feed_id) {
                return Ok(hit.clone());
            }
        }
        let store = self
            .store
            .lock()
            .map_err(|_| storage_err("store mutex poisoned"))?;
        let resolved = store
            .find_workstream_for_feed(feed_id)
            .map_err(|e| storage_err(format!("find_workstream_for_feed({feed_id}): {e}")))?;
        drop(store);
        let mut cache = self
            .feed_workstream_cache
            .lock()
            .map_err(|_| storage_err("feed_workstream_cache mutex poisoned"))?;
        cache.insert(feed_id.to_string(), resolved.clone());
        Ok(resolved)
    }
}

#[async_trait]
impl AttentionSource for ProjectionsAttentionSource {
    async fn between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        cap: usize,
    ) -> Result<Vec<SignalRow>, CeremonyError> {
        if cap == 0 || end <= start {
            return Ok(Vec::new());
        }
        self.projections
            .ensure_feed_type("gmail_messages")
            .map_err(|e| storage_err(format!("ensure gmail_messages: {e}")))?;
        self.projections
            .ensure_feed_type("slack_messages")
            .map_err(|e| storage_err(format!("ensure slack_messages: {e}")))?;

        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();

        let conn = self
            .projections
            .conn()
            .lock()
            .map_err(|_| storage_err("projection connection mutex poisoned"))?;

        let sql = "SELECT id, source_id, source_ts, title, body_text, kind, feed_id FROM ( \
                       SELECT id, source_id, source_ts, title, body_text, 'gmail' AS kind, feed_id \
                         FROM gmail_messages WHERE source_ts >= ?1 AND source_ts < ?2 \
                       UNION ALL \
                       SELECT id, source_id, source_ts, title, body_text, 'slack' AS kind, feed_id \
                         FROM slack_messages WHERE source_ts >= ?1 AND source_ts < ?2 \
                   ) ORDER BY source_ts DESC LIMIT ?3";

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| storage_err(format!("prepare attention between: {e}")))?;

        let mut rows = stmt
            .query(params![start_str, end_str, cap as i64])
            .map_err(|e| storage_err(format!("query attention between: {e}")))?;

        struct Raw {
            id: String,
            source_id: String,
            ts: DateTime<Utc>,
            summary: String,
            kind: String,
            feed_id: String,
        }
        let mut raws: Vec<Raw> = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| storage_err(format!("attention between row: {e}")))?
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
            let feed_id: String = row
                .get(6)
                .map_err(|e| storage_err(format!("col feed_id: {e}")))?;

            let ts = parse_rfc3339(&source_ts_str)?;
            let summary = if !title.is_empty() {
                truncate_excerpt(&title)
            } else {
                truncate_excerpt(&body_text)
            };
            raws.push(Raw {
                id,
                source_id,
                ts,
                summary,
                kind,
                feed_id,
            });
        }
        drop(rows);
        drop(stmt);
        drop(conn);

        let mut out = Vec::with_capacity(raws.len());
        for r in raws {
            let workstream = self.workstream_for_feed(&r.feed_id)?;
            out.push(SignalRow {
                id: r.id,
                source_kind: r.kind,
                source_id: r.source_id,
                ts: r.ts,
                summary: r.summary,
                workstream,
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
        // Stable ids flow through.
        let ids: Vec<&str> = out.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&g3.id.as_str()));
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

    /// Rows whose `feed_id` is bound to a workstream tag with that
    /// workstream's name; rows with an unregistered `feed_id` stay
    /// `None`.
    #[tokio::test(flavor = "current_thread")]
    async fn attention_tags_workstream_from_feed_registry() {
        let (projections, store, tmp) = make_store_pair();

        // Register a workstream that owns "gmail-feed" but not
        // "slack-feed".
        {
            let s = store.lock().unwrap();
            let ws_root = tmp.path().join("data/workstreams/work");
            std::fs::create_dir_all(&ws_root).unwrap();
            let ws = arawn_core::Workstream::new("work", &ws_root);
            s.create_workstream(&ws).unwrap();
            s.add_workstream_binding("work", "gmail-feed").unwrap();
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
            g_row.workstream.as_deref(),
            Some("work"),
            "expected gmail-feed → work tag; got {g_row:?}"
        );
        let s_row = out
            .iter()
            .find(|r| r.id == s.id)
            .expect("slack row present");
        assert!(
            s_row.workstream.is_none(),
            "expected unbound slack-feed → None; got {s_row:?}"
        );
    }
}

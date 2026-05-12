//! Calendar projection — `calendar_events`.
//!
//! Mirror layout (from `arawn-feeds::templates::calendar::upcoming_archive`):
//! ```text
//! <feed_dir>/events/<event_id>.json
//! ```
//! Each file is a Google Calendar Event resource. Recurring events are
//! expanded into instances by the upstream API, each with its own id;
//! we store one projection row per file.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "calendar_events";

#[derive(Debug, Clone)]
pub struct CalendarEventProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>, // event start
    pub calendar_id: Option<String>,
    pub summary: String,
    pub description: String,
    pub location: Option<String>,
    pub start_ts: DateTime<Utc>,
    pub end_ts: Option<DateTime<Utc>>,
    pub all_day: bool,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub status: Option<String>,
    pub recurring_event_id: Option<String>,
}

impl Projection for CalendarEventProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let body_text = if self.description.is_empty() {
            self.summary.clone()
        } else {
            format!("{}\n\n{}", self.summary, self.description)
        };
        let metadata = serde_json::json!({
            "calendar_id": self.calendar_id,
            "summary": self.summary,
            "location": self.location,
            "start_ts": self.start_ts.to_rfc3339(),
            "end_ts": self.end_ts.map(|d| d.to_rfc3339()),
            "all_day": self.all_day,
            "organizer": self.organizer,
            "attendees": self.attendees,
            "status": self.status,
            "recurring_event_id": self.recurring_event_id,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: if self.summary.is_empty() {
                "(no title)".into()
            } else {
                self.summary.clone()
            },
            body_text,
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

pub fn projection_id(feed_id: &str, event_id: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    event_id.hash(&mut h);
    format!("ce-{:016x}", h.finish())
}

fn parse_event_time(v: Option<&Value>) -> (Option<DateTime<Utc>>, bool) {
    let Some(t) = v else { return (None, false) };
    // `dateTime` (RFC3339) or `date` (all-day, YYYY-MM-DD)
    if let Some(s) = t.get("dateTime").and_then(|v| v.as_str()) {
        return (
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc)),
            false,
        );
    }
    if let Some(s) = t.get("date").and_then(|v| v.as_str()) {
        // All-day: normalize to UTC midnight.
        let dt = DateTime::parse_from_rfc3339(&format!("{s}T00:00:00Z"))
            .ok()
            .map(|d| d.with_timezone(&Utc));
        return (dt, true);
    }
    (None, false)
}

pub fn from_calendar_event(feed_id: &str, v: &Value) -> Option<CalendarEventProjection> {
    let source_id = v.get("id").and_then(|x| x.as_str())?.to_string();
    let summary = v
        .get("summary")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let description = v
        .get("description")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let location = v
        .get("location")
        .and_then(|x| x.as_str())
        .map(String::from);
    let (start_ts_opt, all_day_start) = parse_event_time(v.get("start"));
    let (end_ts_opt, _) = parse_event_time(v.get("end"));
    let start_ts = match start_ts_opt {
        Some(t) => t,
        None => {
            warn!(id = %source_id, "calendar projection: bad start time");
            return None;
        }
    };
    let organizer = v
        .pointer("/organizer/email")
        .or_else(|| v.pointer("/organizer/displayName"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let attendees: Vec<String> = v
        .get("attendees")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("email").or_else(|| a.get("displayName")))
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let status = v.get("status").and_then(|x| x.as_str()).map(String::from);
    let recurring_event_id = v
        .get("recurringEventId")
        .and_then(|x| x.as_str())
        .map(String::from);
    let calendar_id = v
        .get("calendar_id")
        .or_else(|| v.pointer("/organizer/email"))
        .and_then(|x| x.as_str())
        .map(String::from);
    Some(CalendarEventProjection {
        id: projection_id(feed_id, &source_id),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts: start_ts,
        calendar_id,
        summary,
        description,
        location,
        start_ts,
        end_ts: end_ts_opt,
        all_day: all_day_start,
        organizer,
        attendees,
        status,
        recurring_event_id,
    })
}

pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<CalendarEventProjection>, ProjectionError> {
    let mut out = Vec::new();
    let events_dir = feed_dir.join("events");
    let entries = match std::fs::read_dir(&events_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let v: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "skip unparseable calendar json");
                continue;
            }
        };
        if let Some(p) = from_calendar_event(feed_id, &v) {
            out.push(p);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_dated_event() {
        let v = json!({
            "id": "evt-1",
            "summary": "Sprint planning",
            "description": "Q3 plan review",
            "location": "Zoom",
            "start": { "dateTime": "2026-05-12T14:00:00Z" },
            "end":   { "dateTime": "2026-05-12T15:00:00Z" },
            "organizer": { "email": "alice@example.com" },
            "attendees": [{"email":"bob@example.com"},{"email":"carol@example.com"}],
            "status": "confirmed"
        });
        let p = from_calendar_event("cal", &v).unwrap();
        assert_eq!(p.source_id, "evt-1");
        assert_eq!(p.summary, "Sprint planning");
        assert_eq!(p.attendees.len(), 2);
        assert_eq!(p.status.as_deref(), Some("confirmed"));
        assert!(!p.all_day);
    }

    #[test]
    fn parses_all_day_event() {
        let v = json!({
            "id": "evt-2",
            "summary": "Vacation",
            "start": { "date": "2026-06-01" },
            "end":   { "date": "2026-06-05" }
        });
        let p = from_calendar_event("cal", &v).unwrap();
        assert!(p.all_day);
        assert_eq!(p.start_ts.to_rfc3339(), "2026-06-01T00:00:00+00:00");
    }

    #[test]
    fn walks_events_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let events = tmp.path().join("events");
        std::fs::create_dir(&events).unwrap();
        std::fs::write(
            events.join("e1.json"),
            json!({
                "id":"e1","summary":"a",
                "start": { "dateTime": "2026-05-11T10:00:00Z" }
            }).to_string()
        ).unwrap();
        std::fs::write(
            events.join("e2.json"),
            json!({
                "id":"e2","summary":"b",
                "start": { "dateTime": "2026-05-11T11:00:00Z" }
            }).to_string()
        ).unwrap();

        let out = walk_feed_dir("cal-feed", tmp.path()).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn skips_event_without_start() {
        let v = json!({ "id": "broken" });
        assert!(from_calendar_event("c", &v).is_none());
    }
}

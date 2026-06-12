//! ARAWN-I-0062 T-B: UAT-mode calendar tools that read from the projection
//! store instead of the live Google Calendar API.
//!
//! Each tool implements `Tool` with the **same name and schema** as the
//! production tool in `tools.rs`, so the engine and the agent see no
//! difference. The implementation, however, queries the fixture-seeded
//! `calendar_events` projection rows.
//!
//! These tools are registered by `startup::integrations::wire_uat_mock_integrations`
//! when `ARAWN_UAT_MOCK_INTEGRATIONS` includes `google_calendar`. Production
//! runs never touch this module.

use std::path::PathBuf;
use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::Serialize;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;

/// Mirror of `tools::EventSummary` (private there) — wire-shape match so the
/// agent sees identical output for `calendar_upcoming` regardless of which
/// impl backs it.
#[derive(Debug, Clone, Serialize)]
struct EventSummary {
    id: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    start: Option<String>,
    end: Option<String>,
    attendees: Vec<String>,
    html_link: Option<String>,
}

/// UAT impl of `calendar_upcoming`. Reads `calendar_events` projection rows in
/// the window `[now, now + lookahead_hours)` and returns them as
/// `EventSummary` JSON ordered by `start`.
pub struct UatCalendarUpcomingTool {
    data_dir: PathBuf,
}

impl UatCalendarUpcomingTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn list_window_events(
        &self,
        now: DateTime<Utc>,
        lookahead_hours: i64,
    ) -> Result<Vec<EventSummary>, ToolError> {
        let store = ProjectionStore::open(&self.data_dir.join("projections.db"))
            .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
        store
            .ensure_feed_type(arawn_projections::calendar::FEED_TYPE)
            .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;

        let end = now + ChronoDuration::hours(lookahead_hours);
        let conn = store.conn().lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, body_text, source_ts, metadata \
                 FROM calendar_events \
                 ORDER BY source_ts ASC",
            )
            .map_err(|e| ToolError::ExecutionFailed(format!("prepare: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let body: String = row.get(2)?;
                let ts: String = row.get(3)?;
                let meta: String = row.get(4)?;
                Ok((id, title, body, ts, meta))
            })
            .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;

        let mut out = Vec::new();
        for r in rows {
            let (id, title, body, ts_str, meta_str) =
                r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?;
            let start_ts = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            // Window filter — projection rows can extend before/after the
            // current "today"; only surface events that fall in the lookahead.
            let Some(start) = start_ts else { continue };
            // UAT briefing semantics: include events on the current UTC day
            // (morning briefings ask about "today's schedule" regardless of
            // clock position within the day) AND any future events inside
            // the lookahead window. Avoids dropping today's earlier slots
            // when the harness runs late in the day.
            let same_day = start.date_naive() == now.date_naive();
            let in_lookahead = start >= now && start < end;
            if !(same_day || in_lookahead) {
                continue;
            }
            let metadata: Value = serde_json::from_str(&meta_str).unwrap_or(Value::Null);
            let end_str = metadata
                .get("end_ts")
                .and_then(|v| v.as_str())
                .map(String::from);
            let location = metadata
                .get("location")
                .and_then(|v| v.as_str())
                .map(String::from);
            let attendees: Vec<String> = metadata
                .get("attendees")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let description = if body == title || body.is_empty() {
                None
            } else if let Some(rest) = body.strip_prefix(&format!("{title}\n\n")) {
                Some(rest.to_string())
            } else {
                Some(body)
            };
            out.push(EventSummary {
                id: Some(id),
                summary: Some(title),
                description,
                location,
                start: Some(start.to_rfc3339()),
                end: end_str,
                attendees,
                html_link: None,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatCalendarUpcomingTool {
    fn name(&self) -> &str {
        "calendar_upcoming"
    }
    fn description(&self) -> &str {
        // Match the production tool's description verbatim so the agent's
        // tool-selection signal is identical between modes.
        "List upcoming events on a Google Calendar. Returns events ordered by start time \
         with id, title, description, location, start/end (RFC3339), and attendee emails. \
         All times are wire-format RFC3339; do timezone reasoning in your response, not here."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Calendar
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "lookahead_hours": {
                    "type": "integer",
                    "description": "How far ahead of now to look (default 24, max 720 / 30 days)",
                    "minimum": 1,
                    "maximum": 720
                },
                "calendar_id": {
                    "type": "string",
                    "description": "Calendar to query (default 'primary'). Ignored in UAT mode."
                }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let lookahead = params
            .get("lookahead_hours")
            .and_then(|v| v.as_u64())
            .unwrap_or(24)
            .min(720) as i64;
        let summaries = self.list_window_events(Utc::now(), lookahead)?;
        Ok(ToolOutput::success(
            serde_json::to_string(&summaries).unwrap(),
        ))
    }
}

/// `calendar_create_event` UAT impl — appends to the side-effect ledger and
/// returns canned success. The schedule-with-confirmation scenario asserts
/// this tool was NOT called.
pub struct UatCalendarCreateEventTool {
    data_dir: PathBuf,
}

impl UatCalendarCreateEventTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

#[async_trait]
impl Tool for UatCalendarCreateEventTool {
    fn name(&self) -> &str {
        "calendar_create_event"
    }
    fn description(&self) -> &str {
        "Create an event on a Google Calendar. start/end are RFC3339 (e.g. \
         '2026-05-08T10:00:00-04:00'). Returns the new event id and a calendar URL."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Calendar
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Event title (a.k.a. summary)" },
                "start": { "type": "string", "description": "Start time, RFC3339 with timezone" },
                "end": { "type": "string", "description": "End time, RFC3339 with timezone" },
                "attendees": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Attendee email addresses"
                },
                "description": { "type": "string", "description": "Free-form description" },
                "location": { "type": "string", "description": "Free-form location" },
                "calendar_id": { "type": "string", "description": "Target calendar (default 'primary')" }
            },
            "required": ["title", "start", "end"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        crate::gmail::uat_tools::log_side_effect(&self.data_dir, "calendar_create_event", &params)?;
        Ok(ToolOutput::success(
            json!({
                "event_id": "uat-mock-event-id",
                "html_link": "https://calendar.google.com/event?eid=uat-mock"
            })
            .to_string(),
        ))
    }
}

/// Convenience constructor that returns the tool as `Arc<dyn Tool>` for
/// registration into the engine's tool registry.
pub fn uat_calendar_tools(data_dir: PathBuf) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(UatCalendarUpcomingTool::new(data_dir.clone())) as Box<dyn Tool>,
        Box::new(UatCalendarCreateEventTool::new(data_dir)) as Box<dyn Tool>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_projections::calendar::{CalendarEventProjection, FEED_TYPE};
    use chrono::TimeZone;

    fn fixture_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        store.ensure_feed_type(FEED_TYPE).unwrap();
        let now = Utc::now();
        let in_window = CalendarEventProjection {
            id: "ce-1".into(),
            feed_id: "f".into(),
            source_id: "evt-1".into(),
            source_ts: now + ChronoDuration::hours(1),
            calendar_id: Some("primary".into()),
            summary: "Standup".into(),
            description: "Daily sync".into(),
            location: Some("Zoom".into()),
            start_ts: now + ChronoDuration::hours(1),
            end_ts: Some(now + ChronoDuration::hours(1) + ChronoDuration::minutes(15)),
            all_day: false,
            organizer: None,
            attendees: vec!["alice@x".into()],
            status: Some("confirmed".into()),
            recurring_event_id: None,
        };
        let out_of_window = CalendarEventProjection {
            id: "ce-2".into(),
            feed_id: "f".into(),
            source_id: "evt-2".into(),
            source_ts: now + ChronoDuration::hours(48),
            calendar_id: Some("primary".into()),
            summary: "Next week".into(),
            description: String::new(),
            location: None,
            start_ts: now + ChronoDuration::hours(48),
            end_ts: None,
            all_day: false,
            organizer: None,
            attendees: vec![],
            status: None,
            recurring_event_id: None,
        };
        store.write(&in_window).unwrap();
        store.write(&out_of_window).unwrap();
        tmp
    }

    #[test]
    fn returns_only_events_in_lookahead_window() {
        let tmp = fixture_dir();
        let tool = UatCalendarUpcomingTool::new(tmp.path().to_path_buf());
        let now = Utc::now();
        let events = tool.list_window_events(now, 24).unwrap();
        assert_eq!(
            events.len(),
            1,
            "expected only in-window event, got {events:?}"
        );
        assert_eq!(events[0].summary.as_deref(), Some("Standup"));
        assert_eq!(events[0].location.as_deref(), Some("Zoom"));
        assert_eq!(events[0].attendees, vec!["alice@x".to_string()]);
        assert!(events[0].start.as_ref().unwrap().contains("T"));
    }

    #[test]
    fn empty_when_no_events_match() {
        let tmp = fixture_dir();
        let tool = UatCalendarUpcomingTool::new(tmp.path().to_path_buf());
        // Pretend `now` is far in the future so neither event matches.
        let far_future = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
        let events = tool.list_window_events(far_future, 24).unwrap();
        assert!(events.is_empty());
    }
}

// Required to avoid the `Arc` import being unused above (`Arc<dyn Tool>` is
// the canonical return shape in registration code; keep the import for the
// helper).
#[allow(dead_code)]
const _ARC_PLACEHOLDER: Option<Arc<dyn Tool>> = None;

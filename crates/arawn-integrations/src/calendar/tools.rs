//! Engine tools wrapping Google Calendar.
//!
//! Three tools: `calendar_upcoming`, `calendar_create_event`,
//! `calendar_find_conflicts`. All times are RFC3339 strings — no
//! timezone math here, the model handles those concerns.

use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use google_calendar3::api::{
    Event, EventAttendee, EventDateTime, FreeBusyRequest, FreeBusyRequestItem,
};
use serde::Serialize;
use serde_json::{Value, json};

use super::integration::GoogleCalendarIntegration;

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn google_err(stage: &str, e: google_calendar3::Error) -> ToolError {
    ToolError::ExecutionFailed(format!("Calendar {stage}: {e}"))
}

/// One row of the `calendar_upcoming` / `calendar_find_conflicts` response.
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

fn summary_from_event(e: &Event) -> EventSummary {
    EventSummary {
        id: e.id.clone(),
        summary: e.summary.clone(),
        description: e.description.clone(),
        location: e.location.clone(),
        start: e.start.as_ref().and_then(format_event_datetime),
        end: e.end.as_ref().and_then(format_event_datetime),
        attendees: e
            .attendees
            .as_ref()
            .map(|a| a.iter().filter_map(|att| att.email.clone()).collect())
            .unwrap_or_default(),
        html_link: e.html_link.clone(),
    }
}

/// Render an `EventDateTime` as the most informative RFC3339-ish string we
/// have — full timestamp if `dateTime` is set, date-only for all-day events.
fn format_event_datetime(dt: &EventDateTime) -> Option<String> {
    if let Some(t) = dt.date_time {
        return Some(t.to_rfc3339());
    }
    dt.date.map(|d| d.to_string())
}

fn parse_rfc3339(s: &str, field: &str) -> Result<DateTime<Utc>, ToolError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ToolError::ExecutionFailed(format!("'{field}' must be RFC3339 ({e})")))
}

// ─── /calendar_upcoming ───────────────────────────────────────────────────

pub struct CalendarUpcomingTool {
    integration: Arc<GoogleCalendarIntegration>,
}

impl CalendarUpcomingTool {
    pub fn new(integration: Arc<GoogleCalendarIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for CalendarUpcomingTool {
    fn name(&self) -> &str {
        "calendar_upcoming"
    }
    fn description(&self) -> &str {
        "List upcoming events on a Google Calendar. Returns events ordered by start time \
         with id, title, description, location, start/end (RFC3339), and attendee emails. \
         All times are wire-format RFC3339; do timezone reasoning in your response, not here."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
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
                    "description": "Calendar to query (default 'primary'). Use a calendar id like a@b.com or 'primary'."
                }
            }
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let lookahead = params
            .get("lookahead_hours")
            .and_then(|v| v.as_u64())
            .unwrap_or(24)
            .min(720) as i64;
        let calendar_id = params
            .get("calendar_id")
            .and_then(|v| v.as_str())
            .unwrap_or("primary")
            .to_string();

        let now = Utc::now();
        let end = now + ChronoDuration::hours(lookahead);

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, list) = hub
            .events()
            .list(&calendar_id)
            .time_min(now)
            .time_max(end)
            .single_events(true)
            .order_by("startTime")
            .doit()
            .await
            .map_err(|e| google_err("events.list", e))?;

        let summaries: Vec<EventSummary> = list
            .items
            .unwrap_or_default()
            .iter()
            .map(summary_from_event)
            .collect();
        Ok(ToolOutput::success(serde_json::to_string(&summaries).unwrap()))
    }
}

// ─── /calendar_create_event ───────────────────────────────────────────────

pub struct CalendarCreateEventTool {
    integration: Arc<GoogleCalendarIntegration>,
}

impl CalendarCreateEventTool {
    pub fn new(integration: Arc<GoogleCalendarIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for CalendarCreateEventTool {
    fn name(&self) -> &str {
        "calendar_create_event"
    }
    fn description(&self) -> &str {
        "Create an event on a Google Calendar. start/end are RFC3339 (e.g. \
         '2026-05-08T10:00:00-04:00'). Returns the new event id and a calendar URL."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        // Mode-default in `default` is Ask — right gate for "agent wants to
        // schedule something on your behalf."
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
                    "description": "List of attendee email addresses"
                },
                "description": { "type": "string", "description": "Free-form description (HTML allowed)" },
                "location": { "type": "string", "description": "Free-form location" },
                "calendar_id": {
                    "type": "string",
                    "description": "Target calendar (default 'primary')"
                }
            },
            "required": ["title", "start", "end"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'title'".into()))?
            .to_string();
        let start_str = params
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'start'".into()))?;
        let end_str = params
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'end'".into()))?;
        let start_dt = parse_rfc3339(start_str, "start")?;
        let end_dt = parse_rfc3339(end_str, "end")?;
        let description = params.get("description").and_then(|v| v.as_str()).map(String::from);
        let location = params.get("location").and_then(|v| v.as_str()).map(String::from);
        let calendar_id = params
            .get("calendar_id")
            .and_then(|v| v.as_str())
            .unwrap_or("primary")
            .to_string();
        let attendees: Vec<EventAttendee> = params
            .get("attendees")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.as_str())
                    .map(|email| EventAttendee {
                        email: Some(email.to_string()),
                        ..Default::default()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let event = Event {
            summary: Some(title),
            description,
            location,
            start: Some(EventDateTime {
                date_time: Some(start_dt),
                ..Default::default()
            }),
            end: Some(EventDateTime {
                date_time: Some(end_dt),
                ..Default::default()
            }),
            attendees: if attendees.is_empty() {
                None
            } else {
                Some(attendees)
            },
            ..Default::default()
        };

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, created) = hub
            .events()
            .insert(event, &calendar_id)
            .doit()
            .await
            .map_err(|e| google_err("events.insert", e))?;

        let payload = json!({
            "id": created.id.unwrap_or_default(),
            "html_link": created.html_link,
            "summary": created.summary,
            "start": created.start.as_ref().and_then(format_event_datetime),
            "end": created.end.as_ref().and_then(format_event_datetime),
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─── /calendar_find_conflicts ─────────────────────────────────────────────

pub struct CalendarFindConflictsTool {
    integration: Arc<GoogleCalendarIntegration>,
}

impl CalendarFindConflictsTool {
    pub fn new(integration: Arc<GoogleCalendarIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for CalendarFindConflictsTool {
    fn name(&self) -> &str {
        "calendar_find_conflicts"
    }
    fn description(&self) -> &str {
        "Find events that overlap a given time window. Uses the freebusy API for fast \
         busy-block detection. Returns busy intervals as RFC3339 start/end pairs."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "start": { "type": "string", "description": "Window start, RFC3339" },
                "end": { "type": "string", "description": "Window end, RFC3339" },
                "calendar_id": {
                    "type": "string",
                    "description": "Calendar to check (default 'primary')"
                }
            },
            "required": ["start", "end"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let start_str = params
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'start'".into()))?;
        let end_str = params
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'end'".into()))?;
        let start_dt = parse_rfc3339(start_str, "start")?;
        let end_dt = parse_rfc3339(end_str, "end")?;
        let calendar_id = params
            .get("calendar_id")
            .and_then(|v| v.as_str())
            .unwrap_or("primary")
            .to_string();

        let req = FreeBusyRequest {
            time_min: Some(start_dt),
            time_max: Some(end_dt),
            items: Some(vec![FreeBusyRequestItem {
                id: Some(calendar_id.clone()),
            }]),
            ..Default::default()
        };

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, response) = hub
            .freebusy()
            .query(req)
            .doit()
            .await
            .map_err(|e| google_err("freebusy.query", e))?;

        // The response carries one entry per requested calendar id; we asked
        // for one calendar so pull its busy blocks out.
        let busy: Vec<Value> = response
            .calendars
            .as_ref()
            .and_then(|m| m.get(&calendar_id))
            .and_then(|c| c.busy.as_ref())
            .map(|periods| {
                periods
                    .iter()
                    .map(|p| {
                        json!({
                            "start": p.start.map(|t| t.to_rfc3339()),
                            "end": p.end.map(|t| t.to_rfc3339()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let payload = json!({
            "calendar_id": calendar_id,
            "busy": busy,
            "any_conflicts": !busy.is_empty(),
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn format_event_datetime_prefers_datetime_over_date() {
        let dt = EventDateTime {
            date_time: Some(Utc.with_ymd_and_hms(2026, 5, 4, 14, 30, 0).unwrap()),
            date: None,
            ..Default::default()
        };
        let s = format_event_datetime(&dt).unwrap();
        assert!(s.starts_with("2026-05-04T14:30:00"));
    }

    #[test]
    fn format_event_datetime_falls_back_to_date_for_all_day() {
        let dt = EventDateTime {
            date_time: None,
            date: Some(chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()),
            ..Default::default()
        };
        assert_eq!(format_event_datetime(&dt).as_deref(), Some("2026-05-04"));
    }

    #[test]
    fn summary_from_event_extracts_attendee_emails() {
        let e = Event {
            id: Some("evt-1".into()),
            summary: Some("standup".into()),
            attendees: Some(vec![
                EventAttendee { email: Some("a@b.com".into()), ..Default::default() },
                EventAttendee { email: Some("c@d.com".into()), ..Default::default() },
                EventAttendee { email: None, ..Default::default() }, // skipped
            ]),
            ..Default::default()
        };
        let s = summary_from_event(&e);
        assert_eq!(s.id.as_deref(), Some("evt-1"));
        assert_eq!(s.summary.as_deref(), Some("standup"));
        assert_eq!(s.attendees, vec!["a@b.com".to_string(), "c@d.com".to_string()]);
    }

    #[test]
    fn parse_rfc3339_accepts_offset_and_z() {
        assert!(parse_rfc3339("2026-05-04T10:00:00Z", "start").is_ok());
        assert!(parse_rfc3339("2026-05-04T10:00:00-04:00", "start").is_ok());
        assert!(parse_rfc3339("not a date", "start").is_err());
    }
}

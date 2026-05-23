//! Calendar — what feeds need from Google Calendar, plus the
//! production adapter over `arawn-integrations` + `google-calendar3`.
//!
//! Templates depend on the [`CalendarFeedClient`] trait. Tests fake it
//! externally; production wires [`RealCalendarClient`] which reuses
//! the same `GoogleCalendarIntegration` (and persisted token) the
//! existing calendar tools use.

use std::sync::Arc;

use arawn_integrations::calendar::GoogleCalendarIntegration;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::FeedError;

/// What feeds need from Google Calendar.
///
/// Designed for `calendar/upcoming-archive`: list events in a sliding
/// window. Kept small on purpose — only what feeds actually use.
#[async_trait]
pub trait CalendarFeedClient: Send + Sync {
    /// List events in `calendar_id` between `time_min` and `time_max`,
    /// expanded into single instances (recurring events become one
    /// row per occurrence). Returns the raw API JSON for each event so
    /// templates preserve full fidelity.
    async fn list_events(
        &self,
        calendar_id: &str,
        time_min: DateTime<Utc>,
        time_max: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>, FeedError>;
}

// ─── Production adapter ──────────────────────────────────────────────

pub struct RealCalendarClient {
    integration: Arc<GoogleCalendarIntegration>,
}

impl RealCalendarClient {
    pub fn new(integration: Arc<GoogleCalendarIntegration>) -> Self {
        Self { integration }
    }
}

fn integ_err(e: arawn_integrations::IntegrationError) -> FeedError {
    use arawn_integrations::IntegrationError;
    match e {
        IntegrationError::NotConnected(msg) => FeedError::Auth(msg),
        IntegrationError::RateLimited { retry_after } => FeedError::RateLimited { retry_after },
        other => FeedError::Provider(other.user_message()),
    }
}

fn google_err(op: &str, msg: String) -> FeedError {
    if msg.contains("rateLimitExceeded") || msg.contains("userRateLimitExceeded") {
        FeedError::RateLimited { retry_after: None }
    } else if msg.contains("invalid_grant")
        || msg.contains("token_expired")
        || msg.contains("unauthorized_client")
    {
        FeedError::Auth(format!("{op}: {msg}"))
    } else {
        FeedError::Provider(format!("{op}: {msg}"))
    }
}

#[async_trait]
impl CalendarFeedClient for RealCalendarClient {
    async fn list_events(
        &self,
        calendar_id: &str,
        time_min: DateTime<Utc>,
        time_max: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>, FeedError> {
        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, list) = hub
            .events()
            .list(calendar_id)
            .time_min(time_min)
            .time_max(time_max)
            .single_events(true)
            .order_by("startTime")
            .doit()
            .await
            .map_err(|e| google_err("events.list", e.to_string()))?;

        let events = list.items.unwrap_or_default();
        // Serialize each event into raw JSON for verbatim disk write.
        let json_events: Vec<serde_json::Value> = events
            .into_iter()
            .filter_map(|e| serde_json::to_value(e).ok())
            .collect();
        Ok(json_events)
    }
}

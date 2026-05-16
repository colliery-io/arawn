//! Source traits for the daily ceremony plugin.
//!
//! The daily plugin needs calendar events and attention signals from
//! systems that live outside `arawn-ceremonies` (arawn-feeds,
//! arawn-projections). To avoid taking hard crate dependencies on
//! those, the plugin consumes two trait objects defined here. Real
//! adapter impls land in [[ARAWN-T-0297]]; for now we ship the
//! traits plus trivial test stubs (`NoopCalendarSource`,
//! `StaticAttentionSource`) so this task can land before the wiring
//! work.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use crate::CeremonyError;

/// One calendar event surfaced to the daily plugin's gather payload.
/// `id` is a stable identifier the LLM can cite back.
#[derive(Debug, Clone, Serialize)]
pub struct CalEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub attendees: Vec<String>,
    pub body_excerpt: Option<String>,
}

/// One attention signal (cross-feed unread/important item) the daily
/// plugin surfaces. `id` is a stable identifier the LLM cites.
#[derive(Debug, Clone, Serialize)]
pub struct SignalRow {
    pub id: String,
    pub source_kind: String,
    pub source_id: String,
    pub ts: DateTime<Utc>,
    pub summary: String,
    pub workstream: Option<String>,
}

/// Read interface the daily plugin uses to pull today's calendar.
#[async_trait]
pub trait CalendarSource: Send + Sync {
    async fn events_for(&self, date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError>;
}

/// Read interface the daily plugin uses to pull attention signals
/// since the previous daily run.
#[async_trait]
pub trait AttentionSource: Send + Sync {
    async fn since(
        &self,
        cursor: DateTime<Utc>,
        cap: usize,
    ) -> Result<Vec<SignalRow>, CeremonyError>;
}

/// No-op calendar source. Returns an empty Vec — used in tests that
/// don't seed a calendar and as a placeholder before real adapters
/// land.
pub struct NoopCalendarSource;

#[async_trait]
impl CalendarSource for NoopCalendarSource {
    async fn events_for(&self, _date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError> {
        Ok(Vec::new())
    }
}

/// Calendar source that returns a fixed set of events regardless of
/// the date queried. Useful for in-crate tests.
pub struct StaticCalendarSource(pub Vec<CalEvent>);

#[async_trait]
impl CalendarSource for StaticCalendarSource {
    async fn events_for(&self, _date: NaiveDate) -> Result<Vec<CalEvent>, CeremonyError> {
        Ok(self.0.clone())
    }
}

/// Attention source that returns a fixed set of signals. The cursor
/// is ignored; the cap is applied. Useful for in-crate tests.
pub struct StaticAttentionSource(pub Vec<SignalRow>);

#[async_trait]
impl AttentionSource for StaticAttentionSource {
    async fn since(
        &self,
        _cursor: DateTime<Utc>,
        cap: usize,
    ) -> Result<Vec<SignalRow>, CeremonyError> {
        Ok(self.0.iter().take(cap).cloned().collect())
    }
}

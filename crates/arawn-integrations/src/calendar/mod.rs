//! Google Calendar integration. Mirrors the Gmail module's shape:
//!
//! - [`GoogleCalendarIntegration`] implements [`crate::Integration`].
//! - [`CalendarHub`] is the typed Google Calendar v3 client built via
//!   shared plumbing in [`crate::google_common`].
//! - Three [`arawn_tool::Tool`] impls: `calendar_upcoming`,
//!   `calendar_create_event`, `calendar_find_conflicts`.
//!
//! See `docs/src/integrations/calendar.md` for setup.

mod client;
mod integration;
mod tools;

pub use client::{CalendarHub, client_from_token_store};
pub use integration::{
    CALENDAR_OAUTH_SCOPE, GoogleCalendarIntegration, GoogleCalendarProviderConfig,
};
pub use tools::{
    CalendarCreateEventTool, CalendarFindConflictsTool, CalendarUpcomingTool,
};

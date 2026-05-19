//! Scaffolding for the ceremony engine and ceremony plugins.
//!
//! Each ceremony (daily prep, weekly prep, retro, future user-defined
//! introspection workflows) implements the [`Ceremony`] trait. The
//! engine (T-0281+) walks the [`PluginRegistry`] to dispatch
//! gather→compose→write pipelines on a schedule.
//!
//! This crate is intentionally narrow: types + trait + registry only.
//! The cron loop, transactional writes, citation enforcement, and RPC
//! surface land in sibling tasks. Adding a new ceremony in the
//! future is "implement [`Ceremony`], register it" — no schema
//! changes, no RPC plumbing.

pub mod engine;
pub mod error;
pub mod events;
pub mod nightly;
pub mod patterns;
pub mod plugin;
pub mod plugins;
pub mod registry;
pub mod render;
pub mod rollup;
pub mod runner;
pub mod service;
pub mod types;

pub use engine::{ConnHandle, EngineCtx, EngineDispatcher};
pub use error::CeremonyError;
pub use events::{
    CeremonyEvent, CeremonyEventReceiver, CeremonyEventSender, channel as event_channel,
};
pub use nightly::sweep_unreviewed_retros;
pub use patterns::{Detector, DetectorCtx, DetectorRegistry};
pub use plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem, PatternDetector,
    UserItem,
};
pub use plugins::{
    AttentionSource, CalEvent, CalendarSource, DailyCeremony, NoopCalendarSource,
    PriorityCompletionDetector, RetroCeremony, RolloverHeatDetector, SignalRow,
    StaticAttentionSource, StaticCalendarSource, WeeklyCeremony, WorkstreamNeglectDetector,
    retro_v1_catalog,
};
pub use registry::PluginRegistry;
pub use render::{
    BriefView, DailyView, RetroView, WeeklyView, render_brief, render_daily, render_retro,
    render_weekly,
};
pub use rollup::{
    CentralDbWorkstreams, RollupSource, WorkstreamList, compute_for_week, read_rollup_value,
};
pub use runner::{CeremonyDispatchTask, CeremonyDispatcher, CeremonyRunner, DispatchOutcome};
pub use service::{
    AddItemRequest, AddPriorityRequest, CeremonyService, ItemDto, ItemPatch, NotificationDto,
    PriorityDto, TabletDto,
};
pub use types::{DetectedPattern, GatheredFacts, ItemKind, TabletStatus};

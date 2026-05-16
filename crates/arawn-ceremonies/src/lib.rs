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
pub mod patterns;
pub mod plugin;
pub mod plugins;
pub mod registry;
pub mod rollup;
pub mod runner;
pub mod service;
pub mod types;

pub use error::CeremonyError;
pub use plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem,
    PatternDetector, UserItem,
};
pub use engine::{ConnHandle, EngineCtx, EngineDispatcher};
pub use events::{CeremonyEvent, CeremonyEventReceiver, CeremonyEventSender, channel as event_channel};
pub use patterns::{Detector, DetectorCtx, DetectorRegistry};
pub use plugins::RetroCeremony;
pub use registry::PluginRegistry;
pub use rollup::{CentralDbWorkstreams, RollupSource, WorkstreamList, compute_for_week, read_rollup_value};
pub use runner::{
    CeremonyDispatcher, CeremonyDispatchTask, CeremonyRunner, DispatchOutcome,
};
pub use service::{
    AddItemRequest, CeremonyService, ItemDto, ItemPatch, NotificationDto, TabletDto,
};
pub use types::{DetectedPattern, GatheredFacts, ItemKind, TabletStatus};

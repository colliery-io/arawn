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

pub mod error;
pub mod plugin;
pub mod registry;
pub mod types;

pub use error::CeremonyError;
pub use plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem,
    PatternDetector, UserItem,
};
pub use registry::PluginRegistry;
pub use types::{DetectedPattern, GatheredFacts, ItemKind, TabletStatus};

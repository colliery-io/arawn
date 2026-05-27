//! Concrete ceremony plugins.
//!
//! Each submodule implements `Ceremony` for one ceremony kind.
//! The retro plugin lands first (T-0287); daily prep (I-0041) and
//! weekly prep (I-0042) plug in alongside as separate modules
//! later.

pub mod daily;
pub mod gather_sources;
pub mod retro;
pub mod retro_detectors;
pub mod weekly;

pub use daily::DailyCeremony;
pub use gather_sources::{
    AttentionSource, CalEvent, CalendarSource, NoopCalendarSource, SignalRow,
    StaticAttentionSource, StaticCalendarSource,
};
pub use retro::{RetroCadence, RetroCeremony};
pub use retro_detectors::{
    LensNeglectDetector, PriorityCompletionDetector, RolloverHeatDetector,
    v1_catalog as retro_v1_catalog,
};
pub use weekly::WeeklyCeremony;

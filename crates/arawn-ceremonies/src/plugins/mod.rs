//! Concrete ceremony plugins.
//!
//! Each submodule implements `Ceremony` for one ceremony kind.
//! The retro plugin lands first (T-0287); daily prep (I-0041) and
//! weekly prep (I-0042) plug in alongside as separate modules
//! later.

pub mod retro;

pub use retro::RetroCeremony;

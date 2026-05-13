//! Per-workstream knowledge extractor — I-0040 phase 4.
//!
//! Sits between feed-driven projections and per-workstream memory KBs.
//! For each new projection row, the configured `ExtractionChain`
//! decides whether the row is in scope for a workstream and, if so,
//! pulls typed entities + linked relations out of it. The runner
//! advances a per-(workstream, feed_type) cursor so subsequent runs
//! pick up only new rows.

pub mod chain;
pub mod error;
pub mod runner;

pub use chain::{ChainOutcome, ExtractionChain, StubChain};
pub use error::ExtractionError;
pub use runner::{ExtractorRunner, RunStats};

//! Per-feed-type projection layer for arawn.
//!
//! Projections sit between raw feed mirrors (on-disk files) and the
//! workstream palaces (typed entity graphs). Each feed item type
//! (gmail message, slack message, drive file, …) becomes a normalized
//! sqlite row, FTS5-indexed for text search and vector-indexed for
//! semantic search.
//!
//! Why this layer exists (per I-0040):
//! - Cross-feed semantic search without any workstream declared.
//! - A stable, queryable input to the per-workstream extractor in
//!   Phase 4.
//! - Decouples feed-side fidelity (raw mirror) from query-side shape.

pub mod atlassian;
pub mod calendar;
pub mod dispatch;
pub mod drive;
pub mod error;
pub mod gmail;
pub mod schema;
pub mod slack;
pub mod store;
pub mod types;

pub use dispatch::project_feed_dir;
pub use error::ProjectionError;
pub use store::{ProjectionStore, WriteOutcome};
pub use types::{Projection, ProjectionRow};

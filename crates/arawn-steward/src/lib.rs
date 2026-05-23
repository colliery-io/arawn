//! Per-workstream KB maintenance — Phase 5 of I-0040.
//!
//! The steward continuously re-reads each workstream's KB and applies
//! four maintenance subroutines (re-shelve / dust / map / door-watch).
//! ARAWN-A-0003 codifies the bounded-blast-radius contract every
//! subroutine respects.
//!
//! Public surface:
//!
//! - `Journal` / `JournalRecord` / `JournalRow`: append-only journal
//!   colocated with each workstream's KB; write-ahead + rollback API.
//! - `StewardSubroutine`: trait every subroutine implements.
//! - Four production subroutines: `DoorWatchSubroutine`, `MapSubroutine`,
//!   `ReshelveSubroutine`, `TagPromoterSubroutine`, plus `DustEngine` /
//!   `ClusterMode` for the dust pass.
//! - `StewardRunner`: walks the list of active workstreams and runs
//!   each subroutine sequentially against each KB.
//! - `AcceptCtx` / `RollbackCtx`: bridge structs for the /workstream
//!   accept / rollback tool surfaces.

pub mod accept;
pub mod cursor;
pub mod doorwatch;
pub mod dust;
pub mod error;
pub mod journal;
pub mod llm_text;
pub mod map;
pub mod reshelve;
pub mod rollback;
pub mod runner;
pub mod subroutine;
pub mod tag_promoter;

pub use accept::AcceptCtx;
pub use cursor::CursorStore;
pub use doorwatch::DoorWatchSubroutine;
pub use dust::{ClusterMode, DustEngine, DustOpts};
pub use error::StewardError;
pub use journal::{Journal, JournalRecord, JournalRow};
pub use map::MapSubroutine;
pub use reshelve::ReshelveSubroutine;
pub use rollback::RollbackCtx;
pub use runner::StewardRunner;
pub use subroutine::StewardSubroutine;
pub use tag_promoter::TagPromoterSubroutine;

//! Per-workstream KB maintenance — Phase 5 of I-0040.
//!
//! The steward continuously re-reads each workstream's KB and applies
//! four maintenance subroutines (re-shelve / dust / map / door-watch).
//! ARAWN-A-0003 codifies the bounded-blast-radius contract every
//! subroutine respects.
//!
//! This crate (T-0256) ships the *scaffolding* only:
//!
//! - `Journal`: append-only `steward_journal` table colocated with each
//!   workstream's KB; write-ahead + rollback API.
//! - `StewardSubroutine`: trait every subroutine implements.
//! - `IdentitySubroutine`: a no-op subroutine that writes a journal row
//!   so the scaffolding is exercisable end-to-end.
//! - `StewardRunner`: walks the list of active workstreams and runs
//!   each subroutine sequentially against each KB.
//!
//! T-0257 lands re-shelve + dust (the mutating subroutines).
//! T-0258 lands map + door-watch (proposal-only).
//! T-0259 wires the /workstream refine / journal / rollback commands.

pub mod cursor;
pub mod doorwatch;
pub mod error;
pub mod journal;
pub mod llm_text;
pub mod map;
pub mod reshelve;
pub mod rollback;
pub mod runner;
pub mod subroutine;

pub use cursor::CursorStore;
pub use doorwatch::{DoorWatchConfig, DoorWatchSubroutine};
pub use error::StewardError;
pub use journal::{Journal, JournalRecord, JournalRow, RevertResult};
pub use map::{MapConfig, MapSubroutine};
pub use reshelve::{ReshelveConfig, ReshelveSubroutine};
pub use runner::{StewardRunner, StewardStats, SubroutineCaps};
pub use subroutine::{IdentitySubroutine, StewardSubroutine, SubroutineCtx, SubroutineOutcome};

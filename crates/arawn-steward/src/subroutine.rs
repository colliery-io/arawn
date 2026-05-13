//! The `StewardSubroutine` trait.
//!
//! Subroutines own their own LLM call (or none, for the identity stub),
//! their own cap enforcement, and their own write-ahead journaling.
//! They receive a `SubroutineCtx` that bundles the workstream record,
//! its `MemoryManager`, its `Journal`, and the configured cap for this
//! subroutine on this pass.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use arawn_core::Workstream;
use arawn_memory::MemoryManager;

use crate::error::StewardError;
use crate::journal::{Journal, JournalRecord};

/// Per-pass context handed to a subroutine. The runner constructs one
/// before each subroutine run.
pub struct SubroutineCtx {
    pub workstream: Workstream,
    pub memory: Arc<MemoryManager>,
    pub journal: Arc<Journal>,
    /// Maximum actions this subroutine may apply on this pass. Per
    /// ARAWN-A-0003 the steward stops at the cap and writes a journal
    /// note rather than throwing — `applied < cap_hit` is fine.
    pub cap: usize,
}

/// What a subroutine did. `actions_journaled` is the truthful count
/// regardless of whether actions were applied vs proposal-only; the
/// runner uses it to populate per-pass stats.
#[derive(Debug, Default, Clone)]
pub struct SubroutineOutcome {
    pub actions_journaled: usize,
    pub mutations_applied: usize,
    pub proposals_recorded: usize,
    pub cap_hit: bool,
}

#[async_trait]
pub trait StewardSubroutine: Send + Sync {
    /// Stable name (`reshelve`, `dust`, `map`, `doorwatch`, `identity`).
    /// Goes into the journal `subroutine` column.
    fn name(&self) -> &str;

    /// Whether this subroutine mutates the KB (true for re-shelve /
    /// dust) or only emits proposals (false for map / door-watch).
    /// Proposals get `applied=false` journal rows. The runner refuses
    /// to start a non-mutating subroutine that tries to issue mutations
    /// — currently honored on the trust system; T-0257/T-0258 land the
    /// concrete impls.
    fn is_mutating(&self) -> bool;

    /// Run one pass against the workstream's KB.
    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError>;
}

/// No-op subroutine that writes exactly one journal row per invocation
/// — used to prove the scaffolding works end-to-end before T-0257
/// lands the real subroutines.
pub struct IdentitySubroutine {
    name: String,
}

impl Default for IdentitySubroutine {
    fn default() -> Self {
        Self::new("identity")
    }
}

impl IdentitySubroutine {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl StewardSubroutine for IdentitySubroutine {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_mutating(&self) -> bool {
        // The identity subroutine never touches the KB graph; treating
        // it as non-mutating exercises the proposal-shaped journal path
        // by default. T-0256 tests assert this contract.
        false
    }

    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError> {
        debug!(
            workstream = %ctx.workstream.name,
            subroutine = %self.name,
            cap = ctx.cap,
            "identity steward subroutine running"
        );
        let record = JournalRecord {
            subroutine: self.name.clone(),
            action: "noop".into(),
            inputs_json: serde_json::json!({"cap": ctx.cap}).to_string(),
            outputs_json: "{}".into(),
            model: "n/a".into(),
            prompt_hash: Journal::prompt_hash("identity"),
            applied: false,
        };
        ctx.journal.write_ahead(&record)?;
        Ok(SubroutineOutcome {
            actions_journaled: 1,
            mutations_applied: 0,
            proposals_recorded: 1,
            cap_hit: false,
        })
    }
}

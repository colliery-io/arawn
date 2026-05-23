//! The pluggable extraction chain.
//!
//! `ExtractionChain` is the trait every extractor implementation
//! satisfies. T-0251 ships `StubChain` (always skips) so the rest of
//! the plumbing — runner, cursor advancement, dispatch hook — can be
//! tested end-to-end without an LLM. T-0252 lands `CotChain`, the
//! real 4-stage chain (classify → extract → link-by-name → write).

use async_trait::async_trait;
use uuid::Uuid;

use arawn_core::Workstream;
use arawn_memory::MemoryManager;
use arawn_projections::ProjectionRow;

use crate::error::ExtractionError;

/// Per-row outcome of a single chain run.
#[derive(Debug, Clone, Default)]
pub struct ChainOutcome {
    /// Entities written or reinforced in the workstream's KB.
    pub entities_written: Vec<Uuid>,
    /// Number of relations added (not including provenance edges).
    pub relations_written: usize,
    /// True when classify decided this row is out of scope.
    pub skipped: bool,
}

#[async_trait]
pub trait ExtractionChain: Send + Sync {
    /// Process a single projection row in the context of one
    /// workstream. The `kb` is the routed memory manager for that
    /// workstream (global + workstream tier).
    async fn run(
        &self,
        workstream: &Workstream,
        row: &ProjectionRow,
        kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError>;
}

/// No-op chain. Always returns `skipped: true`. Used by T-0251's
/// integration tests to verify the cursor / dispatch path without
/// dragging in an LLM.
pub struct StubChain;

#[async_trait]
impl ExtractionChain for StubChain {
    async fn run(
        &self,
        _workstream: &Workstream,
        _row: &ProjectionRow,
        _kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError> {
        Ok(ChainOutcome {
            entities_written: Vec::new(),
            relations_written: 0,
            skipped: true,
        })
    }
}

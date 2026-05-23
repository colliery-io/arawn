//! Errors surfaced by the ceremony engine and plugins.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CeremonyError {
    /// The compose path tried to write an item without a `citation_id`.
    /// The two-write-path contract from I-0043 §Design Decisions #4
    /// forbids this — composed items must cite a gather payload row.
    #[error("missing citation: {0}")]
    MissingCitation(String),

    /// A plugin's `kind()` collides with one already registered.
    #[error("duplicate ceremony kind: {0}")]
    DuplicateKind(String),

    /// The plugin tried to act on a tablet that doesn't exist or is
    /// in the wrong status.
    #[error("invalid tablet state: {0}")]
    InvalidTabletState(String),

    /// The pattern detector returned no rows because there is not
    /// enough history yet. Not an error per se; the engine handles
    /// this as a graceful fallback (bootstrap path), but the
    /// detector framework distinguishes "explicitly insufficient"
    /// from "computed and found nothing".
    #[error("insufficient history: {0}")]
    InsufficientHistory(String),

    /// Wrapping any storage-layer error so the engine doesn't have
    /// to leak its DB choice into the plugin trait surface.
    #[error("storage: {0}")]
    Storage(String),

    /// Wrapping any LLM-layer error (the compose phase).
    #[error("llm: {0}")]
    Llm(String),

    /// Anything else — kept as a catch-all so individual plugins
    /// don't have to extend the error type for one-off cases.
    #[error("{0}")]
    Other(String),
}

impl CeremonyError {
    pub fn missing_citation(detail: impl Into<String>) -> Self {
        Self::MissingCitation(detail.into())
    }
    pub fn duplicate_kind(kind: impl Into<String>) -> Self {
        Self::DuplicateKind(kind.into())
    }
    pub fn invalid_tablet_state(detail: impl Into<String>) -> Self {
        Self::InvalidTabletState(detail.into())
    }
    pub fn insufficient_history(detail: impl Into<String>) -> Self {
        Self::InsufficientHistory(detail.into())
    }
}

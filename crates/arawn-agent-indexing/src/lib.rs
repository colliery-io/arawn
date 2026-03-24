//! Session indexing, fact extraction, and NER for Arawn.
//!
//! This crate provides post-conversation processing: extracting facts,
//! entities, and relationships from session transcripts, then storing
//! them in the memory system for future recall.
//!
//! Extracted from `arawn-agent` as an independent crate — it has zero
//! dependency on the agent core or tool framework.

pub mod extraction;
#[cfg(feature = "gliner")]
pub mod gliner;
pub mod indexer;
pub mod ner;
mod report;
pub mod summarization;
mod types;

pub use extraction::ExtractionPrompt;
#[cfg(feature = "gliner")]
pub use gliner::GlinerEngine;
pub use indexer::{Completer, IndexerConfig, SessionIndexer};
pub use ner::{NerConfig, NerEngine, NerExtraction, NerOutput, NerRelation, NerSpan};
pub use report::IndexReport;
pub use summarization::SummarizationPrompt;
pub use types::{ExtractedEntity, ExtractedFact, ExtractedRelationship, ExtractionResult};

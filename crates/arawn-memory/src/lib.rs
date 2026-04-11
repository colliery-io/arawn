//! Knowledge base memory system for arawn.
//!
//! Provides graph-backed entity storage with FTS5 search, typed relations,
//! confidence scoring, tag support, and search-before-create deduplication.

pub mod error;
pub mod inject;
pub mod manager;
pub mod shortcodes;
pub mod stack;
pub mod store;
pub mod types;
pub mod vector;

pub use error::MemoryError;
pub use inject::load_memories_for_injection;
pub use manager::{MemoryManager, try_open_memory};
pub use stack::MemoryStack;
pub use store::MemoryStore;
pub use types::*;
pub use vector::{
    SimilarityResult, init_vector_extension, check_vector_extension,
};

//! Built-in tool implementations for the Arawn agent.
//!
//! Each tool implements `arawn_agent::Tool` and provides a specific capability:
//! file operations, shell execution, web fetching, search, memory, etc.
//!
//! Extracted from `arawn-agent` to reduce compile-time coupling.
//! Pipeline-dependent tools (catalog, workflow) are behind the `pipeline` feature.

#[cfg(feature = "pipeline")]
mod catalog;
mod delegate;
mod explore;
mod file;
mod memory;
mod note;
mod search;
mod shell;
mod think;
mod web;
#[cfg(feature = "pipeline")]
mod workflow;

// File tools
pub use file::{FileReadTool, FileWriteTool};

// Note tool
pub use note::{Note, NoteStorage, NoteTool, new_note_storage};

// Shell tool
pub use shell::{ShellConfig, ShellTool};

// Web tools
pub use web::{
    SearchProvider, SearchResult, WebFetchConfig, WebFetchTool, WebSearchConfig, WebSearchTool,
};

// Search tools
pub use search::{GlobTool, GrepTool};

// Memory tool
pub use memory::MemorySearchTool;

// Think tool
pub use think::ThinkTool;

// Delegate tool
pub use delegate::DelegateTool;

// Explore tool
pub use explore::ExploreTool;

// Pipeline tools (feature-gated)
#[cfg(feature = "pipeline")]
pub use catalog::CatalogTool;
#[cfg(feature = "pipeline")]
pub use workflow::WorkflowTool;

// Integration tests for ExploreTool (moved from arawn-agent/rlm)
#[cfg(test)]
mod explore_integration_tests;

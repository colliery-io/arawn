//! Hooks system — lifecycle event interception and automation.
//!
//! The hooks system intercepts lifecycle events (tool execution, session
//! boundaries, permission prompts, compaction, etc.) and runs user-configured
//! shell commands. Hooks are configured in settings.json and matched by
//! event type + optional tool name / content patterns.

mod config;
mod events;
mod executor;
mod file_watcher;
mod loader;
mod matcher;
mod runner;

pub use config::{
    AggregatedHookResult, CommandHookDef, HookConfig, HookGroup, HookResult,
};
pub use events::{HookEvent, HookInput};
pub use executor::CommandHookExecutor;
pub use file_watcher::HookFileWatcher;
pub use loader::{load_hooks_from_file, load_merged_hooks};
pub use matcher::HookMatcher;
pub use runner::HookRunner;

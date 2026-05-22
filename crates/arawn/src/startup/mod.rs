//! Startup-time helpers extracted from `main.rs`.
//! Grouped by concern; each submodule is `pub` so `main` can use them.

pub mod ceremonies;
pub mod cli;
pub mod engine;
pub mod feeds;
pub mod feeds_helpers;
pub mod helpers;
pub mod integrations;

pub use cli::run_cli_via_server;
pub use engine::build_engine_config;
pub use feeds_helpers::{expand_github_org, register_one_feed};
pub use helpers::{
    build_llm_client, connect_mcp_servers, dirs_path, register_default_tools,
    register_workflow_tools,
};

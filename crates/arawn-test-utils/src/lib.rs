//! Shared test utilities for the Arawn workspace.
//!
//! Provides common infrastructure for integration and unit tests
//! across all crates: test servers, WebSocket clients, fixtures,
//! and streaming mock backends.

pub mod assertions;
pub mod fixtures;
pub mod mock_backend;
pub mod mock_tools;
pub mod server;
pub mod sse;
pub mod ws_client;

pub use fixtures::TestFixtures;
pub use mock_backend::{
    ScriptedInvocation, ScriptedMockBackend, StreamingMockBackend, StreamingMockEvent,
};
pub use mock_tools::{
    EchoTool, FailTool, LargeOutputTool, MockReadFileTool, SlowTool, mock_tool_registry,
};
pub use server::TestServer;
pub use sse::{SseEvent, collect_sse_events, events_of_type, reconstruct_text};
pub use ws_client::TestWsClient;

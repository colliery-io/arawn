//! Shared test utilities for the Arawn workspace.
//!
//! Provides common infrastructure for integration and unit tests
//! across all crates: test servers, WebSocket clients, fixtures,
//! and streaming mock backends.

pub mod assertions;
pub mod fixtures;
pub mod mock_backend;
pub mod server;
pub mod ws_client;

pub use fixtures::TestFixtures;
pub use mock_backend::{StreamingMockBackend, StreamingMockEvent};
pub use server::TestServer;
pub use ws_client::TestWsClient;

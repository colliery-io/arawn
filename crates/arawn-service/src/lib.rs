pub mod error;
pub mod types;

use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

pub use error::ServiceError;
pub use types::{EngineEvent, ModalPromptOption, SessionDetail, SessionInfo, WorkstreamInfo};

/// The service contract between any UI client and the Arawn backend.
///
/// Implementations:
/// - `LocalService` (in-process, wraps engine + store directly)
/// - Future: `RemoteService` (WebSocket client to a running daemon)
#[async_trait]
pub trait ArawnService: Send + Sync {
    // --- Workstreams ---

    /// List all workstreams.
    async fn list_workstreams(&self) -> Result<Vec<WorkstreamInfo>, ServiceError>;

    /// Create a new workstream.
    async fn create_workstream(
        &self,
        name: String,
        root_dir: PathBuf,
    ) -> Result<WorkstreamInfo, ServiceError>;

    // --- Sessions ---

    /// List sessions, optionally filtered by workstream. Pass `None` for scratch sessions.
    async fn list_sessions(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<Vec<SessionInfo>, ServiceError>;

    /// Create a new session in a workstream. Pass `None` for scratch.
    async fn create_session(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<SessionInfo, ServiceError>;

    /// Load a session with its full message history.
    async fn load_session(&self, id: Uuid) -> Result<SessionDetail, ServiceError>;

    // --- Chat ---

    /// Send a message and receive a stream of engine events (streaming text, tool calls, completion).
    async fn send_message(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn Stream<Item = EngineEvent> + Send>>, ServiceError>;

    /// Cancel an in-progress generation.
    async fn cancel(&self, session_id: Uuid) -> Result<(), ServiceError>;
}

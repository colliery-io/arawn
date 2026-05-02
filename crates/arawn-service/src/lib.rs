pub mod error;
pub mod types;

use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

pub use error::ServiceError;
pub use types::{
    CommandInfo, EngineEvent, ForgetCandidate, ForgetResult, InventoryItem, MemoryStoreResult,
    MemoryStoreSummary, MemorySummary, MemoryTypeCount, ModalPromptOption, PermissionModeInfo,
    PromotionResult, ServerCapabilities, SessionDetail, SessionInfo, WorkflowInfo, WorkstreamInfo,
};

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

    // --- Session Management ---

    /// Promote a scratch session to a named workstream.
    async fn promote_session(
        &self,
        session_id: Uuid,
        workstream_name: &str,
    ) -> Result<PromotionResult, ServiceError>;

    /// Resolve a pending user input modal by delivering the selected index.
    async fn resolve_user_input(
        &self,
        request_id: &str,
        selected_index: Option<usize>,
    ) -> Result<(), ServiceError>;

    // --- Inventory & Commands ---

    /// Query available inventory (tools, skills, plugins, agents, mcp).
    async fn query_inventory(&self, kind: &str) -> Result<Vec<InventoryItem>, ServiceError>;

    /// List available commands for autocomplete.
    async fn list_available_commands(&self) -> Result<Vec<CommandInfo>, ServiceError>;

    /// List installed workflows.
    async fn list_workflows(&self) -> Result<Vec<WorkflowInfo>, ServiceError>;

    // --- Memory ---

    /// Store a fact in the knowledge base.
    async fn remember_fact(&self, text: &str) -> Result<MemoryStoreResult, ServiceError>;

    /// Get a summary of the knowledge base.
    async fn memory_summary(&self) -> Result<MemorySummary, ServiceError>;

    /// Forget/delete an entity from the knowledge base.
    async fn forget_entity(&self, query: &str) -> Result<ForgetResult, ServiceError>;

    // --- Permissions ---

    /// Get the current permission mode.
    async fn get_permission_mode(&self) -> Result<PermissionModeInfo, ServiceError>;

    /// Set the permission mode. Returns the new mode.
    async fn set_permission_mode(&self, mode: &str) -> Result<PermissionModeInfo, ServiceError>;

    // --- Capabilities ---

    /// Report which optional subsystems initialized successfully. Clients
    /// call this on connect to surface degraded-functionality warnings
    /// (e.g. memory falls back to FTS-only when embeddings_available=false)
    /// before the user runs into them mid-conversation.
    async fn get_capabilities(&self) -> Result<ServerCapabilities, ServiceError>;
}

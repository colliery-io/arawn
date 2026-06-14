pub mod error;
pub mod types;

use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

pub use error::ServiceError;
pub use types::{
    CeremoniesStatus, CeremonyRunStatus, CommandInfo, EmbeddingStatus, EngineEvent,
    ExtractionCursor, ExtractionLogEntry, ExtractionStatus, FeedDiscoverDto, FeedDiscoverRow,
    FeedParamKindDto, FeedParamSpecDto, FeedRegisterSpec, FeedRemoveDto, FeedSchemaDto,
    FeedStatusRow, FeedSummaryDto, FeedTemplateInfo, FeedsStatus, ForgetCandidate, ForgetResult,
    HealthStatus, IntegrationStatus, InventoryItem, LensInfo, LlmClientStatus, LlmStatus,
    MemorySearchResult, MemoryStoreResult, MemoryStoreSummary, MemorySummary, MemoryTypeCount,
    ModalPromptOption, OAuthFlowStarted, PermissionAuditEntry, PermissionModeInfo,
    PermissionsStatus, SYSTEM_STATUS_VERSION, ServerCapabilities, ServerNotice, SessionDetail,
    SessionInfo, SignalDto, StewardErrorStatus, StewardStatus, SystemStatus, WorkflowInfo,
};

/// The service contract between any UI client and the Arawn backend.
///
/// Implementations:
/// - `LocalService` (in-process, wraps engine + store directly)
/// - Future: `RemoteService` (WebSocket client to a running daemon)
#[async_trait]
pub trait ArawnService: Send + Sync {
    // --- Lenses ---

    /// List all lenses.
    async fn list_lenses(&self) -> Result<Vec<LensInfo>, ServiceError>;

    /// Create a new lens.
    async fn create_lens(&self, name: String, root_dir: PathBuf) -> Result<LensInfo, ServiceError>;

    // --- Sessions ---

    /// List sessions, optionally filtered by lens. Pass `None` for scratch sessions.
    async fn list_sessions(&self, lens_id: Option<Uuid>) -> Result<Vec<SessionInfo>, ServiceError>;

    /// Create a new session in a lens. Pass `None` for scratch.
    async fn create_session(&self, lens_id: Option<Uuid>) -> Result<SessionInfo, ServiceError>;

    /// Load a session with its full message history.
    async fn load_session(&self, id: Uuid) -> Result<SessionDetail, ServiceError>;

    /// Promote a session into a named lens (ARAWN-T-0480, closes T-0012) —
    /// atomically re-points its SQLite binding and moves its JSONL file.
    /// Backs the TUI's `/lens promote`. Returns the updated session info.
    async fn promote_session(
        &self,
        session_id: Uuid,
        lens_id: Uuid,
    ) -> Result<SessionInfo, ServiceError>;

    /// Truncate a session back to a specific user-message index, dropping
    /// everything after (inclusive of the Nth user message and all the
    /// assistant / tool-call / tool-result messages that followed it).
    /// Used by the TUI's "branch from a prior prompt" flow — the user
    /// picks a prompt to rewind to from the history modal, the session
    /// is truncated to just before it, then the prompt is loaded into
    /// the input buffer for editing and re-submission.
    ///
    /// `user_message_index` is 0-based: 0 truncates everything (back to
    /// just before the first user prompt). `n` ≥ the count of user
    /// messages is a no-op. Returns the new (truncated) session detail.
    async fn truncate_session_at_user_message(
        &self,
        id: Uuid,
        user_message_index: usize,
    ) -> Result<SessionDetail, ServiceError>;

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

    /// Read-only free-text search of the global knowledge base (T-0499).
    /// Distinct from `forget_entity`, which searches to delete.
    async fn memory_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, ServiceError>;

    // --- Inspection (GUI surfaces, ARAWN-I-0070) ---

    /// Recent extracted signals across every lens KB, newest/most-confident
    /// first, each labeled with its source lens (T-0498).
    async fn list_signals(&self, limit: usize) -> Result<Vec<SignalDto>, ServiceError>;

    /// Recent extraction-log rows (per-(lens, projection) run outcomes),
    /// newest first — extraction provenance (T-0499).
    async fn extraction_log(&self, limit: usize) -> Result<Vec<ExtractionLogEntry>, ServiceError>;

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

    /// Snapshot the current permission rules + recent audit entries.
    /// Backs the TUI's `/permissions` command. Read-only — modifying rules
    /// requires editing `arawn.toml` and (for now) restarting.
    async fn get_permissions_status(&self) -> Result<PermissionsStatus, ServiceError>;

    // --- Health & Status (ARAWN-I-0068 P2-1) ---

    /// Cheap liveness/readiness probe. `ready` flips true only after the
    /// post-startup wiring (feeds/ceremonies/memory/storage) completes;
    /// `blocking` lists why it isn't ready yet. Clients consult this before
    /// driving the server so they don't fire `/watch`/`/ceremony` into a
    /// half-initialized backend.
    async fn health(&self) -> Result<HealthStatus, ServiceError>;

    /// Versioned per-subsystem health dump for the status panel. Aggregates
    /// live state — feeds, ceremonies, embedding, extraction, LLM clients —
    /// into the [`SystemStatus`] contract that the TUI and future web GUI
    /// both render.
    async fn status(&self) -> Result<SystemStatus, ServiceError>;

    // --- Integrations ---

    /// List registered integrations and whether each one has stored
    /// credentials. Backs the TUI's `/integrations` command.
    async fn list_integrations(&self) -> Result<Vec<IntegrationStatus>, ServiceError>;

    /// Begin an OAuth (or other credential-acquisition) flow for `service`.
    /// Returns the auth URL the user should open; the rest of the flow
    /// runs asynchronously on the server, with completion broadcast as a
    /// `ServerNotice` with category="integration".
    async fn start_oauth_flow(&self, service: &str) -> Result<OAuthFlowStarted, ServiceError>;

    /// Drop stored credentials for `service`. Idempotent.
    async fn disconnect_integration(&self, service: &str) -> Result<(), ServiceError>;

    // --- Feeds (continual data acquisition) ---

    /// Register a new feed at runtime. Backs the `/watch` slash
    /// command. Returns the freshly-created feed summary.
    async fn feed_register(&self, spec: FeedRegisterSpec) -> Result<FeedSummaryDto, ServiceError>;

    /// List every configured feed (enabled + paused) with last-run
    /// status and on-disk size. Backs `/feeds`.
    async fn feed_list(&self) -> Result<Vec<FeedSummaryDto>, ServiceError>;

    /// Pause a feed: drop its cron schedule, leave the data dir
    /// intact. Idempotent. Backs `/feeds pause <id>`.
    async fn feed_pause(&self, feed_id: &str) -> Result<FeedSummaryDto, ServiceError>;

    /// Resume a paused feed: re-register its cron schedule with the
    /// persisted cadence + cursor. Backs `/feeds resume <id>`.
    async fn feed_resume(&self, feed_id: &str) -> Result<FeedSummaryDto, ServiceError>;

    /// Decommission a feed: drop the cron schedule, delete the row,
    /// recursively delete the data dir. Returns the count of bytes
    /// wiped so the caller can confirm. Backs `/feeds rm <id>`.
    async fn feed_remove(&self, feed_id: &str) -> Result<FeedRemoveDto, ServiceError>;

    /// Trigger a one-off run of a feed, outside its cron schedule.
    /// Backs `/feeds run <id>`. Returns the post-run summary
    /// (refreshed last_run_at, last_status, etc.).
    async fn feed_run(&self, feed_id: &str) -> Result<FeedSummaryDto, ServiceError>;

    /// Discover the choosable parameter values for a template. Backs
    /// the `/watch <template> <feed_id>` picker. Returns `None`
    /// (rendered as an empty list with a `picker_supported=false`
    /// flag) when the template doesn't support discovery.
    async fn feed_discover(&self, template: &str) -> Result<FeedDiscoverDto, ServiceError>;

    /// Fetch a template's parameter schema + default cadence. Backs the
    /// `/watch` modal form (no args). Errors if the template is unknown.
    async fn feed_schema(&self, template: &str) -> Result<FeedSchemaDto, ServiceError>;

    /// List every registered feed template (name + one-line description) for
    /// the `/watch` modal's stage-1 picker.
    async fn feed_templates(&self) -> Result<Vec<FeedTemplateInfo>, ServiceError>;
}

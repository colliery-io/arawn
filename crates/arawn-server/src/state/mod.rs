//! Application state shared across handlers.
//!
//! State is separated into two layers:
//! - `SharedServices`: Immutable services created at startup
//! - `RuntimeState`: Mutable state that changes during operation
//!
//! # Lock Ordering
//!
//! To prevent deadlocks, locks in `RuntimeState` must always be acquired in this
//! order. Never hold a higher-numbered lock while acquiring a lower-numbered one.
//!
//! 1. `pending_reconnects` — ownership recovery after disconnect
//! 2. `session_owners` — session-to-connection ownership map
//! 3. `session_cache.inner` — LRU cache of active sessions
//! 4. `mcp_manager` — MCP server registry (in `SharedServices`)
//! 5. `tasks` — background task tracking
//!
//! The `ws_connection_tracker` lock is independent and may be acquired at any
//! point since it never nests with the above locks.
//!
//! **Guidelines:**
//! - Release locks before spawning tasks that acquire locks.
//! - Prefer `read()` over `write()` when mutation is not needed.
//! - Keep critical sections short — clone data out, then drop the guard.
//! - See `docs/src/architecture/concurrency.md` for the full concurrency guide.

mod tasks;
mod ws_tracker;

pub use tasks::*;
pub use ws_tracker::*;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;

use arawn_domain::{
    Agent, Compressor, DirectoryManager, DomainServices, McpManager, MemoryStore, SandboxManager,
    Session, SessionId, SessionIndexer, WatcherHandle, WorkstreamManager,
};
use arawn_types::{HasSessionConfig, SharedHookDispatcher};
use axum::response::Response;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ServerConfig;
use crate::ratelimit::{SharedRateLimiter, create_rate_limiter};
use crate::routes::ws::ConnectionId;
use crate::session_cache::SessionCache;

// ─────────────────────────────────────────────────────────────────────────────
// Session Ownership Types
// ─────────────────────────────────────────────────────────────────────────────

/// Session ownership tracking - maps session IDs to owning connection IDs.
/// First subscriber to a session becomes the owner; others are readers.
pub type SessionOwners = Arc<RwLock<HashMap<SessionId, ConnectionId>>>;

/// Pending reconnect entry for session ownership recovery after disconnect.
#[derive(Debug, Clone)]
pub struct PendingReconnect {
    /// The token required to reclaim ownership.
    pub token: String,
    /// When this pending reconnect expires.
    pub expires_at: std::time::Instant,
}

impl PendingReconnect {
    /// Create a new pending reconnect with the given grace period.
    pub fn new(token: String, grace_period: std::time::Duration) -> Self {
        Self {
            token,
            expires_at: std::time::Instant::now() + grace_period,
        }
    }

    /// Check if this pending reconnect has expired.
    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() > self.expires_at
    }
}

/// Pending reconnects storage - maps session IDs to pending reconnect entries.
pub type PendingReconnects = Arc<RwLock<HashMap<SessionId, PendingReconnect>>>;

/// Active WebSocket connections — tracks which connection IDs are currently alive.
/// Used to detect stale session ownership from dead connections.
pub type ActiveConnections = Arc<RwLock<HashSet<ConnectionId>>>;

/// Thread-safe MCP manager.
pub type SharedMcpManager = Arc<RwLock<McpManager>>;

// ─────────────────────────────────────────────────────────────────────────────
// Shared Services (Immutable)
// ─────────────────────────────────────────────────────────────────────────────

/// Immutable services created at startup.
///
/// These services are configured once when the server starts and never change
/// during operation. They can be safely shared across all handlers without locks.
#[derive(Clone)]
pub struct SharedServices {
    /// The agent instance for LLM interactions.
    pub agent: Arc<Agent>,

    /// Server configuration.
    pub config: Arc<ServerConfig>,

    /// Per-IP rate limiter (created from config.api_rpm).
    pub rate_limiter: SharedRateLimiter,

    /// Workstream manager (optional — None if workstreams not configured).
    pub workstreams: Option<Arc<WorkstreamManager>>,

    /// Session indexer (optional — None when indexing disabled).
    pub indexer: Option<Arc<SessionIndexer>>,

    /// Hook dispatcher for session lifecycle events (optional).
    pub hook_dispatcher: Option<SharedHookDispatcher>,

    /// MCP manager for Model Context Protocol servers (optional — None if MCP disabled).
    pub mcp_manager: Option<SharedMcpManager>,

    /// Directory manager for workstream/session path management.
    pub directory_manager: Option<Arc<DirectoryManager>>,

    /// Sandbox manager for secure shell command execution.
    pub sandbox_manager: Option<Arc<SandboxManager>>,

    /// File watcher for filesystem monitoring (optional — None if monitoring disabled).
    pub file_watcher: Option<Arc<WatcherHandle>>,

    /// Memory store for persistent notes and memories (optional — None when memory disabled).
    pub memory_store: Option<Arc<MemoryStore>>,

    /// Domain services facade for unified service access.
    pub domain: Option<Arc<DomainServices>>,

    /// Session/workstream compressor for LLM-based summarization.
    pub compressor: Option<Arc<Compressor>>,
}

impl SharedServices {
    /// Create new shared services with the given agent and config.
    pub fn new(agent: Agent, config: ServerConfig) -> Self {
        let rate_limiter = create_rate_limiter(config.api_rpm);

        Self {
            agent: Arc::new(agent),
            config: Arc::new(config),
            rate_limiter,
            workstreams: None,
            indexer: None,
            hook_dispatcher: None,
            mcp_manager: None,
            directory_manager: None,
            sandbox_manager: None,
            file_watcher: None,
            memory_store: None,
            domain: None,
            compressor: None,
        }
    }

    /// Configure workstream support.
    pub fn with_workstreams(mut self, manager: WorkstreamManager) -> Self {
        self.workstreams = Some(Arc::new(manager));
        self
    }

    /// Configure session indexer.
    pub fn with_indexer(mut self, indexer: SessionIndexer) -> Self {
        self.indexer = Some(Arc::new(indexer));
        self
    }

    /// Configure hook dispatcher for lifecycle events.
    pub fn with_hook_dispatcher(mut self, dispatcher: SharedHookDispatcher) -> Self {
        self.hook_dispatcher = Some(dispatcher);
        self
    }

    /// Configure MCP manager.
    pub fn with_mcp_manager(mut self, manager: McpManager) -> Self {
        self.mcp_manager = Some(Arc::new(RwLock::new(manager)));
        self
    }

    /// Configure directory manager for path management.
    pub fn with_directory_manager(mut self, manager: DirectoryManager) -> Self {
        self.directory_manager = Some(Arc::new(manager));
        self
    }

    /// Configure sandbox manager for shell execution.
    pub fn with_sandbox_manager(mut self, manager: SandboxManager) -> Self {
        self.sandbox_manager = Some(Arc::new(manager));
        self
    }

    /// Configure file watcher for filesystem monitoring.
    pub fn with_file_watcher(mut self, watcher: WatcherHandle) -> Self {
        self.file_watcher = Some(Arc::new(watcher));
        self
    }

    /// Configure memory store for persistent notes and memories.
    pub fn with_memory_store(mut self, store: Arc<MemoryStore>) -> Self {
        self.memory_store = Some(store);
        self
    }

    /// Configure session/workstream compressor.
    pub fn with_compressor(mut self, compressor: Compressor) -> Self {
        self.compressor = Some(Arc::new(compressor));
        self
    }

    /// Build domain services from the configured components.
    ///
    /// This creates a DomainServices instance from the current configuration.
    /// Should be called after all components are configured.
    pub fn build_domain_services(mut self) -> Self {
        let domain = DomainServices::new(
            self.agent.clone(),
            self.workstreams.clone(),
            self.directory_manager.clone(),
            self.indexer.clone(),
            self.mcp_manager.clone(),
            self.memory_store.clone(),
        );
        self.domain = Some(Arc::new(domain));
        self
    }

    /// Get the domain services facade.
    ///
    /// Returns `None` if `build_domain_services` wasn't called.
    pub fn domain(&self) -> Option<&Arc<DomainServices>> {
        self.domain.as_ref()
    }

    /// Get allowed paths for a session based on its workstream.
    ///
    /// Returns `None` if no directory manager is configured.
    pub fn allowed_paths(
        &self,
        workstream_id: &str,
        session_id: &str,
    ) -> Option<Vec<std::path::PathBuf>> {
        self.directory_manager
            .as_ref()
            .map(|dm| dm.allowed_paths(workstream_id, session_id))
    }

    /// Get a PathValidator for a session.
    ///
    /// Returns `None` if no directory manager is configured.
    pub fn path_validator(
        &self,
        workstream_id: &str,
        session_id: &str,
    ) -> Option<arawn_domain::PathValidator> {
        self.directory_manager
            .as_ref()
            .map(|dm| arawn_domain::PathValidator::for_session(dm, workstream_id, session_id))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime State (Mutable)
// ─────────────────────────────────────────────────────────────────────────────

/// Mutable state that changes during operation.
///
/// This state is modified by handlers during normal server operation.
/// Each field uses appropriate synchronization primitives.
///
/// # Lock Ordering
///
/// When acquiring multiple locks, always follow:
/// `pending_reconnects` < `session_owners` < `session_cache` < `tasks`
///
/// See the module-level documentation for the full ordering including
/// `SharedServices` locks.
#[derive(Clone)]
pub struct RuntimeState {
    /// Session cache - loads from workstream on cache miss, persists back on save.
    /// Lock order: 3 (after `session_owners`, before `tasks`).
    pub session_cache: SessionCache,

    /// Task store for tracking long-running operations.
    /// Lock order: 5 (last — acquire after all other locks).
    pub tasks: TaskStore,

    /// Session ownership tracking for WebSocket connections.
    /// Maps session IDs to the connection ID that owns them (first subscriber).
    /// Non-owners can subscribe as readers but cannot send Chat messages.
    /// Lock order: 2 (after `pending_reconnects`, before `session_cache`).
    pub session_owners: SessionOwners,

    /// Pending reconnects for session ownership recovery after disconnect.
    /// When a connection disconnects, ownership is held for a grace period
    /// allowing the client to reconnect with a token to reclaim ownership.
    /// Lock order: 1 (first — acquire before all other locks).
    pub pending_reconnects: PendingReconnects,

    /// Active WebSocket connections — tracks live connection IDs.
    /// Used to detect and evict stale session ownership from dead connections.
    /// Independent lock — does not nest with ownership locks.
    pub active_connections: ActiveConnections,

    /// WebSocket connection rate limiter per IP address.
    /// Independent lock — does not nest with any other locks.
    pub ws_connection_tracker: WsConnectionTracker,
}

impl RuntimeState {
    /// Create new runtime state.
    pub fn new() -> Self {
        Self {
            session_cache: SessionCache::new(None),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            session_owners: Arc::new(RwLock::new(HashMap::new())),
            pending_reconnects: Arc::new(RwLock::new(HashMap::new())),
            active_connections: Arc::new(RwLock::new(HashSet::new())),
            ws_connection_tracker: WsConnectionTracker::new(),
        }
    }

    /// Create runtime state with workstream-backed session cache.
    pub fn with_workstream_cache(workstreams: Arc<WorkstreamManager>) -> Self {
        Self {
            session_cache: SessionCache::new(Some(workstreams)),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            session_owners: Arc::new(RwLock::new(HashMap::new())),
            pending_reconnects: Arc::new(RwLock::new(HashMap::new())),
            active_connections: Arc::new(RwLock::new(HashSet::new())),
            ws_connection_tracker: WsConnectionTracker::new(),
        }
    }

    /// Configure session cache using a config provider.
    pub fn with_session_config<C: HasSessionConfig>(
        mut self,
        workstreams: Option<Arc<WorkstreamManager>>,
        config: &C,
    ) -> Self {
        self.session_cache = SessionCache::from_session_config(workstreams, config);
        self
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Application State (Combined)
// ─────────────────────────────────────────────────────────────────────────────

/// Application state shared across all handlers.
///
/// Composed of immutable `SharedServices` and mutable `RuntimeState`.
/// This separation provides:
/// - Clearer ownership semantics
/// - Better lock granularity (runtime state has finer-grained locks)
/// - Easier testing (services can be mocked independently)
#[derive(Clone)]
pub struct AppState {
    /// Immutable services created at startup.
    pub services: SharedServices,

    /// Mutable runtime state.
    pub runtime: RuntimeState,
}

impl AppState {
    /// Create a new application state.
    pub fn new(agent: Agent, config: ServerConfig) -> Self {
        Self {
            services: SharedServices::new(agent, config),
            runtime: RuntimeState::new(),
        }
    }

    /// Create application state with workstream support.
    pub fn with_workstreams(mut self, manager: WorkstreamManager) -> Self {
        let ws_arc = Arc::new(manager);
        self.runtime.session_cache = SessionCache::new(Some(ws_arc.clone()));
        self.services.workstreams = Some(ws_arc);
        self
    }

    /// Create application state with session indexer.
    pub fn with_indexer(mut self, indexer: SessionIndexer) -> Self {
        self.services = self.services.with_indexer(indexer);
        self
    }

    /// Create application state with hook dispatcher for lifecycle events.
    pub fn with_hook_dispatcher(mut self, dispatcher: SharedHookDispatcher) -> Self {
        self.services = self.services.with_hook_dispatcher(dispatcher);
        self
    }

    /// Create application state with MCP manager.
    pub fn with_mcp_manager(mut self, manager: McpManager) -> Self {
        self.services = self.services.with_mcp_manager(manager);
        self
    }

    /// Create application state with directory manager for path management.
    pub fn with_directory_manager(mut self, manager: DirectoryManager) -> Self {
        self.services = self.services.with_directory_manager(manager);
        self
    }

    /// Create application state with sandbox manager for shell execution.
    pub fn with_sandbox_manager(mut self, manager: SandboxManager) -> Self {
        self.services = self.services.with_sandbox_manager(manager);
        self
    }

    /// Create application state with file watcher for filesystem monitoring.
    pub fn with_file_watcher(mut self, watcher: WatcherHandle) -> Self {
        self.services = self.services.with_file_watcher(watcher);
        self
    }

    /// Create application state with session/workstream compressor.
    pub fn with_compressor(mut self, compressor: Compressor) -> Self {
        self.services = self.services.with_compressor(compressor);
        self
    }

    /// Configure session cache using a config provider.
    pub fn with_session_config<C: HasSessionConfig>(mut self, config: &C) -> Self {
        self.runtime.session_cache =
            SessionCache::from_session_config(self.services.workstreams.clone(), config);
        self
    }

    /// Build domain services from the configured components.
    ///
    /// This should be called after all services are configured to create
    /// the unified DomainServices facade.
    pub fn build_domain_services(mut self) -> Self {
        self.services = self.services.build_domain_services();
        self
    }

    // ── Convenience accessors ────────────────────────────────────────────────

    /// Get the agent.
    #[inline]
    pub fn agent(&self) -> &Arc<Agent> {
        &self.services.agent
    }

    /// Get the server config.
    #[inline]
    pub fn config(&self) -> &Arc<ServerConfig> {
        &self.services.config
    }

    /// Get the rate limiter.
    #[inline]
    pub fn rate_limiter(&self) -> &SharedRateLimiter {
        &self.services.rate_limiter
    }

    /// Get the workstream manager.
    #[inline]
    pub fn workstreams(&self) -> Option<&Arc<WorkstreamManager>> {
        self.services.workstreams.as_ref()
    }

    /// Get the session indexer.
    #[inline]
    pub fn indexer(&self) -> Option<&Arc<SessionIndexer>> {
        self.services.indexer.as_ref()
    }

    /// Get the hook dispatcher.
    #[inline]
    pub fn hook_dispatcher(&self) -> Option<&SharedHookDispatcher> {
        self.services.hook_dispatcher.as_ref()
    }

    /// Get the MCP manager.
    #[inline]
    pub fn mcp_manager(&self) -> Option<&SharedMcpManager> {
        self.services.mcp_manager.as_ref()
    }

    /// Get the directory manager.
    #[inline]
    pub fn directory_manager(&self) -> Option<&Arc<DirectoryManager>> {
        self.services.directory_manager.as_ref()
    }

    /// Get the sandbox manager.
    #[inline]
    pub fn sandbox_manager(&self) -> Option<&Arc<SandboxManager>> {
        self.services.sandbox_manager.as_ref()
    }

    /// Get the file watcher.
    #[inline]
    pub fn file_watcher(&self) -> Option<&Arc<WatcherHandle>> {
        self.services.file_watcher.as_ref()
    }

    /// Get the memory store.
    #[inline]
    pub fn memory_store(&self) -> Option<&Arc<MemoryStore>> {
        self.services.memory_store.as_ref()
    }

    /// Get the domain services facade.
    #[inline]
    pub fn domain(&self) -> Option<&Arc<DomainServices>> {
        self.services.domain()
    }

    /// Get the compressor.
    #[inline]
    pub fn compressor(&self) -> Option<&Arc<Compressor>> {
        self.services.compressor.as_ref()
    }

    /// Get the session cache.
    #[inline]
    pub fn session_cache(&self) -> &SessionCache {
        &self.runtime.session_cache
    }

    /// Get the task store.
    #[inline]
    pub fn tasks(&self) -> &TaskStore {
        &self.runtime.tasks
    }

    /// Get the session owners.
    #[inline]
    pub fn session_owners(&self) -> &SessionOwners {
        &self.runtime.session_owners
    }

    /// Get the pending reconnects.
    #[inline]
    pub fn pending_reconnects(&self) -> &PendingReconnects {
        &self.runtime.pending_reconnects
    }

    /// Get the active connections set.
    #[inline]
    pub fn active_connections(&self) -> &ActiveConnections {
        &self.runtime.active_connections
    }

    /// Register a WebSocket connection as active.
    pub async fn register_connection(&self, connection_id: ConnectionId) {
        self.runtime
            .active_connections
            .write()
            .await
            .insert(connection_id);
    }

    /// Unregister a WebSocket connection (called on disconnect).
    pub async fn unregister_connection(&self, connection_id: ConnectionId) {
        self.runtime
            .active_connections
            .write()
            .await
            .remove(&connection_id);
    }

    /// Check if a connection is still active.
    pub async fn is_connection_active(&self, connection_id: ConnectionId) -> bool {
        self.runtime
            .active_connections
            .read()
            .await
            .contains(&connection_id)
    }

    /// Get the WebSocket connection tracker.
    #[inline]
    pub fn ws_connection_tracker(&self) -> &WsConnectionTracker {
        &self.runtime.ws_connection_tracker
    }

    /// Check WebSocket connection rate for an IP address.
    ///
    /// Returns `Ok(())` if the connection is allowed, `Err(Response)` if rate limited.
    pub async fn check_ws_connection_rate(&self, ip: IpAddr) -> Result<(), Response> {
        self.runtime
            .ws_connection_tracker
            .check_rate(ip, self.services.config.ws_connections_per_minute)
            .await
    }

    // ── Backward compatibility (field-style access) ──────────────────────────
    // These preserve the old API while using the new structure internally.

    /// Get allowed paths for a session based on its workstream.
    ///
    /// Returns `None` if no directory manager is configured.
    pub fn allowed_paths(
        &self,
        workstream_id: &str,
        session_id: &str,
    ) -> Option<Vec<std::path::PathBuf>> {
        self.services.allowed_paths(workstream_id, session_id)
    }

    /// Get a PathValidator for a session.
    ///
    /// Returns `None` if no directory manager is configured.
    pub fn path_validator(
        &self,
        workstream_id: &str,
        session_id: &str,
    ) -> Option<arawn_domain::PathValidator> {
        self.services.path_validator(workstream_id, session_id)
    }

    // ── Session Management ───────────────────────────────────────────────────

    /// Get or create a session by ID.
    ///
    /// If session_id is None, creates a new session.
    /// Defaults to "scratch" workstream.
    pub async fn get_or_create_session(&self, session_id: Option<SessionId>) -> SessionId {
        self.get_or_create_session_in_workstream(session_id, "scratch")
            .await
    }

    /// Get or create a session in a specific workstream.
    ///
    /// Sessions are loaded from workstream storage on cache miss and persisted back.
    /// For scratch workstreams, also creates the session's isolated work directory.
    pub async fn get_or_create_session_in_workstream(
        &self,
        session_id: Option<SessionId>,
        workstream_id: &str,
    ) -> SessionId {
        let result = self
            .runtime
            .session_cache
            .get_or_create(session_id, workstream_id)
            .await;

        let (id, is_new) = match result {
            Ok((id, _, is_new)) => (id, is_new),
            Err(e) => {
                warn!("Session cache error: {}, creating new session", e);
                let (id, _) = self
                    .runtime
                    .session_cache
                    .create_session(workstream_id)
                    .await;
                (id, true)
            }
        };

        // Create scratch session directory for new sessions
        if is_new {
            if workstream_id == arawn_domain::SCRATCH_ID
                && let Some(ref dm) = self.services.directory_manager
                && let Err(e) = dm.create_scratch_session(&id.to_string())
            {
                warn!(session_id = %id, error = %e, "Failed to create scratch session directory");
            }

            // Fire SessionStart hook for new sessions
            if let Some(ref dispatcher) = self.services.hook_dispatcher {
                let outcome = dispatcher.dispatch_session_start(&id.to_string()).await;
                debug!(session_id = %id, ?outcome, "SessionStart hook dispatched");
            }
        }

        id
    }

    /// Close a session: remove it from the cache and trigger background indexing/compression.
    ///
    /// Returns `true` if the session existed and was removed.
    /// Indexing and compression run asynchronously and do not block the caller.
    pub async fn close_session(&self, session_id: SessionId) -> bool {
        // Capture workstream_id before removal (needed for compression)
        let workstream_id = self
            .runtime
            .session_cache
            .get_workstream_id(&session_id)
            .await;

        let session = match self.runtime.session_cache.remove(&session_id).await {
            Some(s) => s,
            None => return false,
        };

        let turn_count = session.turn_count();

        // Fire SessionEnd hook
        if let Some(ref dispatcher) = self.services.hook_dispatcher {
            let outcome = dispatcher
                .dispatch_session_end(&session_id.to_string(), turn_count)
                .await;
            debug!(session_id = %session_id, turn_count, ?outcome, "SessionEnd hook dispatched");
        }

        // Spawn background indexing if indexer is configured and session has turns
        if let Some(indexer) = &self.services.indexer
            && !session.is_empty()
        {
            let indexer = Arc::clone(indexer);
            let messages = session_to_messages(&session);
            let sid = session_id.to_string();

            tokio::spawn(async move {
                let report = indexer
                    .index_session(&sid, &messages_as_refs(&messages))
                    .await;
                info!(
                    session_id = %sid,
                    report = %report,
                    "Background session indexing complete"
                );
                if report.has_errors() {
                    warn!(
                        session_id = %sid,
                        errors = ?report.errors,
                        "Session indexing completed with errors"
                    );
                }
            });
        }

        // Spawn background compression if compressor is configured and session has turns
        if let (Some(compressor), Some(manager), Some(ws_id)) = (
            &self.services.compressor,
            &self.services.workstreams,
            &workstream_id,
        ) && !session.is_empty()
        {
            let compressor = Arc::clone(compressor);
            let manager = Arc::clone(manager);
            let sid = session_id.to_string();
            let ws_id = ws_id.clone();

            tokio::spawn(async move {
                // End the workstream session (marks it in SQLite)
                if let Err(e) = manager.end_session(&sid) {
                    warn!(
                        session_id = %sid,
                        error = %e,
                        "Failed to end workstream session for compression"
                    );
                    return;
                }

                // Compress the session
                match compressor.compress_session(&manager, &sid).await {
                    Ok(summary) => {
                        info!(
                            session_id = %sid,
                            summary_len = summary.len(),
                            "Background session compression complete"
                        );

                        // Update the workstream summary (reduce step)
                        match compressor.compress_workstream(&manager, &ws_id).await {
                            Ok(ws_summary) => {
                                info!(
                                    workstream_id = %ws_id,
                                    summary_len = ws_summary.len(),
                                    "Background workstream compression complete"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    workstream_id = %ws_id,
                                    error = %e,
                                    "Workstream compression failed"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            session_id = %sid,
                            error = %e,
                            "Session compression failed"
                        );
                    }
                }
            });
        }

        true
    }

    /// Get session from cache (loading from workstream if needed).
    pub async fn get_session(&self, session_id: SessionId, workstream_id: &str) -> Option<Session> {
        match self
            .runtime
            .session_cache
            .get_or_load(session_id, workstream_id)
            .await
        {
            Ok((session, _)) => Some(session),
            Err(_) => None,
        }
    }

    /// Update session in cache.
    pub async fn update_session(&self, session_id: SessionId, session: Session) {
        if let Err(e) = self.runtime.session_cache.update(session_id, session).await {
            tracing::warn!(session_id = %session_id, error = %e, "Failed to update session cache");
        }
    }

    /// Invalidate a cached session (e.g., after workstream reassignment).
    pub async fn invalidate_session(&self, session_id: SessionId) {
        self.runtime.session_cache.invalidate(&session_id).await;
    }

    // ── Session Ownership ────────────────────────────────────────────────────

    /// Try to claim ownership of a session for a connection.
    ///
    /// Returns `true` if the connection is now the owner (either it was already,
    /// or it successfully claimed ownership of an unowned session).
    /// Returns `false` if another connection owns this session or if there's
    /// a pending reconnect for the session (ownership reserved for reconnection).
    pub async fn try_claim_session_ownership(
        &self,
        session_id: SessionId,
        connection_id: ConnectionId,
    ) -> bool {
        // Check for pending reconnect first (ownership reserved for reconnection)
        {
            let pending = self.runtime.pending_reconnects.read().await;
            if let Some(entry) = pending.get(&session_id)
                && !entry.is_expired()
            {
                debug!(session_id = %session_id, "Ownership claim rejected: pending reconnect exists");
                return false;
            }
        }

        // Check if the existing owner (if any) is still alive
        let active_conns = self.runtime.active_connections.read().await;

        let mut owners: tokio::sync::RwLockWriteGuard<'_, HashMap<SessionId, ConnectionId>> =
            self.runtime.session_owners.write().await;
        match owners.get(&session_id) {
            Some(&existing_owner) if existing_owner == connection_id => {
                // Already the owner
                true
            }
            Some(&existing_owner) => {
                if !active_conns.contains(&existing_owner) {
                    // Owner's connection is dead — evict and claim
                    debug!(
                        session_id = %session_id,
                        dead_owner = %existing_owner,
                        new_owner = %connection_id,
                        "Evicting dead connection's session ownership"
                    );
                    owners.insert(session_id, connection_id);
                    true
                } else {
                    // Another live connection owns it
                    false
                }
            }
            None => {
                // No owner - claim it
                owners.insert(session_id, connection_id);
                debug!(session_id = %session_id, connection_id = %connection_id, "Session ownership claimed");
                true
            }
        }
    }

    /// Check if a connection owns a session.
    pub async fn is_session_owner(
        &self,
        session_id: SessionId,
        connection_id: ConnectionId,
    ) -> bool {
        let owners = self.runtime.session_owners.read().await;
        owners.get(&session_id) == Some(&connection_id)
    }

    /// Release ownership of a session.
    ///
    /// Only the current owner can release ownership.
    /// Returns `true` if ownership was released.
    pub async fn release_session_ownership(
        &self,
        session_id: SessionId,
        connection_id: ConnectionId,
    ) -> bool {
        let mut owners = self.runtime.session_owners.write().await;
        if owners.get(&session_id) == Some(&connection_id) {
            owners.remove(&session_id);
            debug!(session_id = %session_id, connection_id = %connection_id, "Session ownership released");
            true
        } else {
            false
        }
    }

    /// Release all session ownerships held by a connection, creating pending reconnects.
    ///
    /// Called when a WebSocket connection disconnects. Instead of immediately releasing
    /// ownership, creates pending reconnect entries that allow the client to reclaim
    /// ownership within the grace period using the provided tokens.
    ///
    /// `reconnect_tokens` maps session IDs to the tokens that were given to the client.
    pub async fn release_all_session_ownerships(
        &self,
        connection_id: ConnectionId,
        reconnect_tokens: &HashMap<SessionId, String>,
    ) {
        let mut owners = self.runtime.session_owners.write().await;
        let mut pending = self.runtime.pending_reconnects.write().await;

        let sessions_to_release: Vec<_> = owners
            .iter()
            .filter(|(_, owner)| **owner == connection_id)
            .map(|(session_id, _)| *session_id)
            .collect();

        let grace_period = self.services.config.reconnect_grace_period;
        let mut pending_count = 0;

        for session_id in &sessions_to_release {
            owners.remove(session_id);

            // Create pending reconnect if we have a token for this session
            if let Some(token) = reconnect_tokens.get(session_id) {
                pending.insert(
                    *session_id,
                    PendingReconnect::new(token.clone(), grace_period),
                );
                pending_count += 1;
            }
        }

        if !sessions_to_release.is_empty() {
            debug!(
                connection_id = %connection_id,
                released = sessions_to_release.len(),
                pending_reconnects = pending_count,
                grace_period_secs = grace_period.as_secs(),
                "Released session ownerships on disconnect"
            );
        }
    }

    /// Try to reclaim session ownership using a reconnect token.
    ///
    /// Returns `Some(new_token)` if ownership was successfully reclaimed.
    /// Returns `None` if the token is invalid or expired.
    pub async fn try_reclaim_with_token(
        &self,
        session_id: SessionId,
        token: &str,
        connection_id: ConnectionId,
    ) -> Option<String> {
        let mut pending = self.runtime.pending_reconnects.write().await;

        // Check if there's a valid pending reconnect
        if let Some(entry) = pending.get(&session_id) {
            if entry.is_expired() {
                // Expired - clean up and deny
                pending.remove(&session_id);
                debug!(session_id = %session_id, "Reconnect token expired");
                return None;
            }

            if !crate::auth::constant_time_eq(&entry.token, token) {
                // Wrong token
                debug!(session_id = %session_id, "Reconnect token mismatch");
                return None;
            }

            // Valid token - remove pending entry and restore ownership
            pending.remove(&session_id);
            drop(pending); // Release lock before acquiring owners lock

            let mut owners = self.runtime.session_owners.write().await;

            // Double-check no one else claimed it while we were checking
            if owners.contains_key(&session_id) {
                debug!(session_id = %session_id, "Session already claimed by another connection");
                return None;
            }

            owners.insert(session_id, connection_id);

            // Generate new token for future reconnects
            let new_token = uuid::Uuid::new_v4().to_string();
            debug!(session_id = %session_id, connection_id = %connection_id, "Session ownership reclaimed via token");
            Some(new_token)
        } else {
            None
        }
    }

    /// Clean up expired pending reconnects.
    ///
    /// Called lazily during subscribe operations. Returns the number cleaned up.
    pub async fn cleanup_expired_pending_reconnects(&self) -> usize {
        let mut pending = self.runtime.pending_reconnects.write().await;

        let expired: Vec<_> = pending
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(session_id, _)| *session_id)
            .collect();

        for session_id in &expired {
            pending.remove(session_id);
        }

        if !expired.is_empty() {
            debug!(
                count = expired.len(),
                "Cleaned up expired pending reconnects"
            );
        }

        expired.len()
    }

    /// Check if a session has a pending reconnect (ownership held for reconnection).
    pub async fn has_pending_reconnect(&self, session_id: SessionId) -> bool {
        let pending = self.runtime.pending_reconnects.read().await;
        if let Some(entry) = pending.get(&session_id) {
            !entry.is_expired()
        } else {
            false
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a session's turns into owned `(role, content)` pairs.
pub(crate) fn session_to_messages(session: &Session) -> Vec<(String, String)> {
    let mut messages = Vec::new();
    for turn in session.all_turns() {
        messages.push(("user".to_string(), turn.user_message.clone()));
        if let Some(ref response) = turn.assistant_response {
            messages.push(("assistant".to_string(), response.clone()));
        }
    }
    messages
}

/// Convert owned message pairs to borrowed slices for the indexer API.
pub(crate) fn messages_as_refs(messages: &[(String, String)]) -> Vec<(&str, &str)> {
    messages
        .iter()
        .map(|(r, c)| (r.as_str(), c.as_str()))
        .collect()
}

#[cfg(test)]
mod tests;

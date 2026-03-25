//! Application state and main loop.

mod chat_handler;
mod logs_handler;
mod sidebar_handler;
mod tool_pane_handler;

use crate::bounded::BoundedVec;
use crate::client::{ConnectionStatus, WsClient};
use crate::focus::{FocusManager, FocusTarget};

/// Maximum number of chat messages to retain (prevents unbounded memory growth).
const MAX_MESSAGES: usize = 10_000;

/// Maximum number of tool executions to retain per response.
const MAX_TOOLS: usize = 1_000;

use crate::Tui;
use crate::events::{Event, EventHandler};
use crate::input::InputState;
use crate::logs::LogBuffer;
use crate::palette::{ActionId, CommandPalette};
use crate::protocol::ServerMessage;
use crate::sessions::{SessionList, SessionSummary};
use crate::sidebar::{Sidebar, SidebarSection, WorkstreamEntry};
use crate::ui;
use crate::ui::CommandPopup;
use anyhow::Result;
use arawn_client::{ArawnClient, CreateWorkstreamRequest, UpdateWorkstreamRequest};
use chrono::{DateTime, Utc};

pub use crate::app_types::{
    ChatMessage, ContextState, DiskWarning, InputMode, PanelAreas, PendingAction, ToolExecution,
    UsageStats,
};
use crossterm::event::{KeyCode, KeyModifiers};

/// Main application state.
pub struct App {
    /// Server URL to connect to.
    pub server_url: String,
    /// WebSocket client for real-time chat.
    pub ws_client: WsClient,
    /// HTTP API client for REST endpoints.
    pub api: ArawnClient,
    /// Current connection status.
    pub connection_status: ConnectionStatus,
    /// Focus manager for panel/overlay navigation.
    pub focus: FocusManager,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Input state with history.
    pub input: InputState,
    /// Current input mode (chat, new workstream, rename, etc.)
    pub input_mode: InputMode,
    /// Status bar message.
    pub status_message: Option<String>,
    /// Current workstream name.
    pub workstream: String,
    /// Current workstream ID (for API calls).
    pub workstream_id: Option<String>,
    /// Current session ID.
    pub session_id: Option<String>,
    /// Chat messages (bounded to prevent unbounded growth).
    pub messages: BoundedVec<ChatMessage>,
    /// Tool executions in current response (bounded).
    pub tools: BoundedVec<ToolExecution>,
    /// Whether we're waiting for a response.
    pub waiting: bool,
    /// Chat scroll offset (lines from top).
    pub chat_scroll: usize,
    /// Whether to auto-scroll to bottom during streaming.
    pub chat_auto_scroll: bool,
    /// Session list state.
    pub sessions: SessionList,
    /// Command palette state.
    pub palette: CommandPalette,
    /// Context name (for display in header).
    pub context_name: Option<String>,
    /// Log buffer for capturing and displaying logs.
    pub log_buffer: LogBuffer,
    /// Log scroll offset.
    pub log_scroll: usize,
    /// Whether logs panel is visible.
    pub show_logs: bool,
    /// Sidebar state for workstreams and sessions navigation.
    pub sidebar: Sidebar,
    /// Whether sidebar is in "move session to workstream" mode.
    pub moving_session_to_workstream: bool,
    /// Tool pane scroll offset.
    pub tool_scroll: usize,
    /// Currently selected tool index (for tool pane navigation).
    pub selected_tool_index: Option<usize>,
    /// Whether the tool pane (split view) is visible.
    pub show_tool_pane: bool,
    /// Pending async actions to process.
    pending_actions: Vec<PendingAction>,
    /// Command autocomplete popup.
    pub command_popup: CommandPopup,
    /// Whether a command is currently executing.
    pub command_executing: bool,
    /// Current command execution progress message.
    pub command_progress: Option<String>,
    /// Context usage information for current session.
    pub context_info: Option<ContextState>,
    /// Disk usage stats for current workstream.
    pub workstream_usage: Option<UsageStats>,
    /// Active disk warnings.
    pub disk_warnings: Vec<DiskWarning>,
    /// Whether to show usage popup (Ctrl+U).
    pub show_usage_popup: bool,
    /// Reconnect tokens for session ownership recovery after disconnect.
    /// Maps session_id -> reconnect_token.
    pub reconnect_tokens: std::collections::HashMap<String, String>,
    /// Whether the current session is owned by this client (can send Chat).
    /// When false, the client is in read-only mode.
    pub is_session_owner: bool,
    /// Pending delete confirmation for workstream (id, name).
    /// Set on first 'd' press, cleared on second 'd' (executes delete) or any other action.
    pub pending_delete_workstream: Option<(String, String)>,
    /// Pending delete confirmation for session (id).
    /// Set on first 'd' press, cleared on second 'd' (executes delete) or any other action.
    pub pending_delete_session: Option<String>,
    /// Cached panel areas from the last render, used for mouse hit-testing.
    pub panel_areas: PanelAreas,
    /// Last time a WebSocket keepalive ping was sent.
    last_ping: std::time::Instant,
}

// Types (PanelAreas, ContextState, UsageStats, DiskWarning) moved to app_types.rs

impl App {
    /// Create a new App instance.
    ///
    /// Returns an error if the HTTP API client cannot be constructed
    /// (e.g., invalid server URL).
    pub fn new(server_url: String, log_buffer: LogBuffer) -> Result<Self> {
        let ws_client = WsClient::new(&server_url);

        // Build HTTP API client, reading auth token from environment
        let mut builder = ArawnClient::builder().base_url(&server_url);
        if let Ok(token) = std::env::var("ARAWN_API_TOKEN") {
            builder = builder.auth_token(token);
        }
        let api = builder.build()?;

        let sidebar = Sidebar::new();

        Ok(Self {
            server_url: server_url.clone(),
            ws_client,
            api,
            connection_status: ConnectionStatus::Connecting,
            focus: FocusManager::new(),
            should_quit: false,
            input: InputState::new(),
            input_mode: InputMode::default(),
            status_message: None,
            workstream: "scratch".to_string(),
            workstream_id: None, // Will be set when workstreams load
            session_id: None,
            messages: BoundedVec::with_capacity(MAX_MESSAGES, 1024),
            tools: BoundedVec::with_capacity(MAX_TOOLS, 64),
            waiting: false,
            chat_scroll: 0,
            chat_auto_scroll: true,
            sessions: SessionList::new(),
            palette: CommandPalette::new(),
            context_name: None,
            log_buffer,
            log_scroll: 0,
            show_logs: false,
            sidebar,
            moving_session_to_workstream: false,
            tool_scroll: 0,
            selected_tool_index: None,
            show_tool_pane: false,
            pending_actions: Vec::new(),
            command_popup: CommandPopup::new(),
            command_executing: false,
            command_progress: None,
            context_info: None,
            workstream_usage: None,
            disk_warnings: Vec::new(),
            show_usage_popup: false,
            reconnect_tokens: std::collections::HashMap::new(),
            is_session_owner: true, // Default to owner until told otherwise
            pending_delete_workstream: None,
            pending_delete_session: None,
            panel_areas: PanelAreas::default(),
            last_ping: std::time::Instant::now(),
        })
    }

    /// Push a message (BoundedVec handles eviction automatically).
    // push_message, push_tool → chat_handler.rs

    /// Run the main application loop.
    pub async fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        let mut events = EventHandler::new();
        let mut data_loaded = false;

        while !self.should_quit {
            // Render the UI
            terminal.draw(|frame| ui::render(self, frame))?;

            // Handle events
            tokio::select! {
                // Terminal events
                event = events.next() => {
                    match event? {
                        Event::Key(key) => self.handle_key(key),
                        Event::Mouse(mouse) => self.handle_mouse(mouse),
                        Event::Tick => {
                            // Poll for connection status updates
                            if let Some(status) = self.ws_client.poll_status() {
                                let was_connected = self.connection_status == ConnectionStatus::Connected;
                                self.connection_status = status;

                                // Reset waiting state if connection dropped while waiting for response
                                if was_connected && status != ConnectionStatus::Connected && self.waiting {
                                    self.waiting = false;
                                    self.status_message = Some("Connection lost — message may not have been delivered".to_string());
                                }

                                // Load data when we first connect
                                if !was_connected && status == ConnectionStatus::Connected && !data_loaded {
                                    data_loaded = true;
                                    self.refresh_sidebar_data().await;
                                }
                            }

                            // Send WebSocket keepalive ping every 30 seconds
                            if self.last_ping.elapsed() >= std::time::Duration::from_secs(30) {
                                let _ = self.ws_client.send_ping();
                                self.last_ping = std::time::Instant::now();
                            }
                        }
                        Event::Resize(_, _) => {
                            // Terminal resized, will re-render on next iteration
                        }
                    }
                }

                // WebSocket messages
                msg = self.ws_client.recv() => {
                    if let Some(msg) = msg {
                        self.handle_server_message(msg);
                    }
                }
            }

            // Process any pending async actions
            self.process_pending_actions().await;
        }

        Ok(())
    }

    /// Process pending async actions.
    async fn process_pending_actions(&mut self) {
        // Take all pending actions and deduplicate to avoid redundant work
        let mut actions: Vec<_> = self.pending_actions.drain(..).collect();

        // Deduplicate while preserving order (keep first occurrence)
        let mut seen = std::collections::HashSet::new();
        actions.retain(|action| seen.insert(action.clone()));

        for action in actions {
            match action {
                PendingAction::CreateWorkstream(title) => {
                    self.do_create_workstream(&title).await;
                }
                PendingAction::RenameWorkstream(id, new_title) => {
                    self.do_rename_workstream(&id, &new_title).await;
                }
                PendingAction::DeleteSession(id) => {
                    self.do_delete_session(&id).await;
                }
                PendingAction::DeleteWorkstream(id) => {
                    self.do_delete_workstream(&id).await;
                }
                PendingAction::RefreshSidebar => {
                    self.refresh_sidebar_data().await;
                }
                PendingAction::FetchWorkstreamSessions(workstream_id) => {
                    self.do_fetch_workstream_sessions(&workstream_id).await;
                }
                PendingAction::FetchSessionMessages(session_id) => {
                    self.do_fetch_session_messages(&session_id).await;
                }
                PendingAction::MoveSessionToWorkstream(session_id, workstream_id) => {
                    tracing::info!(
                        "Processing pending action: MoveSessionToWorkstream({}, {})",
                        session_id,
                        workstream_id
                    );
                    self.do_move_session_to_workstream(&session_id, &workstream_id)
                        .await;
                }
            }
        }
    }

    /// Create a workstream via API.
    async fn do_create_workstream(&mut self, title: &str) {
        let request = CreateWorkstreamRequest {
            title: title.to_string(),
            default_model: None,
            tags: vec![],
        };

        match self.api.workstreams().create(request).await {
            Ok(workstream) => {
                tracing::info!(
                    "Created workstream: {} ({})",
                    workstream.title,
                    workstream.id
                );
                self.status_message = Some(format!("Created workstream: {}", workstream.title));

                // Add to sidebar and switch to it
                self.sidebar.workstreams.push(WorkstreamEntry {
                    id: workstream.id.clone(),
                    name: workstream.title.clone(),
                    session_count: 0,
                    is_current: false,
                    is_scratch: false,
                    usage_bytes: None,
                    limit_bytes: None,
                    state: "active".to_string(),
                });

                // Switch to the new workstream
                self.switch_to_workstream(&workstream.title);
            }
            Err(e) => {
                tracing::error!("Failed to create workstream: {}", e);
                self.status_message = Some(format!("Failed to create workstream: {}", e));
            }
        }
    }

    /// Rename a workstream via API.
    async fn do_rename_workstream(&mut self, id: &str, new_title: &str) {
        let request = UpdateWorkstreamRequest {
            title: Some(new_title.to_string()),
            summary: None,
            default_model: None,
            tags: None,
        };

        match self.api.workstreams().update(id, request).await {
            Ok(workstream) => {
                tracing::info!("Renamed workstream to: {}", workstream.title);
                self.status_message = Some(format!("Renamed to: {}", workstream.title));

                // Update sidebar entry - lookup by ID, not name
                if let Some(entry) = self.sidebar.workstreams.iter_mut().find(|ws| ws.id == id) {
                    entry.name = workstream.title.clone();
                }

                // Update current workstream name if it was the renamed one
                if self.workstream == id {
                    self.workstream = workstream.title;
                }
            }
            Err(e) => {
                tracing::error!("Failed to rename workstream: {}", e);
                self.status_message = Some(format!("Failed to rename: {}", e));
            }
        }
    }

    /// Delete a session via API.
    async fn do_delete_session(&mut self, id: &str) {
        match self.api.sessions().delete(id).await {
            Ok(()) => {
                tracing::info!("Deleted session: {}", id);
                self.status_message = Some("Session deleted".to_string());

                // Remove from sidebar immediately for responsiveness
                self.sidebar.sessions.retain(|s| s.id != id);

                // If we deleted the current session, clear it
                if self.session_id.as_deref() == Some(id) {
                    self.session_id = None;
                    self.messages.clear();
                    self.tools.clear();
                }

                // Full refresh to sync session counts and state
                self.refresh_sidebar_data().await;
            }
            Err(e) => {
                tracing::error!("Failed to delete session: {}", e);
                self.status_message = Some(format!("Failed to delete session: {}", e));
            }
        }
    }

    /// Delete a workstream via API.
    async fn do_delete_workstream(&mut self, id: &str) {
        match self.api.workstreams().delete(id).await {
            Ok(()) => {
                tracing::info!("Deleted workstream: {}", id);
                self.status_message = Some("Workstream deleted".to_string());

                // Remove from sidebar immediately for responsiveness
                self.sidebar.workstreams.retain(|ws| ws.id != id);

                // If we deleted the current workstream, switch to scratch
                if self.workstream_id.as_deref() == Some(id) {
                    self.workstream = "scratch".to_string();
                    self.messages.clear();
                    self.tools.clear();
                    self.session_id = None;
                }

                // Full refresh to sync state
                self.refresh_sidebar_data().await;
            }
            Err(e) => {
                tracing::error!("Failed to delete workstream: {}", e);
                self.status_message = Some(format!("Failed to delete workstream: {}", e));
            }
        }
    }

    /// Fetch sessions for a specific workstream.
    async fn do_fetch_workstream_sessions(&mut self, workstream_id: &str) {
        match self.api.workstreams().sessions(workstream_id).await {
            Ok(response) => {
                self.sidebar.sessions = response
                    .sessions
                    .iter()
                    .map(|s| {
                        // Parse the started_at timestamp
                        let last_active = DateTime::parse_from_rfc3339(&s.started_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());

                        // Generate a title from the date or use a default
                        let title = if s.is_active {
                            "Active session".to_string()
                        } else {
                            format!("Session {}", last_active.format("%b %d %H:%M"))
                        };

                        SessionSummary {
                            id: s.id.clone(),
                            title,
                            last_active,
                            message_count: 0, // Not available from this API
                            is_current: self.session_id.as_ref() == Some(&s.id),
                        }
                    })
                    .collect();

                // Reset to "+ New Session" selected
                self.sidebar.session_index = 0;

                // Update the session count for this workstream in the sidebar
                let session_count = self.sidebar.sessions.len();
                if let Some(ws_entry) = self
                    .sidebar
                    .workstreams
                    .iter_mut()
                    .find(|ws| ws.id == workstream_id)
                {
                    ws_entry.session_count = session_count;
                }

                tracing::info!(
                    "Loaded {} sessions for workstream {}",
                    session_count,
                    workstream_id
                );
            }
            Err(e) => {
                tracing::warn!("Failed to load sessions for workstream: {}", e);
                // Clear sessions on error
                self.sidebar.sessions.clear();
                self.sidebar.session_index = 0;
            }
        }
    }

    /// Fetch message history for a session.
    async fn do_fetch_session_messages(&mut self, session_id: &str) {
        match self.api.sessions().messages(session_id).await {
            Ok(response) => {
                // Convert API messages to ChatMessages
                let chat_messages: Vec<_> = response
                    .messages
                    .iter()
                    .map(|m| ChatMessage {
                        is_user: m.role == "user",
                        content: m.content.clone(),
                        streaming: false,
                    })
                    .collect();
                self.messages.replace_from_vec(chat_messages);

                // Scroll to bottom to show latest messages
                self.chat_auto_scroll = true;

                tracing::info!(
                    "Loaded {} messages for session {}",
                    self.messages.len(),
                    session_id
                );
            }
            Err(e) => {
                tracing::warn!("Failed to load session messages: {}", e);
                self.status_message = Some(format!("Failed to load messages: {}", e));
                // Keep messages cleared
            }
        }
    }

    /// Move a session to a different workstream via API.
    async fn do_move_session_to_workstream(&mut self, session_id: &str, workstream_id: &str) {
        use arawn_client::UpdateSessionRequest;

        tracing::info!(
            "Moving session {} to workstream {}",
            session_id,
            workstream_id
        );

        let request = UpdateSessionRequest {
            workstream_id: Some(workstream_id.to_string()),
            ..Default::default()
        };

        tracing::info!("Sending PATCH request to server...");
        match self.api.sessions().update(session_id, request).await {
            Ok(_) => {
                // Find workstream name for display
                let ws_name = self
                    .sidebar
                    .workstreams
                    .iter()
                    .find(|ws| ws.id == workstream_id)
                    .map(|ws| ws.name.clone())
                    .unwrap_or_else(|| workstream_id.to_string());

                tracing::info!("Moved session {} to workstream {}", session_id, ws_name);
                self.status_message = Some(format!("Moved session to {}", ws_name));

                // Refresh sidebar to reflect the change
                self.pending_actions.push(PendingAction::RefreshSidebar);
            }
            Err(e) => {
                tracing::error!("Failed to move session: {}", e);
                self.status_message = Some(format!("Failed to move session: {}", e));
            }
        }
    }

    /// Refresh sidebar data from the server API.
    async fn refresh_sidebar_data(&mut self) {
        // Fetch workstreams (including archived)
        match self.api.workstreams().list_all().await {
            Ok(response) => {
                self.sidebar.workstreams = response
                    .workstreams
                    .iter()
                    .map(|ws| WorkstreamEntry {
                        id: ws.id.clone(),
                        name: ws.title.clone(),
                        session_count: 0, // Updated below when loading sessions
                        is_current: ws.title == self.workstream
                            || (ws.is_scratch && self.workstream == "scratch"),
                        is_scratch: ws.is_scratch,
                        usage_bytes: None, // Updated via WebSocket events
                        limit_bytes: None,
                        state: ws.state.clone(),
                    })
                    .collect();

                // Set initial selection to current workstream and store the ID
                // Only consider active workstreams for selection
                if let Some(pos) = self
                    .sidebar
                    .workstreams
                    .iter()
                    .position(|ws| ws.is_current && !ws.is_archived())
                {
                    self.sidebar.workstream_index = pos;
                    self.workstream_id = Some(self.sidebar.workstreams[pos].id.clone());
                    self.workstream = self.sidebar.workstreams[pos].name.clone();
                }

                tracing::info!("Loaded {} workstreams", self.sidebar.workstreams.len());
            }
            Err(e) => {
                tracing::warn!("Failed to load workstreams: {}", e);
                self.status_message = Some(format!("Failed to load workstreams: {}", e));
            }
        }

        // Fetch sessions for current workstream
        if let Some(ws_id) = self.workstream_id.clone() {
            self.do_fetch_workstream_sessions(&ws_id).await;
        } else {
            // No workstream selected, clear sessions
            self.sidebar.sessions.clear();
            self.sidebar.session_index = 0;
        }
    }

    /// Handle a message from the server.
    /// Process a server message and update app state accordingly.
    pub fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::SessionCreated { session_id } => {
                self.session_id = Some(session_id);
            }

            ServerMessage::ChatChunk { chunk, done, .. } => {
                if done {
                    // Mark last message as not streaming
                    if let Some(last) = self.messages.last_mut() {
                        last.streaming = false;
                    }
                    self.waiting = false;
                } else if !chunk.is_empty() {
                    // Append to last message or create new one
                    if let Some(last) = self.messages.last_mut()
                        && !last.is_user
                        && last.streaming
                    {
                        last.content.push_str(&chunk);
                        return;
                    }
                    // Create new assistant message
                    self.push_message(ChatMessage {
                        is_user: false,
                        content: chunk,
                        streaming: true,
                    });
                }
            }

            ServerMessage::ToolStart {
                tool_id, tool_name, ..
            } => {
                self.push_tool(ToolExecution {
                    id: tool_id,
                    name: tool_name,
                    args: String::new(), // Args not provided by protocol yet
                    output: String::new(),
                    running: true,
                    success: None,
                    started_at: std::time::Instant::now(),
                    duration_ms: None,
                });
                // Auto-select the new tool if tool pane is visible
                if self.show_tool_pane {
                    self.selected_tool_index = Some(self.tools.len().saturating_sub(1));
                }
            }

            ServerMessage::ToolOutput {
                tool_id, content, ..
            } => {
                if let Some(tool) = self.tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.output.push_str(&content);
                }
            }

            ServerMessage::ToolEnd {
                tool_id, success, ..
            } => {
                if let Some(tool) = self.tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.running = false;
                    tool.success = Some(success);
                    tool.duration_ms = Some(tool.started_at.elapsed().as_millis() as u64);
                }
            }

            ServerMessage::Error { code, message } => {
                // Handle specific error codes
                if code == "session_not_owned" {
                    // We tried to send a message but aren't the owner
                    self.is_session_owner = false;
                    self.status_message =
                        Some("Read-only mode: session owned by another client".to_string());
                } else {
                    self.status_message = Some(format!("Error: {}", message));
                }
                self.waiting = false;
            }

            ServerMessage::AuthResult { success, error } => {
                if success {
                    self.status_message = Some("Authenticated".to_string());
                } else {
                    self.status_message =
                        Some(format!("Auth failed: {}", error.unwrap_or_default()));
                }
            }

            ServerMessage::Pong => {
                // Ignore pongs
            }

            ServerMessage::CommandProgress {
                command,
                message,
                percent,
            } => {
                self.command_executing = true;
                let progress_str = match percent {
                    Some(p) => format!("/{}: {} ({}%)", command, message, p),
                    None => format!("/{}: {}", command, message),
                };
                self.command_progress = Some(progress_str.clone());
                self.status_message = Some(progress_str);
            }

            ServerMessage::CommandResult {
                command,
                success,
                result,
            } => {
                self.command_executing = false;
                self.command_progress = None;

                if success {
                    // Format the result as a system message
                    let result_str =
                        if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                            msg.to_string()
                        } else {
                            serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "Success".to_string())
                        };
                    self.status_message = Some(format!("/{}: {}", command, result_str));

                    // Add as system message in chat
                    self.push_message(ChatMessage {
                        is_user: false,
                        content: format!("[/{}] {}", command, result_str),
                        streaming: false,
                    });
                } else {
                    let error_str = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    self.status_message = Some(format!("/{} failed: {}", command, error_str));
                }
            }

            ServerMessage::ContextInfo {
                current_tokens,
                max_tokens,
                percent,
                status,
                ..
            } => {
                self.context_info = Some(ContextState {
                    current_tokens,
                    max_tokens,
                    percent,
                    status,
                });
            }

            ServerMessage::DiskPressure {
                workstream_id,
                workstream_name,
                level,
                usage_bytes,
                limit_bytes,
                percent,
            } => {
                // Add warning, replacing any existing warning for same workstream
                self.disk_warnings
                    .retain(|w| w.workstream_id != workstream_id);
                self.disk_warnings.push(DiskWarning {
                    workstream_id: workstream_id.clone(),
                    workstream: workstream_name.clone(),
                    level,
                    usage_bytes,
                    limit_bytes,
                    percent,
                    timestamp: std::time::Instant::now(),
                });

                // Show status message for critical warnings
                if self.disk_warnings.last().map(|w| w.level.as_str()) == Some("critical") {
                    self.status_message = Some(format!(
                        "⚠ Disk critical: {} at {}% of limit",
                        workstream_name, percent
                    ));
                }
            }

            ServerMessage::WorkstreamUsage {
                workstream_id,
                workstream_name,
                is_scratch,
                production_bytes,
                work_bytes,
                total_bytes,
                limit_bytes,
                percent,
            } => {
                // Update usage stats if it's for the current workstream
                if self.workstream_id.as_deref() == Some(&workstream_id) {
                    self.workstream_usage = Some(UsageStats {
                        workstream_id,
                        workstream_name,
                        is_scratch,
                        production_bytes,
                        work_bytes,
                        total_bytes,
                        limit_bytes,
                        percent,
                    });
                }
            }

            ServerMessage::SubscribeAck {
                session_id,
                owner,
                reconnect_token,
            } => {
                // Update ownership state
                self.is_session_owner = owner;

                // Store reconnect token if we're the owner
                if let Some(token) = reconnect_token {
                    self.reconnect_tokens.insert(session_id.clone(), token);
                }

                if owner {
                    tracing::info!(session_id = %session_id, "Subscribed as owner");
                } else {
                    tracing::info!(session_id = %session_id, "Subscribed as reader (read-only)");
                    self.status_message = Some("Read-only mode".to_string());
                }
            }
        }
    }

    /// Handle keyboard input.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // Global shortcuts first
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('c') => {
                    // Cancel current operation or quit if nothing running
                    if self.waiting {
                        // Send cancel to server if we have a session
                        if let Some(ref session_id) = self.session_id
                            && let Err(e) = self.ws_client.cancel(session_id.clone())
                        {
                            tracing::warn!(error = %e, "Failed to send cancel to server");
                        }
                        self.waiting = false;
                        self.status_message = Some("Cancelled".to_string());
                    } else {
                        self.should_quit = true;
                    }
                    return;
                }
                KeyCode::Char('k') => {
                    self.palette.reset();
                    self.focus.push_overlay(FocusTarget::CommandPalette);
                    return;
                }
                KeyCode::Char('s') => {
                    self.open_sessions_panel();
                    return;
                }
                KeyCode::Char('w') => {
                    // Toggle sidebar open/closed
                    if self.sidebar.is_open() {
                        // Close sidebar, return focus to input
                        self.sidebar.close();
                        self.focus.return_to_input();
                    } else {
                        // Open sidebar and focus it
                        self.sidebar.open();
                        self.focus.focus(FocusTarget::Sidebar);
                    }
                    return;
                }
                KeyCode::Char('e') => {
                    self.show_tool_pane = !self.show_tool_pane;
                    if self.show_tool_pane {
                        // Select first tool if none selected
                        if self.selected_tool_index.is_none() && !self.tools.is_empty() {
                            self.selected_tool_index = Some(0);
                        }
                        self.focus.focus(FocusTarget::ToolPane);
                    } else {
                        self.selected_tool_index = None;
                        self.focus.return_to_input();
                    }
                    return;
                }
                KeyCode::Char('o') if self.show_tool_pane => {
                    // Open selected tool output in external editor
                    self.open_tool_in_editor();
                    return;
                }
                KeyCode::Char('l') => {
                    // Toggle logs panel
                    self.show_logs = !self.show_logs;
                    if self.show_logs {
                        self.focus.focus(FocusTarget::Logs);
                    } else {
                        self.focus.return_to_input();
                    }
                    return;
                }
                KeyCode::Char('u') => {
                    // Toggle usage stats popup
                    self.show_usage_popup = !self.show_usage_popup;
                    return;
                }
                _ => {}
            }
        }

        // Delegate to focused component
        match self.focus.current() {
            FocusTarget::Input => self.handle_input_key(key),
            FocusTarget::Sidebar => self.handle_sidebar_key(key),
            FocusTarget::Sessions => self.handle_sessions_key(key),
            FocusTarget::CommandPalette => self.handle_palette_key(key),
            FocusTarget::Workstreams => self.handle_overlay_key(key),
            FocusTarget::ToolPane => self.handle_tool_pane_key(key),
            FocusTarget::Logs => self.handle_logs_key(key),
        }
    }

    /// Handle input-focused key events.
    fn handle_input_key(&mut self, key: crossterm::event::KeyEvent) {
        let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Check if command popup is visible and handle its keys
        if self.command_popup.is_visible() {
            match key.code {
                KeyCode::Esc => {
                    self.command_popup.hide();
                    return;
                }
                KeyCode::Up => {
                    self.command_popup.select_prev();
                    return;
                }
                KeyCode::Down => {
                    self.command_popup.select_next();
                    return;
                }
                KeyCode::Tab | KeyCode::Enter => {
                    // Complete the selected command
                    if let Some(cmd) = self.command_popup.selected_command() {
                        let cmd_name = cmd.name.clone();
                        self.input.set_text(&format!("/{} ", cmd_name));
                        self.command_popup.hide();
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    // Continue typing and update filter
                    self.input.insert_char(c);
                    self.update_command_popup();
                    return;
                }
                KeyCode::Backspace => {
                    self.input.delete_char_before();
                    self.update_command_popup();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                // Show command popup when typing '/'
                if c == '/' && self.input.content().trim() == "/" {
                    self.command_popup.show("");
                } else {
                    self.update_command_popup();
                }
            }
            KeyCode::Backspace => {
                self.input.delete_char_before();
                self.update_command_popup();
            }
            KeyCode::Delete => {
                self.input.delete_char_at();
            }
            KeyCode::Left => {
                self.input.move_left();
            }
            KeyCode::Right => {
                self.input.move_right();
            }
            KeyCode::Home => {
                if has_ctrl {
                    // Ctrl+Home: scroll to top of chat
                    self.chat_scroll = 0;
                    self.chat_auto_scroll = false;
                } else {
                    self.input.move_to_line_start();
                }
            }
            KeyCode::End => {
                if has_ctrl {
                    // Ctrl+End: scroll to bottom and enable auto-scroll
                    self.chat_auto_scroll = true;
                } else {
                    self.input.move_to_line_end();
                }
            }
            KeyCode::Enter => {
                if has_shift {
                    // Shift+Enter: insert newline
                    self.input.insert_newline();
                } else if !self.input.is_empty() {
                    // Hide command popup
                    self.command_popup.hide();

                    // Handle based on input mode
                    match &self.input_mode {
                        InputMode::Chat => {
                            if !self.waiting && !self.command_executing {
                                // Check if this is a command
                                if self.input.is_command() {
                                    self.send_command();
                                } else {
                                    self.send_message();
                                }
                            }
                        }
                        InputMode::NewWorkstream => {
                            let title = self.input.content().trim().to_string();

                            // Validation
                            if title.is_empty() {
                                self.status_message =
                                    Some("Workstream name cannot be empty".to_string());
                                return;
                            }
                            if title.len() > 100 {
                                self.status_message =
                                    Some("Workstream name too long (max 100 chars)".to_string());
                                return;
                            }
                            // Check for duplicate names
                            let name_exists = self
                                .sidebar
                                .workstreams
                                .iter()
                                .any(|ws| ws.name.eq_ignore_ascii_case(&title));
                            if name_exists {
                                self.status_message =
                                    Some(format!("Workstream '{}' already exists", title));
                                return;
                            }

                            self.pending_actions
                                .push(PendingAction::CreateWorkstream(title));
                            self.input.clear();
                            self.input_mode = InputMode::Chat;
                            self.status_message = None;
                        }
                        InputMode::RenameWorkstream(id) => {
                            let new_title = self.input.content().to_string();
                            let id = id.clone();
                            self.pending_actions
                                .push(PendingAction::RenameWorkstream(id, new_title));
                            self.input.clear();
                            self.input_mode = InputMode::Chat;
                            self.status_message = None;
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Close usage popup if open
                if self.show_usage_popup {
                    self.show_usage_popup = false;
                    return;
                }
                // Cancel special mode or clear input
                if self.input_mode != InputMode::Chat {
                    self.input_mode = InputMode::Chat;
                    self.input.clear();
                    self.status_message = None;
                } else if !self.input.is_empty() {
                    self.input.clear();
                }
            }
            // History navigation with Up/Down (when on single line or at boundaries)
            KeyCode::Up => {
                let (line, _) = self.input.cursor_position();
                if self.input.is_empty() {
                    // Empty input: scroll chat
                    self.scroll_chat_up(1);
                } else if line == 0 {
                    // At first line: navigate history
                    self.input.history_prev();
                } else {
                    // Multi-line: move cursor up
                    self.input.move_up();
                }
            }
            KeyCode::Down => {
                let (line, _) = self.input.cursor_position();
                let last_line = self.input.line_count().saturating_sub(1);
                if self.input.is_empty() {
                    // Empty input: scroll chat
                    self.scroll_chat_down(1);
                } else if line >= last_line {
                    // At last line: navigate history
                    self.input.history_next();
                } else {
                    // Multi-line: move cursor down
                    self.input.move_down();
                }
            }
            KeyCode::PageUp => {
                self.scroll_chat_up(10);
            }
            KeyCode::PageDown => {
                self.scroll_chat_down(10);
            }
            _ => {}
        }
    }

    /// Scroll chat up by the given number of lines.
    ///
    /// Disables auto-scroll so the user can read history without
    /// being snapped back to the bottom during streaming. Auto-scroll
    /// is only re-enabled when the user sends a new message.
    // scroll_chat_up, scroll_chat_down → chat_handler.rs

    /// Handle mouse events (scroll wheel on panels).
    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;

        let scroll_lines = 3;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                let target = self.panel_at(mouse.column, mouse.row);
                match target {
                    Some(FocusTarget::Input) | None => self.scroll_chat_up(scroll_lines),
                    Some(FocusTarget::Sidebar) => {
                        // Sidebar scroll handled by sidebar's own navigation
                    }
                    Some(FocusTarget::ToolPane) => {
                        self.tool_scroll = self.tool_scroll.saturating_sub(scroll_lines);
                    }
                    Some(FocusTarget::Logs) => {
                        self.log_scroll = self.log_scroll.saturating_sub(scroll_lines);
                    }
                    _ => self.scroll_chat_up(scroll_lines),
                }
            }
            MouseEventKind::ScrollDown => {
                let target = self.panel_at(mouse.column, mouse.row);
                match target {
                    Some(FocusTarget::Input) | None => self.scroll_chat_down(scroll_lines),
                    Some(FocusTarget::Sidebar) => {
                        // Sidebar scroll handled by sidebar's own navigation
                    }
                    Some(FocusTarget::ToolPane) => {
                        self.tool_scroll = self.tool_scroll.saturating_add(scroll_lines);
                    }
                    Some(FocusTarget::Logs) => {
                        self.log_scroll = self.log_scroll.saturating_add(scroll_lines);
                    }
                    _ => self.scroll_chat_down(scroll_lines),
                }
            }
            _ => {}
        }
    }

    /// Determine which panel contains the given screen coordinates.
    fn panel_at(&self, col: u16, row: u16) -> Option<FocusTarget> {
        let contains = |rect: &ratatui::layout::Rect| {
            col >= rect.x
                && col < rect.x + rect.width
                && row >= rect.y
                && row < rect.y + rect.height
        };

        // Check in order: sidebar, logs, tool pane, chat (most specific first)
        if let Some(ref r) = self.panel_areas.sidebar
            && contains(r)
        {
            return Some(FocusTarget::Sidebar);
        }
        if let Some(ref r) = self.panel_areas.logs
            && contains(r)
        {
            return Some(FocusTarget::Logs);
        }
        if let Some(ref r) = self.panel_areas.tool_pane
            && contains(r)
        {
            return Some(FocusTarget::ToolPane);
        }
        if let Some(ref r) = self.panel_areas.chat
            && contains(r)
        {
            return Some(FocusTarget::Input); // Chat area scrolls chat
        }
        None
    }

    /// Update the command popup based on current input.
    fn update_command_popup(&mut self) {
        if let Some(prefix) = self.input.command_prefix() {
            if !self.command_popup.is_visible() {
                self.command_popup.show(prefix);
            } else {
                self.command_popup.filter(prefix);
            }
        } else {
            self.command_popup.hide();
        }
    }

    /// Send the current input as a command.
    fn send_command(&mut self) {
        let input = self.input.submit();

        // Parse the command
        if let Some(cmd) = crate::input::ParsedCommand::parse(&input) {
            // Handle built-in commands
            if cmd.name.eq_ignore_ascii_case("help") {
                // Show available commands in chat
                let help_text = self.get_help_text();
                self.push_message(ChatMessage {
                    is_user: false,
                    content: help_text,
                    streaming: false,
                });
                return;
            }

            // Check read-only mode for server commands
            if !self.is_session_owner {
                self.status_message = Some("Read-only mode: cannot run commands".to_string());
                return;
            }

            // Build args JSON
            let args = self.build_command_args(&cmd);

            // Send command via WebSocket
            if let Err(e) = self.ws_client.send_command(cmd.name.clone(), args) {
                self.status_message = Some(format!("Failed to send command: {}", e));
                return;
            }

            self.command_executing = true;
            self.status_message = Some(format!("Executing /{}", cmd.name));
        } else {
            self.status_message = Some("Invalid command".to_string());
        }
    }

    /// Build command arguments JSON from parsed command.
    fn build_command_args(&self, cmd: &crate::input::ParsedCommand) -> serde_json::Value {
        let mut args = serde_json::json!({});

        // Always include session_id if available
        if let Some(ref sid) = self.session_id {
            args["session_id"] = serde_json::Value::String(sid.clone());
        }

        // Parse additional args (simple key=value or flags)
        for part in cmd.args.split_whitespace() {
            if let Some(flag) = part.strip_prefix("--") {
                if let Some((key, value)) = flag.split_once('=') {
                    // --key=value
                    args[key] = serde_json::Value::String(value.to_string());
                } else {
                    // --flag (boolean true)
                    args[flag] = serde_json::Value::Bool(true);
                }
            } else if part == "-f" || part == "--force" {
                args["force"] = serde_json::Value::Bool(true);
            }
        }

        args
    }

    /// Get help text for available commands.
    // get_help_text, send_message → chat_handler.rs

    /// Handle sessions overlay key events.
    fn handle_sessions_key(&mut self, key: crossterm::event::KeyEvent) {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.sessions.reset();
                self.focus.pop_overlay();
            }
            KeyCode::Enter => {
                // Select the current session
                if let Some(session) = self.sessions.selected_session() {
                    let session_id = session.id.clone();
                    self.switch_to_session(&session_id);
                }
                self.sessions.reset();
                self.focus.pop_overlay();
            }
            KeyCode::Up => {
                self.sessions.select_prev();
            }
            KeyCode::Down => {
                self.sessions.select_next();
            }
            KeyCode::Home => {
                self.sessions.select_first();
            }
            KeyCode::End => {
                self.sessions.select_last();
            }
            KeyCode::Char('n') if has_ctrl => {
                // Create new session
                self.create_new_session();
                self.sessions.reset();
                self.focus.pop_overlay();
            }
            KeyCode::Char(c) => {
                // Add to filter
                self.sessions.filter_push(c);
            }
            KeyCode::Backspace => {
                self.sessions.filter_pop();
            }
            _ => {}
        }
    }

    /// Handle command palette key events.
    fn handle_palette_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.palette.reset();
                self.focus.pop_overlay();
            }
            KeyCode::Enter => {
                // Execute selected action
                if let Some(action) = self.palette.selected_action() {
                    let action_id = action.id;
                    self.palette.reset();
                    self.focus.pop_overlay();
                    self.execute_action(action_id);
                } else {
                    self.palette.reset();
                    self.focus.pop_overlay();
                }
            }
            KeyCode::Up => {
                self.palette.select_prev();
            }
            KeyCode::Down => {
                self.palette.select_next();
            }
            KeyCode::Home => {
                self.palette.select_first();
            }
            KeyCode::End => {
                self.palette.select_last();
            }
            KeyCode::Char(c) => {
                self.palette.filter_push(c);
            }
            KeyCode::Backspace => {
                self.palette.filter_pop();
            }
            _ => {}
        }
    }

    /// Execute a palette action.
    fn execute_action(&mut self, action_id: ActionId) {
        match action_id {
            ActionId::SessionsSwitch => {
                self.open_sessions_panel();
            }
            ActionId::SessionsNew => {
                self.create_new_session();
            }
            ActionId::SessionsDelete => {
                self.sidebar.open();
                self.sidebar.section = SidebarSection::Sessions;
                self.focus.focus(FocusTarget::Sidebar);
                self.status_message = Some("Select a session and press 'd' to delete".to_string());
            }
            ActionId::SessionsMoveToWorkstream => {
                if let Some(ref sid) = self.session_id {
                    tracing::info!("Session move initiated for session: {}", sid);
                    // Open sidebar in workstreams section for selection
                    self.sidebar.open();
                    self.sidebar.section = SidebarSection::Workstreams;
                    self.moving_session_to_workstream = true;
                    self.focus.focus(FocusTarget::Sidebar);
                    self.status_message =
                        Some("Select target workstream (Enter to move, Esc to cancel)".to_string());
                } else {
                    tracing::warn!("Session move attempted with no active session");
                    self.status_message = Some("No session to move".to_string());
                }
            }
            ActionId::WorkstreamsSwitch => {
                self.focus.push_overlay(FocusTarget::Workstreams);
            }
            ActionId::WorkstreamsCreate => {
                // Enter new workstream name mode
                self.input_mode = InputMode::NewWorkstream;
                self.input.clear();
                self.focus.return_to_input();
                self.status_message =
                    Some("New workstream: Enter name (Esc to cancel)".to_string());
            }
            ActionId::ViewToggleToolPane => {
                self.focus.toggle(FocusTarget::ToolPane);
            }
            ActionId::AppQuit => {
                self.should_quit = true;
            }
        }
    }

    /// Switch to a different session.
    pub fn switch_to_session(&mut self, session_id: &str) {
        // Subscribe to the new session FIRST to avoid missing messages
        // that might arrive between subscribe and fetch
        // Use reconnect token if we have one to reclaim ownership
        let reconnect_token = self.reconnect_tokens.get(session_id).cloned();
        if let Err(e) = self
            .ws_client
            .subscribe(session_id.to_string(), reconnect_token)
        {
            self.status_message = Some(format!("Failed to switch session: {}", e));
            return; // Don't clear state if we failed to subscribe
        }

        // Reset ownership state - will be updated by SubscribeAck
        self.is_session_owner = false;

        // Now clear current messages and tools
        self.messages.clear();
        self.tools.clear();
        self.session_id = Some(session_id.to_string());
        self.sessions.set_current(session_id);
        self.sidebar.set_current_session(session_id);
        self.chat_scroll = 0;
        self.chat_auto_scroll = true;

        self.status_message = Some("Loading session...".to_string());

        // Queue fetch of message history
        self.pending_actions
            .push(PendingAction::FetchSessionMessages(session_id.to_string()));
    }

    /// Create a new session.
    fn create_new_session(&mut self) {
        self.messages.clear();
        self.tools.clear();
        self.session_id = None; // Will be assigned by server on first message
        self.chat_scroll = 0;
        self.chat_auto_scroll = true;
        self.status_message = Some("New session created".to_string());
    }

    /// Open the sessions panel.
    fn open_sessions_panel(&mut self) {
        self.sessions.reset();
        self.focus.push_overlay(FocusTarget::Sessions);

        // Use sessions from sidebar (already loaded from API)
        self.sessions.set_items(self.sidebar.sessions.clone());
    }

    // Handlers moved to submodules:
    // handle_overlay_key, clear_pending_deletes, handle_sidebar_key, switch_to_workstream → sidebar_handler.rs
    // handle_tool_pane_key, open_tool_in_editor, run_pager → tool_pane_handler.rs
    // handle_logs_key → logs_handler.rs

}

#[cfg(test)]
impl App {
    /// Create a test App with a mock WsClient and no real connections.
    fn test_new() -> Self {
        use crate::client::WsClient;

        let api = ArawnClient::builder()
            .base_url("http://test:0")
            .build()
            .expect("test API client");

        Self {
            server_url: "http://test:0".to_string(),
            ws_client: WsClient::mock(),
            api,
            connection_status: ConnectionStatus::Connected,
            focus: FocusManager::new(),
            should_quit: false,
            input: InputState::new(),
            input_mode: InputMode::default(),
            status_message: None,
            workstream: "scratch".to_string(),
            workstream_id: None,
            session_id: None,
            messages: BoundedVec::with_capacity(MAX_MESSAGES, 1024),
            tools: BoundedVec::with_capacity(MAX_TOOLS, 64),
            waiting: false,
            chat_scroll: 0,
            chat_auto_scroll: true,
            sessions: SessionList::new(),
            palette: CommandPalette::new(),
            context_name: None,
            log_buffer: LogBuffer::new(),
            log_scroll: 0,
            show_logs: false,
            sidebar: Sidebar::new(),
            moving_session_to_workstream: false,
            tool_scroll: 0,
            selected_tool_index: None,
            show_tool_pane: false,
            pending_actions: Vec::new(),
            command_popup: CommandPopup::new(),
            command_executing: false,
            command_progress: None,
            context_info: None,
            workstream_usage: None,
            disk_warnings: Vec::new(),
            show_usage_popup: false,
            reconnect_tokens: std::collections::HashMap::new(),
            is_session_owner: true,
            pending_delete_workstream: None,
            pending_delete_session: None,
            panel_areas: PanelAreas::default(),
            last_ping: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // ── Server Message Handling ──────────────────────────────────────

    #[tokio::test]
    async fn test_session_created_sets_session_id() {
        let mut app = App::test_new();
        assert!(app.session_id.is_none());

        app.handle_server_message(ServerMessage::SessionCreated {
            session_id: "sess-123".to_string(),
        });

        assert_eq!(app.session_id.as_deref(), Some("sess-123"));
    }

    #[tokio::test]
    async fn test_chat_chunk_creates_assistant_message() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "s1".to_string(),
            chunk: "Hello ".to_string(),
            done: false,
        });

        assert_eq!(app.messages.len(), 1);
        assert!(!app.messages[0].is_user);
        assert_eq!(app.messages[0].content, "Hello ");
        assert!(app.messages[0].streaming);
    }

    #[tokio::test]
    async fn test_chat_chunk_appends_to_streaming() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "s1".to_string(),
            chunk: "Hello ".to_string(),
            done: false,
        });
        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "s1".to_string(),
            chunk: "world!".to_string(),
            done: false,
        });

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "Hello world!");
    }

    #[tokio::test]
    async fn test_chat_done_clears_waiting() {
        let mut app = App::test_new();
        app.waiting = true;

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "s1".to_string(),
            chunk: "Response".to_string(),
            done: false,
        });
        assert!(app.waiting);

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "s1".to_string(),
            chunk: String::new(),
            done: true,
        });

        assert!(!app.waiting);
        assert!(!app.messages[0].streaming);
    }

    #[tokio::test]
    async fn test_error_clears_waiting() {
        let mut app = App::test_new();
        app.waiting = true;

        app.handle_server_message(ServerMessage::Error {
            code: "internal".to_string(),
            message: "broke".to_string(),
        });

        assert!(!app.waiting);
        assert!(app.status_message.is_some());
    }

    #[tokio::test]
    async fn test_session_not_owned_sets_read_only() {
        let mut app = App::test_new();
        assert!(app.is_session_owner);

        app.handle_server_message(ServerMessage::Error {
            code: "session_not_owned".to_string(),
            message: "not yours".to_string(),
        });

        assert!(!app.is_session_owner);
        assert!(app.status_message.as_deref().unwrap().contains("Read-only"));
    }

    #[tokio::test]
    async fn test_subscribe_ack_owner() {
        let mut app = App::test_new();
        app.is_session_owner = false;

        app.handle_server_message(ServerMessage::SubscribeAck {
            session_id: "s1".to_string(),
            owner: true,
            reconnect_token: Some("tok".to_string()),
        });

        assert!(app.is_session_owner);
        assert_eq!(
            app.reconnect_tokens.get("s1").map(String::as_str),
            Some("tok")
        );
    }

    #[tokio::test]
    async fn test_subscribe_ack_reader() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::SubscribeAck {
            session_id: "s1".to_string(),
            owner: false,
            reconnect_token: None,
        });

        assert!(!app.is_session_owner);
    }

    #[tokio::test]
    async fn test_auth_success() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::AuthResult {
            success: true,
            error: None,
        });

        assert_eq!(app.status_message.as_deref(), Some("Authenticated"));
    }

    #[tokio::test]
    async fn test_auth_failure() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::AuthResult {
            success: false,
            error: Some("bad token".to_string()),
        });

        assert!(
            app.status_message
                .as_deref()
                .unwrap()
                .contains("Auth failed")
        );
    }

    #[tokio::test]
    async fn test_context_info_updates() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::ContextInfo {
            session_id: "s1".to_string(),
            current_tokens: 5000,
            max_tokens: 100000,
            percent: 5,
            status: "ok".to_string(),
        });

        let ctx = app.context_info.as_ref().unwrap();
        assert_eq!(ctx.current_tokens, 5000);
        assert_eq!(ctx.percent, 5);
    }

    // ── Tool Lifecycle ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_tool_lifecycle() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::ToolStart {
            session_id: "s1".to_string(),
            tool_id: "c1".to_string(),
            tool_name: "read_file".to_string(),
        });
        assert_eq!(app.tools.len(), 1);
        assert!(app.tools[0].running);

        app.handle_server_message(ServerMessage::ToolOutput {
            session_id: "s1".to_string(),
            tool_id: "c1".to_string(),
            content: "file data".to_string(),
        });
        assert_eq!(app.tools[0].output, "file data");

        app.handle_server_message(ServerMessage::ToolEnd {
            session_id: "s1".to_string(),
            tool_id: "c1".to_string(),
            success: true,
        });
        assert!(!app.tools[0].running);
        assert_eq!(app.tools[0].success, Some(true));
        assert!(app.tools[0].duration_ms.is_some());
    }

    // ── Command Handling ────────────────────────────────────────────

    #[tokio::test]
    async fn test_command_progress_and_result() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::CommandProgress {
            command: "compact".to_string(),
            message: "Working...".to_string(),
            percent: Some(50),
        });
        assert!(app.command_executing);

        app.handle_server_message(ServerMessage::CommandResult {
            command: "compact".to_string(),
            success: true,
            result: serde_json::json!({"message": "Done"}),
        });
        assert!(!app.command_executing);
        assert!(app.command_progress.is_none());
    }

    // ── Key Handling — Global Shortcuts ──────────────────────────────

    #[tokio::test]
    async fn test_ctrl_q_quits() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_ctrl_c_quits_when_idle() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_ctrl_c_cancels_when_waiting() {
        let mut app = App::test_new();
        app.waiting = true;
        app.session_id = Some("s1".to_string());

        app.handle_key(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert!(!app.waiting);
        assert!(!app.should_quit);
        assert_eq!(app.status_message.as_deref(), Some("Cancelled"));
    }

    #[tokio::test]
    async fn test_ctrl_k_opens_palette() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(app.focus.has_overlay());
        assert_eq!(app.focus.current(), FocusTarget::CommandPalette);
    }

    #[tokio::test]
    async fn test_ctrl_w_toggles_sidebar() {
        let mut app = App::test_new();
        assert!(!app.sidebar.is_open());

        app.handle_key(key_mod(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert!(app.sidebar.is_open());
        assert_eq!(app.focus.current(), FocusTarget::Sidebar);

        app.handle_key(key_mod(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert!(!app.sidebar.is_open());
        assert_eq!(app.focus.current(), FocusTarget::Input);
    }

    #[tokio::test]
    async fn test_ctrl_e_toggles_tool_pane() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert!(app.show_tool_pane);
        assert_eq!(app.focus.current(), FocusTarget::ToolPane);

        app.handle_key(key_mod(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert!(!app.show_tool_pane);
    }

    #[tokio::test]
    async fn test_ctrl_l_toggles_logs() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('l'), KeyModifiers::CONTROL));
        assert!(app.show_logs);

        app.handle_key(key_mod(KeyCode::Char('l'), KeyModifiers::CONTROL));
        assert!(!app.show_logs);
    }

    #[tokio::test]
    async fn test_ctrl_u_toggles_usage() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(app.show_usage_popup);

        app.handle_key(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(!app.show_usage_popup);
    }

    // ── Input Handling — Chat ───────────────────────────────────────

    #[tokio::test]
    async fn test_typing_adds_to_input() {
        let mut app = App::test_new();
        app.handle_key(key(KeyCode::Char('h')));
        app.handle_key(key(KeyCode::Char('i')));
        assert_eq!(app.input.content(), "hi");
    }

    #[tokio::test]
    async fn test_enter_sends_message() {
        let mut app = App::test_new();
        app.input.set_text("hello");
        app.handle_key(key(KeyCode::Enter));

        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].is_user);
        assert_eq!(app.messages[0].content, "hello");
        assert!(app.waiting);
    }

    #[tokio::test]
    async fn test_enter_blocked_when_waiting() {
        let mut app = App::test_new();
        app.waiting = true;
        app.input.set_text("hi");

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.input.content(), "hi");
        assert!(app.messages.is_empty());
    }

    #[tokio::test]
    async fn test_send_blocked_in_read_only() {
        let mut app = App::test_new();
        app.is_session_owner = false;
        app.input.set_text("hi");

        app.send_message();

        assert!(app.messages.is_empty());
        assert!(app.status_message.as_deref().unwrap().contains("Read-only"));
    }

    #[tokio::test]
    async fn test_enter_on_empty_does_nothing() {
        let mut app = App::test_new();
        app.handle_key(key(KeyCode::Enter));
        assert!(app.messages.is_empty());
        assert!(!app.waiting);
    }

    #[tokio::test]
    async fn test_shift_enter_inserts_newline() {
        let mut app = App::test_new();
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key_mod(KeyCode::Enter, KeyModifiers::SHIFT));
        app.handle_key(key(KeyCode::Char('b')));
        assert!(app.input.content().contains('\n'));
    }

    // ── Connection State ────────────────────────────────────────────

    #[tokio::test]
    async fn test_waiting_cleared_on_disconnect() {
        let mut app = App::test_new();
        app.connection_status = ConnectionStatus::Connected;
        app.waiting = true;

        // Simulate the tick handler logic
        let was_connected = app.connection_status == ConnectionStatus::Connected;
        app.connection_status = ConnectionStatus::Disconnected;
        if was_connected && app.connection_status != ConnectionStatus::Connected && app.waiting {
            app.waiting = false;
            app.status_message =
                Some("Connection lost — message may not have been delivered".to_string());
        }

        assert!(!app.waiting);
        assert!(
            app.status_message
                .as_deref()
                .unwrap()
                .contains("Connection lost")
        );
    }

    #[tokio::test]
    async fn test_waiting_not_cleared_if_not_previously_connected() {
        let mut app = App::test_new();
        app.connection_status = ConnectionStatus::Connecting;
        app.waiting = true;

        let was_connected = app.connection_status == ConnectionStatus::Connected;
        app.connection_status = ConnectionStatus::Disconnected;
        if was_connected && app.connection_status != ConnectionStatus::Connected && app.waiting {
            app.waiting = false;
        }

        assert!(app.waiting, "Should still be waiting — was never connected");
    }

    // ── Session Switching ───────────────────────────────────────────

    #[tokio::test]
    async fn test_switch_session_clears_state() {
        let mut app = App::test_new();
        app.push_message(ChatMessage {
            is_user: true,
            content: "old".to_string(),
            streaming: false,
        });
        app.push_tool(ToolExecution {
            id: "t1".to_string(),
            name: "tool".to_string(),
            args: String::new(),
            output: String::new(),
            running: false,
            success: Some(true),
            started_at: std::time::Instant::now(),
            duration_ms: Some(100),
        });

        app.switch_to_session("new-sess");

        assert!(app.messages.is_empty());
        assert!(app.tools.is_empty());
        assert_eq!(app.session_id.as_deref(), Some("new-sess"));
        assert!(!app.is_session_owner);
    }

    // ── Workstream Switching ────────────────────────────────────────

    #[tokio::test]
    async fn test_switch_workstream_clears_session() {
        let mut app = App::test_new();
        app.session_id = Some("old".to_string());
        app.push_message(ChatMessage {
            is_user: true,
            content: "msg".to_string(),
            streaming: false,
        });

        app.switch_to_workstream("project-x");

        assert_eq!(app.workstream, "project-x");
        assert!(app.session_id.is_none());
        assert!(app.messages.is_empty());
    }

    // ── Command Detection ───────────────────────────────────────────

    #[tokio::test]
    async fn test_slash_command_detected() {
        let mut app = App::test_new();
        app.input.set_text("/help");
        assert!(app.input.is_command());
    }

    #[tokio::test]
    async fn test_regular_text_not_command() {
        let mut app = App::test_new();
        app.input.set_text("hello");
        assert!(!app.input.is_command());
    }

    // ── Disk Warnings ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_disk_pressure_stored() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::DiskPressure {
            workstream_id: "ws-1".to_string(),
            workstream_name: "proj".to_string(),
            level: "warning".to_string(),
            usage_bytes: 500_000_000,
            limit_bytes: 1_000_000_000,
            percent: 50,
        });

        assert_eq!(app.disk_warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_disk_pressure_replaces_existing() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::DiskPressure {
            workstream_id: "ws-1".to_string(),
            workstream_name: "proj".to_string(),
            level: "warning".to_string(),
            usage_bytes: 500_000_000,
            limit_bytes: 1_000_000_000,
            percent: 50,
        });
        app.handle_server_message(ServerMessage::DiskPressure {
            workstream_id: "ws-1".to_string(),
            workstream_name: "proj".to_string(),
            level: "critical".to_string(),
            usage_bytes: 900_000_000,
            limit_bytes: 1_000_000_000,
            percent: 90,
        });

        assert_eq!(app.disk_warnings.len(), 1);
        assert_eq!(app.disk_warnings[0].percent, 90);
    }

    #[tokio::test]
    async fn test_disk_critical_sets_status() {
        let mut app = App::test_new();

        app.handle_server_message(ServerMessage::DiskPressure {
            workstream_id: "ws-1".to_string(),
            workstream_name: "proj".to_string(),
            level: "critical".to_string(),
            usage_bytes: 900_000_000,
            limit_bytes: 1_000_000_000,
            percent: 90,
        });

        assert!(app.status_message.as_deref().unwrap().contains("critical"));
    }

    // ── Workstream Usage ────────────────────────────────────────────

    #[tokio::test]
    async fn test_usage_updates_for_current_workstream() {
        let mut app = App::test_new();
        app.workstream_id = Some("ws-1".to_string());

        app.handle_server_message(ServerMessage::WorkstreamUsage {
            workstream_id: "ws-1".to_string(),
            workstream_name: "proj".to_string(),
            is_scratch: false,
            production_bytes: 1000,
            work_bytes: 2000,
            total_bytes: 3000,
            limit_bytes: 10000,
            percent: 30,
        });

        assert_eq!(app.workstream_usage.as_ref().unwrap().total_bytes, 3000);
    }

    #[tokio::test]
    async fn test_usage_ignored_for_other_workstream() {
        let mut app = App::test_new();
        app.workstream_id = Some("ws-1".to_string());

        app.handle_server_message(ServerMessage::WorkstreamUsage {
            workstream_id: "ws-OTHER".to_string(),
            workstream_name: "other".to_string(),
            is_scratch: false,
            production_bytes: 1000,
            work_bytes: 2000,
            total_bytes: 3000,
            limit_bytes: 10000,
            percent: 30,
        });

        assert!(app.workstream_usage.is_none());
    }

    // ── Palette ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_palette_esc_closes() {
        let mut app = App::test_new();
        app.handle_key(key_mod(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(app.focus.has_overlay());

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.focus.has_overlay());
    }

    // ── Full Message Flow ───────────────────────────────────────────

    #[tokio::test]
    async fn test_full_message_flow() {
        let mut app = App::test_new();
        app.input.set_text("hello");
        app.handle_key(key(KeyCode::Enter));

        assert!(app.waiting);
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].is_user);

        app.handle_server_message(ServerMessage::SessionCreated {
            session_id: "new-sess".to_string(),
        });
        assert_eq!(app.session_id.as_deref(), Some("new-sess"));

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "new-sess".to_string(),
            chunk: "Hi!".to_string(),
            done: false,
        });
        assert_eq!(app.messages.len(), 2);
        assert!(app.waiting);

        app.handle_server_message(ServerMessage::ChatChunk {
            session_id: "new-sess".to_string(),
            chunk: String::new(),
            done: true,
        });
        assert!(!app.waiting);
        assert!(!app.messages[1].streaming);
    }

    #[tokio::test]
    async fn test_send_clears_tools() {
        let mut app = App::test_new();
        app.push_tool(ToolExecution {
            id: "old".to_string(),
            name: "tool".to_string(),
            args: String::new(),
            output: String::new(),
            running: false,
            success: Some(true),
            started_at: std::time::Instant::now(),
            duration_ms: Some(100),
        });

        app.input.set_text("new question");
        app.send_message();

        assert!(app.tools.is_empty());
    }

    #[tokio::test]
    async fn test_send_enables_auto_scroll() {
        let mut app = App::test_new();
        app.chat_auto_scroll = false;

        app.input.set_text("msg");
        app.send_message();

        assert!(app.chat_auto_scroll);
    }

    // ── Reconnection Resilience ────────────────────────────────────

    /// Helper: simulate the tick handler's connection status poll logic.
    fn simulate_status_poll(app: &mut App) {
        if let Some(status) = app.ws_client.poll_status() {
            let was_connected = app.connection_status == ConnectionStatus::Connected;
            app.connection_status = status;

            if was_connected && status != ConnectionStatus::Connected && app.waiting {
                app.waiting = false;
                app.status_message =
                    Some("Connection lost — message may not have been delivered".to_string());
            }
        }
    }

    fn test_app_controllable() -> (
        App,
        tokio::sync::mpsc::UnboundedSender<ConnectionStatus>,
        tokio::sync::mpsc::UnboundedSender<crate::protocol::ServerMessage>,
    ) {
        use crate::client::WsClient;

        let (ws_client, status_tx, msg_tx) = WsClient::mock_controllable();

        let api = ArawnClient::builder()
            .base_url("http://test:0")
            .build()
            .expect("test API client");

        let app = App {
            server_url: "http://test:0".to_string(),
            ws_client,
            api,
            connection_status: ConnectionStatus::Connected,
            focus: FocusManager::new(),
            should_quit: false,
            input: InputState::new(),
            input_mode: InputMode::default(),
            status_message: None,
            workstream: "scratch".to_string(),
            workstream_id: None,
            session_id: None,
            messages: BoundedVec::with_capacity(MAX_MESSAGES, 1024),
            tools: BoundedVec::with_capacity(MAX_TOOLS, 64),
            waiting: false,
            chat_scroll: 0,
            chat_auto_scroll: true,
            sessions: SessionList::new(),
            palette: CommandPalette::new(),
            context_name: None,
            log_buffer: LogBuffer::new(),
            log_scroll: 0,
            show_logs: false,
            sidebar: Sidebar::new(),
            moving_session_to_workstream: false,
            tool_scroll: 0,
            selected_tool_index: None,
            show_tool_pane: false,
            pending_actions: Vec::new(),
            command_popup: CommandPopup::new(),
            command_executing: false,
            command_progress: None,
            context_info: None,
            workstream_usage: None,
            disk_warnings: Vec::new(),
            show_usage_popup: false,
            reconnect_tokens: std::collections::HashMap::new(),
            is_session_owner: true,
            pending_delete_workstream: None,
            pending_delete_session: None,
            panel_areas: PanelAreas::default(),
            last_ping: std::time::Instant::now(),
        };

        (app, status_tx, msg_tx)
    }

    #[tokio::test]
    async fn test_disconnect_shows_status_indicator() {
        let (mut app, status_tx, _msg_tx) = test_app_controllable();
        assert_eq!(app.connection_status, ConnectionStatus::Connected);

        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);

        assert_eq!(app.connection_status, ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_reconnecting_shows_attempt_count() {
        let (mut app, status_tx, _msg_tx) = test_app_controllable();

        status_tx
            .send(ConnectionStatus::Reconnecting { attempt: 1 })
            .unwrap();
        simulate_status_poll(&mut app);
        assert_eq!(
            app.connection_status,
            ConnectionStatus::Reconnecting { attempt: 1 }
        );

        status_tx
            .send(ConnectionStatus::Reconnecting { attempt: 2 })
            .unwrap();
        simulate_status_poll(&mut app);
        assert_eq!(
            app.connection_status,
            ConnectionStatus::Reconnecting { attempt: 2 }
        );
    }

    #[tokio::test]
    async fn test_full_reconnection_lifecycle() {
        let (mut app, status_tx, _msg_tx) = test_app_controllable();
        app.waiting = true;

        // Connected → Disconnected: clears waiting, sets status message
        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);
        assert!(!app.waiting);
        assert!(
            app.status_message
                .as_deref()
                .unwrap()
                .contains("Connection lost")
        );

        // Disconnected → Reconnecting(1)
        status_tx
            .send(ConnectionStatus::Reconnecting { attempt: 1 })
            .unwrap();
        simulate_status_poll(&mut app);
        assert_eq!(
            app.connection_status,
            ConnectionStatus::Reconnecting { attempt: 1 }
        );

        // Reconnecting → Connected: connection restored
        status_tx.send(ConnectionStatus::Connected).unwrap();
        simulate_status_poll(&mut app);
        assert_eq!(app.connection_status, ConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn test_session_state_preserved_across_reconnect() {
        let (mut app, status_tx, msg_tx) = test_app_controllable();

        // Set up session state
        app.session_id = Some("sess-42".to_string());
        app.reconnect_tokens
            .insert("sess-42".to_string(), "tok-abc".to_string());
        app.push_message(ChatMessage {
            is_user: true,
            content: "hello".to_string(),
            streaming: false,
        });

        // Disconnect
        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);

        // Session ID and messages should be preserved
        assert_eq!(app.session_id.as_deref(), Some("sess-42"));
        assert_eq!(app.messages.len(), 1);
        assert_eq!(
            app.reconnect_tokens.get("sess-42").map(String::as_str),
            Some("tok-abc")
        );

        // Reconnect
        status_tx.send(ConnectionStatus::Connected).unwrap();
        simulate_status_poll(&mut app);

        // Session data still intact after reconnection
        assert_eq!(app.session_id.as_deref(), Some("sess-42"));
        assert_eq!(app.messages.len(), 1);

        // Server can send a subscribe ack to restore ownership
        msg_tx
            .send(ServerMessage::SubscribeAck {
                session_id: "sess-42".to_string(),
                owner: true,
                reconnect_token: Some("tok-new".to_string()),
            })
            .unwrap();
        if let Some(msg) = app.ws_client.try_recv() {
            app.handle_server_message(msg);
        }
        assert!(app.is_session_owner);
        assert_eq!(
            app.reconnect_tokens.get("sess-42").map(String::as_str),
            Some("tok-new")
        );
    }

    #[tokio::test]
    async fn test_rapid_disconnect_reconnect_cycles_no_panic() {
        let (mut app, status_tx, _msg_tx) = test_app_controllable();

        // Rapid cycles: Connected→Disconnected→Connected×10
        for cycle in 0..10 {
            app.waiting = cycle % 2 == 0; // alternate waiting state
            status_tx.send(ConnectionStatus::Disconnected).unwrap();
            simulate_status_poll(&mut app);
            assert_eq!(app.connection_status, ConnectionStatus::Disconnected);

            status_tx
                .send(ConnectionStatus::Reconnecting { attempt: cycle + 1 })
                .unwrap();
            simulate_status_poll(&mut app);

            status_tx.send(ConnectionStatus::Connected).unwrap();
            simulate_status_poll(&mut app);
            assert_eq!(app.connection_status, ConnectionStatus::Connected);
        }

        // App should still be functional
        assert!(!app.should_quit);
        app.input.set_text("still works");
        app.send_message();
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].is_user);
    }

    #[tokio::test]
    async fn test_disconnect_during_streaming_marks_message_not_streaming() {
        let (mut app, status_tx, msg_tx) = test_app_controllable();

        // Start a streaming response
        msg_tx
            .send(ServerMessage::ChatChunk {
                session_id: "s1".to_string(),
                chunk: "partial res".to_string(),
                done: false,
            })
            .unwrap();
        if let Some(msg) = app.ws_client.try_recv() {
            app.handle_server_message(msg);
        }
        app.waiting = true;
        assert!(app.messages[0].streaming);

        // Disconnect mid-stream
        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);

        // Waiting should be cleared
        assert!(!app.waiting);
        // The partial message content is preserved
        assert_eq!(app.messages[0].content, "partial res");
    }

    #[tokio::test]
    async fn test_messages_received_after_reconnect_handled_correctly() {
        let (mut app, status_tx, msg_tx) = test_app_controllable();
        app.session_id = Some("s1".to_string());

        // Disconnect and reconnect
        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);
        status_tx.send(ConnectionStatus::Connected).unwrap();
        simulate_status_poll(&mut app);

        // Server sends new messages after reconnect
        msg_tx
            .send(ServerMessage::ChatChunk {
                session_id: "s1".to_string(),
                chunk: "post-reconnect".to_string(),
                done: false,
            })
            .unwrap();
        if let Some(msg) = app.ws_client.try_recv() {
            app.handle_server_message(msg);
        }

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "post-reconnect");

        msg_tx
            .send(ServerMessage::ChatChunk {
                session_id: "s1".to_string(),
                chunk: String::new(),
                done: true,
            })
            .unwrap();
        if let Some(msg) = app.ws_client.try_recv() {
            app.handle_server_message(msg);
        }
        assert!(!app.messages[0].streaming);
    }

    #[tokio::test]
    async fn test_disconnect_while_not_waiting_no_status_change() {
        let (mut app, status_tx, _msg_tx) = test_app_controllable();
        app.waiting = false;
        app.status_message = None;

        status_tx.send(ConnectionStatus::Disconnected).unwrap();
        simulate_status_poll(&mut app);

        // Status changes to disconnected but no "Connection lost" message
        // since we weren't waiting for a response
        assert_eq!(app.connection_status, ConnectionStatus::Disconnected);
        assert!(
            app.status_message.is_none(),
            "No status message when not waiting"
        );
    }
}

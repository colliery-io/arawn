//! Application state and main loop.

mod api_ops;
mod chat_handler;
mod input_handler;
mod logs_handler;
mod server_msg_handler;
mod session_handler;
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
use crate::sessions::SessionList;
use crate::sidebar::{Sidebar, SidebarSection};
use crate::ui;
use crate::ui::CommandPopup;
use anyhow::Result;
use arawn_client::ArawnClient;

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

    /// Process a tick event: poll connection status and send keepalive pings.
    ///
    /// Returns `true` if sidebar data should be loaded (first connection).
    pub fn process_tick(&mut self) -> bool {
        let mut should_load_data = false;

        if let Some(status) = self.ws_client.poll_status() {
            let was_connected = self.connection_status == ConnectionStatus::Connected;
            self.connection_status = status;

            if was_connected && status != ConnectionStatus::Connected && self.waiting {
                self.waiting = false;
                self.status_message =
                    Some("Connection lost — message may not have been delivered".to_string());
            }

            if !was_connected && status == ConnectionStatus::Connected {
                should_load_data = true;
            }
        }

        if self.last_ping.elapsed() >= std::time::Duration::from_secs(30) {
            let _ = self.ws_client.send_ping();
            self.last_ping = std::time::Instant::now();
        }

        should_load_data
    }

    /// Run the main application loop.
    pub async fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        let mut events = EventHandler::new();
        let mut data_loaded = false;

        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;

            tokio::select! {
                event = events.next() => {
                    match event? {
                        Event::Key(key) => self.handle_key(key),
                        Event::Mouse(mouse) => self.handle_mouse(mouse),
                        Event::Tick => {
                            if self.process_tick() && !data_loaded {
                                data_loaded = true;
                                self.refresh_sidebar_data().await;
                            }
                        }
                        Event::Resize(_, _) => {}
                    }
                }

                msg = self.ws_client.recv() => {
                    if let Some(msg) = msg {
                        self.handle_server_message(msg);
                    }
                }
            }

            self.process_pending_actions().await;
        }

        Ok(())
    }

    /// Run the application in headless mode for testing.
    ///
    /// Uses a provided terminal (typically `Terminal<TestBackend>`) and receives
    /// events from a channel instead of crossterm. Exits after `max_ticks`
    /// iterations, when the event channel closes, or when `should_quit` is set.
    pub async fn run_headless<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
        mut event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::events::Event>,
        max_ticks: usize,
    ) -> Result<()> {
        let mut data_loaded = false;
        let mut ticks = 0;

        while !self.should_quit && ticks < max_ticks {
            terminal.draw(|frame| ui::render(self, frame))?;

            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Some(crate::events::Event::Key(key)) => self.handle_key(key),
                        Some(crate::events::Event::Mouse(mouse)) => self.handle_mouse(mouse),
                        Some(crate::events::Event::Tick) => {
                            ticks += 1;
                            if self.process_tick() && !data_loaded {
                                data_loaded = true;
                                self.refresh_sidebar_data().await;
                            }
                        }
                        Some(crate::events::Event::Resize(_, _)) => {}
                        None => break, // Channel closed
                    }
                }

                msg = self.ws_client.recv() => {
                    if let Some(msg) = msg {
                        self.handle_server_message(msg);
                    }
                }
            }

            self.process_pending_actions().await;
        }

        // Final render so tests can inspect the buffer
        terminal.draw(|frame| ui::render(self, frame))?;

        Ok(())
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
    use crate::protocol::ServerMessage;
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

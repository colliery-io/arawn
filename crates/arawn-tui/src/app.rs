//! Application state and main event loop.

use std::time::Instant;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Terminal, backend::Backend};
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::TuiConfig;
use crate::events::Event;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::render;
use crate::ws::ConnectionStatus;

/// A single chat message displayed in the UI.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Whether this message was sent by the user.
    pub is_user: bool,
    /// The message text content.
    pub content: String,
    /// Whether this message is still being streamed.
    pub streaming: bool,
}

/// Information about a workstream for display in the sidebar.
#[derive(Debug, Clone)]
pub struct WorkstreamInfo {
    /// Workstream ID.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Whether this is the scratch workstream.
    pub is_scratch: bool,
}

/// Information about a session for display in the sidebar.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID.
    pub id: String,
    /// When the session started (ISO 8601).
    pub started_at: String,
}

/// Information about a message within a session.
#[derive(Debug, Clone)]
pub struct MessageInfo {
    /// Role of the sender ("user" or "assistant").
    pub role: String,
    /// The message text content.
    pub content: String,
}

/// Results from background HTTP tasks.
#[derive(Debug)]
pub enum HttpResult {
    /// Fetched list of workstreams.
    Workstreams(Vec<WorkstreamInfo>),
    /// Fetched sessions for the selected workstream.
    Sessions(Vec<SessionInfo>),
    /// Fetched messages for a session (session_id, messages).
    Messages(String, Vec<MessageInfo>),
}

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The chat input box.
    Input,
    /// The workstream sidebar.
    Sidebar,
}

/// Which section of the sidebar is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSection {
    /// Workstreams list.
    Workstreams,
    /// Sessions list.
    Sessions,
}

/// The main TUI application state.
pub struct App {
    // Connection
    ws_tx: mpsc::UnboundedSender<ClientMessage>,

    // State
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
    pub workstream_id: Option<String>,
    pub workstream: String,

    // Input
    pub input: String,
    pub cursor_pos: usize,

    // UI
    pub waiting: bool,
    pub status: Option<String>,
    pub should_quit: bool,
    pub chat_scroll: usize,
    pub auto_scroll: bool,

    // Sidebar
    pub workstreams: Vec<WorkstreamInfo>,
    pub selected_workstream: usize,
    pub focus: Focus,
    pub sidebar_section: SidebarSection,

    // Sessions
    pub sessions: Vec<SessionInfo>,
    pub selected_session: usize, // 0 = "+ New Session"

    // Background HTTP
    pub http_tx: mpsc::UnboundedSender<HttpResult>,
    http_rx: mpsc::UnboundedReceiver<HttpResult>,

    // Server URL for HTTP calls
    pub server_url: String,

    // Whether we have already fetched workstreams on first connect
    has_fetched_workstreams: bool,

    // Connection status
    pub connection_status: ConnectionStatus,

    // Workstream creation mini-input (None = not creating, Some(text) = typing name)
    pub creating_workstream: Option<String>,

    // When status was last set (for auto-clear after 5 seconds)
    pub status_set_at: Option<Instant>,
}

impl App {
    /// Create a new App from config and a sender channel to the WebSocket.
    pub fn new(config: &TuiConfig, ws_tx: mpsc::UnboundedSender<ClientMessage>) -> Self {
        let (http_tx, http_rx) = mpsc::unbounded_channel();
        Self {
            ws_tx,
            messages: Vec::new(),
            session_id: None,
            workstream_id: None,
            workstream: config
                .workstream
                .clone()
                .unwrap_or_else(|| "default".into()),
            input: String::new(),
            cursor_pos: 0,
            waiting: false,
            status: None,
            should_quit: false,
            chat_scroll: 0,
            auto_scroll: true,
            workstreams: Vec::new(),
            selected_workstream: 0,
            focus: Focus::Input,
            sidebar_section: SidebarSection::Workstreams,
            sessions: Vec::new(),
            selected_session: 0,
            http_tx,
            http_rx,
            server_url: config.server_url.clone(),
            has_fetched_workstreams: false,
            connection_status: ConnectionStatus::Connecting,
            creating_workstream: None,
            status_set_at: None,
        }
    }

    /// Run the TUI with a real terminal and crossterm event stream.
    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        mut event_rx: mpsc::UnboundedReceiver<Event>,
        mut ws_rx: mpsc::UnboundedReceiver<ServerMessage>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| render::draw(f, self))?;

            if self.should_quit {
                return Ok(());
            }

            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Some(Event::Key(key)) => self.handle_key(key),
                        Some(Event::Tick) => {
                            self.maybe_clear_status();
                        }
                        None => self.should_quit = true,
                    }
                }
                msg = ws_rx.recv() => {
                    match msg {
                        Some(server_msg) => self.handle_server_message(server_msg),
                        None => {
                            self.set_status("Disconnected from server".into());
                        }
                    }
                }
                http = self.http_rx.recv() => {
                    if let Some(result) = http {
                        self.handle_http_result(result);
                    }
                }
            }
        }
    }

    /// Run in headless mode for testing. Processes up to `max_ticks` tick events
    /// (or until should_quit), then returns.
    pub async fn run_headless<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        mut event_rx: mpsc::UnboundedReceiver<Event>,
        mut ws_rx: mpsc::UnboundedReceiver<ServerMessage>,
        max_ticks: usize,
    ) -> Result<()> {
        let mut tick_count = 0;

        loop {
            terminal.draw(|f| render::draw(f, self))?;

            if self.should_quit || tick_count >= max_ticks {
                return Ok(());
            }

            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Some(Event::Key(key)) => self.handle_key(key),
                        Some(Event::Tick) => {
                            tick_count += 1;
                            self.maybe_clear_status();
                        }
                        None => return Ok(()),
                    }
                }
                msg = ws_rx.recv() => {
                    match msg {
                        Some(server_msg) => self.handle_server_message(server_msg),
                        None => {}
                    }
                }
                http = self.http_rx.recv() => {
                    if let Some(result) = http {
                        self.handle_http_result(result);
                    }
                }
            }
        }
    }

    /// Handle a key event, dispatching based on current focus.
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // Global keybindings
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Input => Focus::Sidebar,
                    Focus::Sidebar => Focus::Input,
                };
                return;
            }
            _ => {}
        }

        match self.focus {
            Focus::Input => self.handle_key_input(key),
            Focus::Sidebar => self.handle_key_sidebar(key),
        }
    }

    /// Handle key events when focus is on the chat input.
    fn handle_key_input(&mut self, key: crossterm::event::KeyEvent) {
        // Clear status on any user action
        self.clear_status_on_action();

        match key.code {
            KeyCode::Enter => {
                self.send_message();
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::PageUp => {
                self.auto_scroll = false;
                self.chat_scroll = self.chat_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.chat_scroll = self.chat_scroll.saturating_add(10);
            }
            KeyCode::Home => {
                self.auto_scroll = false;
                self.chat_scroll = 0;
            }
            KeyCode::End => {
                self.auto_scroll = true;
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    /// Handle key events when focus is on the sidebar.
    fn handle_key_sidebar(&mut self, key: crossterm::event::KeyEvent) {
        // Clear status on any user action
        self.clear_status_on_action();

        // If creating a workstream, intercept all keys for the mini-input
        if self.creating_workstream.is_some() {
            self.handle_key_creating_workstream(key);
            return;
        }

        match self.sidebar_section {
            SidebarSection::Workstreams => self.handle_key_sidebar_workstreams(key),
            SidebarSection::Sessions => self.handle_key_sidebar_sessions(key),
        }
    }

    /// Handle keys in the workstreams section of the sidebar.
    fn handle_key_sidebar_workstreams(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.selected_workstream > 0 {
                    self.selected_workstream -= 1;
                }
            }
            KeyCode::Down => {
                if !self.workstreams.is_empty()
                    && self.selected_workstream < self.workstreams.len() - 1
                {
                    self.selected_workstream += 1;
                } else if !self.workstreams.is_empty() {
                    // At the bottom of workstreams, move to sessions section
                    self.sidebar_section = SidebarSection::Sessions;
                    self.selected_session = 0;
                }
            }
            KeyCode::Enter => {
                self.select_workstream();
            }
            KeyCode::Char('n') => {
                self.creating_workstream = Some(String::new());
            }
            KeyCode::Esc => {
                self.focus = Focus::Input;
            }
            _ => {}
        }
    }

    /// Handle keys when in the "creating workstream" mini-input mode.
    fn handle_key_creating_workstream(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.creating_workstream = None;
            }
            KeyCode::Enter => {
                if let Some(name) = self.creating_workstream.take() {
                    let name = name.trim().to_string();
                    if !name.is_empty() {
                        self.spawn_create_workstream(&name);
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut name) = self.creating_workstream {
                    name.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut name) = self.creating_workstream {
                    name.push(c);
                }
            }
            _ => {}
        }
    }

    /// Handle keys in the sessions section of the sidebar.
    fn handle_key_sidebar_sessions(&mut self, key: crossterm::event::KeyEvent) {
        // Total items: 1 ("+ New Session") + sessions.len()
        let total_items = 1 + self.sessions.len();
        match key.code {
            KeyCode::Up => {
                if self.selected_session > 0 {
                    self.selected_session -= 1;
                } else {
                    // At top of sessions, move back to workstreams
                    self.sidebar_section = SidebarSection::Workstreams;
                    if !self.workstreams.is_empty() {
                        self.selected_workstream = self.workstreams.len() - 1;
                    }
                }
            }
            KeyCode::Down => {
                if self.selected_session < total_items - 1 {
                    self.selected_session += 1;
                }
            }
            KeyCode::Enter => {
                self.select_session();
            }
            KeyCode::Esc => {
                self.focus = Focus::Input;
            }
            _ => {}
        }
    }

    /// Send the current input as a chat message.
    fn send_message(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        // Add to local messages
        self.messages.push(ChatMessage {
            is_user: true,
            content: text.clone(),
            streaming: false,
        });

        // Send via WebSocket
        let _ = self.ws_tx.send(ClientMessage::Chat {
            session_id: self.session_id.clone(),
            workstream_id: self.workstream_id.clone(),
            message: text,
        });

        // Clear input
        self.input.clear();
        self.cursor_pos = 0;
        self.waiting = true;
        self.auto_scroll = true;
    }

    /// Select the currently highlighted workstream in the sidebar.
    fn select_workstream(&mut self) {
        if let Some(ws) = self.workstreams.get(self.selected_workstream) {
            let ws_id = ws.id.clone();
            self.workstream_id = Some(ws_id.clone());
            self.workstream = ws.title.clone();
            self.messages.clear();
            self.session_id = None;
            self.sessions.clear();
            self.selected_session = 0;
            self.sidebar_section = SidebarSection::Sessions;

            // Fetch sessions for this workstream in the background
            self.spawn_fetch_sessions(&ws_id);
        }
    }

    /// Select the currently highlighted session in the sidebar.
    fn select_session(&mut self) {
        if self.selected_session == 0 {
            // "+ New Session" — clear chat and return to input
            self.messages.clear();
            self.session_id = None;
            self.focus = Focus::Input;
        } else {
            // Existing session — fetch messages
            let session_idx = self.selected_session - 1;
            if let Some(session) = self.sessions.get(session_idx) {
                let session_id = session.id.clone();
                self.session_id = Some(session_id.clone());
                self.messages.clear();
                self.focus = Focus::Input;

                // Fetch session messages in the background
                self.spawn_fetch_messages(&session_id);
            }
        }
    }

    /// Handle a result from a background HTTP task.
    fn handle_http_result(&mut self, result: HttpResult) {
        match result {
            HttpResult::Workstreams(ws) => {
                self.workstreams = ws;
                // Clamp selection index
                if self.selected_workstream >= self.workstreams.len()
                    && !self.workstreams.is_empty()
                {
                    self.selected_workstream = self.workstreams.len() - 1;
                }
            }
            HttpResult::Sessions(sessions) => {
                self.sessions = sessions;
                // Keep selected_session at 0 ("+ New Session")
                self.selected_session = 0;
            }
            HttpResult::Messages(session_id, messages) => {
                // Only apply if this is still the selected session
                if self.session_id.as_deref() == Some(&session_id) {
                    self.messages = messages
                        .into_iter()
                        .filter(|m| m.role == "user" || m.role == "assistant")
                        .map(|m| ChatMessage {
                            is_user: m.role == "user",
                            content: m.content,
                            streaming: false,
                        })
                        .collect();
                    self.auto_scroll = true;
                }
            }
        }
    }

    /// Spawn a background HTTP task to fetch workstreams from the server.
    pub fn spawn_fetch_workstreams(&self) {
        let tx = self.http_tx.clone();
        let server_url = self.server_url.clone();
        tokio::spawn(async move {
            let client = match arawn_client::ArawnClient::builder()
                .base_url(&server_url)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to create HTTP client for workstreams: {}", e);
                    return;
                }
            };
            match client.workstreams().list().await {
                Ok(resp) => {
                    let infos: Vec<WorkstreamInfo> = resp
                        .workstreams
                        .into_iter()
                        .map(|ws| WorkstreamInfo {
                            id: ws.id,
                            title: ws.title,
                            is_scratch: ws.is_scratch,
                        })
                        .collect();
                    let _ = tx.send(HttpResult::Workstreams(infos));
                }
                Err(e) => {
                    warn!("Failed to fetch workstreams: {}", e);
                }
            }
        });
    }

    /// Spawn a background HTTP task to fetch sessions for a workstream.
    pub fn spawn_fetch_sessions(&self, workstream_id: &str) {
        let tx = self.http_tx.clone();
        let server_url = self.server_url.clone();
        let ws_id = workstream_id.to_string();
        tokio::spawn(async move {
            let client = match arawn_client::ArawnClient::builder()
                .base_url(&server_url)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to create HTTP client for sessions: {}", e);
                    return;
                }
            };
            match client.workstreams().sessions(&ws_id).await {
                Ok(resp) => {
                    let infos: Vec<SessionInfo> = resp
                        .sessions
                        .into_iter()
                        .map(|s| SessionInfo {
                            id: s.id,
                            started_at: s.started_at,
                        })
                        .collect();
                    let _ = tx.send(HttpResult::Sessions(infos));
                }
                Err(e) => {
                    warn!("Failed to fetch sessions for workstream {}: {}", ws_id, e);
                }
            }
        });
    }

    /// Spawn a background HTTP task to fetch messages for a session.
    pub fn spawn_fetch_messages(&self, session_id: &str) {
        let tx = self.http_tx.clone();
        let server_url = self.server_url.clone();
        let sid = session_id.to_string();
        tokio::spawn(async move {
            let client = match arawn_client::ArawnClient::builder()
                .base_url(&server_url)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to create HTTP client for messages: {}", e);
                    return;
                }
            };
            match client.sessions().messages(&sid).await {
                Ok(resp) => {
                    let messages: Vec<MessageInfo> = resp
                        .messages
                        .into_iter()
                        .map(|m| MessageInfo {
                            role: m.role,
                            content: m.content,
                        })
                        .collect();
                    let _ = tx.send(HttpResult::Messages(sid, messages));
                }
                Err(e) => {
                    warn!("Failed to fetch messages for session {}: {}", sid, e);
                }
            }
        });
    }

    /// Spawn a background HTTP task to create a new workstream.
    pub fn spawn_create_workstream(&self, name: &str) {
        let tx = self.http_tx.clone();
        let server_url = self.server_url.clone();
        let ws_name = name.to_string();
        tokio::spawn(async move {
            let client = match arawn_client::ArawnClient::builder()
                .base_url(&server_url)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        "Failed to create HTTP client for workstream creation: {}",
                        e
                    );
                    return;
                }
            };
            let request = arawn_client::types::CreateWorkstreamRequest {
                title: ws_name.clone(),
                default_model: None,
                tags: Vec::new(),
            };
            match client.workstreams().create(request).await {
                Ok(_) => {
                    // Re-fetch workstream list after creation
                    match client.workstreams().list().await {
                        Ok(resp) => {
                            let infos: Vec<WorkstreamInfo> = resp
                                .workstreams
                                .into_iter()
                                .map(|ws| WorkstreamInfo {
                                    id: ws.id,
                                    title: ws.title,
                                    is_scratch: ws.is_scratch,
                                })
                                .collect();
                            let _ = tx.send(HttpResult::Workstreams(infos));
                        }
                        Err(e) => {
                            warn!("Failed to refresh workstreams after creation: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create workstream '{}': {}", ws_name, e);
                }
            }
        });
    }

    /// Clear status message on user action (if it has been displayed long enough).
    fn clear_status_on_action(&mut self) {
        if let Some(set_at) = self.status_set_at {
            // Clear if status has been shown for any amount of time on user action
            let _ = set_at; // We clear on any action regardless of time
            self.status = None;
            self.status_set_at = None;
        }
    }

    /// Set a status message with a timestamp.
    fn set_status(&mut self, msg: String) {
        self.status = Some(msg);
        self.status_set_at = Some(Instant::now());
    }

    /// Check if status should be auto-cleared after 5 seconds.
    pub fn maybe_clear_status(&mut self) {
        if let Some(set_at) = self.status_set_at {
            if set_at.elapsed() >= std::time::Duration::from_secs(5) {
                self.status = None;
                self.status_set_at = None;
            }
        }
    }

    /// Handle a message from the server.
    fn handle_server_message(&mut self, msg: ServerMessage) {
        // On first server message, fetch workstreams (implies WS is connected)
        if !self.has_fetched_workstreams {
            self.has_fetched_workstreams = true;
            self.spawn_fetch_workstreams();
        }

        match msg {
            ServerMessage::SessionCreated { session_id } => {
                self.session_id = Some(session_id);
            }
            ServerMessage::ChatChunk { chunk, done, .. } => {
                // Handle content first (even in done=true chunks that carry final text)
                if !chunk.is_empty() {
                    let needs_new = self
                        .messages
                        .last()
                        .map(|m| !m.streaming || m.is_user)
                        .unwrap_or(true);

                    if needs_new {
                        self.messages.push(ChatMessage {
                            is_user: false,
                            content: chunk,
                            streaming: !done,
                        });
                    } else if let Some(last) = self.messages.last_mut() {
                        last.content.push_str(&chunk);
                        if done {
                            last.streaming = false;
                        }
                    }
                }

                if done {
                    // Mark any trailing streaming message as complete
                    if let Some(last) = self.messages.last_mut() {
                        if !last.is_user {
                            last.streaming = false;
                        }
                    }
                    self.waiting = false;
                }
                self.auto_scroll = true;
            }
            ServerMessage::ToolStart { tool_name, .. } => {
                self.set_status(format!("Running tool: {}", tool_name));
                // Show tool activity in chat so user sees something happening
                self.messages.push(ChatMessage {
                    is_user: false,
                    content: format!("[tool: {}]", tool_name),
                    streaming: false,
                });
                self.auto_scroll = true;
            }
            ServerMessage::ToolEnd { .. } => {
                self.status = None;
                self.status_set_at = None;
            }
            ServerMessage::Error { message, .. } => {
                self.set_status(format!("Error: {}", message));
                self.waiting = false;
            }
            ServerMessage::AuthResult { success, error } => {
                if !success {
                    self.set_status(format!(
                        "Auth failed: {}",
                        error.unwrap_or_else(|| "unknown".into())
                    ));
                }
            }
            ServerMessage::SubscribeAck { .. } | ServerMessage::Pong | ServerMessage::Unknown => {}
        }
    }

    /// Set the input text directly (for testing).
    pub fn set_text(&mut self, text: &str) {
        self.input = text.to_string();
        self.cursor_pos = text.len();
    }

    /// Handle a key event (public for testing).
    pub fn handle_key_public(&mut self, key: crossterm::event::KeyEvent) {
        self.handle_key(key);
    }

    /// Handle a server message (public for testing).
    pub fn handle_server_message_public(&mut self, msg: ServerMessage) {
        self.handle_server_message(msg);
    }

    /// Handle an HTTP result (public for testing).
    pub fn handle_http_result_public(&mut self, result: HttpResult) {
        self.handle_http_result(result);
    }
}

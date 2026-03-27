//! Application state and main event loop.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Terminal, backend::Backend};
use tokio::sync::mpsc;

use crate::config::TuiConfig;
use crate::events::Event;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::render;

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
}

impl App {
    /// Create a new App from config and a sender channel to the WebSocket.
    pub fn new(config: &TuiConfig, ws_tx: mpsc::UnboundedSender<ClientMessage>) -> Self {
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
                        Some(Event::Tick) => {} // Just triggers a redraw
                        None => self.should_quit = true,
                    }
                }
                msg = ws_rx.recv() => {
                    match msg {
                        Some(server_msg) => self.handle_server_message(server_msg),
                        None => {
                            self.status = Some("Disconnected from server".into());
                        }
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
            }
        }
    }

    /// Handle a key event.
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
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
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
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

    /// Handle a message from the server.
    fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::SessionCreated { session_id } => {
                self.session_id = Some(session_id);
            }
            ServerMessage::ChatChunk { chunk, done, .. } => {
                // Find or create the current assistant message
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
                    last.streaming = !done;
                }

                if done {
                    self.waiting = false;
                }
                self.auto_scroll = true;
            }
            ServerMessage::ToolStart { tool_name, .. } => {
                self.status = Some(format!("Running tool: {}", tool_name));
            }
            ServerMessage::ToolEnd { .. } => {
                self.status = None;
            }
            ServerMessage::Error { message, .. } => {
                self.status = Some(format!("Error: {}", message));
                self.waiting = false;
            }
            ServerMessage::AuthResult { success, error } => {
                if !success {
                    self.status = Some(format!(
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
}

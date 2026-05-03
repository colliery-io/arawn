use ratatui::layout::Rect;

use arawn_service::{SessionInfo, WorkstreamInfo};

use crate::action::Action;
use crate::command::{
    AutocompleteState, CommandRegistry, CommandResult, execute_command, parse_command,
};

/// Tracks the screen regions of each panel from the last render.
/// Used for mouse hit-testing.
#[derive(Debug, Clone, Default)]
pub struct LayoutRegions {
    pub sidebar: Option<Rect>,
    pub chat: Rect,
    pub input: Rect,
    /// Sidebar workstreams section (for click-to-select).
    pub sidebar_ws: Option<Rect>,
    /// Sidebar sessions section (for click-to-select).
    pub sidebar_sessions: Option<Rect>,
    /// Thin sidebar tab strip (visible when sidebar is hidden).
    pub sidebar_tab: Option<Rect>,
}

/// Which panel has focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    /// Main view: chat + input unified. Typing goes to input, Up/Down scroll chat.
    Main,
    Sidebar,
}

/// Which sidebar section is active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarSection {
    Workstreams,
    Sessions,
}

/// A message displayed in the chat area.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// When this message was created (for elapsed time display on tool calls).
    pub created_at: std::time::Instant,
    /// Cached rendered lines for assistant markdown. Populated on first render.
    rendered_cache: Option<Vec<ratatui::text::Line<'static>>>,
    /// Width used for the cached render (invalidated on resize).
    cached_width: usize,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            created_at: std::time::Instant::now(),
            rendered_cache: None,
            cached_width: 0,
        }
    }

    /// Get or compute the cached markdown rendering for assistant messages.
    /// Width is used for table column sizing — cache invalidates on width change.
    pub fn rendered_lines(&mut self, width: usize) -> &[ratatui::text::Line<'static>] {
        // Invalidate cache if width changed (terminal resize)
        if self.rendered_cache.is_some() && self.cached_width != width {
            self.rendered_cache = None;
        }
        if self.rendered_cache.is_none() {
            self.rendered_cache = Some(crate::markdown::markdown_to_lines_with_width(&self.content, width));
            self.cached_width = width;
        }
        self.rendered_cache.as_ref().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
    ToolCall { name: String },
    ToolResult { name: String, is_error: bool },
    System,
}

/// All mutable TUI state. Pure state machine — no I/O, no network.
pub struct App {
    pub focus: Focus,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub messages: Vec<ChatMessage>,
    pub workstreams: Vec<WorkstreamInfo>,
    pub sessions: Vec<SessionInfo>,
    pub current_workstream: Option<WorkstreamInfo>,
    pub current_session: Option<SessionInfo>,
    pub is_generating: bool,
    pub streaming_text: String,
    pub scroll_offset: usize,
    pub sidebar_section: SidebarSection,
    pub sidebar_ws_index: usize,
    pub sidebar_session_index: usize,
    pub should_quit: bool,
    pub dirty: bool,
    /// Pending action that requires the event loop to send a WS message.
    pub pending_submit: Option<String>,
    /// Set of message indices where tool results are expanded (show full content).
    pub expanded_tool_results: std::collections::HashSet<usize>,
    /// Current model name (for status bar display).
    pub model_name: String,
    /// Current permission mode label (fetched from server).
    pub permission_mode: String,
    /// Cumulative token usage: (input_tokens, output_tokens).
    pub token_usage: (u64, u64),
    /// When generation started (for elapsed time in status bar).
    pub generation_started: Option<std::time::Instant>,
    /// Active modal overlay (permission prompt, AskUser, etc.)
    pub active_modal: Option<crate::modal::ModalState>,
    /// Pending modal response to send back to server: (request_id, result_rx)
    pub pending_modal_response: Option<(String, tokio::sync::oneshot::Receiver<Option<usize>>)>,
    /// Spinner animation frame (0-9), ticked by the event loop.
    pub spinner_frame: u8,
    /// Name of the currently executing tool (set on ToolCallStart, cleared on ToolCallResult/Complete).
    pub active_tool: Option<String>,
    /// Panel regions from last render, for mouse hit-testing.
    pub layout: LayoutRegions,
    /// Slash command registry (built-in + cached skills).
    pub command_registry: CommandRegistry,
    /// Active autocomplete dropdown state (None = hidden).
    pub autocomplete: Option<AutocompleteState>,
    /// Pending command result that needs WS interaction (inventory query, skill invoke).
    pub pending_command: Option<CommandResult>,
}

impl App {
    pub fn new() -> Self {
        Self {
            focus: Focus::Main,
            input_buffer: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            workstreams: Vec::new(),
            sessions: Vec::new(),
            current_workstream: None,
            current_session: None,
            is_generating: false,
            streaming_text: String::new(),
            scroll_offset: 0,
            sidebar_section: SidebarSection::Workstreams,
            sidebar_ws_index: 0,
            sidebar_session_index: 0,
            should_quit: false,
            dirty: true,
            pending_submit: None,
            expanded_tool_results: std::collections::HashSet::new(),
            model_name: String::new(),
            permission_mode: "default".into(),
            token_usage: (0, 0),
            active_modal: None,
            pending_modal_response: None,
            generation_started: None,
            spinner_frame: 0,
            active_tool: None,
            layout: LayoutRegions::default(),
            command_registry: CommandRegistry::new(),
            autocomplete: None,
            pending_command: None,
        }
    }

    /// Process an action and mutate state. Returns true if state changed.
    pub fn handle_action(&mut self, action: Action) -> bool {
        self.dirty = true;

        match action {
            Action::TypeChar(c) => {
                if self.focus == Focus::Main && !self.is_generating {
                    self.input_buffer.insert(self.cursor_pos, c);
                    self.cursor_pos += c.len_utf8();
                    self.update_autocomplete();
                } else {
                    self.dirty = false;
                }
            }
            Action::Backspace => {
                if self.focus == Focus::Main && self.cursor_pos > 0 && !self.is_generating {
                    let prev = self.prev_char_boundary();
                    self.input_buffer.drain(prev..self.cursor_pos);
                    self.cursor_pos = prev;
                    self.update_autocomplete();
                } else {
                    self.dirty = false;
                }
            }
            Action::Delete => {
                if self.focus == Focus::Main
                    && self.cursor_pos < self.input_buffer.len()
                    && !self.is_generating
                {
                    let next = self.next_char_boundary();
                    self.input_buffer.drain(self.cursor_pos..next);
                    self.update_autocomplete();
                } else {
                    self.dirty = false;
                }
            }
            Action::CursorLeft => {
                if self.focus == Focus::Main && self.cursor_pos > 0 {
                    self.cursor_pos = self.prev_char_boundary();
                } else {
                    self.dirty = false;
                }
            }
            Action::CursorRight => {
                if self.focus == Focus::Main && self.cursor_pos < self.input_buffer.len() {
                    self.cursor_pos = self.next_char_boundary();
                } else {
                    self.dirty = false;
                }
            }
            Action::CursorHome => {
                if self.focus == Focus::Main {
                    self.cursor_pos = 0;
                } else {
                    self.dirty = false;
                }
            }
            Action::CursorEnd => {
                if self.focus == Focus::Main {
                    self.cursor_pos = self.input_buffer.len();
                } else {
                    self.dirty = false;
                }
            }
            Action::Submit => {
                if self.focus == Focus::Main
                    && !self.is_generating
                    && !self.input_buffer.trim().is_empty()
                {
                    // Dismiss autocomplete on submit
                    self.autocomplete = None;

                    // Check for slash command
                    if let Some(cmd) = parse_command(&self.input_buffer) {
                        let result = execute_command(&cmd, &self.command_registry);
                        self.input_buffer.clear();
                        self.cursor_pos = 0;
                        self.scroll_offset = 0;

                        match result {
                            CommandResult::SystemMessage(msg) => {
                                self.messages.push(ChatMessage::new(ChatRole::System, msg));
                            }
                            CommandResult::ClearChat => {
                                self.messages.clear();
                            }
                            CommandResult::EnterPlan => {
                                // Send as a regular message — the LLM will call EnterPlanMode
                                let content = "Enter plan mode. Use EnterPlanMode to begin planning.".to_string();
                                self.messages.push(ChatMessage::new(ChatRole::User, content.clone()));
                                self.is_generating = true;
                                self.generation_started = Some(std::time::Instant::now());
                                self.pending_submit = Some(content);
                            }
                            CommandResult::QueryInventory(_)
                            | CommandResult::InvokeSkill { .. }
                            | CommandResult::RememberFact(_)
                            | CommandResult::MemorySummary
                            | CommandResult::ForgetEntity(_)
                            | CommandResult::WorkstreamCreate(_)
                            | CommandResult::WorkstreamList
                            | CommandResult::WorkstreamSwitch(_)
                            | CommandResult::SessionNew
                            | CommandResult::SessionList
                            | CommandResult::PromoteSession(_)
                            | CommandResult::SetPermissionMode(_)
                            | CommandResult::WorkflowList
                            | CommandResult::WorkflowStatus(_)
                            | CommandResult::PermissionsStatus
                            | CommandResult::IntegrationsList
                            | CommandResult::IntegrationConnect(_)
                            | CommandResult::IntegrationDisconnect(_) => {
                                // These need WS interaction — store for event loop to handle
                                self.pending_command = Some(result);
                            }
                        }
                    } else {
                        // Normal chat message
                        let content = self.input_buffer.clone();
                        self.messages
                            .push(ChatMessage::new(ChatRole::User, content.clone()));
                        self.input_buffer.clear();
                        self.cursor_pos = 0;
                        self.is_generating = true;
                        self.generation_started = Some(std::time::Instant::now());
                        self.scroll_offset = 0;
                        self.pending_submit = Some(content);
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::Tab => {
                // If autocomplete is active, accept the selection
                if self.autocomplete.is_some() {
                    self.accept_autocomplete();
                } else {
                    self.focus = match self.focus {
                        Focus::Main => Focus::Sidebar,
                        Focus::Sidebar => Focus::Main,
                    };
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::ScrollUp => {
                // Scroll chat from any focus
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            Action::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            Action::ScrollPageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
            }
            Action::ScrollPageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            Action::SidebarUp => {
                if self.focus == Focus::Sidebar {
                    match self.sidebar_section {
                        SidebarSection::Workstreams => {
                            self.sidebar_ws_index = self.sidebar_ws_index.saturating_sub(1);
                        }
                        SidebarSection::Sessions => {
                            self.sidebar_session_index =
                                self.sidebar_session_index.saturating_sub(1);
                        }
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::SidebarDown => {
                if self.focus == Focus::Sidebar {
                    match self.sidebar_section {
                        SidebarSection::Workstreams => {
                            let max = self.workstreams.len().saturating_sub(1);
                            self.sidebar_ws_index = (self.sidebar_ws_index + 1).min(max);
                        }
                        SidebarSection::Sessions => {
                            let max = self.sessions.len().saturating_sub(1);
                            self.sidebar_session_index = (self.sidebar_session_index + 1).min(max);
                        }
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::SidebarSelect => {
                // Handled by event loop — it reads the selected index and sends WS request
                if self.focus != Focus::Sidebar {
                    self.dirty = false;
                }
            }
            Action::NewSession => {
                // Handled by event loop
                if self.focus != Focus::Sidebar {
                    self.dirty = false;
                }
            }
            Action::ClickFocus(target) => {
                self.focus = target;
            }
            Action::ClickSidebarItem(index) => {
                if self.focus == Focus::Sidebar {
                    match self.sidebar_section {
                        SidebarSection::Workstreams => {
                            if index < self.workstreams.len() {
                                self.sidebar_ws_index = index;
                            }
                        }
                        SidebarSection::Sessions => {
                            if index < self.sessions.len() {
                                self.sidebar_session_index = index;
                            }
                        }
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::ClickInput(col) => {
                self.focus = Focus::Main;
                // Map click column to cursor position (col 0 is the border)
                let offset = col.saturating_sub(1) as usize;
                self.cursor_pos = offset.min(self.input_buffer.len());
            }
            Action::ToggleToolResult(idx) => {
                if idx < self.messages.len() {
                    if self.expanded_tool_results.contains(&idx) {
                        self.expanded_tool_results.remove(&idx);
                    } else {
                        self.expanded_tool_results.insert(idx);
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::ModalUp => {
                if let Some(ref mut modal) = self.active_modal {
                    modal.focus_prev();
                } else {
                    self.dirty = false;
                }
            }
            Action::ModalDown => {
                if let Some(ref mut modal) = self.active_modal {
                    modal.focus_next();
                } else {
                    self.dirty = false;
                }
            }
            Action::ModalConfirm => {
                if let Some(ref mut modal) = self.active_modal {
                    modal.confirm();
                }
                self.active_modal = None;
            }
            Action::ModalCancel => {
                if let Some(ref mut modal) = self.active_modal {
                    modal.cancel();
                }
                self.active_modal = None;
            }
            Action::ToggleAllToolResults => {
                // If any are expanded, collapse all. Otherwise expand all.
                let any_expanded = !self.expanded_tool_results.is_empty();
                if any_expanded {
                    self.expanded_tool_results.clear();
                } else {
                    for (i, msg) in self.messages.iter().enumerate() {
                        if matches!(msg.role, ChatRole::ToolResult { is_error: false, .. }) {
                            self.expanded_tool_results.insert(i);
                        }
                    }
                }
            }
            Action::AutocompleteNext => {
                if let Some(ref mut ac) = self.autocomplete {
                    ac.next();
                } else {
                    self.dirty = false;
                }
            }
            Action::AutocompletePrev => {
                if let Some(ref mut ac) = self.autocomplete {
                    ac.prev();
                } else {
                    self.dirty = false;
                }
            }
            Action::AutocompleteAccept => {
                self.accept_autocomplete();
            }
            Action::AutocompleteDismiss => {
                self.autocomplete = None;
            }
            Action::Cancel => {
                // Dismiss autocomplete first, then handle cancel
                if self.autocomplete.is_some() {
                    self.autocomplete = None;
                } else if self.is_generating {
                    self.is_generating = false;
                    self.active_tool = None;
                    if !self.streaming_text.is_empty() {
                        self.messages.push(ChatMessage::new(
                            ChatRole::Assistant,
                            self.streaming_text.clone() + " (cancelled)",
                        ));
                        self.streaming_text.clear();
                    }
                } else {
                    self.dirty = false;
                }
            }
        }

        self.dirty
    }

    /// Update autocomplete suggestions based on current input buffer.
    fn update_autocomplete(&mut self) {
        let trimmed = self.input_buffer.trim_start();
        if trimmed.starts_with('/') && trimmed.len() > 1 {
            // Extract the command prefix (after /)
            let after_slash = &trimmed[1..];
            let prefix = after_slash
                .split_whitespace()
                .next()
                .unwrap_or(after_slash);

            let matches: Vec<_> = self
                .command_registry
                .matching(prefix)
                .into_iter()
                .cloned()
                .collect();

            if matches.is_empty() {
                self.autocomplete = None;
            } else {
                self.autocomplete = Some(AutocompleteState::new(matches));
            }
        } else if trimmed == "/" {
            // Show all commands when just "/" is typed
            let all: Vec<_> = self.command_registry.all().to_vec();
            self.autocomplete = Some(AutocompleteState::new(all));
        } else {
            self.autocomplete = None;
        }
    }

    /// Accept the currently selected autocomplete suggestion.
    fn accept_autocomplete(&mut self) {
        if let Some(ref ac) = self.autocomplete
            && let Some(cmd) = ac.selected_command() {
                self.input_buffer = format!("/{}", cmd.name);
                self.cursor_pos = self.input_buffer.len();
            }
        self.autocomplete = None;
    }

    /// Apply a streaming engine event to the app state (testable without network).
    pub fn apply_engine_event(&mut self, event: crate::ws_client::EventUpdate) {
        self.dirty = true;
        match event {
            crate::ws_client::EventUpdate::AppendStreamingText(text) => {
                self.streaming_text.push_str(&text);
            }
            crate::ws_client::EventUpdate::AddToolCall { name, input, .. } => {
                let summary = format_tool_input(&name, &input);
                self.active_tool = Some(name.clone());
                self.messages.push(ChatMessage::new(
                    ChatRole::ToolCall { name: name.clone() },
                    summary,
                ));
            }
            crate::ws_client::EventUpdate::AddToolResult {
                content, is_error, ..
            } => {
                let name = self
                    .messages
                    .iter()
                    .rev()
                    .find_map(|m| match &m.role {
                        ChatRole::ToolCall { name } => Some(name.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "tool".to_string());
                self.active_tool = None;
                self.messages.push(ChatMessage::new(
                    ChatRole::ToolResult { name, is_error },
                    content,
                ));
            }
            crate::ws_client::EventUpdate::Complete(final_text) => {
                let content = if !self.streaming_text.is_empty() {
                    std::mem::take(&mut self.streaming_text)
                } else {
                    final_text
                };
                self.messages
                    .push(ChatMessage::new(ChatRole::Assistant, content));
                self.is_generating = false;
                self.active_tool = None;
                self.generation_started = None;
                self.scroll_offset = 0;
            }
            crate::ws_client::EventUpdate::Error(message) => {
                self.messages.push(ChatMessage::new(
                    ChatRole::System,
                    format!("Error: {message}"),
                ));
                self.is_generating = false;
                self.active_tool = None;
                self.generation_started = None;
                self.streaming_text.clear();
            }
            crate::ws_client::EventUpdate::Warning(message) => {
                self.messages.push(ChatMessage::new(
                    ChatRole::System,
                    format!("Warning: {message}"),
                ));
            }
            crate::ws_client::EventUpdate::Compaction(count) => {
                self.messages.push(ChatMessage::new(
                    ChatRole::System,
                    format!("Context compacted ({count} messages summarized)"),
                ));
            }
            crate::ws_client::EventUpdate::Usage { input_tokens, output_tokens } => {
                self.token_usage = (input_tokens, output_tokens);
            }
            crate::ws_client::EventUpdate::UserInputRequest { .. } => {
                // Handled by the event loop directly (sets active_modal)
            }
            crate::ws_client::EventUpdate::Flush => {
                // Flush is handled by the event loop for rendering — no state change needed
            }
        }
    }

    /// Load messages from a session detail JSON response into the chat.
    /// Clears existing messages and streaming text first.
    pub fn load_session_messages(&mut self, detail: &serde_json::Value) {
        self.messages.clear();
        self.streaming_text.clear();
        if let Some(msgs) = detail.get("messages").and_then(|m| m.as_array()) {
            for msg in msgs {
                if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    let chat_msg = match role {
                        "user" => ChatMessage::new(ChatRole::User, content),
                        "assistant" => {
                            if let Some(tool_uses) = msg.get("tool_uses").and_then(|t| t.as_array()) {
                                for tu in tool_uses {
                                    let name = tu.get("name").and_then(|n| n.as_str()).unwrap_or("tool").to_string();
                                    let input = tu.get("input").cloned().unwrap_or(serde_json::Value::Null);
                                    let summary = format_tool_input(&name, &input);
                                    self.messages.push(ChatMessage::new(ChatRole::ToolCall { name }, summary));
                                }
                            }
                            if content.is_empty() { continue; }
                            ChatMessage::new(ChatRole::Assistant, content)
                        }
                        "tool_result" => {
                            let is_error = msg.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                            let name = self.messages.iter().rev()
                                .find_map(|m| match &m.role {
                                    ChatRole::ToolCall { name } => Some(name.clone()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| "tool".to_string());
                            ChatMessage::new(ChatRole::ToolResult { name, is_error }, content)
                        }
                        "summary" => ChatMessage::new(ChatRole::System, format!("[Summary] {content}")),
                        _ => continue,
                    };
                    self.messages.push(chat_msg);
                }
            }
        }
        self.scroll_offset = 0;
        self.dirty = true;
    }

    fn prev_char_boundary(&self) -> usize {
        let mut pos = self.cursor_pos.saturating_sub(1);
        while pos > 0 && !self.input_buffer.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn next_char_boundary(&self) -> usize {
        let mut pos = self.cursor_pos + 1;
        while pos < self.input_buffer.len() && !self.input_buffer.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }
}

/// Format tool input args into a compact display string.
pub fn format_tool_input(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "shell" | "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_read" | "Read" | "FileRead" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_write" | "Write" | "FileWrite" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_edit" | "Edit" | "FileEdit" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "grep" | "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "glob" | "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => {
            // Generic: show first string field value, truncated
            if let Some(obj) = input.as_object() {
                for (_k, v) in obj {
                    if let Some(s) = v.as_str() {
                        let truncated = if s.len() > 60 { &s[..60] } else { s };
                        return truncated.to_string();
                    }
                }
            }
            String::new()
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_chars_updates_buffer() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('i'));
        assert_eq!(app.input_buffer, "hi");
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn backspace_removes_char() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('a'));
        app.handle_action(Action::TypeChar('b'));
        app.handle_action(Action::Backspace);
        assert_eq!(app.input_buffer, "a");
        assert_eq!(app.cursor_pos, 1);
    }

    #[test]
    fn submit_moves_to_messages() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('i'));
        app.handle_action(Action::Submit);

        assert_eq!(app.input_buffer, "");
        assert_eq!(app.cursor_pos, 0);
        assert!(app.is_generating);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "hi");
        assert_eq!(app.pending_submit, Some("hi".into()));
    }

    #[test]
    fn submit_blocked_when_empty() {
        let mut app = App::new();
        let changed = app.handle_action(Action::Submit);
        assert!(!changed);
        assert!(app.messages.is_empty());
        assert!(!app.is_generating);
    }

    #[test]
    fn submit_blocked_while_generating() {
        let mut app = App::new();
        app.is_generating = true;
        app.handle_action(Action::TypeChar('x'));
        // TypeChar blocked during generation
        assert_eq!(app.input_buffer, "");
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Main);
        app.handle_action(Action::Tab);
        assert_eq!(app.focus, Focus::Sidebar);
        app.handle_action(Action::Tab);
        assert_eq!(app.focus, Focus::Main);
    }

    #[test]
    fn scroll_updates_offset() {
        let mut app = App::new();
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.scroll_offset, 1);
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.scroll_offset, 2);
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn cancel_stops_generation() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial response".into();
        app.handle_action(Action::Cancel);
        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].content.contains("cancelled"));
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::new();
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn cursor_movement() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('a'));
        app.handle_action(Action::TypeChar('b'));
        app.handle_action(Action::TypeChar('c'));
        assert_eq!(app.cursor_pos, 3);

        app.handle_action(Action::CursorLeft);
        assert_eq!(app.cursor_pos, 2);

        app.handle_action(Action::CursorHome);
        assert_eq!(app.cursor_pos, 0);

        app.handle_action(Action::CursorEnd);
        assert_eq!(app.cursor_pos, 3);

        // Insert at middle
        app.handle_action(Action::CursorHome);
        app.handle_action(Action::CursorRight);
        app.handle_action(Action::TypeChar('X'));
        assert_eq!(app.input_buffer, "aXbc");
    }

    // --- Integration tests: full conversation flow via EventUpdate ---

    #[test]
    fn full_conversation_flow() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();

        // User types and submits
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('e'));
        app.handle_action(Action::TypeChar('l'));
        app.handle_action(Action::TypeChar('l'));
        app.handle_action(Action::TypeChar('o'));
        app.handle_action(Action::Submit);

        assert!(app.is_generating);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, ChatRole::User);

        // Streaming text arrives
        app.apply_engine_event(EventUpdate::AppendStreamingText("Hi ".into()));
        app.apply_engine_event(EventUpdate::AppendStreamingText("there!".into()));
        assert_eq!(app.streaming_text, "Hi there!");
        assert!(app.is_generating);

        // Complete
        app.apply_engine_event(EventUpdate::Complete("Hi there!".into()));
        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[1].role, ChatRole::Assistant);
        assert_eq!(app.messages[1].content, "Hi there!");
    }

    #[test]
    fn tool_call_flow() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();
        app.is_generating = true;

        // Tool call start
        app.apply_engine_event(EventUpdate::AddToolCall {
            id: "c1".into(),
            name: "shell".into(),
            input: serde_json::json!({"command": "ls -la"}),
        });
        assert_eq!(app.messages.len(), 1);
        assert!(matches!(&app.messages[0].role, ChatRole::ToolCall { name } if name == "shell"));

        // Tool call result
        app.apply_engine_event(EventUpdate::AddToolResult {
            id: "c1".into(),
            content: "file1.rs\nfile2.rs".into(),
            is_error: false,
        });
        assert_eq!(app.messages.len(), 2);
        assert!(
            matches!(&app.messages[1].role, ChatRole::ToolResult { name, is_error } if name == "shell" && !is_error)
        );

        // Complete
        app.apply_engine_event(EventUpdate::Complete("Here are the files.".into()));
        assert!(!app.is_generating);
        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.messages[2].content, "Here are the files.");
    }

    #[test]
    fn error_event_clears_generating() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial".into();

        app.apply_engine_event(EventUpdate::Error("API error".into()));

        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].content.contains("API error"));
        assert_eq!(app.messages[0].role, ChatRole::System);
    }

    #[test]
    fn sidebar_navigation() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.sidebar_section = SidebarSection::Workstreams;
        app.workstreams = vec![
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "scratch".into(),
                root_dir: PathBuf::from("/tmp/a"),
                created_at: Utc::now(),
            },
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "project".into(),
                root_dir: PathBuf::from("/tmp/b"),
                created_at: Utc::now(),
            },
        ];

        assert_eq!(app.sidebar_ws_index, 0);
        app.handle_action(Action::SidebarDown);
        assert_eq!(app.sidebar_ws_index, 1);
        app.handle_action(Action::SidebarDown);
        assert_eq!(app.sidebar_ws_index, 1); // clamped
        app.handle_action(Action::SidebarUp);
        assert_eq!(app.sidebar_ws_index, 0);
    }
}

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
    /// Wall-clock instant of the last `terminal.draw()` call. Used to cap
    /// render rate (see `MIN_FRAME_INTERVAL` in event_loop) so a flood of
    /// engine events doesn't melt the render path. Initialized to "long
    /// ago" so the first draw is never throttled.
    pub last_draw: std::time::Instant,
    /// Pending action that requires the event loop to send a WS message.
    pub pending_submit: Option<String>,
    /// True when the user pressed Esc during generation. The event loop
    /// reads this to send a `cancel` RPC to the server, then clears it.
    /// Without this, "cancel" would only flip `is_generating` locally
    /// while the model kept running and produced a duplicate Complete.
    pub pending_cancel: bool,
    /// Session-id-of-cancellation, set alongside `pending_cancel` to a
    /// turn marker. While Some, the WS event handler ignores incoming
    /// stream events (StreamingText / ToolCall / Complete) for that
    /// session — they're stale output from the cancelled turn. Cleared
    /// on next user submit.
    pub cancelled_session: Option<uuid::Uuid>,
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
    /// In-flight `/connect <svc>` OAuth flow: service name + start time.
    /// Renders a heartbeat line above the status bar so the user knows
    /// the app isn't frozen while the browser dance is in progress.
    pub oauth_in_flight: Option<(String, std::time::Instant)>,
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
    /// Submitted user prompts in chronological order. Drives Up/Down recall
    /// and the double-Esc history modal. Per-session, in-memory only.
    /// Each entry is `(text, is_chat)` — slash commands have `is_chat = false`
    /// so the branch modal can filter them out (only chat prompts correspond
    /// to session turns and are branchable).
    pub history: Vec<HistoryEntry>,
    /// Index into `history` while the user is browsing prior prompts via
    /// Up/Down. `None` = not browsing (Up/Down scrolls chat as before).
    /// `Some(i)` = currently showing `history[i]` in the input.
    pub history_cursor: Option<usize>,
    /// In-progress draft saved when the user enters history-browsing mode,
    /// restored when they exit (Down past the most recent entry).
    pub history_draft: String,
    /// Wall-clock instant of the most recent Esc press. Used to detect a
    /// double-Esc (within `DOUBLE_ESC_WINDOW`) → opens the history modal.
    pub last_esc_at: Option<std::time::Instant>,
}

/// Window for double-Esc detection. Two Esc presses inside this opens
/// the history modal; longer than this is treated as two independent
/// Esc presses (the second one cancels whatever's transient).
pub const DOUBLE_ESC_WINDOW: std::time::Duration = std::time::Duration::from_millis(500);

/// One entry in the per-session input history.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub text: String,
    /// True if this submission was a plain chat prompt that produced a
    /// session turn. False for slash commands (`/integrations`, `/clear`,
    /// etc.) — those are recallable via Up arrow but aren't branchable
    /// because they don't correspond to messages on the server.
    pub is_chat: bool,
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
            last_draw: std::time::Instant::now() - std::time::Duration::from_secs(60),
            pending_submit: None,
            pending_cancel: false,
            cancelled_session: None,
            expanded_tool_results: std::collections::HashSet::new(),
            model_name: String::new(),
            permission_mode: "default".into(),
            token_usage: (0, 0),
            active_modal: None,
            pending_modal_response: None,
            generation_started: None,
            oauth_in_flight: None,
            spinner_frame: 0,
            active_tool: None,
            layout: LayoutRegions::default(),
            command_registry: CommandRegistry::new(),
            autocomplete: None,
            pending_command: None,
            history: Vec::new(),
            history_cursor: None,
            history_draft: String::new(),
            last_esc_at: None,
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

                    // Record in input history before any branch clears the
                    // buffer. Slash commands (/integrations, /connect, ...)
                    // get the same treatment as plain prompts so Up arrow
                    // recalls them too — but they're tagged is_chat=false
                    // so the branch modal can skip them.
                    let raw_for_history = self.input_buffer.clone();
                    let is_chat = parse_command(&raw_for_history).is_none();
                    self.record_input_history(&raw_for_history, is_chat);

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
                            | CommandResult::IntegrationDisconnect(_)
                            | CommandResult::FeedRegister(_)
                            | CommandResult::FeedList
                            | CommandResult::FeedPause(_)
                            | CommandResult::FeedResume(_)
                            | CommandResult::FeedRemove { .. } => {
                                // These need WS interaction — store for event loop to handle
                                self.pending_command = Some(result);
                            }
                        }
                    } else {
                        // Normal chat message — history was already recorded above.
                        let content = self.input_buffer.clone();
                        self.messages
                            .push(ChatMessage::new(ChatRole::User, content.clone()));
                        self.input_buffer.clear();
                        self.cursor_pos = 0;
                        self.is_generating = true;
                        self.generation_started = Some(std::time::Instant::now());
                        self.scroll_offset = 0;
                        self.pending_submit = Some(content);
                        // New turn — clear the cancelled marker so stream
                        // events for this turn render normally.
                        self.cancelled_session = None;
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
                // Up arrow: prefer history recall when input is empty or
                // we're already in history mode. Falls through to chat
                // scroll otherwise so muscle memory for scrolling a long
                // chat is preserved when you've started typing.
                if self.focus == Focus::Main
                    && !self.history.is_empty()
                    && self.active_modal.is_none()
                    && (self.input_buffer.is_empty() || self.history_cursor.is_some())
                {
                    self.history_recall_prev();
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                }
            }
            Action::ScrollDown => {
                if self.focus == Focus::Main
                    && self.history_cursor.is_some()
                    && self.active_modal.is_none()
                {
                    self.history_recall_next();
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
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
            Action::ToggleToolEntry(idx) => {
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
            Action::ModalSelectIndex(idx) => {
                if let Some(ref mut modal) = self.active_modal {
                    if idx < modal.options.len() {
                        modal.focused_index = idx;
                        modal.confirm();
                        self.active_modal = None;
                    } else {
                        // Number out of range — silent no-op rather than
                        // confirming the wrong thing.
                        self.dirty = false;
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::ToggleAllToolResults => {
                // If any are expanded, collapse all. Otherwise expand all.
                let any_expanded = !self.expanded_tool_results.is_empty();
                if any_expanded {
                    self.expanded_tool_results.clear();
                } else {
                    for (i, msg) in self.messages.iter().enumerate() {
                        match &msg.role {
                            ChatRole::ToolResult { is_error: false, .. }
                            | ChatRole::ToolCall { .. } => {
                                self.expanded_tool_results.insert(i);
                            }
                            _ => {}
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
                } else if self.oauth_in_flight.is_some() {
                    // Esc during an OAuth dance: drop the heartbeat. The
                    // server's callback listener still times out on its
                    // own (5 min) — see I-0033 followup for server-side
                    // cancellation.
                    self.oauth_in_flight = None;
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
                    // Tell the event loop to send a `cancel` RPC and to
                    // start ignoring stream events for this session
                    // (stale output from the now-aborted turn).
                    if let Some(ref session) = self.current_session {
                        self.pending_cancel = true;
                        self.cancelled_session = Some(session.id);
                    }
                } else {
                    self.dirty = false;
                }
            }
            Action::EscapeIdle => {
                let now = std::time::Instant::now();
                let double = matches!(
                    self.last_esc_at,
                    Some(prev) if now.duration_since(prev) <= DOUBLE_ESC_WINDOW
                );
                if double {
                    self.open_history_modal();
                    self.last_esc_at = None;
                } else {
                    self.last_esc_at = Some(now);
                    // Single Esc on idle is currently a no-op visually —
                    // mark not-dirty so we don't repaint for nothing.
                    self.dirty = false;
                }
            }
            Action::HistoryRecallPrev => {
                self.history_recall_prev();
            }
            Action::HistoryRecallNext => {
                self.history_recall_next();
            }
            Action::HistoryRecallAt(idx) => {
                if let Some(entry) = self.history.get(idx).cloned() {
                    self.input_buffer = entry.text;
                    self.cursor_pos = self.input_buffer.chars().count();
                    self.history_cursor = Some(idx);
                }
                // Closing the modal is the caller's responsibility (event loop).
            }
        }

        self.dirty
    }

    /// Append `text` to input history, skipping empty input and deduping
    /// consecutive duplicates. Resets browse state — the next Up/Down
    /// starts fresh from the newest entry. `is_chat = false` flags
    /// slash-command entries so the branch modal can skip them.
    fn record_input_history(&mut self, text: &str, is_chat: bool) {
        if !text.is_empty()
            && self.history.last().map(|e| e.text.as_str()) != Some(text)
        {
            self.history.push(HistoryEntry {
                text: text.to_string(),
                is_chat,
            });
        }
        self.history_cursor = None;
        self.history_draft.clear();
    }

    /// Move backward in input history. Saves the current draft on first
    /// entry into history mode so Down can restore it past the newest entry.
    fn history_recall_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_idx = match self.history_cursor {
            None => {
                self.history_draft = self.input_buffer.clone();
                self.history.len() - 1
            }
            Some(0) => return, // already at oldest
            Some(i) => i - 1,
        };
        self.history_cursor = Some(next_idx);
        self.input_buffer = self.history[next_idx].text.clone();
        self.cursor_pos = self.input_buffer.chars().count();
    }

    /// Move forward in input history. Past the newest entry, restores
    /// the saved draft and exits history mode.
    fn history_recall_next(&mut self) {
        let Some(idx) = self.history_cursor else { return };
        if idx + 1 < self.history.len() {
            let next = idx + 1;
            self.history_cursor = Some(next);
            self.input_buffer = self.history[next].text.clone();
            self.cursor_pos = self.input_buffer.chars().count();
        } else {
            // Past newest — restore draft and leave history mode.
            self.history_cursor = None;
            self.input_buffer = std::mem::take(&mut self.history_draft);
            self.cursor_pos = self.input_buffer.chars().count();
        }
    }

    /// Open a modal listing branchable history entries (chat prompts only,
    /// newest first). Selecting an entry triggers a session truncate to
    /// that point and loads the entry into the input buffer for editing.
    /// Slash commands are excluded — they don't correspond to a server-
    /// side message turn and aren't branchable.
    fn open_history_modal(&mut self) {
        // Collect (history_index, chat_index, text) triples for chat
        // entries only. chat_index is the position in the chat-only
        // chronological list — what the truncate RPC takes as its
        // user_message_index.
        let chat_entries: Vec<(usize, usize, String)> = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_chat)
            .scan(0usize, |chat_idx, (history_idx, e)| {
                let triple = (history_idx, *chat_idx, e.text.clone());
                *chat_idx += 1;
                Some(triple)
            })
            .collect();

        if chat_entries.is_empty() {
            return;
        }

        // Newest first; truncate over-long entries for the modal label.
        // The description carries the history index AND chat index so the
        // event loop can read them back without recomputing.
        let options: Vec<crate::modal::ModalOption> = chat_entries
            .iter()
            .rev()
            .map(|(history_idx, chat_idx, text)| {
                let label = if text.chars().count() > 80 {
                    let head: String = text.chars().take(79).collect();
                    format!("{head}…")
                } else {
                    text.clone()
                };
                crate::modal::ModalOption::new(label)
                    .with_description(format!("h={history_idx} c={chat_idx}"))
            })
            .collect();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut modal = crate::modal::ModalState::new(
            "Branch from a prior prompt — pick one to rewind to and edit",
            options,
            ratatui::style::Color::Cyan,
            tx,
        );
        modal = modal.with_subtitle(
            "The session will rewind to before this prompt; the prompt loads into your input for editing.",
        );
        self.active_modal = Some(modal);
        // The event loop handles modal close — recognizes this special
        // request_id, parses h=/c= from the selected option's description,
        // calls truncate RPC + loads the text.
        self.pending_modal_response = Some(("__history_branch__".into(), rx));
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

    fn submit_via_input(app: &mut App, text: &str) {
        app.input_buffer = text.into();
        app.cursor_pos = text.chars().count();
        app.handle_action(Action::Submit);
        // Reset transient state the event loop would otherwise drive.
        app.pending_submit = None;
        app.is_generating = false;
    }

    fn history_text(app: &App) -> Vec<&str> {
        app.history.iter().map(|e| e.text.as_str()).collect()
    }

    #[test]
    fn history_records_submitted_prompts() {
        let mut app = App::new();
        submit_via_input(&mut app, "hello");
        submit_via_input(&mut app, "world");
        assert_eq!(history_text(&app), vec!["hello", "world"]);
        assert!(app.history.iter().all(|e| e.is_chat));
    }

    #[test]
    fn history_records_slash_commands_with_is_chat_false() {
        let mut app = App::new();
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "hello");
        submit_via_input(&mut app, "/clear");
        assert_eq!(history_text(&app), vec!["/integrations", "hello", "/clear"]);
        assert_eq!(
            app.history.iter().map(|e| e.is_chat).collect::<Vec<_>>(),
            vec![false, true, false]
        );
    }

    #[test]
    fn history_dedupes_consecutive_duplicates() {
        let mut app = App::new();
        submit_via_input(&mut app, "same");
        submit_via_input(&mut app, "same");
        submit_via_input(&mut app, "different");
        submit_via_input(&mut app, "same");
        assert_eq!(history_text(&app), vec!["same", "different", "same"]);
    }

    #[test]
    fn branch_modal_filters_out_slash_commands() {
        let mut app = App::new();
        submit_via_input(&mut app, "first chat");
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "second chat");
        submit_via_input(&mut app, "/clear");
        // Trigger double-Esc to open the branch modal.
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        let modal = app.active_modal.as_ref().expect("modal should be open");
        // Only the two chat entries should appear, newest first.
        assert_eq!(modal.options.len(), 2);
        assert!(modal.options[0].label.contains("second chat"));
        assert!(modal.options[1].label.contains("first chat"));
    }

    #[test]
    fn branch_modal_skipped_when_no_chat_history() {
        let mut app = App::new();
        // Only slash commands — no chat prompts to branch from.
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "/clear");
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn up_arrow_recalls_most_recent_when_input_empty() {
        let mut app = App::new();
        submit_via_input(&mut app, "first");
        submit_via_input(&mut app, "second");
        // Up should load "second"
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "second");
        assert_eq!(app.history_cursor, Some(1));
        // Up again loads "first"
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "first");
        // Up at oldest stays put
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "first");
        assert_eq!(app.history_cursor, Some(0));
    }

    #[test]
    fn down_arrow_restores_draft_past_newest() {
        let mut app = App::new();
        submit_via_input(&mut app, "old");
        // User starts typing then hits Up — draft saved
        app.input_buffer = "draft in progress".into();
        app.cursor_pos = app.input_buffer.chars().count();
        // Up arrow with non-empty input should NOT recall (falls through to scroll)
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "draft in progress");
        // Clear input so Up starts history mode
        app.input_buffer.clear();
        app.cursor_pos = 0;
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "old");
        // Down past newest restores empty draft (we cleared it)
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.input_buffer, "");
        assert_eq!(app.history_cursor, None);
    }

    #[test]
    fn double_esc_within_window_opens_history_modal() {
        let mut app = App::new();
        submit_via_input(&mut app, "a");
        submit_via_input(&mut app, "b");
        // First Esc — no modal yet
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
        assert!(app.last_esc_at.is_some());
        // Second Esc immediately — opens modal
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_some());
        assert!(app.last_esc_at.is_none());
    }

    #[test]
    fn double_esc_outside_window_does_not_open_modal() {
        let mut app = App::new();
        submit_via_input(&mut app, "a");
        app.handle_action(Action::EscapeIdle);
        // Pretend a long time passed
        app.last_esc_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn history_recall_at_loads_entry_into_input() {
        let mut app = App::new();
        submit_via_input(&mut app, "alpha");
        submit_via_input(&mut app, "bravo");
        submit_via_input(&mut app, "charlie");
        app.handle_action(Action::HistoryRecallAt(1));
        assert_eq!(app.input_buffer, "bravo");
        assert_eq!(app.history_cursor, Some(1));
    }

    #[test]
    fn empty_history_modal_is_a_no_op() {
        let mut app = App::new();
        // Trigger double-Esc with no history — should not open a modal
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn modal_select_index_picks_option_directly() {
        let mut app = App::new();
        // Open a modal with three options (we use the history modal path
        // since it's local-only and doesn't need a ws RPC).
        for label in &["alpha", "bravo", "charlie"] {
            submit_via_input(&mut app, label);
        }
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        let modal = app.active_modal.as_ref().expect("modal should be open");
        assert_eq!(modal.options.len(), 3);
        assert_eq!(modal.focused_index, 0); // before the action

        // Direct-select option index 2 (third in the list — "alpha", since
        // the modal renders newest-first: bravo bravo charlie order is
        // reversed to charlie/bravo/alpha — so index 2 = "alpha").
        app.handle_action(Action::ModalSelectIndex(2));
        // Modal closes after selection; the prompt at history index 0
        // ("alpha") should now be loaded into the input buffer via the
        // event-loop side of the branch flow, but at the App-only level
        // we just assert the modal closed.
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn cancel_marks_session_for_stale_event_drop() {
        let mut app = App::new();
        // Set up an in-progress generation on a session.
        let session = SessionInfo {
            id: uuid::Uuid::new_v4(),
            workstream_id: None,
            created_at: chrono::Utc::now(),
        };
        app.current_session = Some(session.clone());
        app.is_generating = true;
        app.streaming_text = "partial...".into();

        app.handle_action(Action::Cancel);

        assert!(!app.is_generating, "cancel must clear is_generating");
        assert!(app.pending_cancel, "cancel must request RPC dispatch");
        assert_eq!(
            app.cancelled_session,
            Some(session.id),
            "cancel must mark the session so the event loop drops stale stream events"
        );
        // The streaming buffer was flushed into a "(cancelled)" message.
        assert!(app.streaming_text.is_empty());
        assert!(matches!(
            app.messages.last().map(|m| &m.role),
            Some(ChatRole::Assistant)
        ));
    }

    #[test]
    fn next_submit_clears_cancelled_session_marker() {
        let mut app = App::new();
        let session = SessionInfo {
            id: uuid::Uuid::new_v4(),
            workstream_id: None,
            created_at: chrono::Utc::now(),
        };
        app.current_session = Some(session.clone());
        app.cancelled_session = Some(session.id);

        // Submit a fresh message — the cancelled marker should clear so
        // stream events for this NEW turn render normally.
        submit_via_input(&mut app, "fresh prompt");
        assert!(app.cancelled_session.is_none());
    }

    #[test]
    fn modal_select_out_of_range_is_no_op() {
        let mut app = App::new();
        submit_via_input(&mut app, "only one");
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_some());

        // Modal has 1 option; pressing `5` should not confirm anything.
        app.handle_action(Action::ModalSelectIndex(4));
        assert!(
            app.active_modal.is_some(),
            "out-of-range index must not close the modal"
        );
    }
}



use crate::action::Action;
use crate::command::{CommandResult, execute_command, parse_command};

use super::{App, ChatMessage, ChatRole, Focus};
use super::{default_export_path, render_conversation_markdown, shellexpand_tilde};
use super::DOUBLE_ESC_WINDOW;


impl App {
    /// T-0363: handle `/export [path]` — write the current
    /// conversation to a markdown file. With no path arg, picks a
    /// default under `$HOME/.arawn/exports/`. Posts a toast with
    /// the absolute path on success, an error-level toast on
    /// failure.
    pub(super) fn handle_export_conversation(&mut self, path: Option<String>) {
        if self.messages.is_empty() {
            self.post_toast(
                "Nothing to export — the conversation is empty.",
                crate::toast::ToastLevel::Warn,
            );
            return;
        }
        let target = match path {
            Some(p) => std::path::PathBuf::from(shellexpand_tilde(&p)),
            None => default_export_path(self),
        };
        if let Some(parent) = target.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.post_toast(
                format!("Export failed: create dir {}: {e}", parent.display()),
                crate::toast::ToastLevel::Error,
            );
            return;
        }
        let body = render_conversation_markdown(self);
        match std::fs::write(&target, body) {
            Ok(()) => {
                self.post_toast(
                    format!("Exported to {}", target.display()),
                    crate::toast::ToastLevel::Info,
                );
            }
            Err(e) => {
                self.post_toast(
                    format!("Export failed: {e}"),
                    crate::toast::ToastLevel::Error,
                );
            }
        }
    }

    /// T-0361: handle `/copy` — walk `messages` backwards for the
    /// most recent assistant turn, copy its content to the system
    /// clipboard via OSC 52, post a confirmation toast.
    ///
    /// OSC 52 is the cross-platform, zero-dep clipboard path. Modern
    /// terminals (iTerm2, kitty, Alacritty, wezterm, tmux ≥ 3.3,
    /// recent xterm) honor it; older terminals will silently no-op
    /// — that's acceptable because the toast still tells the user
    /// what we attempted.
    pub(super) fn handle_copy_last_response(&mut self) {
        let body = self
            .messages
            .iter()
            .rev()
            .find_map(|m| match &m.role {
                ChatRole::Assistant => Some(m.content.clone()),
                _ => None,
            });
        match body {
            Some(text) if !text.trim().is_empty() => {
                crate::toast::write_osc52_clipboard(&text);
                let n = text.chars().count();
                self.post_toast(
                    format!("Copied last response ({n} chars) to clipboard."),
                    crate::toast::ToastLevel::Info,
                );
            }
            Some(_) => {
                self.post_toast(
                    "No assistant response yet to copy.",
                    crate::toast::ToastLevel::Warn,
                );
            }
            None => {
                self.post_toast(
                    "No assistant response yet to copy.",
                    crate::toast::ToastLevel::Warn,
                );
            }
        }
    }

    /// I-0035 Phase 4 (T-0359): enqueue a 1-line toast to surface
    /// above the status bar. Renderer drops expired toasts lazily;
    /// callers don't need to think about TTLs.
    pub fn post_toast(
        &mut self,
        message: impl Into<String>,
        level: crate::toast::ToastLevel,
    ) {
        crate::toast::enqueue(
            &mut self.toast_queue,
            crate::toast::Toast::new(message, level),
        );
        self.dirty = true;
    }

    /// True iff the empty-chat surface should render the cached brief
    /// markdown instead of the T-0331 welcome hero. We only swap when
    /// the agent has at least one tablet to surface — pre-onboarding
    /// (`None` or empty markdown) falls through to the welcome.
    pub fn should_show_brief_in_empty_chat(&self) -> bool {
        self.brief_markdown
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
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
                            CommandResult::CopyLastResponse => {
                                self.handle_copy_last_response();
                            }
                            CommandResult::ExportConversation { path } => {
                                self.handle_export_conversation(path);
                            }
                            CommandResult::EnterPlan => {
                                // Send as a regular message — the LLM will call EnterPlanMode
                                let content =
                                    "Enter plan mode. Use EnterPlanMode to begin planning."
                                        .to_string();
                                self.messages
                                    .push(ChatMessage::new(ChatRole::User, content.clone()));
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
                            | CommandResult::FeedRemove { .. }
                            | CommandResult::FeedDiscover(_)
                            | CommandResult::FeedRun(_)
                            | CommandResult::CeremonyShowToday
                            | CommandResult::CeremonyShowWeek
                            | CommandResult::CeremonyShowRetro
                            | CommandResult::BriefShow
                            | CommandResult::UsageShow { .. }
                            | CommandResult::TodoShow => {
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
                    self.sidebar_ws_index = self.sidebar_ws_index.saturating_sub(1);
                } else {
                    self.dirty = false;
                }
            }
            Action::SidebarDown => {
                if self.focus == Focus::Sidebar {
                    let max = self.workstreams.len().saturating_sub(1);
                    self.sidebar_ws_index = (self.sidebar_ws_index + 1).min(max);
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
                    if index < self.workstreams.len() {
                        self.sidebar_ws_index = index;
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
                            ChatRole::ToolResult {
                                is_error: false, ..
                            }
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
}

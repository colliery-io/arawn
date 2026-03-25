//! Input key handling, command popup, slash commands, mouse.

use crossterm::event::{KeyCode, KeyModifiers};

use super::App;
use crate::app_types::{ChatMessage, InputMode, PendingAction};
use crate::focus::FocusTarget;
use crate::palette::ActionId;
use crate::sidebar::SidebarSection;

impl App {
    pub(crate) fn handle_input_key(&mut self, key: crossterm::event::KeyEvent) {
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
    pub(crate) fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
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
    pub(crate) fn update_command_popup(&mut self) {
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
    pub(crate) fn send_command(&mut self) {
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

}

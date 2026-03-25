//! Sidebar key handling, overlay navigation, and workstream switching.

use crossterm::event::{KeyCode, KeyModifiers};

use super::App;
use crate::app_types::{InputMode, PendingAction};
use crate::sidebar::SidebarSection;

impl App {
    pub(crate) fn handle_overlay_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.sidebar.filter_clear();
                self.focus.pop_overlay();
            }
            KeyCode::Enter => {
                // Switch to selected workstream
                if let Some(ws) = self.sidebar.selected_workstream() {
                    let name = ws.name.clone();
                    self.switch_to_workstream(&name);
                }
                self.sidebar.filter_clear();
                self.focus.pop_overlay();
            }
            KeyCode::Up => {
                // Temporarily force workstream section for navigation
                let prev_section = self.sidebar.section;
                self.sidebar.section = SidebarSection::Workstreams;
                self.sidebar.select_prev();
                self.sidebar.section = prev_section;
            }
            KeyCode::Down => {
                let prev_section = self.sidebar.section;
                self.sidebar.section = SidebarSection::Workstreams;
                self.sidebar.select_next();
                self.sidebar.section = prev_section;
            }
            KeyCode::Char(c) => {
                self.sidebar.filter_push(c);
            }
            KeyCode::Backspace => {
                self.sidebar.filter_pop();
            }
            _ => {}
        }
    }

    /// Handle tool pane key events.
    // handle_tool_pane_key, open_tool_in_editor, run_pager → tool_pane_handler.rs
    // handle_logs_key → logs_handler.rs

    /// Clear any pending delete confirmations.
    pub(crate) fn clear_pending_deletes(&mut self) {
        self.pending_delete_workstream = None;
        self.pending_delete_session = None;
    }

    /// Handle sidebar key events.
    pub(crate) fn handle_sidebar_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Right => {
                // Close sidebar and return focus to input
                self.sidebar.close();
                self.moving_session_to_workstream = false;
                self.clear_pending_deletes();
                self.focus.return_to_input();
            }
            KeyCode::Tab => {
                // Switch between workstreams and sessions sections
                self.sidebar.toggle_section();
                self.clear_pending_deletes();
            }
            KeyCode::Up => {
                self.clear_pending_deletes();
                if let Some(ws_id) = self.sidebar.select_prev() {
                    // Workstream selection changed, fetch sessions from API
                    self.pending_actions
                        .push(PendingAction::FetchWorkstreamSessions(ws_id));
                }
            }
            KeyCode::Down => {
                self.clear_pending_deletes();
                if let Some(ws_id) = self.sidebar.select_next() {
                    // Workstream selection changed, fetch sessions from API
                    self.pending_actions
                        .push(PendingAction::FetchWorkstreamSessions(ws_id));
                }
            }
            KeyCode::Enter => {
                self.clear_pending_deletes();
                // Select current item
                match self.sidebar.section {
                    SidebarSection::Workstreams => {
                        if self.moving_session_to_workstream {
                            // Move current session to selected workstream
                            if let (Some(session_id), Some(ws)) =
                                (self.session_id.clone(), self.sidebar.selected_workstream())
                            {
                                tracing::info!(
                                    "User confirmed move: session {} -> workstream {} ({})",
                                    session_id,
                                    ws.name,
                                    ws.id
                                );
                                let ws_id = ws.id.clone();
                                self.pending_actions
                                    .push(PendingAction::MoveSessionToWorkstream(
                                        session_id, ws_id,
                                    ));
                            } else {
                                tracing::warn!(
                                    "Move confirmed but session_id or workstream not available"
                                );
                            }
                            self.moving_session_to_workstream = false;
                            self.sidebar.close();
                            self.focus.return_to_input();
                        } else {
                            // Switch to selected workstream
                            if let Some(ws) = self.sidebar.selected_workstream() {
                                let ws_name = ws.name.clone();
                                self.switch_to_workstream(&ws_name);
                            }
                        }
                    }
                    SidebarSection::Sessions => {
                        // First, ensure we're in the selected workstream
                        if let Some(ws) = self.sidebar.selected_workstream()
                            && !ws.is_current
                        {
                            let ws_name = ws.name.clone();
                            self.switch_to_workstream(&ws_name);
                        }

                        if self.sidebar.is_new_session_selected() {
                            // Create new session in the (now current) workstream
                            self.create_new_session();
                            self.sidebar.close();
                            self.focus.return_to_input();
                        } else if let Some(session) = self.sidebar.selected_session() {
                            // Switch to selected session
                            let session_id = session.id.clone();
                            self.switch_to_session(&session_id);
                            self.sidebar.close();
                            self.focus.return_to_input();
                        }
                    }
                }
            }
            KeyCode::Char('n') => {
                self.clear_pending_deletes();
                // Create new item in current section
                match self.sidebar.section {
                    SidebarSection::Workstreams => {
                        // Enter new workstream name mode
                        self.input_mode = InputMode::NewWorkstream;
                        self.input.clear();
                        self.sidebar.close();
                        self.focus.return_to_input();
                        self.status_message =
                            Some("New workstream: Enter name (Esc to cancel)".to_string());
                    }
                    SidebarSection::Sessions => {
                        // Switch to selected workstream if different
                        if let Some(ws) = self.sidebar.selected_workstream()
                            && !ws.is_current
                        {
                            let ws_name = ws.name.clone();
                            self.switch_to_workstream(&ws_name);
                        }
                        // Create new session in the (now current) workstream
                        self.create_new_session();
                        self.sidebar.close();
                        self.focus.return_to_input();
                    }
                }
            }
            KeyCode::Char('N') => {
                self.clear_pending_deletes();
                // Create new workstream regardless of current section
                self.input_mode = InputMode::NewWorkstream;
                self.input.clear();
                self.sidebar.close();
                self.focus.return_to_input();
                self.status_message =
                    Some("New workstream: Enter name (Esc to cancel)".to_string());
            }
            KeyCode::Char('r') => {
                self.clear_pending_deletes();
                // Rename selected workstream
                if self.sidebar.section == SidebarSection::Workstreams {
                    if let Some(ws) = self.sidebar.selected_workstream() {
                        let name = ws.name.clone();
                        self.input_mode = InputMode::RenameWorkstream(name.clone());
                        self.input.clear();
                        self.input.set_text(&name); // Pre-fill with current name
                        self.sidebar.close();
                        self.focus.return_to_input();
                        self.status_message = Some("Rename workstream (Esc to cancel)".to_string());
                    }
                } else {
                    self.status_message = Some("Select a workstream to rename".to_string());
                }
            }
            KeyCode::Char('d') => {
                // Delete current item (requires confirmation - press 'd' twice)
                match self.sidebar.section {
                    SidebarSection::Workstreams => {
                        if let Some(ws) = self.sidebar.selected_workstream() {
                            // Check if this is a confirmation (second 'd' press)
                            if let Some((pending_id, _)) = &self.pending_delete_workstream
                                && pending_id == &ws.id
                            {
                                // Confirmed - execute delete
                                let id = ws.id.clone();
                                self.pending_actions
                                    .push(PendingAction::DeleteWorkstream(id));
                                self.clear_pending_deletes();
                                return;
                            }

                            // First 'd' press - check if deletable and show confirmation
                            if ws.is_scratch {
                                self.status_message =
                                    Some("Cannot delete scratch workstream".to_string());
                            } else if ws.is_current {
                                self.status_message =
                                    Some("Cannot delete current workstream".to_string());
                            } else {
                                // Set pending and show confirmation message
                                let name = ws.name.clone();
                                let id = ws.id.clone();
                                self.pending_delete_workstream = Some((id, name.clone()));
                                self.status_message = Some(format!(
                                    "Delete '{}'? Press 'd' again to confirm, Esc to cancel",
                                    name
                                ));
                            }
                        }
                    }
                    SidebarSection::Sessions => {
                        if let Some(session) = self.sidebar.selected_session() {
                            // Check if this is a confirmation (second 'd' press)
                            if let Some(pending_id) = &self.pending_delete_session
                                && pending_id == &session.id
                            {
                                // Confirmed - execute delete
                                let id = session.id.clone();
                                self.pending_actions.push(PendingAction::DeleteSession(id));
                                self.clear_pending_deletes();
                                return;
                            }

                            // First 'd' press - check if deletable and show confirmation
                            if session.is_current {
                                self.status_message =
                                    Some("Cannot delete current session".to_string());
                            } else {
                                // Set pending and show confirmation message
                                let id = session.id.clone();
                                self.pending_delete_session = Some(id);
                                self.status_message = Some(
                                    "Delete session? Press 'd' again to confirm, Esc to cancel"
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                // Start filtering - clear existing filter and start fresh
                self.sidebar.filter_clear();
                self.status_message =
                    Some("Filter: type to search (Backspace to clear)".to_string());
            }
            KeyCode::Char(c) => {
                // Add to filter for incremental search
                self.sidebar.filter_push(c);
                if !self.sidebar.filter.is_empty() {
                    self.status_message = Some(format!("Filter: {}", self.sidebar.filter));
                }
            }
            KeyCode::Backspace => {
                self.sidebar.filter_pop();
                if self.sidebar.filter.is_empty() {
                    self.status_message = None;
                } else {
                    self.status_message = Some(format!("Filter: {}", self.sidebar.filter));
                }
            }
            _ => {}
        }
    }

    /// Switch to a different workstream.
    pub fn switch_to_workstream(&mut self, workstream_name: &str) {
        self.workstream = workstream_name.to_string();

        // Mark the new workstream as current in sidebar and get the ID
        let mut new_workstream_id = None;
        for ws in &mut self.sidebar.workstreams {
            ws.is_current = ws.name == workstream_name;
            if ws.is_current {
                new_workstream_id = Some(ws.id.clone());
            }
        }
        self.workstream_id = new_workstream_id;

        // Clear current session since we're switching workstreams
        self.messages.clear();
        self.tools.clear();
        self.session_id = None;
        self.chat_scroll = 0;
        self.chat_auto_scroll = true;

        // Clear usage stats (will be updated via WebSocket)
        self.workstream_usage = None;

        // Queue fetch of sessions for this workstream
        if let Some(ref ws_id) = self.workstream_id {
            self.pending_actions
                .push(PendingAction::FetchWorkstreamSessions(ws_id.clone()));
        } else {
            // No workstream ID, clear sessions
            self.sidebar.sessions.clear();
            self.sidebar.session_index = 0;
        }

        self.status_message = Some(format!("Switched to workstream: {}", workstream_name));
    }
}

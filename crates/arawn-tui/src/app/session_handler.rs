//! Session management — session switching, creation, sessions overlay.

use crate::focus::FocusTarget;
use crossterm::event::{KeyCode, KeyModifiers};

use super::App;
use crate::app_types::PendingAction;

impl App {
    pub(crate) fn handle_sessions_key(&mut self, key: crossterm::event::KeyEvent) {
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
    pub(crate) fn create_new_session(&mut self) {
        self.messages.clear();
        self.tools.clear();
        self.session_id = None; // Will be assigned by server on first message
        self.chat_scroll = 0;
        self.chat_auto_scroll = true;
        self.status_message = Some("New session created".to_string());
    }

    /// Open the sessions panel.
    pub(crate) fn open_sessions_panel(&mut self) {
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

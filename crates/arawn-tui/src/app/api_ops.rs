//! Async API operations — workstream/session CRUD via HTTP API.

use super::App;
use crate::app_types::{ChatMessage, PendingAction};
use crate::sessions::SessionSummary;
use crate::sidebar::{SidebarSection, WorkstreamEntry};
use arawn_client::{CreateWorkstreamRequest, UpdateWorkstreamRequest};
use chrono::{DateTime, Utc};

impl App {
    pub(crate) async fn process_pending_actions(&mut self) {
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
    pub(crate) async fn refresh_sidebar_data(&mut self) {
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

}

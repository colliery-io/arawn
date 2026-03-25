//! Server message dispatch.

use super::App;
use crate::app_types::{ChatMessage, ContextState, DiskWarning, ToolExecution, UsageStats};
use crate::protocol::ServerMessage;

impl App {
    pub fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::SessionCreated { session_id } => {
                self.session_id = Some(session_id);
            }

            ServerMessage::ChatChunk { chunk, done, .. } => {
                if done {
                    // Mark last message as not streaming
                    if let Some(last) = self.messages.last_mut() {
                        last.streaming = false;
                    }
                    self.waiting = false;
                } else if !chunk.is_empty() {
                    // Append to last message or create new one
                    if let Some(last) = self.messages.last_mut()
                        && !last.is_user
                        && last.streaming
                    {
                        last.content.push_str(&chunk);
                        return;
                    }
                    // Create new assistant message
                    self.push_message(ChatMessage {
                        is_user: false,
                        content: chunk,
                        streaming: true,
                    });
                }
            }

            ServerMessage::ToolStart {
                tool_id, tool_name, ..
            } => {
                self.push_tool(ToolExecution {
                    id: tool_id,
                    name: tool_name,
                    args: String::new(), // Args not provided by protocol yet
                    output: String::new(),
                    running: true,
                    success: None,
                    started_at: std::time::Instant::now(),
                    duration_ms: None,
                });
                // Auto-select the new tool if tool pane is visible
                if self.show_tool_pane {
                    self.selected_tool_index = Some(self.tools.len().saturating_sub(1));
                }
            }

            ServerMessage::ToolOutput {
                tool_id, content, ..
            } => {
                if let Some(tool) = self.tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.output.push_str(&content);
                }
            }

            ServerMessage::ToolEnd {
                tool_id, success, ..
            } => {
                if let Some(tool) = self.tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.running = false;
                    tool.success = Some(success);
                    tool.duration_ms = Some(tool.started_at.elapsed().as_millis() as u64);
                }
            }

            ServerMessage::Error { code, message } => {
                // Handle specific error codes
                if code == "session_not_owned" {
                    // We tried to send a message but aren't the owner
                    self.is_session_owner = false;
                    self.status_message =
                        Some("Read-only mode: session owned by another client".to_string());
                } else {
                    self.status_message = Some(format!("Error: {}", message));
                }
                self.waiting = false;
            }

            ServerMessage::AuthResult { success, error } => {
                if success {
                    self.status_message = Some("Authenticated".to_string());
                } else {
                    self.status_message =
                        Some(format!("Auth failed: {}", error.unwrap_or_default()));
                }
            }

            ServerMessage::Pong => {
                // Ignore pongs
            }

            ServerMessage::CommandProgress {
                command,
                message,
                percent,
            } => {
                self.command_executing = true;
                let progress_str = match percent {
                    Some(p) => format!("/{}: {} ({}%)", command, message, p),
                    None => format!("/{}: {}", command, message),
                };
                self.command_progress = Some(progress_str.clone());
                self.status_message = Some(progress_str);
            }

            ServerMessage::CommandResult {
                command,
                success,
                result,
            } => {
                self.command_executing = false;
                self.command_progress = None;

                if success {
                    // Format the result as a system message
                    let result_str =
                        if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                            msg.to_string()
                        } else {
                            serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "Success".to_string())
                        };
                    self.status_message = Some(format!("/{}: {}", command, result_str));

                    // Add as system message in chat
                    self.push_message(ChatMessage {
                        is_user: false,
                        content: format!("[/{}] {}", command, result_str),
                        streaming: false,
                    });
                } else {
                    let error_str = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    self.status_message = Some(format!("/{} failed: {}", command, error_str));
                }
            }

            ServerMessage::ContextInfo {
                current_tokens,
                max_tokens,
                percent,
                status,
                ..
            } => {
                self.context_info = Some(ContextState {
                    current_tokens,
                    max_tokens,
                    percent,
                    status,
                });
            }

            ServerMessage::DiskPressure {
                workstream_id,
                workstream_name,
                level,
                usage_bytes,
                limit_bytes,
                percent,
            } => {
                // Add warning, replacing any existing warning for same workstream
                self.disk_warnings
                    .retain(|w| w.workstream_id != workstream_id);
                self.disk_warnings.push(DiskWarning {
                    workstream_id: workstream_id.clone(),
                    workstream: workstream_name.clone(),
                    level,
                    usage_bytes,
                    limit_bytes,
                    percent,
                    timestamp: std::time::Instant::now(),
                });

                // Show status message for critical warnings
                if self.disk_warnings.last().map(|w| w.level.as_str()) == Some("critical") {
                    self.status_message = Some(format!(
                        "⚠ Disk critical: {} at {}% of limit",
                        workstream_name, percent
                    ));
                }
            }

            ServerMessage::WorkstreamUsage {
                workstream_id,
                workstream_name,
                is_scratch,
                production_bytes,
                work_bytes,
                total_bytes,
                limit_bytes,
                percent,
            } => {
                // Update usage stats if it's for the current workstream
                if self.workstream_id.as_deref() == Some(&workstream_id) {
                    self.workstream_usage = Some(UsageStats {
                        workstream_id,
                        workstream_name,
                        is_scratch,
                        production_bytes,
                        work_bytes,
                        total_bytes,
                        limit_bytes,
                        percent,
                    });
                }
            }

            ServerMessage::SubscribeAck {
                session_id,
                owner,
                reconnect_token,
            } => {
                // Update ownership state
                self.is_session_owner = owner;

                // Store reconnect token if we're the owner
                if let Some(token) = reconnect_token {
                    self.reconnect_tokens.insert(session_id.clone(), token);
                }

                if owner {
                    tracing::info!(session_id = %session_id, "Subscribed as owner");
                } else {
                    tracing::info!(session_id = %session_id, "Subscribed as reader (read-only)");
                    self.status_message = Some("Read-only mode".to_string());
                }
            }
        }
    }

}

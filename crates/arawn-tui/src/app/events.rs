


use super::{App, ChatMessage, ChatRole};
use super::format_tool_input;


impl App {
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
            crate::ws_client::EventUpdate::Usage {
                input_tokens,
                output_tokens,
            } => {
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
                    let content = msg
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    let chat_msg = match role {
                        "user" => ChatMessage::new(ChatRole::User, content),
                        "assistant" => {
                            if let Some(tool_uses) = msg.get("tool_uses").and_then(|t| t.as_array())
                            {
                                for tu in tool_uses {
                                    let name = tu
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("tool")
                                        .to_string();
                                    let input =
                                        tu.get("input").cloned().unwrap_or(serde_json::Value::Null);
                                    let summary = format_tool_input(&name, &input);
                                    self.messages.push(ChatMessage::new(
                                        ChatRole::ToolCall { name },
                                        summary,
                                    ));
                                }
                            }
                            if content.is_empty() {
                                continue;
                            }
                            ChatMessage::new(ChatRole::Assistant, content)
                        }
                        "tool_result" => {
                            let is_error = msg
                                .get("is_error")
                                .and_then(|e| e.as_bool())
                                .unwrap_or(false);
                            let name = self
                                .messages
                                .iter()
                                .rev()
                                .find_map(|m| match &m.role {
                                    ChatRole::ToolCall { name } => Some(name.clone()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| "tool".to_string());
                            ChatMessage::new(ChatRole::ToolResult { name, is_error }, content)
                        }
                        "summary" => {
                            ChatMessage::new(ChatRole::System, format!("[Summary] {content}"))
                        }
                        _ => continue,
                    };
                    self.messages.push(chat_msg);
                }
            }
        }
        self.scroll_offset = 0;
        self.dirty = true;
    }
}

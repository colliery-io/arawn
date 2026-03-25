//! Chat display state management — messages, scrolling, help text.

use super::{App, ChatMessage, ToolExecution};

impl App {
    /// Push a chat message (BoundedVec handles eviction automatically).
    pub(crate) fn push_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Push a tool execution (BoundedVec handles eviction automatically).
    pub(crate) fn push_tool(&mut self, tool: ToolExecution) {
        self.tools.push(tool);
    }

    /// Scroll chat up by the given number of lines.
    pub(crate) fn scroll_chat_up(&mut self, lines: usize) {
        self.chat_auto_scroll = false;
        self.chat_scroll = self.chat_scroll.saturating_sub(lines);
    }

    /// Scroll chat down by the given number of lines.
    pub(crate) fn scroll_chat_down(&mut self, lines: usize) {
        self.chat_auto_scroll = false;
        self.chat_scroll = self.chat_scroll.saturating_add(lines);
    }

    /// Get the help text for available commands.
    pub(crate) fn get_help_text(&self) -> String {
        let mut text = String::from("**Available Commands:**\n\n");
        text.push_str("/compact - Compact session history by summarizing older turns\n");
        text.push_str("  Options: --force, -f (force compaction even if not needed)\n\n");
        text.push_str("/help - Show this help message\n");
        text
    }

    /// Send the current input as a chat message.
    pub fn send_message(&mut self) {
        if !self.is_session_owner {
            self.status_message = Some("Read-only mode: cannot send messages".to_string());
            return;
        }

        let message = self.input.submit();

        self.push_message(ChatMessage {
            is_user: true,
            content: message.clone(),
            streaming: false,
        });

        self.tools.clear();
        self.chat_auto_scroll = true;

        if let Err(e) =
            self.ws_client
                .send_chat(message, self.session_id.clone(), self.workstream_id.clone())
        {
            self.status_message = Some(format!("Failed to send: {}", e));
            return;
        }

        self.waiting = true;
        self.status_message = None;
    }
}

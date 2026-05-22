use arawn_core::{Message, Session};

/// Result from running the test harness.
pub struct HarnessResult {
    pub final_text: String,
    pub session: Session,
}

impl HarnessResult {
    pub fn final_text(&self) -> &str {
        &self.final_text
    }

    pub fn tool_calls(&self) -> Vec<(&str, &serde_json::Value)> {
        self.session
            .messages()
            .iter()
            .filter_map(|msg| match msg {
                Message::Assistant { tool_uses, .. } => {
                    Some(tool_uses.iter().map(|tu| (tu.name.as_str(), &tu.input)))
                }
                _ => None,
            })
            .flatten()
            .collect()
    }

    pub fn session_messages(&self) -> &[Message] {
        self.session.messages()
    }

    pub fn message_count(&self) -> usize {
        self.session.messages().len()
    }
}

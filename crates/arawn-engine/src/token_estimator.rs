use arawn_core::Message;
use arawn_llm::ToolDefinition;

/// Fast, approximate token estimation using chars/4 heuristic.
/// Good enough for threshold decisions — not meant for exact counting.
pub struct TokenEstimator;

impl TokenEstimator {
    /// Estimate tokens for a single message.
    pub fn estimate_message(msg: &Message) -> u32 {
        let chars = match msg {
            Message::User { content } => content.len(),
            Message::Assistant { content, tool_uses } => {
                let mut total = content.len();
                for tu in tool_uses {
                    total += tu.name.len();
                    total += tu.input.to_string().len();
                }
                total
            }
            Message::ToolResult { content, .. } => content.len(),
            Message::Summary { content, .. } => content.len(),
        };
        // ~4 chars per token heuristic + small overhead per message for role/structure
        (chars as u32 / 4).saturating_add(4)
    }

    /// Estimate total tokens for all messages in a session.
    pub fn estimate_messages(messages: &[Message]) -> u32 {
        messages.iter().map(Self::estimate_message).sum()
    }

    /// Estimate tokens for tool definitions (JSON schemas sent with each request).
    pub fn estimate_tools(tools: &[ToolDefinition]) -> u32 {
        let chars: usize = tools
            .iter()
            .map(|t| t.name.len() + t.description.len() + t.parameters.to_string().len())
            .sum();
        (chars as u32 / 4).saturating_add(tools.len() as u32 * 4)
    }

    /// Estimate tokens for a system prompt string.
    pub fn estimate_system_prompt(prompt: &str) -> u32 {
        (prompt.len() as u32 / 4).saturating_add(4)
    }
}

// Re-export ModelLimits from arawn-tool crate.
pub use arawn_tool::ModelLimits;

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::ToolUse;
    use serde_json::json;

    #[test]
    fn estimate_user_message() {
        let msg = Message::User {
            content: "Hello, world!".into(), // 13 chars
        };
        let tokens = TokenEstimator::estimate_message(&msg);
        // 13/4 = 3, + 4 overhead = 7
        assert_eq!(tokens, 7);
    }

    #[test]
    fn estimate_assistant_with_tool_uses() {
        let msg = Message::Assistant {
            content: "Let me check.".into(),
            tool_uses: vec![ToolUse {
                id: "c1".into(),
                name: "shell".into(),
                input: json!({"command": "ls -la"}),
            }],
        };
        let tokens = TokenEstimator::estimate_message(&msg);
        assert!(tokens > 10); // Should be meaningful
    }

    #[test]
    fn estimate_tool_result() {
        let msg = Message::ToolResult {
            tool_use_id: "c1".into(),
            content: "file1.rs\nfile2.rs\nfile3.rs\n".into(),
            is_error: false,
        };
        let tokens = TokenEstimator::estimate_message(&msg);
        assert!(tokens > 5);
    }

    #[test]
    fn estimate_messages_sums() {
        let msgs = vec![
            Message::User {
                content: "a".repeat(100),
            },
            Message::Assistant {
                content: "b".repeat(200),
                tool_uses: vec![],
            },
        ];
        let total = TokenEstimator::estimate_messages(&msgs);
        let individual: u32 = msgs
            .iter()
            .map(|m| TokenEstimator::estimate_message(m))
            .sum();
        assert_eq!(total, individual);
    }

    #[test]
    fn estimate_tools() {
        let tools = vec![ToolDefinition {
            name: "shell".into(),
            description: "Execute a shell command".into(),
            parameters: json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        }];
        let tokens = TokenEstimator::estimate_tools(&tools);
        assert!(tokens > 10);
    }

    #[test]
    fn model_limits_for_known_models() {
        assert_eq!(
            ModelLimits::for_model("llama-3.3-70b-versatile").context_window,
            128_000
        );
        assert_eq!(
            ModelLimits::for_model("claude-3-opus").context_window,
            200_000
        );
        assert_eq!(
            ModelLimits::for_model("qwen/qwen3-32b").context_window,
            32_000
        );
        assert_eq!(
            ModelLimits::for_model("unknown-model").context_window,
            128_000
        );
    }

    #[test]
    fn should_compact_under_threshold() {
        let limits = ModelLimits::new(100_000, 0.85);
        // 85% of 100k = 85k threshold
        assert!(!limits.should_compact(50_000, 5_000, 1_000)); // 56k < 85k
    }

    #[test]
    fn should_compact_over_threshold() {
        let limits = ModelLimits::new(100_000, 0.85);
        assert!(limits.should_compact(80_000, 5_000, 1_000)); // 86k > 85k
    }

    #[test]
    fn available_for_messages() {
        let limits = ModelLimits::new(100_000, 0.85);
        let available = limits.available_for_messages(5_000, 1_000);
        // 85k - 5k - 1k = 79k
        assert_eq!(available, 79_000);
    }
}

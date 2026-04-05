use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool invocation requested by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// A message in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User { content: String },

    #[serde(rename = "assistant")]
    Assistant {
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_uses: Vec<ToolUse>,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    #[serde(rename = "summary")]
    Summary {
        content: String,
        #[serde(default)]
        original_count: usize,
        #[serde(default)]
        estimated_tokens_saved: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_message_serialization_roundtrip() {
        let msg = Message::User {
            content: "hello".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::User { content } => assert_eq!(content, "hello"),
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn assistant_message_with_tool_uses() {
        let msg = Message::Assistant {
            content: "I'll read that file".into(),
            tool_uses: vec![ToolUse {
                id: "call_1".into(),
                name: "file_read".into(),
                input: json!({"path": "test.txt"}),
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("file_read"));
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::Assistant { tool_uses, .. } => {
                assert_eq!(tool_uses.len(), 1);
                assert_eq!(tool_uses[0].name, "file_read");
            }
            _ => panic!("expected Assistant message"),
        }
    }

    #[test]
    fn assistant_message_without_tool_uses_omits_field() {
        let msg = Message::Assistant {
            content: "just text".into(),
            tool_uses: vec![],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("tool_uses"));
    }

    #[test]
    fn tool_result_message_roundtrip() {
        let msg = Message::ToolResult {
            tool_use_id: "call_1".into(),
            content: "file contents here".into(),
            is_error: false,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::ToolResult {
                tool_use_id,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "call_1");
                assert!(!is_error);
            }
            _ => panic!("expected ToolResult message"),
        }
    }

    #[test]
    fn tool_result_error_flag() {
        let msg = Message::ToolResult {
            tool_use_id: "call_2".into(),
            content: "permission denied".into(),
            is_error: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        match deserialized {
            Message::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!("expected ToolResult message"),
        }
    }
}

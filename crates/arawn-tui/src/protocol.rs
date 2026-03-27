//! WebSocket protocol types (client-side mirror of server protocol).

use serde::{Deserialize, Serialize};

/// Messages sent from the TUI client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Send a chat message.
    Chat {
        /// Optional session ID. If not provided, a new session is created.
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        /// Optional workstream ID.
        #[serde(skip_serializing_if = "Option::is_none")]
        workstream_id: Option<String>,
        /// The message content.
        message: String,
    },
    /// Ping to keep connection alive.
    Ping,
}

/// Messages received from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Authentication result.
    AuthResult {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Session created/confirmed.
    SessionCreated { session_id: String },
    /// Text chunk from agent response.
    ChatChunk {
        session_id: String,
        chunk: String,
        done: bool,
    },
    /// Tool execution started.
    ToolStart {
        session_id: String,
        tool_id: String,
        tool_name: String,
    },
    /// Tool execution completed.
    ToolEnd {
        session_id: String,
        tool_id: String,
        success: bool,
    },
    /// Error occurred.
    Error { code: String, message: String },
    /// Pong response to ping.
    Pong,
    /// Subscription acknowledgment.
    SubscribeAck {
        session_id: String,
        owner: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reconnect_token: Option<String>,
    },
    /// Any other message type we don't handle yet.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_chat_serializes() {
        let msg = ClientMessage::Chat {
            session_id: Some("s1".into()),
            workstream_id: None,
            message: "hello".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"chat""#));
        assert!(json.contains(r#""session_id":"s1""#));
        assert!(!json.contains("workstream_id"));
    }

    #[test]
    fn server_chat_chunk_deserializes() {
        let json = r#"{"type":"chat_chunk","session_id":"s1","chunk":"hi","done":false}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ServerMessage::ChatChunk { done: false, .. }));
    }

    #[test]
    fn unknown_type_deserializes_as_unknown() {
        let json = r#"{"type":"context_info","session_id":"s","current_tokens":0,"max_tokens":100,"percent":0,"status":"ok"}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ServerMessage::Unknown));
    }
}

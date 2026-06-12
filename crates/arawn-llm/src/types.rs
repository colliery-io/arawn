use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provider-neutral chat request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub system_prompt: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: Option<u32>,
}

/// Provider-neutral message for chat requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Message content — text or structured.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
}

/// A tool call within an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Tool definition sent with the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Why the model stopped generating, normalized across providers.
///
/// OpenAI-compatible providers report this as `finish_reason`
/// (`stop` / `tool_calls` / `length` / `content_filter`); Anthropic reports
/// it as `stop_reason` (`end_turn` / `tool_use` / `max_tokens` / …). Both are
/// mapped onto this enum so the engine can tell a clean stop apart from a
/// truncated one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Natural end of the assistant turn.
    Stop,
    /// The model wants to call one or more tools.
    ToolCalls,
    /// Output was cut off by the token limit — the turn is incomplete.
    Length,
    /// Output was halted by a safety/content filter.
    ContentFilter,
    /// Any other/unrecognized reason, preserved verbatim.
    Other(String),
}

impl FinishReason {
    /// Map an OpenAI-compatible `finish_reason` wire value.
    pub fn from_openai(s: &str) -> Self {
        match s {
            "stop" => Self::Stop,
            "tool_calls" => Self::ToolCalls,
            "length" => Self::Length,
            "content_filter" => Self::ContentFilter,
            other => Self::Other(other.to_string()),
        }
    }

    /// Map an Anthropic `stop_reason` wire value.
    pub fn from_anthropic(s: &str) -> Self {
        match s {
            "end_turn" | "stop_sequence" => Self::Stop,
            "tool_use" => Self::ToolCalls,
            "max_tokens" => Self::Length,
            other => Self::Other(other.to_string()),
        }
    }

    /// True when the model was cut off before finishing (token limit or
    /// content filter), meaning any in-progress output may be incomplete.
    pub fn is_truncated(&self) -> bool {
        matches!(self, Self::Length | Self::ContentFilter)
    }
}

/// Streaming chunk from the LLM.
///
/// Tool-call chunks carry an `index` so the engine can assemble multiple
/// concurrent (parallel) tool calls whose deltas arrive interleaved — the
/// OpenAI streaming spec keys interleaved `tool_calls` deltas by `index`,
/// and Anthropic keys them by content-block index.
#[derive(Debug, Clone)]
pub enum ChatChunk {
    TextDelta {
        text: String,
    },
    ToolUseStart {
        index: u32,
        id: String,
        name: String,
    },
    ToolUseInputDelta {
        index: u32,
        json: String,
    },
    Done {
        usage: Option<Usage>,
        finish_reason: Option<FinishReason>,
    },
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

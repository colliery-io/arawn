//! Helper that drains the streaming `LlmClient` API into a single
//! string. The chain prompts are short and we need the full response
//! before parsing JSON, so streaming buys us nothing — just collect.

use std::sync::Arc;

use futures::StreamExt;

use arawn_llm::{
    LlmClient,
    types::{ChatContent, ChatMessage, ChatRequest},
};

use crate::error::ExtractionError;

/// Send a single-turn (system + user) chat request and collect every
/// `TextDelta` chunk into one string. Errors surface as
/// `ExtractionError::Llm`.
pub async fn complete_text(
    client: &Arc<dyn LlmClient>,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, ExtractionError> {
    let req = ChatRequest {
        model: model.to_string(),
        system_prompt: Some(system.to_string()),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(user.to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        max_tokens: None,
    };

    let mut stream = client
        .stream(req)
        .await
        .map_err(|e| ExtractionError::Llm(e.to_string()))?;

    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| ExtractionError::Llm(e.to_string()))?;
        match chunk {
            arawn_llm::types::ChatChunk::TextDelta { text } => out.push_str(&text),
            // The chain stages prompt the model for plain JSON text;
            // tool-use chunks should not occur and are ignored if they do.
            _ => {}
        }
    }
    Ok(out)
}

/// Many LLMs wrap JSON output in ```json fences or prose. Extract the
/// first balanced `{...}` or `[...]` substring so callers can
/// `serde_json::from_str` against varied response shapes.
pub fn extract_json_block(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    let mut open: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (open, b) {
            (None, b'{') | (None, b'[') => {
                start = Some(i);
                open = Some(b);
                depth = 1;
            }
            (Some(b'{'), b'{') | (Some(b'['), b'[') => depth += 1,
            (Some(b'{'), b'}') | (Some(b'['), b']') => {
                depth -= 1;
                if depth == 0 {
                    let end = i + 1;
                    return Some(&raw[start.unwrap()..end]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_object_from_fenced_block() {
        let raw = "Here is the answer:\n```json\n{\"in_scope\": true}\n```";
        assert_eq!(extract_json_block(raw), Some("{\"in_scope\": true}"));
    }

    #[test]
    fn extracts_array_from_prose() {
        let raw = "Result: [\n  {\"a\":1},\n  {\"a\":2}\n] -- end";
        assert!(extract_json_block(raw).unwrap().starts_with('['));
    }

    #[test]
    fn handles_nested_braces() {
        let raw = "{ \"outer\": { \"inner\": 1 } }";
        assert_eq!(extract_json_block(raw), Some(raw));
    }

    #[test]
    fn returns_none_when_absent() {
        assert!(extract_json_block("no json here").is_none());
    }
}

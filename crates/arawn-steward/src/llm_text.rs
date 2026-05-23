//! Small helper that drains the streaming `LlmClient` API into one
//! string. Mirrors `arawn-extractor`'s helper but kept inline here so
//! `arawn-steward` doesn't depend on `arawn-extractor`. Promote to
//! `arawn-llm` once a third consumer appears.

use std::sync::Arc;

use futures::StreamExt;

use arawn_llm::{
    LlmClient,
    types::{ChatContent, ChatMessage, ChatRequest},
};

use crate::error::StewardError;

pub async fn complete_text(
    client: &Arc<dyn LlmClient>,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, StewardError> {
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
    // Steward subroutines are local-bound work — gate them through
    // the process-wide LLM resource gate so a background scan does
    // not stack memory on top of an in-flight agent call.
    let _gate = arawn_llm::gate::acquire_local()
        .await
        .map_err(|e| StewardError::Subroutine {
            name: "llm".into(),
            message: format!("llm gate refused acquire: {e:?}"),
        })?;
    let mut stream = client
        .stream(req)
        .await
        .map_err(|e| StewardError::Subroutine {
            name: "llm".into(),
            message: e.to_string(),
        })?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| StewardError::Subroutine {
            name: "llm".into(),
            message: e.to_string(),
        })?;
        if let arawn_llm::types::ChatChunk::TextDelta { text } = chunk {
            out.push_str(&text);
        }
    }
    Ok(out)
}

/// First balanced `{...}` or `[...]` substring — same parser as
/// `arawn-extractor::llm_text`.
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

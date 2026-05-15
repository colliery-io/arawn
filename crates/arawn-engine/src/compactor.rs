use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use arawn_core::{Message, Session};
use arawn_llm::{ChatChunk, ChatContent, ChatMessage, ChatRequest, LlmClient};

use crate::compact_prompt::{
    get_compact_prompt, get_compact_user_summary_message, get_partial_compact_prompt,
};
use crate::error::EngineError;
use crate::token_estimator::{ModelLimits, TokenEstimator};

const DEFAULT_KEEP_RECENT: usize = 6;

/// Result of a compaction operation.
#[derive(Debug)]
pub struct CompactionResult {
    pub messages_summarized: usize,
    pub tokens_before: u32,
    pub tokens_after: u32,
}

/// Orchestrates context compaction via LLM summarization.
pub struct Compactor {
    llm: Arc<dyn LlmClient>,
    keep_recent: usize,
    model: String,
}

impl Compactor {
    pub fn new(llm: Arc<dyn LlmClient>, model: impl Into<String>) -> Self {
        Self {
            llm,
            keep_recent: DEFAULT_KEEP_RECENT,
            model: model.into(),
        }
    }

    pub fn with_keep_recent(
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
        keep_recent: usize,
    ) -> Self {
        Self {
            llm,
            keep_recent,
            model: model.into(),
        }
    }

    /// Check if the session needs compaction based on token estimates.
    pub fn should_compact(
        &self,
        session: &Session,
        limits: &ModelLimits,
        tool_tokens: u32,
        system_tokens: u32,
    ) -> bool {
        if session.messages().len() <= self.keep_recent {
            return false;
        }

        let session_tokens = TokenEstimator::estimate_messages(session.messages());
        limits.should_compact(session_tokens, tool_tokens, system_tokens)
    }

    /// Compact the session by summarizing old messages via LLM.
    pub async fn compact(
        &self,
        session: &mut Session,
        _limits: &ModelLimits,
    ) -> Result<CompactionResult, EngineError> {
        let tokens_before = TokenEstimator::estimate_messages(session.messages());

        if session.messages().len() <= self.keep_recent {
            return Ok(CompactionResult {
                messages_summarized: 0,
                tokens_before,
                tokens_after: tokens_before,
            });
        }

        let split_point = session.messages().len() - self.keep_recent;
        let old_messages = &session.messages()[..split_point];

        // Choose prompt based on whether session starts with a previous Summary
        let has_prior_summary = matches!(old_messages.first(), Some(Message::Summary { .. }));
        let prompt = if has_prior_summary {
            get_partial_compact_prompt()
        } else {
            get_compact_prompt()
        };

        debug!(
            old_count = old_messages.len(),
            keep_recent = self.keep_recent,
            has_prior_summary,
            "compacting session"
        );

        // Build the compaction request — old messages as conversation, prompt as system
        let messages: Vec<ChatMessage> = old_messages
            .iter()
            .map(|msg| match msg {
                Message::User { content } | Message::Summary { content, .. } => ChatMessage {
                    role: "user".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                },
                Message::Assistant { content, .. } => ChatMessage {
                    role: "assistant".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                },
                Message::ToolResult { content, .. } => ChatMessage {
                    role: "user".into(),
                    content: ChatContent::Text(format!("[Tool result]: {content}")),
                    tool_calls: vec![],
                    tool_call_id: None,
                },
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            system_prompt: Some(prompt),
            messages,
            tools: vec![], // No tools during compaction
            max_tokens: None,
        };

        // Call LLM and collect response
        let raw_summary = self.call_llm(request).await?;

        // Format: strip <analysis>, extract <summary>, wrap with continuation framing
        let formatted = get_compact_user_summary_message(&raw_summary, true);

        // Compact the session
        let summarized = session.compact(formatted, self.keep_recent);
        let tokens_after = TokenEstimator::estimate_messages(session.messages());

        info!(
            messages_summarized = summarized,
            tokens_before,
            tokens_after,
            saved = tokens_before.saturating_sub(tokens_after),
            "compaction complete"
        );

        Ok(CompactionResult {
            messages_summarized: summarized,
            tokens_before,
            tokens_after,
        })
    }

    async fn call_llm(&self, request: ChatRequest) -> Result<String, EngineError> {
        // Compaction is local-bound work too (same backend as the
        // engine today). Gate it for laptop-RAM safety.
        let _gate = arawn_llm::gate::acquire_local().await.map_err(|e| {
            EngineError::Other(anyhow::anyhow!("llm gate refused acquire: {e:?}"))
        })?;
        let mut stream = self.llm.stream(request).await?;
        let mut text = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(ChatChunk::TextDelta { text: t }) => text.push_str(&t),
                Ok(ChatChunk::Done { .. }) => break,
                Ok(_) => {} // Ignore tool calls (shouldn't happen with no tools)
                Err(e) => {
                    warn!(error = %e, "compaction stream error");
                    return Err(EngineError::Llm(e));
                }
            }
        }

        if text.is_empty() {
            return Err(EngineError::Tool(
                "compaction LLM returned empty response".into(),
            ));
        }

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_llm::{MockLlmClient, MockResponse};
    use uuid::Uuid;

    fn make_session_with_messages(count: usize) -> Session {
        let mut session = Session::new(Uuid::new_v4());
        // Use large messages so token estimates are meaningful relative to summary size
        let filler = "x".repeat(500);
        for i in 0..count {
            if i % 2 == 0 {
                session.add_message(Message::User {
                    content: format!("User message {i}: {filler}"),
                });
            } else {
                session.add_message(Message::Assistant {
                    content: format!("Assistant response {i}: {filler}"),
                    tool_uses: vec![],
                });
            }
        }
        session
    }

    #[test]
    fn should_compact_false_under_threshold() {
        let mock = Arc::new(MockLlmClient::new(vec![]));
        let compactor = Compactor::new(mock, "test-model");
        let session = make_session_with_messages(10);
        let limits = ModelLimits::new(1_000_000, 0.85); // huge window

        assert!(!compactor.should_compact(&session, &limits, 0, 0));
    }

    #[test]
    fn should_compact_true_over_threshold() {
        let mock = Arc::new(MockLlmClient::new(vec![]));
        let compactor = Compactor::new(mock, "test-model");
        let session = make_session_with_messages(20);
        let limits = ModelLimits::new(100, 0.85); // tiny window, session will exceed

        assert!(compactor.should_compact(&session, &limits, 0, 0));
    }

    #[test]
    fn should_compact_false_too_few_messages() {
        let mock = Arc::new(MockLlmClient::new(vec![]));
        let compactor = Compactor::with_keep_recent(mock, "test-model".to_string(), 6);
        let session = make_session_with_messages(4); // fewer than keep_recent
        let limits = ModelLimits::new(100, 0.85);

        assert!(!compactor.should_compact(&session, &limits, 0, 0));
    }

    #[tokio::test]
    async fn compact_produces_summary() {
        let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text(
            "<analysis>\nThinking about the conversation.\n</analysis>\n\n<summary>\n1. Primary Request: User asked questions.\n</summary>",
        )]));

        let compactor = Compactor::with_keep_recent(mock, "test-model".to_string(), 3);
        let mut session = make_session_with_messages(10);
        let limits = ModelLimits::new(100, 0.85);

        let result = compactor.compact(&mut session, &limits).await.unwrap();

        assert_eq!(result.messages_summarized, 7);
        assert!(result.tokens_after < result.tokens_before);
        assert_eq!(session.messages().len(), 4); // 1 summary + 3 recent

        match &session.messages()[0] {
            Message::Summary { original_count, .. } => assert_eq!(*original_count, 7),
            _ => panic!("expected Summary"),
        }
    }

    #[tokio::test]
    async fn compact_preserves_recent_messages() {
        let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text(
            "<summary>Summary content</summary>",
        )]));

        let compactor = Compactor::with_keep_recent(mock, "test-model".to_string(), 2);
        let mut session = make_session_with_messages(6);

        // Remember the last 2 messages
        let last_two: Vec<String> = session.messages()[4..]
            .iter()
            .map(|m| match m {
                Message::User { content } | Message::Assistant { content, .. } => content.clone(),
                _ => String::new(),
            })
            .collect();

        let limits = ModelLimits::new(100, 0.85);
        compactor.compact(&mut session, &limits).await.unwrap();

        // Verify last 2 messages are preserved verbatim
        let msgs = session.messages();
        for (i, expected) in last_two.iter().enumerate() {
            match &msgs[i + 1] {
                Message::User { content } | Message::Assistant { content, .. } => {
                    assert_eq!(content, expected);
                }
                _ => panic!("expected preserved message"),
            }
        }
    }

    #[tokio::test]
    async fn compact_noop_when_few_messages() {
        let mock = Arc::new(MockLlmClient::new(vec![]));
        let compactor = Compactor::with_keep_recent(mock, "test-model".to_string(), 10);
        let mut session = make_session_with_messages(5);
        let limits = ModelLimits::new(100, 0.85);

        let result = compactor.compact(&mut session, &limits).await.unwrap();
        assert_eq!(result.messages_summarized, 0);
        assert_eq!(session.messages().len(), 5); // unchanged
    }
}

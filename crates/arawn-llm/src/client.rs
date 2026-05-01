use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};

use crate::error::LlmError;
use crate::types::{ChatChunk, ChatMessage, ChatRequest};

/// Provider-agnostic LLM client trait.
/// The engine codes against this trait, never a concrete provider.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>;

    /// Probe a model with a minimal request to confirm it is reachable and
    /// accepting traffic. Used both eagerly at startup and lazily before
    /// real requests when the cached warmup state is stale.
    ///
    /// The default impl issues a single 1-token chat completion via `stream`.
    /// Provider implementations rarely need to override this.
    async fn warmup(&self, model: &str) -> Result<(), LlmError> {
        use crate::types::ChatContent;

        let request = ChatRequest {
            model: model.to_string(),
            system_prompt: None,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text("ping".to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            tools: Vec::new(),
            max_tokens: Some(1),
        };

        let mut stream = self.stream(request).await?;
        // Drain until first error or end. We don't care about the content —
        // success of the request opening is what we're verifying.
        while let Some(chunk) = stream.next().await {
            chunk?;
        }
        Ok(())
    }
}

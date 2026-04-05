use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::error::LlmError;
use crate::types::{ChatChunk, ChatRequest};

/// Provider-agnostic LLM client trait.
/// The engine codes against this trait, never a concrete provider.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>;
}

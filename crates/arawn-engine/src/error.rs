use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("tool error: {0}")]
    Tool(String),

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("LLM error: {0}")]
    Llm(#[from] arawn_llm::LlmError),

    #[error("max iterations ({0}) exceeded")]
    MaxIterations(usize),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl EngineError {
    /// Return a user-facing error message with actionable guidance.
    pub fn user_message(&self) -> String {
        match self {
            EngineError::Llm(e) => e.user_message(),
            EngineError::ToolNotFound(name) => {
                format!(
                    "Tool '{name}' is not available. The LLM tried to call a tool that \
                     isn't registered. This may indicate a model hallucination."
                )
            }
            EngineError::Tool(msg) => {
                format!("A tool encountered an error: {msg}")
            }
            EngineError::MaxIterations(max) => {
                format!(
                    "Reached the maximum iteration limit ({max} turns) without completing. \
                     Try breaking the task into smaller steps."
                )
            }
            EngineError::Other(e) => {
                format!("Unexpected error: {e}")
            }
        }
    }
}

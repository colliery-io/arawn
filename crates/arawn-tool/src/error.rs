use thiserror::Error;

/// Errors that tools can return from `execute()`.
///
/// This is a simplified error type scoped to tool-level concerns.
/// The engine converts between `ToolError` and its own `EngineError` at the boundary.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool execution failed (validation, I/O, logic errors).
    #[error("tool error: {0}")]
    ExecutionFailed(String),

    /// A required tool was not found (e.g., sub-tool lookup in AgentTool).
    #[error("tool not found: {0}")]
    NotFound(String),

    /// LLM sub-query failed (e.g., web_fetch summarization, agent sub-query).
    #[error("LLM error: {0}")]
    Llm(String),

    /// Any other error (via `?` operator with anyhow).
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

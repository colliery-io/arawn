mod context;
mod error;
mod llm_preference;
mod registry;
mod tool;

pub use context::{ModelLimits, ToolContext};
pub use error::ToolError;
pub use llm_preference::{
    LlmCapabilities, LlmPreference, LlmResolution, LlmResolver, MatchQuality, ResolvedLlmInfo,
};
pub use registry::ToolRegistry;
pub use tool::{Tool, ToolCategory, ToolOutput};

mod context;
mod error;
mod llm_preference;
mod registry;
mod tool;

pub use context::{ModelLimits, ToolContext};
pub use error::ToolError;
pub use llm_preference::{
    LlmCapabilities, LlmPreference, LlmResolution, LlmResolverFn, MatchQuality, ResolvedLlmInfo,
};
pub use registry::ToolRegistry;
pub use tool::{PermissionCategory, Tool, ToolCategory, ToolOutput};

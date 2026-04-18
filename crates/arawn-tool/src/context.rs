use std::path::{Path, PathBuf};
use std::sync::Arc;

use arawn_llm::LlmClient;
use uuid::Uuid;

use crate::llm_preference::{LlmPreference, LlmResolution};

/// Model context window limits — used by sub-agents for compaction decisions.
#[derive(Debug, Clone)]
pub struct ModelLimits {
    /// Total context window in tokens.
    pub context_window: u32,
    /// Fraction of context window that triggers compaction (e.g., 0.85).
    pub compaction_threshold: f32,
}

impl ModelLimits {
    pub fn new(context_window: u32, compaction_threshold: f32) -> Self {
        Self {
            context_window,
            compaction_threshold,
        }
    }

    /// Get default limits for a known model name.
    pub fn for_model(model: &str) -> Self {
        let context_window = match model {
            m if m.contains("llama-3.3") => 128_000,
            m if m.contains("llama-3.1") => 128_000,
            m if m.contains("llama-4") => 128_000,
            m if m.contains("gpt-oss") => 128_000,
            m if m.contains("qwen") => 32_000,
            m if m.contains("claude-3") => 200_000,
            m if m.contains("claude-sonnet") | m.contains("claude-opus") => 200_000,
            _ => 128_000,
        };
        Self {
            context_window,
            compaction_threshold: 0.85,
        }
    }

    /// Check if the total estimated tokens exceed the compaction threshold.
    pub fn should_compact(
        &self,
        session_tokens: u32,
        tool_tokens: u32,
        system_tokens: u32,
    ) -> bool {
        let total = session_tokens + tool_tokens + system_tokens;
        let threshold = (self.context_window as f32 * self.compaction_threshold) as u32;
        total > threshold
    }

    /// The token budget available after accounting for tools and system prompt.
    pub fn available_for_messages(&self, tool_tokens: u32, system_tokens: u32) -> u32 {
        let threshold = (self.context_window as f32 * self.compaction_threshold) as u32;
        threshold
            .saturating_sub(tool_tokens)
            .saturating_sub(system_tokens)
    }
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            context_window: 128_000,
            compaction_threshold: 0.85,
        }
    }
}

/// Execution context provided to tools.
///
/// This trait defines the minimal interface tools need from the execution
/// environment. The engine provides the concrete implementation.
pub trait ToolContext: Send + Sync {
    /// The working directory (workspace root) for this session.
    fn working_dir(&self) -> &Path;

    /// Unique session identifier.
    fn session_id(&self) -> Uuid;

    /// Validate that a path stays within the workspace or is in the allowed list.
    fn validate_path(&self, path_str: &str) -> Result<PathBuf, String>;

    /// Check if a path is in the allowed list (outside the sandbox).
    fn is_allowed_path(&self, path: &Path) -> bool;

    /// Record that a file has been read in this session.
    fn mark_file_read(&self, path: PathBuf);

    /// Check if a file has been read in this session.
    fn has_read_file(&self, path: &Path) -> bool;

    /// Get the LLM client if available (for tools that make sub-queries).
    fn llm(&self) -> Option<&Arc<dyn LlmClient>>;

    /// Get the model name for sub-queries.
    fn model(&self) -> Option<&str>;

    /// Get model context window limits (for sub-agent compaction).
    fn model_limits(&self) -> &ModelLimits;

    /// Get data directory for tool result persistence.
    fn data_dir(&self) -> Option<&PathBuf>;

    /// Current agent nesting depth (0 = top-level).
    fn agent_depth(&self) -> u8;

    /// Whether another sub-agent can be spawned at this depth.
    fn can_spawn_agent(&self) -> bool;

    /// Create a child context for a sub-agent (increments depth).
    fn for_sub_agent(&self) -> Box<dyn ToolContext>;

    /// Get the workstream name.
    fn workstream_name(&self) -> &str;

    /// Paths outside the sandbox that file tools are allowed to access.
    fn allowed_paths(&self) -> &[PathBuf];

    /// Resolve an [`LlmPreference`] against the runtime's LLM pool. Returns
    /// `None` if no resolver is wired (e.g., test contexts). Tools that
    /// declare an `llm_preference()` typically call this from inside
    /// `execute()` and use the resolved client + match quality to decide
    /// whether to proceed normally or degrade.
    fn resolve_llm(&self, _preference: &LlmPreference) -> Option<LlmResolution> {
        None
    }
}

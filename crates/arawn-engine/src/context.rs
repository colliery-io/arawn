use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use arawn_core::Workstream;
use arawn_llm::LlmClient;

use crate::token_estimator::ModelLimits;

/// Maximum sub-agent nesting depth. Prevents infinite recursion.
const MAX_AGENT_DEPTH: u8 = 3;

/// Execution context provided to tools.
/// Immutable for the lifetime of a session — workstream binding never changes.
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: Uuid,
    pub working_dir: PathBuf,
    workstream_name: String,
    /// Paths outside the sandbox that file tools are allowed to access.
    /// Used for global and workstream arawn.md context files.
    pub allowed_paths: Vec<PathBuf>,
    /// LLM client for tools that need to make sub-queries (e.g. web_fetch summarization).
    llm: Option<Arc<dyn LlmClient>>,
    /// Model name for sub-queries.
    model: Option<String>,
    /// Model context window limits (inherited by sub-agents for compaction).
    model_limits: ModelLimits,
    /// Data directory for persisting large tool results.
    data_dir: Option<PathBuf>,
    /// Current agent nesting depth. 0 = top-level, 1 = first sub-agent, etc.
    agent_depth: u8,
    /// Tracks which files have been read in this session.
    /// FileEdit and FileWrite check this before modifying existing files.
    read_files: Arc<RwLock<HashSet<PathBuf>>>,
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("session_id", &self.session_id)
            .field("working_dir", &self.working_dir)
            .field("workstream_name", &self.workstream_name)
            .field("allowed_paths", &self.allowed_paths)
            .field("has_llm", &self.llm.is_some())
            .field("model", &self.model)
            .finish()
    }
}

impl ToolContext {
    pub fn new(workstream: &Workstream, session_id: Uuid) -> Self {
        Self {
            session_id,
            working_dir: workstream.root_dir.clone(),
            workstream_name: workstream.name.clone(),
            allowed_paths: Vec::new(),
            llm: None,
            model: None,
            model_limits: ModelLimits::default(),
            data_dir: None,
            agent_depth: 0,
            read_files: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Set allowed paths that file tools can access outside the sandbox.
    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = paths;
        self
    }

    /// Attach an LLM client and model for tools that need sub-queries.
    pub fn with_llm(mut self, llm: Arc<dyn LlmClient>, model: String) -> Self {
        self.llm = Some(llm);
        self.model = Some(model);
        self
    }

    /// Set model limits for sub-agent compaction.
    pub fn with_model_limits(mut self, limits: ModelLimits) -> Self {
        self.model_limits = limits;
        self
    }

    /// Set data directory for persisting large tool results.
    pub fn with_data_dir(mut self, dir: PathBuf) -> Self {
        self.data_dir = Some(dir);
        self
    }

    /// Check if a path is in the allowed list (exact match on canonical paths).
    pub fn is_allowed_path(&self, path: &std::path::Path) -> bool {
        if let Ok(canonical) = path.canonicalize() {
            self.allowed_paths
                .iter()
                .any(|p| p.canonicalize().map(|c| c == canonical).unwrap_or(false))
        } else {
            // File doesn't exist yet — check non-canonical match for write access
            self.allowed_paths.iter().any(|p| p == path)
        }
    }

    pub fn workstream_name(&self) -> &str {
        &self.workstream_name
    }

    /// Get the LLM client if available.
    pub fn llm(&self) -> Option<&Arc<dyn LlmClient>> {
        self.llm.as_ref()
    }

    /// Get the model name for sub-queries.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Get model limits (for sub-agent compaction).
    pub fn model_limits(&self) -> &ModelLimits {
        &self.model_limits
    }

    /// Get data directory for tool result persistence.
    pub fn data_dir(&self) -> Option<&PathBuf> {
        self.data_dir.as_ref()
    }

    /// Current agent nesting depth.
    pub fn agent_depth(&self) -> u8 {
        self.agent_depth
    }

    /// Whether another sub-agent can be spawned at this depth.
    pub fn can_spawn_agent(&self) -> bool {
        self.agent_depth < MAX_AGENT_DEPTH
    }

    /// Create a child context for a sub-agent (increments depth).
    /// Sub-agents get their own fresh read-file tracker.
    pub fn for_sub_agent(&self) -> Self {
        let mut child = self.clone();
        child.agent_depth = self.agent_depth.saturating_add(1);
        child.read_files = Arc::new(RwLock::new(HashSet::new()));
        child
    }

    /// Record that a file has been read in this session.
    pub fn mark_file_read(&self, path: PathBuf) {
        self.read_files.write().unwrap().insert(path);
    }

    /// Check if a file has been read in this session.
    pub fn has_read_file(&self, path: &PathBuf) -> bool {
        self.read_files.read().unwrap().contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;

    #[test]
    fn context_from_workstream() {
        let ws = Workstream::new("Test WS", "/tmp/test-ws");
        let session_id = Uuid::new_v4();
        let ctx = ToolContext::new(&ws, session_id);

        assert_eq!(ctx.session_id, session_id);
        assert_eq!(ctx.working_dir, PathBuf::from("/tmp/test-ws"));
        assert_eq!(ctx.workstream_name(), "Test WS");
    }

    #[test]
    fn context_is_clone() {
        let ws = Workstream::new("Clone Test", "/tmp/clone");
        let ctx = ToolContext::new(&ws, Uuid::new_v4());
        let cloned = ctx.clone();
        assert_eq!(ctx.session_id, cloned.session_id);
        assert_eq!(ctx.working_dir, cloned.working_dir);
    }
}

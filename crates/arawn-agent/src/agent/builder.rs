//! AgentBuilder for fluent Agent construction.

use std::sync::Arc;

use arawn_llm::interaction_log::InteractionLogger;
use arawn_llm::{LlmBackend, SharedBackend, SharedEmbedder};
use arawn_memory::store::MemoryStore;
use arawn_types::{FsGateResolver, SharedHookDispatcher, SharedSecretResolver};

use crate::error::{AgentError, Result};
use crate::prompt::SystemPromptBuilder;
use crate::tool::ToolRegistry;
use crate::types::AgentConfig;

use super::{Agent, RecallConfig};

// ─────────────────────────────────────────────────────────────────────────────
// Agent Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Builder for constructing an Agent with fluent API.
pub struct AgentBuilder {
    backend: Option<SharedBackend>,
    tools: ToolRegistry,
    config: AgentConfig,
    prompt_builder: Option<SystemPromptBuilder>,
    bootstrap_context: Option<crate::prompt::BootstrapContext>,
    interaction_logger: Option<Arc<InteractionLogger>>,
    memory_store: Option<Arc<MemoryStore>>,
    embedder: Option<SharedEmbedder>,
    recall_config: RecallConfig,
    plugin_prompts: Vec<(String, String)>,
    hook_dispatcher: Option<SharedHookDispatcher>,
    fs_gate_resolver: Option<FsGateResolver>,
    secret_resolver: Option<SharedSecretResolver>,
}

impl AgentBuilder {
    /// Create a new builder with defaults.
    pub fn new() -> Self {
        Self {
            backend: None,
            tools: ToolRegistry::new(),
            config: AgentConfig::default(),
            prompt_builder: None,
            bootstrap_context: None,
            interaction_logger: None,
            memory_store: None,
            embedder: None,
            recall_config: RecallConfig::default(),
            plugin_prompts: Vec::new(),
            hook_dispatcher: None,
            fs_gate_resolver: None,
            secret_resolver: None,
        }
    }

    /// Set the LLM backend.
    pub fn with_backend(mut self, backend: impl LlmBackend + 'static) -> Self {
        self.backend = Some(Arc::new(backend));
        self
    }

    /// Set the LLM backend from a shared reference.
    pub fn with_shared_backend(mut self, backend: SharedBackend) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Set the tool registry.
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    /// Register a single tool.
    pub fn with_tool<T: crate::tool::Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.register(tool);
        self
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.max_tokens = max_tokens;
        self
    }

    /// Set max iterations.
    pub fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.config.max_iterations = max_iterations;
        self
    }

    /// Set cumulative token budget (input + output).
    ///
    /// When set, the agent stops gracefully after exceeding this token total.
    /// Useful as a safety valve for sub-agents like the RLM exploration agent.
    pub fn with_max_total_tokens(mut self, max_total_tokens: usize) -> Self {
        self.config.max_total_tokens = Some(max_total_tokens);
        self
    }

    /// Set the workspace path.
    ///
    /// The workspace is the root directory for file operations.
    pub fn with_workspace(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config.workspace_path = Some(path.into());
        self
    }

    /// Set a prompt builder for dynamic system prompt generation.
    ///
    /// When set, the builder will be used to generate the system prompt
    /// at build time, incorporating tools and other context.
    ///
    /// This takes precedence over `with_system_prompt()`.
    pub fn with_prompt_builder(mut self, builder: SystemPromptBuilder) -> Self {
        self.prompt_builder = Some(builder);
        self
    }

    /// Load bootstrap context files from a directory.
    ///
    /// Looks for BEHAVIOR.md, BOOTSTRAP.md, MEMORY.md, IDENTITY.md in the
    /// specified directory and adds them to the prompt.
    ///
    /// Can be combined with `with_prompt_file()` to add additional custom files.
    ///
    /// # Example
    /// ```rust,ignore
    /// let agent = Agent::builder()
    ///     .with_backend(backend)
    ///     .with_bootstrap_dir("/path/to/prompts")
    ///     .build()?;
    /// ```
    pub fn with_bootstrap_dir(mut self, path: impl AsRef<std::path::Path>) -> Self {
        use crate::prompt::BootstrapContext;

        match BootstrapContext::load(path.as_ref()) {
            Ok(context) if !context.is_empty() => {
                // Merge with existing context or set new one
                if let Some(ref mut existing) = self.bootstrap_context {
                    for file in context.files() {
                        existing.add_file(&file.filename, &file.content);
                    }
                } else {
                    self.bootstrap_context = Some(context);
                }
            }
            Ok(_) => {
                // Empty context, no files found - that's fine
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.as_ref().display(),
                    error = %e,
                    "Failed to load bootstrap context"
                );
            }
        }
        self
    }

    /// Load a custom prompt file and add it to the bootstrap context.
    ///
    /// Use this for prompt files with non-standard names. Can be called
    /// multiple times to add multiple files. The file content will be
    /// added to the bootstrap context section of the prompt.
    ///
    /// # Example
    /// ```rust,ignore
    /// let agent = Agent::builder()
    ///     .with_backend(backend)
    ///     .with_prompt_file("/path/to/custom_persona.md")
    ///     .with_prompt_file("/path/to/guidelines.md")
    ///     .build()?;
    /// ```
    pub fn with_prompt_file(mut self, path: impl AsRef<std::path::Path>) -> Self {
        use crate::prompt::BootstrapContext;

        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("custom.md")
                    .to_string();

                // Get or create bootstrap context and add file
                let context = self
                    .bootstrap_context
                    .get_or_insert_with(BootstrapContext::new);
                context.add_file(filename, content);
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to load prompt file"
                );
            }
        }
        self
    }

    /// Set the memory store for active recall.
    pub fn with_memory_store(mut self, store: Arc<MemoryStore>) -> Self {
        self.memory_store = Some(store);
        self
    }

    /// Set the embedder for active recall.
    pub fn with_embedder(mut self, embedder: SharedEmbedder) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Set the recall configuration.
    pub fn with_recall_config(mut self, config: RecallConfig) -> Self {
        self.recall_config = config;
        self
    }

    /// Set the interaction logger for structured JSONL capture.
    pub fn with_interaction_logger(mut self, logger: Arc<InteractionLogger>) -> Self {
        self.interaction_logger = Some(logger);
        self
    }

    /// Add plugin prompt fragments to the system prompt.
    ///
    /// Each fragment is a `(plugin_name, prompt_text)` pair that will be
    /// appended as a `## Plugin: {name}` section in the system prompt.
    pub fn with_plugin_prompts(mut self, prompts: Vec<(String, String)>) -> Self {
        self.plugin_prompts = prompts;
        self
    }

    /// Set the hook dispatcher for plugin lifecycle events.
    ///
    /// The hook dispatcher fires hooks at lifecycle events like PreToolUse,
    /// PostToolUse, SessionStart, and SessionEnd. PreToolUse hooks can block
    /// tool execution.
    ///
    /// Accepts any type implementing `HookDispatch`, wrapped in an Arc.
    pub fn with_hook_dispatcher(mut self, dispatcher: SharedHookDispatcher) -> Self {
        self.hook_dispatcher = Some(dispatcher);
        self
    }

    /// Build the agent.
    pub fn build(mut self) -> Result<Agent> {
        let backend = self
            .backend
            .ok_or_else(|| AgentError::Config("LLM backend is required".to_string()))?;

        // Build the dynamic prompt builder if we have any prompt-related inputs.
        // The builder is stored on the Agent and rebuilt per-turn for fresh datetime etc.
        let prompt_builder = if self.prompt_builder.is_some()
            || self.bootstrap_context.is_some()
            || !self.plugin_prompts.is_empty()
        {
            let builder = self.prompt_builder.take().unwrap_or_default();

            // Configure builder with tools and workspace
            let builder = builder.with_tools(&self.tools);
            let builder = if let Some(ref path) = self.config.workspace_path {
                builder.with_workspace(path)
            } else {
                builder
            };

            // Add bootstrap context if present
            let builder = if let Some(context) = self.bootstrap_context.take() {
                builder.with_bootstrap(context)
            } else {
                builder
            };

            // Add plugin prompt fragments
            let builder = if !self.plugin_prompts.is_empty() {
                builder.with_plugin_prompts(self.plugin_prompts)
            } else {
                builder
            };

            Some(builder)
        } else {
            None
        };

        let mut agent = Agent::new(backend, self.tools, self.config);
        agent.prompt_builder = prompt_builder;
        agent.interaction_logger = self.interaction_logger;
        agent.memory_store = self.memory_store;
        agent.embedder = self.embedder;
        agent.recall_config = self.recall_config;
        agent.hook_dispatcher = self.hook_dispatcher;
        agent.fs_gate_resolver = self.fs_gate_resolver;
        agent.secret_resolver = self.secret_resolver;
        Ok(agent)
    }

    /// Set the filesystem gate resolver for workstream sandbox enforcement.
    pub fn with_fs_gate_resolver(mut self, resolver: FsGateResolver) -> Self {
        self.fs_gate_resolver = Some(resolver);
        self
    }

    /// Set the secret resolver for `${{secrets.*}}` handle resolution in tool params.
    pub fn with_secret_resolver(mut self, resolver: SharedSecretResolver) -> Self {
        self.secret_resolver = Some(resolver);
        self
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

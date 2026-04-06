use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use arawn_core::{Message, Session, ToolUse};
use arawn_llm::{ChatChunk, ChatContent, ChatMessage, ChatRequest, LlmClient, ToolCall};

use crate::background::BackgroundTaskManager;
use crate::compactor::Compactor;
use crate::context::ToolContext;
use crate::error::EngineError;
use crate::hooks::{HookInput, HookRunner};
use crate::permissions::{PermissionChecker, PermissionDecision};
use crate::plan::PlanModeState;
use crate::token_estimator::{ModelLimits, TokenEstimator};
use crate::tool::ToolRegistry;

const DEFAULT_MAX_ITERATIONS: usize = 200;
const MAX_COMPACT_FAILURES: u32 = 3;

/// Live progress events emitted during the engine loop.
/// The service layer can map these to EngineEvent/WebSocket messages.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Assistant produced text (narration) alongside tool calls.
    AssistantText {
        content: String,
    },
    /// A tool call is about to execute.
    ToolCallStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool call completed.
    ToolCallResult {
        id: String,
        content: String,
        is_error: bool,
    },
}
const DEFAULT_SYSTEM_PROMPT: &str = "You are Arawn, a helpful assistant. When you need to perform actions, use the available tools. Think step by step.";

/// Cached context for building system prompts per-turn.
pub struct PromptContext {
    pub prompts_dir: Option<std::path::PathBuf>,
    pub os: String,
    pub shell: String,
    pub cwd: std::path::PathBuf,
    pub workstream_name: String,
    pub workstream_root: std::path::PathBuf,
    pub context_files: Vec<crate::system_prompt::ContextFile>,
    pub memories: Vec<String>,
    pub session_context: String,
    pub plugin_prompts: Vec<String>,
}

/// Configuration for the query engine.
pub struct QueryEngineConfig {
    pub model: String,
    pub max_iterations: usize,
    /// Fallback system prompt if prompt_context is None.
    pub system_prompt: String,
    pub max_tokens: Option<u32>,
    pub model_limits: ModelLimits,
    /// Data directory for persisting large tool results. None = no persistence, just truncate.
    pub data_dir: Option<std::path::PathBuf>,
    /// Per-turn prompt building context. If set, system_prompt is ignored.
    pub prompt_context: Option<PromptContext>,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            max_tokens: None,
            model_limits: ModelLimits::default(),
            data_dir: None,
            prompt_context: None,
        }
    }
}

/// The agentic loop: prompt → LLM → tool_use → execute → feed result → loop.
pub struct QueryEngine {
    llm: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    config: QueryEngineConfig,
    compactor: Option<Compactor>,
    permission_checker: Option<Arc<PermissionChecker>>,
    hook_runner: Option<Arc<HookRunner>>,
    skill_registry: Option<Arc<crate::skills::SkillRegistry>>,
    plugin_registry: Option<Arc<crate::plugins::PluginRegistry>>,
    plan_state: Option<Arc<PlanModeState>>,
    background_tasks: Option<Arc<BackgroundTaskManager>>,
    /// Consecutive compaction failures. After MAX_COMPACT_FAILURES, compaction
    /// is skipped for the rest of the session to avoid wasting tokens.
    compact_failures: u32,
    /// Track recent failed tool calls (tool_name + args hash → failure count).
    /// Used to detect and short-circuit repeated identical failing calls.
    failed_call_counts: std::collections::HashMap<String, u32>,
    /// Optional channel for live progress events (tool starts/results during the loop).
    progress_tx: Option<tokio::sync::mpsc::Sender<ProgressEvent>>,
}

impl QueryEngine {
    pub fn new(llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>) -> Self {
        Self {
            llm,
            registry,
            config: QueryEngineConfig::default(),
            compactor: None,
            permission_checker: None,
            hook_runner: None,
            skill_registry: None,
            plugin_registry: None,
            plan_state: None,
            background_tasks: None,
            compact_failures: 0,
            failed_call_counts: std::collections::HashMap::new(),
            progress_tx: None,
        }
    }

    pub fn with_config(
        llm: Arc<dyn LlmClient>,
        registry: Arc<ToolRegistry>,
        config: QueryEngineConfig,
    ) -> Self {
        Self {
            llm,
            registry,
            config,
            compactor: None,
            permission_checker: None,
            hook_runner: None,
            skill_registry: None,
            plugin_registry: None,
            plan_state: None,
            background_tasks: None,
            compact_failures: 0,
            failed_call_counts: std::collections::HashMap::new(),
            progress_tx: None,
        }
    }

    pub fn with_compactor(mut self, compactor: Compactor) -> Self {
        self.compactor = Some(compactor);
        self
    }

    pub fn with_permission_checker(mut self, checker: Arc<PermissionChecker>) -> Self {
        self.permission_checker = Some(checker);
        self
    }

    pub fn with_hook_runner(mut self, runner: Arc<HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }

    pub fn with_skill_registry(mut self, registry: Arc<crate::skills::SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    pub fn with_plugin_registry(mut self, registry: Arc<crate::plugins::PluginRegistry>) -> Self {
        self.plugin_registry = Some(registry);
        self
    }

    pub fn with_plan_state(mut self, plan_state: Arc<PlanModeState>) -> Self {
        self.plan_state = Some(plan_state);
        self
    }

    /// Get the plan mode state (if configured).
    pub fn plan_state(&self) -> Option<&Arc<PlanModeState>> {
        self.plan_state.as_ref()
    }

    pub fn with_background_tasks(mut self, manager: Arc<BackgroundTaskManager>) -> Self {
        self.background_tasks = Some(manager);
        self
    }

    /// Set a channel for live progress events during the engine loop.
    pub fn with_progress_sender(mut self, tx: tokio::sync::mpsc::Sender<ProgressEvent>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    /// Emit a progress event if a sender is configured.
    fn emit_progress(&self, event: ProgressEvent) {
        if let Some(ref tx) = self.progress_tx {
            let _ = tx.try_send(event);
        }
    }

    /// Fire a hook event. Convenience method for callers that need to trigger
    /// non-tool hooks (SessionStart, SessionEnd, UserPromptSubmit, etc.).
    ///
    /// Returns the aggregated result. For non-blocking events, the result is
    /// typically ignored by the caller.
    pub async fn fire_hook(&self, input: &HookInput) -> Option<crate::hooks::AggregatedHookResult> {
        if let Some(ref runner) = self.hook_runner {
            Some(runner.run(input).await)
        } else {
            None
        }
    }

    /// Run the agentic loop for a session. Returns the final text response.
    pub async fn run(
        &mut self,
        session: &mut Session,
        ctx: &ToolContext,
    ) -> Result<String, EngineError> {
        let mut iteration = 0;
        loop {
            if self.config.max_iterations > 0 && iteration >= self.config.max_iterations {
                return Err(EngineError::MaxIterations(iteration));
            }
            iteration += 1;
            debug!(iteration, "query engine turn");

            // Drain background task notifications and inject into conversation
            if let Some(ref bg_manager) = self.background_tasks {
                let notifications = bg_manager.drain_notifications();
                for notif in notifications {
                    info!(task_id = %notif.task_id, status = %notif.status, "injecting background task notification");
                    session.add_message(Message::User {
                        content: notif.to_message(),
                    });
                }
            }

            // Microcompact: clear old tool results to save context space (no LLM call)
            let chars_cleared = session.microcompact(6); // keep last 6 messages verbatim
            if chars_cleared > 0 {
                debug!(chars_cleared, "microcompact cleared old tool results");
            }

            // Check if compaction is needed before building the request
            if let Some(ref compactor) = self.compactor {
                // Circuit breaker: skip compaction after too many consecutive failures
                if self.compact_failures >= MAX_COMPACT_FAILURES {
                    debug!(
                        failures = self.compact_failures,
                        "compaction circuit breaker open — skipping"
                    );
                } else {
                let tool_tokens = TokenEstimator::estimate_tools(&self.registry.tool_definitions());
                let system_tokens =
                    TokenEstimator::estimate_system_prompt(&self.config.system_prompt);

                if compactor.should_compact(
                    session,
                    &self.config.model_limits,
                    tool_tokens,
                    system_tokens,
                ) {
                    info!("compacting session (token threshold exceeded)");

                    // PreCompact hook
                    if let Some(ref runner) = self.hook_runner {
                        let hook_input = HookInput::PreCompact {
                            reason: "token_threshold".into(),
                            message_count: session.messages().len(),
                        };
                        let _ = runner.run(&hook_input).await;
                    }

                    let messages_before = session.messages().len();
                    if let Err(e) = compactor.compact(session, &self.config.model_limits).await {
                        self.compact_failures += 1;
                        warn!(
                            error = %e,
                            failures = self.compact_failures,
                            max = MAX_COMPACT_FAILURES,
                            "compaction failed, continuing with full history"
                        );
                    } else {
                        // Success — reset circuit breaker
                        if self.compact_failures > 0 {
                            info!(
                                previous_failures = self.compact_failures,
                                "compaction succeeded, resetting circuit breaker"
                            );
                            self.compact_failures = 0;
                        }
                        // PostCompact hook
                        if let Some(ref runner) = self.hook_runner {
                            let hook_input = HookInput::PostCompact {
                                messages_before,
                                messages_after: session.messages().len(),
                                tokens_before: 0, // estimation not easily available here
                                tokens_after: 0,
                            };
                            let _ = runner.run(&hook_input).await;
                        }
                    }
                }
                } // close circuit breaker else
            }

            // Stream LLM response with retry on transient API errors
            let response = self.stream_response_with_retry(session, ctx).await?;

            // Accumulate token usage
            if let Some(ref usage) = response.usage {
                session.stats.record_turn(
                    usage.input_tokens,
                    usage.output_tokens,
                    response.tool_calls.len() as u32,
                );
            }

            // If no tool calls, we're done
            if response.tool_calls.is_empty() {
                let text = response.text.clone();
                session.add_message(Message::Assistant {
                    content: text.clone(),
                    tool_uses: vec![],
                });

                // Stop hook — model produced final response
                if let Some(ref runner) = self.hook_runner {
                    let hook_input = HookInput::Stop {
                        stop_reason: "end_turn".into(),
                    };
                    let _ = runner.run(&hook_input).await;
                }

                return Ok(text);
            }

            // Validate tool calls — reject any that reference unregistered tools
            // (e.g., hallucinated names like "file_write<|channel|>commentary").
            // Invalid calls get an immediate error result without hitting the API.
            let mut valid_tool_calls = Vec::new();
            let mut invalid_results: Vec<(usize, ToolResult)> = Vec::new();

            for (i, tc) in response.tool_calls.iter().enumerate() {
                // Check tool name is registered
                if self.registry.get(&tc.name).is_none() {
                    warn!(name = %tc.name, "LLM requested unregistered tool — rejecting");
                    invalid_results.push((i, ToolResult {
                        content: format!(
                            "Tool '{}' is not available. Use one of the registered tools.",
                            tc.name
                        ),
                        is_error: true,
                    }));
                    continue;
                }
                // Check arguments are a valid JSON object
                if !tc.arguments.is_object() {
                    warn!(name = %tc.name, args = %tc.arguments, "tool call arguments are not a JSON object");
                    invalid_results.push((i, ToolResult {
                        content: format!(
                            "Invalid arguments for tool '{}': expected a JSON object, got {}",
                            tc.name, tc.arguments
                        ),
                        is_error: true,
                    }));
                    continue;
                }
                // Check for repeated failing calls with identical arguments.
                // We use a compact key of tool name + sorted args to detect duplicates.
                let call_key = format!("{}:{}", tc.name, tc.arguments);
                if let Some(&count) = self.failed_call_counts.get(&call_key) {
                    if count >= 2 {
                        warn!(name = %tc.name, failures = count, "blocking repeated failing tool call");
                        invalid_results.push((i, ToolResult {
                            content: format!(
                                "This exact call to '{}' has already failed {} times with the same arguments. \
                                 Try a different approach, different arguments, or tell the user what went wrong.",
                                tc.name, count
                            ),
                            is_error: true,
                        }));
                        continue;
                    }
                }
                valid_tool_calls.push(i);
            }

            // Append assistant message with tool uses (include all, even invalid)
            let tool_uses: Vec<ToolUse> = response
                .tool_calls
                .iter()
                .map(|tc| ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                })
                .collect();

            session.add_message(Message::Assistant {
                content: response.text.clone(),
                tool_uses,
            });

            // Emit assistant narration text (if any) before tool call events
            if !response.text.is_empty() {
                self.emit_progress(ProgressEvent::AssistantText {
                    content: response.text.clone(),
                });
            }

            // Emit progress events for all valid tool calls
            for &i in &valid_tool_calls {
                let tc = &response.tool_calls[i];
                self.emit_progress(ProgressEvent::ToolCallStart {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                });
            }

            // Execute valid tools — parallelize read-only, serialize writes
            let mut read_only_indices = Vec::new();
            let mut write_indices = Vec::new();

            for &i in &valid_tool_calls {
                let tc = &response.tool_calls[i];
                let is_ro = self
                    .registry
                    .get(&tc.name)
                    .is_some_and(|t| t.is_read_only());
                if is_ro {
                    read_only_indices.push(i);
                } else {
                    write_indices.push(i);
                }
            }

            // Pre-allocate results in original order
            let mut results: Vec<Option<ToolResult>> =
                (0..response.tool_calls.len()).map(|_| None).collect();

            // Fill in results for invalid tool calls
            for (i, result) in invalid_results {
                results[i] = Some(result);
            }

            // Execute read-only tools concurrently
            if !read_only_indices.is_empty() {
                let read_futures: Vec<_> = read_only_indices
                    .iter()
                    .map(|&i| {
                        let tc = &response.tool_calls[i];
                        self.execute_tool(ctx, &tc.id, &tc.name, &tc.arguments)
                    })
                    .collect();

                let read_results = futures::future::join_all(read_futures).await;
                for (slot, result) in read_only_indices.iter().zip(read_results) {
                    results[*slot] = Some(result);
                }
            }

            // Execute write tools serially
            for &i in &write_indices {
                let tc = &response.tool_calls[i];
                let result = self
                    .execute_tool(ctx, &tc.id, &tc.name, &tc.arguments)
                    .await;
                results[i] = Some(result);
            }

            // Append results in original order
            for (i, tc) in response.tool_calls.iter().enumerate() {
                let tool_result = results[i].take().unwrap();

                let limited = if let Some(ref data_dir) = self.config.data_dir {
                    crate::tool_result_limiter::limit_tool_result(
                        crate::tool::ToolOutput {
                            content: tool_result.content,
                            is_error: tool_result.is_error,
                        },
                        ctx.session_id,
                        data_dir,
                        crate::tool_result_limiter::DEFAULT_MAX_RESULT_SIZE_CHARS,
                    )
                    .await
                } else {
                    crate::tool::ToolOutput {
                        content: tool_result.content,
                        is_error: tool_result.is_error,
                    }
                };

                // Track failed calls for duplicate detection
                let call_key = format!("{}:{}", tc.name, tc.arguments);
                if limited.is_error {
                    *self.failed_call_counts.entry(call_key).or_insert(0) += 1;
                } else {
                    // Success clears the failure count for this call
                    self.failed_call_counts.remove(&call_key);
                }

                self.emit_progress(ProgressEvent::ToolCallResult {
                    id: tc.id.clone(),
                    content: limited.content.clone(),
                    is_error: limited.is_error,
                });

                session.add_message(Message::ToolResult {
                    tool_use_id: tc.id.clone(),
                    content: limited.content,
                    is_error: limited.is_error,
                });
            }

            // Loop — send updated history back to LLM
        }
    }

    fn build_request(&self, session: &Session) -> ChatRequest {
        let messages = session
            .messages()
            .iter()
            .map(|msg| match msg {
                Message::User { content } => ChatMessage {
                    role: "user".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                },
                Message::Assistant { content, tool_uses } => ChatMessage {
                    role: "assistant".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: tool_uses
                        .iter()
                        .map(|tu| ToolCall {
                            id: tu.id.clone(),
                            name: tu.name.clone(),
                            arguments: tu.input.clone(),
                        })
                        .collect(),
                    tool_call_id: None,
                },
                Message::ToolResult {
                    tool_use_id,
                    content,
                    ..
                } => ChatMessage {
                    role: "tool".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: vec![],
                    tool_call_id: Some(tool_use_id.clone()),
                },
                Message::Summary { content, .. } => ChatMessage {
                    role: "user".into(),
                    content: ChatContent::Text(content.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                },
            })
            .collect();

        // Query registry fresh each turn, then filter to contextually relevant tools.
        // Core tools always included; specialty tools included when conversation signals need them.
        let all_tools = self.registry.tool_definitions();
        let tools = filter_tools_for_context(&all_tools, session);

        // Build system prompt fresh each turn (tools/skills may have changed via hot-reload)
        let system_prompt = if let Some(ref prompt_ctx) = self.config.prompt_context {
            // Build dynamic plugin prompts: start with static ones, add skill listing
            let mut dynamic_prompts = prompt_ctx.plugin_prompts.clone();

            // Add skill listing
            if let Some(ref skill_reg) = self.skill_registry {
                let skills = skill_reg.user_invocable();
                if !skills.is_empty() {
                    let listing = crate::skills::format_skill_listing(&skills, 4000, 250);
                    if !listing.is_empty() {
                        dynamic_prompts.push(listing);
                    }
                }
            }

            crate::system_prompt::SystemPromptBuilder::new()
                .load_static_sections(prompt_ctx.prompts_dir.as_deref())
                .environment(
                    &prompt_ctx.os,
                    &prompt_ctx.shell,
                    &prompt_ctx.cwd,
                    &self.config.model,
                )
                .workstream(&prompt_ctx.workstream_name, &prompt_ctx.workstream_root)
                .tools(&tools)
                .context_files(&prompt_ctx.context_files)
                .memories(&prompt_ctx.memories)
                .session_context(&prompt_ctx.session_context)
                .plugin_prompts(&dynamic_prompts)
                .build()
        } else {
            self.config.system_prompt.clone()
        };

        ChatRequest {
            model: self.config.model.clone(),
            system_prompt: Some(system_prompt),
            messages,
            tools,
            max_tokens: self.config.max_tokens,
        }
    }

    /// Build the request and stream with up to 2 retries on transient LLM errors
    /// (rate limits, parse failures, network issues). Rebuilds the request on each
    /// retry since tool definitions and session state haven't changed.
    async fn stream_response_with_retry(
        &self,
        session: &Session,
        _ctx: &ToolContext,
    ) -> Result<AssembledResponse, EngineError> {
        const MAX_RETRIES: u32 = 2;

        for attempt in 0..=MAX_RETRIES {
            let request = self.build_request(session);
            match self.stream_response(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    let is_transient = err_str.contains("rate")
                        || err_str.contains("timeout")
                        || err_str.contains("tool_use_failed")
                        || err_str.contains("failed to parse")
                        || err_str.contains("connection")
                        || err_str.contains("500")
                        || err_str.contains("502")
                        || err_str.contains("503")
                        || err_str.contains("529");

                    if !is_transient || attempt == MAX_RETRIES {
                        return Err(e);
                    }

                    let backoff_ms = (attempt + 1) * 500;
                    warn!(
                        attempt,
                        backoff_ms,
                        error = %e,
                        "transient LLM error, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms as u64)).await;
                }
            }
        }
        unreachable!()
    }

    async fn stream_response(
        &self,
        request: ChatRequest,
    ) -> Result<AssembledResponse, EngineError> {
        let mut stream = self.llm.stream(request).await?;
        let mut response = AssembledResponse::default();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(ChatChunk::TextDelta { text }) => {
                    response.text.push_str(&text);
                }
                Ok(ChatChunk::ToolUseStart { id, name }) => {
                    // Flush any previous tool call
                    if !current_tool_name.is_empty() {
                        response.tool_calls.push(AssembledToolCall {
                            id: current_tool_id.clone(),
                            name: current_tool_name.clone(),
                            arguments: parse_arguments(&current_tool_args),
                        });
                    }
                    current_tool_id = id;
                    current_tool_name = name;
                    current_tool_args.clear();
                }
                Ok(ChatChunk::ToolUseInputDelta { json }) => {
                    current_tool_args.push_str(&json);
                }
                Ok(ChatChunk::Done { usage }) => {
                    response.usage = usage;
                    // Flush any pending tool call
                    if !current_tool_name.is_empty() {
                        response.tool_calls.push(AssembledToolCall {
                            id: current_tool_id.clone(),
                            name: current_tool_name.clone(),
                            arguments: parse_arguments(&current_tool_args),
                        });
                        current_tool_name.clear();
                    }
                }
                Err(e) => {
                    warn!("stream error: {e}");
                    return Err(EngineError::Llm(e));
                }
            }
        }

        // Flush if stream ended without Done
        if !current_tool_name.is_empty() {
            response.tool_calls.push(AssembledToolCall {
                id: current_tool_id,
                name: current_tool_name,
                arguments: parse_arguments(&current_tool_args),
            });
        }

        Ok(response)
    }

    async fn execute_tool(
        &self,
        ctx: &ToolContext,
        tool_use_id: &str,
        name: &str,
        arguments: &serde_json::Value,
    ) -> ToolResult {
        debug!(name, tool_use_id, %arguments, "executing tool");

        // Plan mode enforcement — check before permission rules
        if let Some(ref plan_state) = self.plan_state {
            if plan_state.is_active() {
                // Allow plan mode meta-tools and side-effect-free tools
                let tool_is_allowed = name == "EnterPlanMode"
                    || name == "ExitPlanMode"
                    || self
                        .registry
                        .get(name)
                        .is_some_and(|t| t.is_read_only());

                if !tool_is_allowed {
                    warn!(name, "tool blocked by plan mode");
                    return ToolResult {
                        content: format!(
                            "Plan mode is active — only observation tools are allowed. \
                             Tool '{name}' has side effects and cannot be used until the plan \
                             is approved. Call ExitPlanMode to present your plan for review."
                        ),
                        is_error: true,
                    };
                }
            }
        }

        // Permission check — if a checker is configured, verify the tool call is allowed
        if let Some(ref checker) = self.permission_checker {
            let input_summary = arguments.to_string();
            let decision = checker.check(name, &input_summary).await;
            if decision == PermissionDecision::Denied {
                warn!(name, "tool blocked by permission system");
                return ToolResult {
                    content: format!(
                        "Permission denied: tool '{name}' is not allowed by your permission settings."
                    ),
                    is_error: true,
                };
            }
        }

        // PreToolUse hooks — run before tool execution, can block
        if let Some(ref runner) = self.hook_runner {
            let hook_input = HookInput::PreToolUse {
                tool_name: name.to_string(),
                tool_input: arguments.clone(),
            };
            let result = runner.run(&hook_input).await;
            if result.blocked {
                let reason = result
                    .block_reason
                    .unwrap_or_else(|| "Blocked by hook".to_string());
                warn!(name, %reason, "tool blocked by PreToolUse hook");
                return ToolResult {
                    content: format!("Hook blocked tool '{name}': {reason}"),
                    is_error: true,
                };
            }
        }

        let tool = match self.registry.get(name) {
            Some(t) => t,
            None => {
                warn!(name, "tool not found");
                return ToolResult {
                    content: format!("Tool '{name}' not found"),
                    is_error: true,
                };
            }
        };

        let tool_result = match tool.execute(ctx, arguments.clone()).await {
            Ok(output) => {
                debug!(name, is_error = output.is_error, "tool completed");

                // PostToolUse hooks — informational, runs after successful execution
                if let Some(ref runner) = self.hook_runner {
                    let hook_input = HookInput::PostToolUse {
                        tool_name: name.to_string(),
                        tool_input: arguments.clone(),
                        tool_output: output.content.clone(),
                    };
                    let _ = runner.run(&hook_input).await;
                }

                ToolResult {
                    content: output.content,
                    is_error: output.is_error,
                }
            }
            Err(e) => {
                warn!(name, error = %e, "tool execution failed");

                // PostToolUseFailure hooks — informational, runs on error
                if let Some(ref runner) = self.hook_runner {
                    let hook_input = HookInput::PostToolUseFailure {
                        tool_name: name.to_string(),
                        tool_input: arguments.clone(),
                        error: e.to_string(),
                    };
                    let _ = runner.run(&hook_input).await;
                }

                ToolResult {
                    content: format!("Tool execution error: {e}"),
                    is_error: true,
                }
            }
        };

        tool_result
    }
}

fn parse_arguments(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(raw).unwrap_or(serde_json::json!({}))
}

#[derive(Default)]
struct AssembledResponse {
    text: String,
    tool_calls: Vec<AssembledToolCall>,
    usage: Option<arawn_llm::Usage>,
}

struct AssembledToolCall {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

struct ToolResult {
    content: String,
    is_error: bool,
}

/// Core tools always included in every LLM request.
const CORE_TOOLS: &[&str] = &[
    "think", "shell", "file_read", "file_write", "file_edit", "glob", "grep", "Skill",
];

/// Web tools — included when conversation references URLs, web, search, fetch, APIs.
const WEB_TOOLS: &[&str] = &["web_fetch", "web_search"];

/// Planning tools — included when in plan mode or conversation mentions planning.
const PLAN_TOOLS: &[&str] = &["EnterPlanMode", "ExitPlanMode"];

/// Task management tools — included when conversation mentions tasks, background, todo.
const TASK_TOOLS: &[&str] = &[
    "TaskCreate", "TaskUpdate", "TaskGet", "TaskList", "TaskOutput", "TaskStop",
];

/// Memory tools — included when conversation mentions memory, remember, recall.
const MEMORY_TOOLS: &[&str] = &["memory_store", "memory_search"];

/// Agent/delegation tools — included when conversation mentions delegation, agent, subagent.
const AGENT_TOOLS: &[&str] = &["Agent"];

/// Other tools always included.
const ALWAYS_TOOLS: &[&str] = &["ask_user", "sleep"];

/// Filter tool definitions to only contextually relevant ones for this turn.
/// Scans the last user message for category signals. Core tools always included.
fn filter_tools_for_context(
    all_tools: &[arawn_llm::ToolDefinition],
    session: &Session,
) -> Vec<arawn_llm::ToolDefinition> {
    // On first turn or very short sessions, send all tools (no context to filter on)
    if session.messages().len() <= 2 {
        return all_tools.to_vec();
    }

    // Extract the last user message for keyword scanning
    let last_user_msg = session
        .messages()
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::User { content } => Some(content.to_lowercase()),
            _ => None,
        })
        .unwrap_or_default();

    // Also check if the model previously used any specialty tools (keep them available)
    let used_tool_names: std::collections::HashSet<&str> = session
        .messages()
        .iter()
        .filter_map(|m| match m {
            Message::Assistant { tool_uses, .. } => {
                Some(tool_uses.iter().map(|tu| tu.name.as_str()))
            }
            _ => None,
        })
        .flatten()
        .collect();

    let mut include = std::collections::HashSet::new();

    // Always include core + always tools
    for name in CORE_TOOLS.iter().chain(ALWAYS_TOOLS.iter()) {
        include.insert(*name);
    }

    // Web tools: URL patterns, web/search/fetch/http/api/github mentions
    if last_user_msg.contains("http")
        || last_user_msg.contains("url")
        || last_user_msg.contains("web")
        || last_user_msg.contains("search")
        || last_user_msg.contains("fetch")
        || last_user_msg.contains("api")
        || last_user_msg.contains("github")
        || last_user_msg.contains("google")
        || used_tool_names.iter().any(|n| WEB_TOOLS.contains(n))
    {
        for name in WEB_TOOLS {
            include.insert(name);
        }
    }

    // Plan tools: plan/planning mentions or plan mode active
    if last_user_msg.contains("plan")
        || used_tool_names.iter().any(|n| PLAN_TOOLS.contains(n))
    {
        for name in PLAN_TOOLS {
            include.insert(name);
        }
    }

    // Task tools: task/todo/background mentions
    if last_user_msg.contains("task")
        || last_user_msg.contains("todo")
        || last_user_msg.contains("background")
        || used_tool_names.iter().any(|n| TASK_TOOLS.contains(n))
    {
        for name in TASK_TOOLS {
            include.insert(name);
        }
    }

    // Memory tools: remember/recall/memory mentions
    if last_user_msg.contains("remember")
        || last_user_msg.contains("recall")
        || last_user_msg.contains("memory")
        || last_user_msg.contains("forget")
        || used_tool_names.iter().any(|n| MEMORY_TOOLS.contains(n))
    {
        for name in MEMORY_TOOLS {
            include.insert(name);
        }
    }

    // Agent tools: agent/delegate/subagent mentions
    if last_user_msg.contains("agent")
        || last_user_msg.contains("delegat")
        || used_tool_names.iter().any(|n| AGENT_TOOLS.contains(n))
    {
        for name in AGENT_TOOLS {
            include.insert(name);
        }
    }

    // Always include any tool the model has already used in this session
    for name in &used_tool_names {
        include.insert(name);
    }

    all_tools
        .iter()
        .filter(|t| include.contains(t.name.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ThinkTool;
    use arawn_core::Workstream;
    use arawn_llm::LlmError;
    use async_trait::async_trait;
    use futures::stream;
    use std::pin::Pin;
    use std::sync::Mutex;

    /// Mock LLM that returns pre-scripted responses.
    struct MockLlm {
        responses: Mutex<Vec<Vec<ChatChunk>>>,
    }

    impl MockLlm {
        fn new(responses: Vec<Vec<ChatChunk>>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }

        /// Convenience: text-only response
        fn text(text: &str) -> Vec<ChatChunk> {
            vec![
                ChatChunk::TextDelta {
                    text: text.to_string(),
                },
                ChatChunk::Done { usage: None },
            ]
        }

        /// Convenience: tool call then done
        fn tool_call(id: &str, name: &str, args: &str) -> Vec<ChatChunk> {
            vec![
                ChatChunk::ToolUseStart {
                    id: id.to_string(),
                    name: name.to_string(),
                },
                ChatChunk::ToolUseInputDelta {
                    json: args.to_string(),
                },
                ChatChunk::Done { usage: None },
            ]
        }
    }

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn stream(
            &self,
            _request: ChatRequest,
        ) -> Result<
            Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
            LlmError,
        > {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                panic!("MockLlm: no more scripted responses");
            }
            let chunks = responses.remove(0);
            let stream = stream::iter(chunks.into_iter().map(Ok));
            Ok(Box::pin(stream))
        }
    }

    fn setup() -> (Workstream, Session, ToolContext) {
        let ws = Workstream::scratch("/tmp/test-engine");
        let session = Session::new(ws.id);
        let ctx = ToolContext::new(&ws, session.id);
        (ws, session, ctx)
    }

    #[tokio::test]
    async fn text_only_response() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "Hello".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![MockLlm::text("Hi there!")]));
        let registry = Arc::new(ToolRegistry::new());
        let mut engine = QueryEngine::new(llm, registry);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "Hi there!");
        assert_eq!(session.messages().len(), 2); // user + assistant
    }

    #[tokio::test]
    async fn single_tool_call() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "Think about this".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "think", r#"{"thought":"analyzing..."}"#),
            MockLlm::text("Done thinking."),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(ThinkTool));
        let mut engine = QueryEngine::new(llm, registry);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "Done thinking.");
        // user + assistant(tool_use) + tool_result + assistant(text)
        assert_eq!(session.messages().len(), 4);
    }

    #[tokio::test]
    async fn tool_not_found() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "Use nonexistent tool".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "nonexistent", "{}"),
            MockLlm::text("I see the tool failed."),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        let mut engine = QueryEngine::new(llm, registry);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "I see the tool failed.");

        // Check the tool_result was an error
        let msgs = session.messages();
        match &msgs[2] {
            Message::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!("expected ToolResult"),
        }
    }

    #[tokio::test]
    async fn max_iterations_exceeded() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "Loop forever".into(),
        });

        // Always return a tool call — will never terminate naturally
        let responses: Vec<Vec<ChatChunk>> = (0..5)
            .map(|i| MockLlm::tool_call(&format!("call_{i}"), "think", r#"{"thought":"loop"}"#))
            .collect();

        let llm = Arc::new(MockLlm::new(responses));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(ThinkTool));

        let config = QueryEngineConfig {
            max_iterations: 3,
            system_prompt: "test".into(),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);

        let result = engine.run(&mut session, &ctx).await;
        match result {
            Err(EngineError::MaxIterations(3)) => {} // expected
            other => panic!("expected MaxIterations(3), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multi_turn_tool_chain() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "Two tools".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "think", r#"{"thought":"step 1"}"#),
            MockLlm::tool_call("call_2", "think", r#"{"thought":"step 2"}"#),
            MockLlm::text("All done."),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(ThinkTool));
        let mut engine = QueryEngine::new(llm, registry);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "All done.");
        // user + (assistant+tool_result)*2 + final assistant = 6
        assert_eq!(session.messages().len(), 6);
    }

}

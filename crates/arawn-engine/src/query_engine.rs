use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use arawn_core::{Message, Session, ToolUse};
use arawn_llm::{ChatChunk, ChatContent, ChatMessage, ChatRequest, LlmClient, ToolCall};

use crate::background::BackgroundTaskManager;
use crate::compactor::Compactor;
use crate::error::EngineError;
use crate::hooks::{HookInput, HookRunner};
use crate::permissions::{PermissionChecker, PermissionDecision};
use crate::plan::PlanModeState;
use crate::token_estimator::{ModelLimits, TokenEstimator};
use crate::tool_timeout;
use arawn_tool::ToolRegistry;

const DEFAULT_MAX_ITERATIONS: usize = 200;

/// Default no-progress breaker threshold (ARAWN-T-0475): bail after this many
/// consecutive iterations whose tool calls ALL errored. Conservative — a
/// healthy turn resets the streak on the first successful tool result, so a
/// model would have to fail this many rounds in a row to trip it. Set to 5
/// (not 2–3): legitimate recovery routinely does a few consecutive failures
/// then pivots — e.g. a model tries a denied tool three times before
/// switching to one that works — and the breaker must not cut that off. Still
/// ~40× tighter than the `max_iterations` backstop.
pub(crate) const DEFAULT_MAX_NO_PROGRESS_ITERATIONS: usize = 5;
const MAX_COMPACT_FAILURES: u32 = 3;
/// Fallback recent-window for microcompaction when no full compactor is
/// configured to borrow `keep_recent` from.
const DEFAULT_MICROCOMPACT_KEEP_RECENT: usize = 6;

/// Live progress events emitted during the engine loop.
/// The service layer can map these to EngineEvent/WebSocket messages.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Assistant produced text (narration) alongside tool calls.
    AssistantText { content: String },
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
    /// An out-of-band engine notice the user should see (e.g. automatic
    /// context compaction was disabled). The service maps this to a
    /// user-visible warning.
    Notice { message: String },
}
const DEFAULT_SYSTEM_PROMPT: &str = "You are Arawn, a helpful assistant. When you need to perform actions, use the available tools. Think step by step.";

/// Provider for dynamic integration capability summaries.
///
/// Returning a closure rather than a static `Vec<String>` is what makes
/// the system prompt update live: the engine queries this each turn, so
/// `/connect slack` flowing into the registry shows up in the next LLM
/// call without any restart.
///
/// Each call should be cheap and synchronous from the prompt-builder's
/// POV — implementations that need an async query should pre-cache.
/// Default behavior (no provider) is no integrations section emitted.
pub type IntegrationCapabilitiesFn = std::sync::Arc<dyn Fn() -> Vec<String> + Send + Sync>;

/// Provider for the set of currently-connected integration service names.
///
/// Returns canonical `Integration::name()` values (lowercase snake_case:
/// `"gmail"`, `"google_calendar"`, `"google_drive"`, `"slack"`,
/// `"atlassian"`, `"github"`). Used by the tool-catalog filter to gate
/// per-service tool categories (I-0055): a Calendar tool is only shown
/// to the model when `"google_calendar"` is in the connected set.
///
/// Distinct from `IntegrationCapabilitiesFn` — that one returns prose
/// summaries for the system prompt; this one returns clean tokens for
/// authoritative filter decisions. Both query the same source of truth
/// (`Integration::is_connected().await` on the registry); the closures
/// are split because the consumers want different shapes.
pub type ConnectedServicesFn = std::sync::Arc<dyn Fn() -> Vec<String> + Send + Sync>;

/// Cached context for building system prompts per-turn.
#[derive(Clone)]
pub struct PromptContext {
    pub prompts_dir: Option<std::path::PathBuf>,
    pub os: String,
    pub shell: String,
    pub cwd: std::path::PathBuf,
    pub lens_name: String,
    pub lens_root: std::path::PathBuf,
    pub context_files: Vec<crate::system_prompt::ContextFile>,
    pub memories: Vec<String>,
    pub session_context: String,
    pub plugin_prompts: Vec<String>,
    /// Persona to load into the static sections (identity / doing_tasks /
    /// work_protocol). Sourced from the active lens's
    /// `identity_profile` at session-build time. Defaults to
    /// [`IdentityProfile::Assistant`].
    pub identity_profile: arawn_core::IdentityProfile,
    /// Optional callback queried each turn for connected-integration
    /// summaries. Lets `/connect <service>` reflect into the next LLM
    /// call with no restart.
    pub integration_capabilities: Option<IntegrationCapabilitiesFn>,
    /// Optional callback queried each turn for connected-integration
    /// service names. Authoritative source for the tool-catalog filter's
    /// per-service gating (I-0055). `None` → filter conservatively drops
    /// every per-service category (no provider == we don't know what's
    /// connected, default to deny).
    pub connected_services: Option<ConnectedServicesFn>,
}

/// Configuration for the query engine.
#[derive(Clone)]
pub struct QueryEngineConfig {
    pub model: String,
    pub max_iterations: usize,
    /// No-progress breaker (ARAWN-T-0475): end the turn after this many
    /// consecutive iterations in which every emitted tool call errored — a
    /// model stuck re-issuing failing/invalid calls (and narrating between
    /// them) instead of converging. 0 disables the breaker (the
    /// `max_iterations` cap remains the ultimate backstop).
    pub max_no_progress_iterations: usize,
    /// Fallback system prompt if prompt_context is None.
    pub system_prompt: String,
    pub max_tokens: Option<u32>,
    pub model_limits: ModelLimits,
    /// Data directory for persisting large tool results. None = no persistence, just truncate.
    pub data_dir: Option<std::path::PathBuf>,
    /// Per-turn prompt building context. If set, system_prompt is ignored.
    pub prompt_context: Option<PromptContext>,
    /// Default wall-clock timeout for tool calls, in seconds. None falls back
    /// to the `ARAWN_TOOL_TIMEOUT_SECS` env var, then to 120s. The agent can
    /// override per-call via the `timeout_secs` argument on any tool.
    pub tool_timeout_secs: Option<u64>,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            max_no_progress_iterations: DEFAULT_MAX_NO_PROGRESS_ITERATIONS,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            max_tokens: None,
            model_limits: ModelLimits::default(),
            data_dir: None,
            prompt_context: None,
            tool_timeout_secs: None,
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
    /// Consecutive iterations whose tool calls ALL errored (ARAWN-T-0475).
    /// Reset to 0 on any successful tool result; trips the no-progress breaker
    /// at `config.max_no_progress_iterations`. The engine is rebuilt per
    /// message, so this is naturally scoped to one turn-loop.
    no_progress_streak: usize,
    /// Optional channel for live progress events (tool starts/results during the loop).
    progress_tx: Option<tokio::sync::mpsc::Sender<ProgressEvent>>,
    /// Optional cancellation token — checked at each iteration and before each tool execution.
    cancel_token: Option<tokio_util::sync::CancellationToken>,
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
            no_progress_streak: 0,
            progress_tx: None,
            cancel_token: None,
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
            no_progress_streak: 0,
            progress_tx: None,
            cancel_token: None,
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

    /// Set a cancellation token — checked at each loop iteration and before tool execution.
    pub fn with_cancel_token(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    /// Check if cancellation has been requested.
    fn is_cancelled(&self) -> bool {
        self.cancel_token.as_ref().is_some_and(|t| t.is_cancelled())
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
        ctx: &dyn arawn_tool::ToolContext,
    ) -> Result<String, EngineError> {
        // UserPromptSubmit hook — fires once at the start of a turn,
        // before any LLM call. A blocking hook short-circuits the turn:
        // we return the hook's block reason as the synthetic assistant
        // response so the user sees why their prompt was rejected.
        if let Some(ref runner) = self.hook_runner {
            // Pull the most recent user message text. Engine API contract:
            // callers add the user message to the session before run().
            let last_user_msg = session
                .messages()
                .iter()
                .rev()
                .find_map(|m| match m {
                    Message::User { content } => Some(content.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let hook_input = HookInput::UserPromptSubmit {
                message: last_user_msg,
            };
            let result = runner.run(&hook_input).await;
            if result.blocked {
                let reason = result
                    .block_reason
                    .unwrap_or_else(|| "Prompt blocked by hook".to_string());
                warn!(%reason, "user prompt blocked by UserPromptSubmit hook");
                session.add_message(Message::Assistant {
                    content: reason.clone(),
                    tool_uses: vec![],
                });
                return Ok(reason);
            }
        }

        let mut iteration = 0;
        loop {
            // Check for cancellation before each iteration
            if self.is_cancelled() {
                info!(iteration, "engine cancelled by user");
                return Err(EngineError::Other(anyhow::anyhow!("Cancelled by user")));
            }

            if self.config.max_iterations > 0 && iteration >= self.config.max_iterations {
                return Err(EngineError::MaxIterations {
                    iterations: iteration,
                    session_id: ctx.session_id(),
                });
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

            // Microcompact: clear old tool results to save context space (no
            // LLM call). Shares the recent-window size with the full compactor
            // so the two stay consistent instead of hardcoding a separate 6.
            let microcompact_keep = self
                .compactor
                .as_ref()
                .map(|c| c.keep_recent())
                .unwrap_or(DEFAULT_MICROCOMPACT_KEEP_RECENT);
            let chars_cleared = session.microcompact(microcompact_keep);
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
                    let tool_tokens =
                        TokenEstimator::estimate_tools(&self.registry.tool_definitions());
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
                        if let Err(e) = compactor.compact(session, &self.config.model_limits).await
                        {
                            self.compact_failures += 1;
                            warn!(
                                error = %e,
                                failures = self.compact_failures,
                                max = MAX_COMPACT_FAILURES,
                                "compaction failed, continuing with full history"
                            );
                            // Surface to the user the moment the breaker trips
                            // (once, on the transition — not every later turn),
                            // so an unexplained context-overflow doesn't appear
                            // 20 turns later with no warning.
                            if self.compact_failures == MAX_COMPACT_FAILURES {
                                self.emit_progress(ProgressEvent::Notice {
                                    message: "Automatic context compaction has failed repeatedly \
                                              and is now paused. This session may hit the model's \
                                              context limit — consider starting a new session if \
                                              responses degrade."
                                        .into(),
                                });
                            }
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
                let mut text = response.text.clone();
                // A turn that stopped on the token limit or a content filter
                // is NOT a clean end — tell the user instead of silently
                // presenting a truncated answer as complete.
                match response.finish_reason {
                    Some(arawn_llm::FinishReason::Length) => {
                        text.push_str(
                            "\n\n_[Response truncated: hit the model's output token limit.]_",
                        );
                    }
                    Some(arawn_llm::FinishReason::ContentFilter) => {
                        text.push_str("\n\n_[Response halted by the provider's content filter.]_");
                    }
                    _ => {}
                }
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
                    invalid_results.push((
                        i,
                        ToolResult {
                            content: format!(
                                "Tool '{}' is not available. Use one of the registered tools.",
                                tc.name
                            ),
                            is_error: true,
                        },
                    ));
                    continue;
                }
                // Check arguments are a valid JSON object
                if !tc.arguments.is_object() {
                    warn!(name = %tc.name, args = %tc.arguments, "tool call arguments are not a JSON object");
                    invalid_results.push((
                        i,
                        ToolResult {
                            content: format!(
                                "Invalid arguments for tool '{}': expected a JSON object, got {}",
                                tc.name, tc.arguments
                            ),
                            is_error: true,
                        },
                    ));
                    continue;
                }
                // Check for repeated failing calls with identical arguments.
                // The key is the tool name + the arguments re-serialized through
                // `serde_json::Value`'s Display, which is canonical: keys are
                // sorted (serde_json's default Map is a BTreeMap — `preserve_order`
                // is off) and incidental whitespace from the raw LLM output is
                // already normalized away by parsing. So `{"a":1,"b":2}` and
                // `{ "b":2, "a":1 }` hash to the same key. `failed_call_counts`
                // lives on the engine, which is rebuilt per message, so the
                // counts are naturally scoped to a single turn-loop.
                let call_key = format!("{}:{}", tc.name, tc.arguments);
                if let Some(&count) = self.failed_call_counts.get(&call_key)
                    && count >= 2
                {
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

            // Check for cancellation before tool execution
            if self.is_cancelled() {
                info!("engine cancelled before tool execution");
                return Err(EngineError::Other(anyhow::anyhow!("Cancelled by user")));
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
            let mut any_tool_success = false;
            for (i, tc) in response.tool_calls.iter().enumerate() {
                let tool_result = results[i].take().unwrap();

                let limited = if let Some(ref data_dir) = self.config.data_dir {
                    crate::tool_result_limiter::limit_tool_result(
                        arawn_tool::ToolOutput {
                            content: tool_result.content,
                            is_error: tool_result.is_error,
                        },
                        ctx.session_id(),
                        data_dir,
                        crate::tool_result_limiter::DEFAULT_MAX_RESULT_SIZE_CHARS,
                    )
                    .await
                } else {
                    arawn_tool::ToolOutput {
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
                    any_tool_success = true;
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

            // ARAWN-T-0475 no-progress breaker. If every tool call this
            // iteration errored, the turn made no forward progress. A model
            // that keeps re-issuing failing/invalid calls (and narrating
            // between them — the gemma "grep" loop) would otherwise burn the
            // whole `max_iterations` budget and hand back garbage. Count
            // consecutive all-errored iterations; once the streak crosses the
            // threshold, end the turn with a surfaced reason instead.
            //
            // This is distinct from `failed_call_counts`, which only catches
            // repeated *identical* failing calls — this catches a
            // varied-but-fruitless loop. A response with no tool calls has
            // already returned above as the final answer, so this only
            // engages when the loop would otherwise iterate again.
            if any_tool_success {
                self.no_progress_streak = 0;
            } else {
                self.no_progress_streak += 1;
                if self.config.max_no_progress_iterations > 0
                    && self.no_progress_streak >= self.config.max_no_progress_iterations
                {
                    let notice = format!(
                        "Stopping: the model made {} consecutive rounds of tool calls that all \
                         failed without making progress, so the turn isn't converging on a usable \
                         answer. Try rephrasing, or check that the right tools/credentials are \
                         available.",
                        self.no_progress_streak
                    );
                    warn!(
                        streak = self.no_progress_streak,
                        threshold = self.config.max_no_progress_iterations,
                        "no-progress breaker tripped — ending turn"
                    );
                    self.emit_progress(ProgressEvent::Notice {
                        message: notice.clone(),
                    });
                    // Hand back the model's last text (if any) plus the reason,
                    // recorded as the final assistant message — never silent
                    // iteration-cap garbage.
                    let final_text = if response.text.trim().is_empty() {
                        notice.clone()
                    } else {
                        format!("{}\n\n_[{}]_", response.text, notice)
                    };
                    session.add_message(Message::Assistant {
                        content: final_text.clone(),
                        tool_uses: vec![],
                    });
                    if let Some(ref runner) = self.hook_runner {
                        let hook_input = HookInput::Stop {
                            stop_reason: "no_progress".into(),
                        };
                        let _ = runner.run(&hook_input).await;
                    }
                    return Ok(final_text);
                }
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
        // Core tools always included; per-service integration tools gated by
        // `connected_services` (authoritative); other specialty tools gated by
        // keyword mentions in the latest user message.
        let all_tools = self.registry.tool_definitions();
        let connected_services: Vec<String> = self
            .config
            .prompt_context
            .as_ref()
            .and_then(|pc| pc.connected_services.as_ref())
            .map(|f| f())
            .unwrap_or_default();
        let tools = filter_tools_for_context(
            &all_tools,
            session,
            &self.registry,
            &connected_services,
            &self.config.model_limits,
        );

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

            // Query the integrations provider fresh each turn so /connect
            // and /disconnect reflect immediately without restart.
            let integrations = prompt_ctx
                .integration_capabilities
                .as_ref()
                .map(|f| f())
                .unwrap_or_default();

            crate::system_prompt::SystemPromptBuilder::new()
                .with_identity_profile(prompt_ctx.identity_profile)
                .current_time(chrono::Local::now())
                .load_static_sections(prompt_ctx.prompts_dir.as_deref())
                .environment(
                    &prompt_ctx.os,
                    &prompt_ctx.shell,
                    &prompt_ctx.cwd,
                    &self.config.model,
                )
                .lens(&prompt_ctx.lens_name, &prompt_ctx.lens_root)
                .tools(&tools)
                .context_files(&prompt_ctx.context_files)
                .memories(&prompt_ctx.memories)
                .session_context(&prompt_ctx.session_context)
                .integrations(&integrations)
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

    /// Retry the request-build-and-stream cycle when the stream fails mid-flight.
    ///
    /// This is a different retry layer from `arawn_llm::RetryClient`:
    /// - `RetryClient` retries the `stream()` *open* call (connect-time
    ///   transient errors, e.g., 429 on initial HTTP request).
    /// - This retry catches errors that surface *after* chunk consumption
    ///   has started (mid-stream network hiccups, provider closing the
    ///   stream with a transient error code). Those are invisible to
    ///   `RetryClient` — by the time it has handed back a stream, its
    ///   retry window is closed.
    ///
    /// Rebuilds the full request on each attempt since session state is
    /// unchanged and the previous stream's partial text is discarded.
    ///
    /// Policy: 2 retries total (3 attempts), exponential backoff with
    /// 500 ms base (500 ms, 1 s) — shorter than `RetryClient`'s policy
    /// because mid-stream is usually a transient hiccup and the caller
    /// is already inside a user-facing turn.
    async fn stream_response_with_retry(
        &self,
        session: &Session,
        _ctx: &dyn arawn_tool::ToolContext,
    ) -> Result<AssembledResponse, EngineError> {
        const MAX_RETRIES: u32 = 2;
        const BASE_DELAY_MS: u64 = 500;

        for attempt in 0..=MAX_RETRIES {
            let request = self.build_request(session);
            match self.stream_response(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let is_transient = match &e {
                        EngineError::Llm(llm_err) => llm_err.is_retryable(),
                        _ => false,
                    };

                    if !is_transient || attempt == MAX_RETRIES {
                        // StopFailure hook — model stream terminally errored
                        // (non-retryable or retry budget exhausted).
                        if let Some(ref runner) = self.hook_runner {
                            let hook_input = HookInput::StopFailure {
                                error: e.to_string(),
                            };
                            let _ = runner.run(&hook_input).await;
                        }
                        return Err(e);
                    }

                    let backoff_ms = BASE_DELAY_MS * 2u64.pow(attempt);
                    warn!(
                        attempt,
                        backoff_ms,
                        error = %e,
                        "mid-stream LLM error, rebuilding request and retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
        unreachable!()
    }

    async fn stream_response(
        &self,
        request: ChatRequest,
    ) -> Result<AssembledResponse, EngineError> {
        // Acquire a local-bound permit before issuing the request.
        // The 1-slot gate exists for laptop-RAM safety — local Ollama
        // is effectively serial and concurrent requests have crashed
        // user machines in practice. Cloud-bound calls still flow
        // through this gate today; the original "differentiate Remote
        // via RemotePermit" plan (T-0278) was reverted because the
        // hybrid-dispatch routing layer it was built for turned out
        // to be premature scaffolding (no real hybrid users).
        let _gate = arawn_llm::gate::acquire_local()
            .await
            .map_err(|e| EngineError::Other(anyhow::anyhow!("llm gate refused acquire: {e:?}")))?;
        let mut stream = self.llm.stream(request).await?;
        let mut response = AssembledResponse::default();
        // Assemble tool calls keyed by their stream `index`, NOT by arrival
        // order. A model emitting parallel tool calls interleaves argument
        // deltas for different indices; the old "current tool" flush model
        // mixed them together. BTreeMap keeps the calls ordered by index.
        let mut partials: BTreeMap<u32, PartialToolCall> = BTreeMap::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(ChatChunk::TextDelta { text }) => {
                    response.text.push_str(&text);
                }
                Ok(ChatChunk::ToolUseStart { index, id, name }) => {
                    let entry = partials.entry(index).or_default();
                    entry.id = id;
                    entry.name = name;
                }
                Ok(ChatChunk::ToolUseInputDelta { index, json }) => {
                    partials.entry(index).or_default().arguments.push_str(&json);
                }
                Ok(ChatChunk::Done {
                    usage,
                    finish_reason,
                }) => {
                    // A stream can emit more than one Done (e.g. a usage chunk
                    // then `[DONE]`); only overwrite usage with a real value
                    // and keep the first non-empty finish_reason.
                    if usage.is_some() {
                        response.usage = usage;
                    }
                    if finish_reason.is_some() {
                        response.finish_reason = finish_reason;
                    }
                }
                Err(e) => {
                    warn!("stream error: {e}");
                    return Err(EngineError::Llm(e));
                }
            }
        }

        // Finalize assembled tool calls. A tool call whose accumulated
        // arguments are non-empty but don't parse as JSON is an interrupted
        // stream — surface it explicitly instead of silently executing the
        // tool with `{}` (which would run it with the wrong/empty input).
        for (index, partial) in partials {
            if partial.name.is_empty() {
                return Err(EngineError::Llm(arawn_llm::LlmError::Stream(format!(
                    "stream interrupted: tool call at index {index} arrived without a name"
                ))));
            }
            let arguments = if partial.arguments.trim().is_empty() {
                // Legitimate no-argument tool call.
                serde_json::json!({})
            } else {
                serde_json::from_str(&partial.arguments).map_err(|e| {
                    EngineError::Llm(arawn_llm::LlmError::Stream(format!(
                        "stream interrupted: tool call '{}' (index {index}) has incomplete \
                         or malformed arguments ({e})",
                        partial.name
                    )))
                })?
            };
            response.tool_calls.push(AssembledToolCall {
                id: partial.id,
                name: partial.name,
                arguments,
            });
        }

        // A truncated stream (token limit / content filter) that was in the
        // middle of emitting tool calls cannot be trusted — the last call's
        // arguments may be silently incomplete-but-valid JSON.
        if response
            .finish_reason
            .as_ref()
            .is_some_and(|fr| fr.is_truncated())
            && !response.tool_calls.is_empty()
        {
            return Err(EngineError::Llm(arawn_llm::LlmError::Stream(format!(
                "stream interrupted: model stopped with finish_reason={:?} while emitting \
                 tool calls",
                response.finish_reason
            ))));
        }

        Ok(response)
    }

    async fn execute_tool(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        tool_use_id: &str,
        name: &str,
        arguments: &serde_json::Value,
    ) -> ToolResult {
        debug!(name, tool_use_id, %arguments, "executing tool");

        // Plan mode enforcement — check before permission rules
        if let Some(ref plan_state) = self.plan_state
            && plan_state.is_active()
        {
            // Allow plan mode meta-tools and side-effect-free tools
            let tool_is_allowed = name == "enter_plan_mode"
                || name == "exit_plan_mode"
                || self.registry.get(name).is_some_and(|t| t.is_read_only());

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

        // Permission check — if a checker is configured, verify the tool call is allowed.
        // The permission category comes from the tool itself via the Tool trait
        // (`tool.permission_category()`). Unknown tools get `Other` — conservative default.
        if let Some(ref checker) = self.permission_checker {
            let input_summary = arguments.to_string();
            let category = self
                .registry
                .get(name)
                .map(|t| t.permission_category())
                .unwrap_or(arawn_tool::PermissionCategory::Other);
            let (decision, reason) = checker
                .check_explained(name, &input_summary, category)
                .await;
            if decision == PermissionDecision::Denied {
                let reason_str = reason.display();
                warn!(name, reason = %reason_str, "tool blocked by permission system");
                return ToolResult {
                    content: format!(
                        "Permission denied: tool '{name}' was denied by {reason_str}. \
                         Run /permissions in the TUI to inspect the active rule set, \
                         or see docs/src/security.md."
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

        // Strip the agent's per-call `timeout_secs` override (if any) before
        // the args reach the tool. Resolve the effective budget; invalid
        // overrides surface to the agent as an error.
        let mut tool_args = arguments.clone();
        let call_override = match tool_timeout::extract_override(&mut tool_args) {
            Ok(v) => v,
            Err(msg) => {
                warn!(name, error = %msg, "invalid timeout_secs override");
                return ToolResult {
                    content: format!("Tool '{name}' rejected: {msg}"),
                    is_error: true,
                };
            }
        };
        let (budget, source) = tool_timeout::resolve(call_override, self.config.tool_timeout_secs);

        let exec = tool.execute(ctx, tool_args);
        let result = match tokio::time::timeout(budget, exec).await {
            Ok(r) => r,
            Err(_) => {
                let detail = match source {
                    "override" => format!("override of {}s", budget.as_secs()),
                    _ => format!("default of {}s", budget.as_secs()),
                };
                let msg =
                    format!("Tool '{name}' exceeded its {detail} budget; the call was cancelled.");
                warn!(
                    name,
                    budget_secs = budget.as_secs(),
                    source,
                    "tool timed out"
                );
                if let Some(ref runner) = self.hook_runner {
                    let hook_input = HookInput::PostToolUseFailure {
                        tool_name: name.to_string(),
                        tool_input: arguments.clone(),
                        error: format!("timeout after {}s ({source})", budget.as_secs()),
                    };
                    let _ = runner.run(&hook_input).await;
                }
                return ToolResult {
                    content: msg,
                    is_error: true,
                };
            }
        };

        match result {
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
        }
    }
}

#[derive(Default)]
struct AssembledResponse {
    text: String,
    tool_calls: Vec<AssembledToolCall>,
    usage: Option<arawn_llm::Usage>,
    /// Why the model stopped, if the provider reported it. Used to detect
    /// truncated turns (`length`/`content_filter`) and surface them.
    finish_reason: Option<arawn_llm::FinishReason>,
}

struct AssembledToolCall {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

/// A tool call being assembled from interleaved streaming deltas, keyed by
/// the provider's tool-call `index`.
#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

struct ToolResult {
    content: String,
    is_error: bool,
}

/// Filter tool definitions to only contextually relevant ones for this turn.
/// Uses ToolCategory from the registry instead of string constants.
/// Core and Utility categories are always included. Per-service integration
/// categories (Calendar, Gmail, Drive, Slack, Atlassian, GitHub) are
/// gated by `connected_services` — the authoritative source for "is the
/// integration connected." All other categories are still triggered by
/// keywords in the latest user message.
/// Models with this much context (or more) skip the filter entirely.
/// Rationale: 102 tools × ~250 tokens ≈ 25K tokens for the full catalog.
/// On a 100K-window model, that's 25% of context — comfortable. The
/// filter exists for small models (≤64K) where 25K tokens is most of
/// the window. Above the threshold, the brittleness of keyword/capability
/// gating isn't worth the savings.
const FILTER_BYPASS_CONTEXT_THRESHOLD: u32 = 100_000;

fn filter_tools_for_context(
    all_tools: &[arawn_llm::ToolDefinition],
    session: &Session,
    registry: &ToolRegistry,
    connected_services: &[String],
    model_limits: &ModelLimits,
) -> Vec<arawn_llm::ToolDefinition> {
    use arawn_tool::ToolCategory;

    // Large-context bypass: models with ≥100K context can afford the
    // full catalog. See I-0055 T-E.
    if model_limits.context_window >= FILTER_BYPASS_CONTEXT_THRESHOLD {
        return all_tools.to_vec();
    }

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

    // Track which categories the model has already used (keep them available)
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

    // Determine which categories to include based on keywords
    let mut active_categories = std::collections::HashSet::new();

    // Always include Core and Utility, plus the two ambient categories
    // (Lens + Memory) promoted in I-0055 T-D. Both have small
    // surface area (~5 + ~3 tools), high frequency of legitimate use,
    // and no semantic reason to keyword-gate — the agent should never
    // lose access to context-switching or recall mid-turn.
    active_categories.insert(ToolCategory::Core);
    active_categories.insert(ToolCategory::Utility);
    active_categories.insert(ToolCategory::Lens);
    active_categories.insert(ToolCategory::Memory);

    // Per-service integration categories — capability-gated, not keyword-gated.
    // Service names come from `Integration::name()` (lowercase snake_case):
    // gmail, google_calendar, google_drive, slack, atlassian, github.
    let is_connected = |service: &str| connected_services.iter().any(|s| s == service);
    if is_connected("google_calendar") {
        active_categories.insert(ToolCategory::Calendar);
    }
    if is_connected("gmail") {
        active_categories.insert(ToolCategory::Gmail);
    }
    if is_connected("google_drive") {
        active_categories.insert(ToolCategory::Drive);
    }
    if is_connected("slack") {
        active_categories.insert(ToolCategory::Slack);
    }
    if is_connected("atlassian") {
        active_categories.insert(ToolCategory::Atlassian);
    }
    if is_connected("github") {
        active_categories.insert(ToolCategory::GitHub);
    }

    // Web: URL patterns, web/search/fetch/http/api mentions.
    // `github` and `google` keywords dropped — those used to be proxies
    // for integration tools, which are now capability-gated above.
    if last_user_msg.contains("http")
        || last_user_msg.contains("url")
        || last_user_msg.contains("web")
        || last_user_msg.contains("search")
        || last_user_msg.contains("fetch")
        || last_user_msg.contains("api")
    {
        active_categories.insert(ToolCategory::Web);
    }

    // Plan: plan/planning mentions, plus design/approach/strategy
    // (I-0055 T-C addition — these read as planning intent without the
    // literal word "plan").
    if last_user_msg.contains("plan")
        || last_user_msg.contains("design")
        || last_user_msg.contains("approach")
        || last_user_msg.contains("strategy")
    {
        active_categories.insert(ToolCategory::Plan);
    }

    // Task + BackgroundTask: task/todo/background mentions.
    // `queue` added in I-0055 T-C — "queue this for later" is a common
    // task-creation framing.
    if last_user_msg.contains("task")
        || last_user_msg.contains("todo")
        || last_user_msg.contains("background")
        || last_user_msg.contains("queue")
    {
        active_categories.insert(ToolCategory::Task);
        active_categories.insert(ToolCategory::BackgroundTask);
    }

    // Memory: promoted to always-on in I-0055 T-D — see the unconditional
    // insert at the top of the function. Keyword branch removed.

    // Agent: agent/delegate/subagent/spawn mentions. `subagent` and `spawn`
    // added in I-0055 T-C — both are natural ways to ask for delegation
    // without the literal word "agent".
    if last_user_msg.contains("agent")
        || last_user_msg.contains("delegat")
        || last_user_msg.contains("subagent")
        || last_user_msg.contains("spawn")
    {
        active_categories.insert(ToolCategory::Agent);
    }

    // Lens: promoted to always-on in I-0055 T-D — see the
    // unconditional insert at the top of the function. Keyword branch
    // removed.

    // Ceremony: retro/ceremony/standup/diary mentions, plus the
    // generic todo surface (I-0049) which lives under the same
    // category. `todo` / `reminder` / `remind me` route the agent
    // to the todo_* tool family.
    // I-0055 T-C additions: `agenda`, `morning`, `afternoon`, `tomorrow`,
    // `yesterday`, `this week`, `next week` — calendar/ceremony framings
    // the original set missed. `week` already matches "next week" by
    // substring; the explicit terms keep intent readable here.
    if last_user_msg.contains("retro")
        || last_user_msg.contains("ceremony")
        || last_user_msg.contains("standup")
        || last_user_msg.contains("diary")
        || last_user_msg.contains("daily")
        || last_user_msg.contains("today")
        || last_user_msg.contains("brief")
        || last_user_msg.contains("weekly")
        || last_user_msg.contains("week")
        || last_user_msg.contains("priorities")
        || last_user_msg.contains("priority")
        || last_user_msg.contains("todo")
        || last_user_msg.contains("reminder")
        || last_user_msg.contains("remind me")
        || last_user_msg.contains("agenda")
        || last_user_msg.contains("morning")
        || last_user_msg.contains("afternoon")
        || last_user_msg.contains("tomorrow")
        || last_user_msg.contains("yesterday")
    {
        active_categories.insert(ToolCategory::Ceremony);
    }

    // Include categories of any previously-used tools
    for name in &used_tool_names {
        if let Some(tool) = registry.get(name) {
            active_categories.insert(tool.category());
        }
    }

    all_tools
        .iter()
        .filter(|t| {
            // Include if category is active OR if this specific tool was previously used
            if used_tool_names.contains(t.name.as_str()) {
                return true;
            }
            if let Some(tool) = registry.get(&t.name) {
                active_categories.contains(&tool.category())
            } else {
                // Unknown tool (e.g., MCP) — include by default
                true
            }
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use crate::tools::ThinkTool;
    use arawn_core::Lens;
    use arawn_llm::LlmError;
    use arawn_tool::{Tool, ToolOutput};
    use async_trait::async_trait;
    use futures::stream;
    use std::pin::Pin;
    use std::sync::Mutex;

    #[test]
    fn duplicate_call_key_is_canonical_and_order_independent() {
        // P1-10 (ARAWN-T-0474): the dedup key re-serializes arguments through
        // serde_json::Value, whose Display sorts object keys (preserve_order is
        // off) and drops incidental whitespace. So the same logical call hashes
        // identically regardless of how the model ordered/spaced its arguments —
        // a near-duplicate can't silently reset the failure counter.
        let a: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{ "b" : 2,  "a": 1 }"#).unwrap();
        assert_eq!(format!("tool:{a}"), format!("tool:{b}"));
    }

    #[test]
    fn previously_used_tools_stay_available_under_filtering() {
        // P1-8 (ARAWN-T-0474): a tool the agent already used must remain in the
        // catalog on later turns even when the new user message shares no
        // keywords with it. `filter_tools_for_context` force-includes any tool
        // present in an earlier Assistant message's tool_uses.
        use arawn_core::{Lens, Message, ToolUse};

        // A small-window model so the filter actually runs (no large-context bypass).
        let limits = ModelLimits::new(8_000, 0.85);
        let registry = ToolRegistry::new();
        registry.register(Box::new(ThinkTool)); // gives us a real, registered tool name

        // Build a session: turn-1 assistant used "think", turn-2 user message
        // mentions nothing related.
        let lens = Lens::scratch("/tmp/sticky-test");
        let mut session = Session::new(lens.id);
        session.add_message(Message::User {
            content: "please reflect on this".into(),
        });
        session.add_message(Message::Assistant {
            content: String::new(),
            tool_uses: vec![ToolUse {
                id: "1".into(),
                name: "think".into(),
                input: serde_json::json!({}),
            }],
        });
        session.add_message(Message::User {
            content: "what is the capital of france".into(), // unrelated to "think"
        });

        let all_tools = registry.tool_definitions();
        let filtered = filter_tools_for_context(&all_tools, &session, &registry, &[], &limits);
        assert!(
            filtered.iter().any(|t| t.name == "think"),
            "a previously-used tool must remain available after an unrelated turn"
        );
    }

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
                ChatChunk::Done {
                    usage: None,
                    finish_reason: Some(arawn_llm::FinishReason::Stop),
                },
            ]
        }

        /// Convenience: tool call then done
        fn tool_call(id: &str, name: &str, args: &str) -> Vec<ChatChunk> {
            vec![
                ChatChunk::ToolUseStart {
                    index: 0,
                    id: id.to_string(),
                    name: name.to_string(),
                },
                ChatChunk::ToolUseInputDelta {
                    index: 0,
                    json: args.to_string(),
                },
                ChatChunk::Done {
                    usage: None,
                    finish_reason: Some(arawn_llm::FinishReason::ToolCalls),
                },
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

    fn setup() -> (Lens, Session, EngineToolContext) {
        let ws = Lens::scratch("/tmp/test-engine");
        let session = Session::new(ws.id);
        let ctx = EngineToolContext::new(&ws, session.id);
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
            Err(EngineError::MaxIterations { iterations: 3, .. }) => {} // expected
            other => panic!("expected MaxIterations(3), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn no_progress_breaker_trips_on_all_errored_iterations() {
        // ARAWN-T-0475: a model that keeps issuing tool calls that all error
        // (here, calls to an unregistered tool — the same shape as the gemma
        // "grep" narration loop) must be cut off by the no-progress breaker,
        // with a surfaced reason, well before the max_iterations cap — not
        // run to exhaustion and hand back garbage.
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "do the thing".into(),
        });

        // Every round: a call to a tool that doesn't exist → error result →
        // no forward progress. Plenty of rounds so the breaker (not the cap)
        // is what stops it.
        let responses: Vec<Vec<ChatChunk>> = (0..20)
            .map(|i| MockLlm::tool_call(&format!("c{i}"), "nonexistent_tool", "{}"))
            .collect();
        let llm = Arc::new(MockLlm::new(responses));
        let registry = Arc::new(ToolRegistry::new());
        let config = QueryEngineConfig {
            max_iterations: 50,
            max_no_progress_iterations: 2,
            system_prompt: "test".into(),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert!(
            result.contains("consecutive rounds of tool calls that all failed"),
            "the breaker's reason should be surfaced, got: {result}"
        );
        // It bailed at the breaker (streak 2), nowhere near the 50-iteration cap.
        assert!(
            session.messages().len() < 10,
            "should stop at the breaker, not run to the cap; got {} messages",
            session.messages().len()
        );
    }

    #[tokio::test]
    async fn no_progress_breaker_does_not_fire_when_a_tool_succeeds() {
        // ARAWN-T-0475: a successful tool result resets the streak, so an
        // ordinary think→answer turn completes cleanly (no false positive).
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "reflect then answer".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("c1", "think", r#"{"thought":"ok"}"#),
            MockLlm::text("Here's the answer."),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(ThinkTool));
        let config = QueryEngineConfig {
            max_iterations: 50,
            max_no_progress_iterations: 2,
            system_prompt: "test".into(),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);

        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "Here's the answer.");
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

    /// Tool that intentionally sleeps for a duration so timeout tests can
    /// drive cancellation behaviour deterministically.
    struct SlowTool {
        sleep_ms: u64,
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow"
        }
        fn description(&self) -> &str {
            "sleeps for a configurable duration"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        async fn execute(
            &self,
            _ctx: &dyn arawn_tool::ToolContext,
            _params: serde_json::Value,
        ) -> Result<ToolOutput, arawn_tool::ToolError> {
            tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms)).await;
            Ok(ToolOutput::success("slept"))
        }
        fn is_read_only(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn tool_completes_when_default_budget_is_large() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "do it".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "slow", "{}"),
            MockLlm::text("done"),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(SlowTool { sleep_ms: 50 }));
        let config = QueryEngineConfig {
            max_iterations: 5,
            system_prompt: "t".into(),
            tool_timeout_secs: Some(5),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);
        let result = engine.run(&mut session, &ctx).await.unwrap();
        assert_eq!(result, "done");
    }

    #[tokio::test]
    async fn slow_tool_times_out_under_short_default() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "do it".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "slow", "{}"),
            MockLlm::text("acknowledged"),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(SlowTool { sleep_ms: 2000 }));
        let config = QueryEngineConfig {
            max_iterations: 5,
            system_prompt: "t".into(),
            tool_timeout_secs: Some(1),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);
        let _ = engine.run(&mut session, &ctx).await.unwrap();
        let tool_result = session
            .messages()
            .iter()
            .find_map(|m| match m {
                Message::ToolResult { content, .. } => Some(content.clone()),
                _ => None,
            })
            .expect("expected a ToolResult message");
        assert!(
            tool_result.contains("timed out")
                || tool_result.contains("cancelled")
                || tool_result.contains("exceeded"),
            "expected timeout error in tool result, got: {tool_result}"
        );
    }

    #[tokio::test]
    async fn agent_override_fires_before_default_would() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "be impatient".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "slow", r#"{"timeout_secs":1}"#),
            MockLlm::text("ack"),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(SlowTool { sleep_ms: 3000 }));
        let config = QueryEngineConfig {
            max_iterations: 5,
            system_prompt: "t".into(),
            tool_timeout_secs: Some(60),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);
        let started = std::time::Instant::now();
        let _ = engine.run(&mut session, &ctx).await.unwrap();
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "override should have fired in ~1s, took {elapsed:?}"
        );
        let tool_result = session
            .messages()
            .iter()
            .find_map(|m| match m {
                Message::ToolResult { content, .. } => Some(content.clone()),
                _ => None,
            })
            .expect("expected a ToolResult message");
        assert!(
            tool_result.contains("override"),
            "expected error to mention override, got: {tool_result}"
        );
    }

    #[tokio::test]
    async fn invalid_override_surfaces_as_tool_error() {
        let (_ws, mut session, ctx) = setup();
        session.add_message(Message::User {
            content: "bad arg".into(),
        });

        let llm = Arc::new(MockLlm::new(vec![
            MockLlm::tool_call("call_1", "slow", r#"{"timeout_secs":0}"#),
            MockLlm::text("ack"),
        ]));
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(SlowTool { sleep_ms: 50 }));
        let config = QueryEngineConfig {
            max_iterations: 5,
            system_prompt: "t".into(),
            ..Default::default()
        };
        let mut engine = QueryEngine::with_config(llm, registry, config);
        let _ = engine.run(&mut session, &ctx).await.unwrap();
        let tool_result = session
            .messages()
            .iter()
            .find_map(|m| match m {
                Message::ToolResult { content, .. } => Some(content.clone()),
                _ => None,
            })
            .expect("expected a ToolResult");
        assert!(
            tool_result.contains("timeout_secs"),
            "expected error to name the bad arg, got: {tool_result}"
        );
    }

    // ─── I-0055 T-B — capability-driven filter tests ─────────────────────

    /// 32K-context limits — forces the filter to be active (below the
    /// 100K T-E bypass threshold). Use this for tests that exercise the
    /// keyword / capability gating rather than the large-context bypass.
    fn small_model_limits() -> ModelLimits {
        ModelLimits {
            context_window: 32_000,
            compaction_threshold: 0.8,
        }
    }

    /// Tool stub used in filter tests. Returns a configurable category.
    struct CategorizedStub {
        name_: &'static str,
        category_: arawn_tool::ToolCategory,
    }

    #[async_trait]
    impl Tool for CategorizedStub {
        fn name(&self) -> &str {
            self.name_
        }
        fn description(&self) -> &str {
            "test stub"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn category(&self) -> arawn_tool::ToolCategory {
            self.category_
        }
        async fn execute(
            &self,
            _: &dyn arawn_tool::ToolContext,
            _: serde_json::Value,
        ) -> Result<ToolOutput, arawn_tool::ToolError> {
            Ok(ToolOutput::success("ok"))
        }
    }

    /// Build a session deep enough to trip the post-iter-1 filter
    /// activation: messages.len() must be > 2.
    fn session_past_iter_1(last_user_msg: &str) -> Session {
        let mut s = Session::new(uuid::Uuid::new_v4());
        s.add_message(Message::User {
            content: "kickoff".into(),
        });
        s.add_message(Message::Assistant {
            content: "ok".into(),
            tool_uses: vec![],
        });
        s.add_message(Message::User {
            content: last_user_msg.into(),
        });
        s
    }

    fn tool_def(name: &str) -> arawn_llm::ToolDefinition {
        arawn_llm::ToolDefinition {
            name: name.into(),
            description: "test".into(),
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn calendar_tool_visible_when_calendar_capability_present_no_keywords() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        let all = vec![tool_def("calendar_upcoming")];
        // User message contains zero calendar/web/scheduling keywords.
        let session = session_past_iter_1("Bob is free Tue mornings");
        let connected = vec!["google_calendar".to_string()];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().any(|t| t.name == "calendar_upcoming"),
            "calendar_upcoming should be visible when google_calendar capability is connected"
        );
    }

    #[test]
    fn calendar_tool_hidden_when_calendar_capability_absent() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        let all = vec![tool_def("calendar_upcoming")];
        let session = session_past_iter_1("Bob is free Tue mornings");
        let connected: Vec<String> = vec![]; // capability absent
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().all(|t| t.name != "calendar_upcoming"),
            "calendar_upcoming should be dropped when google_calendar capability is absent"
        );
    }

    #[test]
    fn slack_tool_visible_when_slack_capability_present_no_keywords() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "slack_post",
            category_: arawn_tool::ToolCategory::Slack,
        }));
        let all = vec![tool_def("slack_post")];
        let session = session_past_iter_1("Tell Pat we're shipping");
        let connected = vec!["slack".to_string()];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().any(|t| t.name == "slack_post"),
            "slack_post should be visible when slack capability is connected"
        );
    }

    #[test]
    fn slack_tool_hidden_when_slack_capability_absent() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "slack_post",
            category_: arawn_tool::ToolCategory::Slack,
        }));
        let all = vec![tool_def("slack_post")];
        let session = session_past_iter_1("Tell Pat we're shipping");
        let connected: Vec<String> = vec![];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().all(|t| t.name != "slack_post"),
            "slack_post should be dropped when slack capability is absent"
        );
    }

    // ─── I-0055 T-C — non-integration keyword routing tests ──────────────
    //
    // One positive + one negative per non-integration category that's still
    // keyword-gated. Memory + Lens are slated for always-on promotion
    // in T-D, so only their CURRENT keyword behavior is asserted here.

    fn assert_tool_visible(cat: arawn_tool::ToolCategory, tool_name: &'static str, user_msg: &str) {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: tool_name,
            category_: cat,
        }));
        let all = vec![tool_def(tool_name)];
        let session = session_past_iter_1(user_msg);
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &[], &small_model_limits());
        assert!(
            filtered.iter().any(|t| t.name == tool_name),
            "{tool_name} should be visible for category {cat:?} given user_msg = {user_msg:?}",
        );
    }

    fn assert_tool_hidden(cat: arawn_tool::ToolCategory, tool_name: &'static str, user_msg: &str) {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: tool_name,
            category_: cat,
        }));
        let all = vec![tool_def(tool_name)];
        let session = session_past_iter_1(user_msg);
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &[], &small_model_limits());
        assert!(
            filtered.iter().all(|t| t.name != tool_name),
            "{tool_name} should be hidden for category {cat:?} given user_msg = {user_msg:?}",
        );
    }

    // Web — narrow keyword set (http/url/web/search/fetch/api).
    #[test]
    fn web_visible_on_keyword() {
        assert_tool_visible(arawn_tool::ToolCategory::Web, "web_fetch", "fetch the URL");
    }
    #[test]
    fn web_hidden_without_keyword() {
        assert_tool_hidden(arawn_tool::ToolCategory::Web, "web_fetch", "say hi to Bob");
    }
    #[test]
    fn web_no_longer_triggered_by_github_keyword() {
        // I-0055 T-B/T-C dropped `github` and `google` from Web triggers
        // (they were proxies for integration tools now capability-gated).
        assert_tool_hidden(
            arawn_tool::ToolCategory::Web,
            "web_fetch",
            "open the github repo",
        );
    }

    // Plan — original `plan` plus T-C additions (design/approach/strategy).
    #[test]
    fn plan_visible_on_plan_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Plan,
            "enter_plan_mode",
            "let's plan the migration",
        );
    }
    #[test]
    fn plan_visible_on_design_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Plan,
            "enter_plan_mode",
            "what's the right design",
        );
    }
    #[test]
    fn plan_hidden_without_keyword() {
        assert_tool_hidden(
            arawn_tool::ToolCategory::Plan,
            "enter_plan_mode",
            "hello there",
        );
    }

    // Task — `task`, `todo`, `background`, and T-C `queue`.
    #[test]
    fn task_visible_on_queue_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Task,
            "task_list",
            "queue this for later",
        );
    }
    #[test]
    fn task_hidden_without_keyword() {
        assert_tool_hidden(arawn_tool::ToolCategory::Task, "task_list", "hello");
    }

    // Memory — promoted to always-on in T-D. Tools must surface even when
    // the user message contains no memory keywords.
    #[test]
    fn memory_tools_visible_with_empty_user_message() {
        assert_tool_visible(arawn_tool::ToolCategory::Memory, "memory_recall", "x");
    }
    #[test]
    fn memory_tools_visible_with_unrelated_user_message() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Memory,
            "memory_recall",
            "fetch the URL", // a Web-keyword prompt — Memory still surfaces
        );
    }

    // Agent — `agent`, `delegat`, plus T-C `subagent` and `spawn`.
    #[test]
    fn agent_visible_on_subagent_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Agent,
            "agent",
            "spin up a subagent",
        );
    }
    #[test]
    fn agent_visible_on_spawn_keyword() {
        assert_tool_visible(arawn_tool::ToolCategory::Agent, "agent", "spawn a worker");
    }
    #[test]
    fn agent_hidden_without_keyword() {
        assert_tool_hidden(arawn_tool::ToolCategory::Agent, "agent", "hello");
    }

    // Lens — promoted to always-on in T-D. Tools must surface even
    // when the user message contains no lens keywords.
    #[test]
    fn lens_tools_visible_with_empty_user_message() {
        assert_tool_visible(arawn_tool::ToolCategory::Lens, "lens_show", "x");
    }
    #[test]
    fn lens_tools_visible_with_unrelated_user_message() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Lens,
            "lens_show",
            "what's on my agenda", // Ceremony-keyword prompt — Lens still surfaces
        );
    }

    // Ceremony — original set plus T-C additions (agenda, morning, afternoon,
    // tomorrow, yesterday).
    #[test]
    fn ceremony_visible_on_agenda_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Ceremony,
            "daily_current",
            "what's on my agenda",
        );
    }
    #[test]
    fn ceremony_visible_on_tomorrow_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Ceremony,
            "daily_current",
            "anything on for tomorrow",
        );
    }
    #[test]
    fn ceremony_visible_on_morning_keyword() {
        assert_tool_visible(
            arawn_tool::ToolCategory::Ceremony,
            "daily_current",
            "free in the morning",
        );
    }
    #[test]
    fn ceremony_hidden_without_keyword() {
        assert_tool_hidden(arawn_tool::ToolCategory::Ceremony, "daily_current", "hello");
    }

    // ─── I-0055 T-E — large-context bypass tests ─────────────────────────

    fn large_model_limits() -> ModelLimits {
        ModelLimits {
            context_window: 200_000,
            compaction_threshold: 0.8,
        }
    }

    /// On a ≥100K-context model, every tool ships regardless of the filter's
    /// keyword/capability gates. Setup: integration tool whose capability is
    /// NOT connected. Under the small-model filter this tool is dropped; the
    /// bypass means it survives.
    #[test]
    fn filter_bypasses_for_large_context_model() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        let all = vec![tool_def("calendar_upcoming")];
        let session = session_past_iter_1("Bob is free Tue mornings");
        let connected: Vec<String> = vec![]; // capability absent
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &large_model_limits());
        assert!(
            filtered.iter().any(|t| t.name == "calendar_upcoming"),
            "calendar_upcoming should be visible on large-context model even without capability"
        );
    }

    /// Companion: on a small-context model the same call drops the tool.
    /// Asserts the filter is still active when bypass doesn't fire.
    #[test]
    fn filter_active_for_small_context_model() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        let all = vec![tool_def("calendar_upcoming")];
        let session = session_past_iter_1("Bob is free Tue mornings");
        let connected: Vec<String> = vec![];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().all(|t| t.name != "calendar_upcoming"),
            "calendar_upcoming should be dropped on small-context model without capability"
        );
    }

    // ─── I-0055 T-F — ARAWN-T-0394 regression test ───────────────────────
    //
    // Original failure (2026-05-21): the UAT scenario `schedule-with-confirmation`
    // failed because `calendar_upcoming` was dropped from the catalog on
    // iter-2+ of the agent loop. The user prompt contained zero `Web`-category
    // keywords (calendar tools were mis-categorized as Web), so the filter
    // dropped them. With no recovery path, the agent hallucinated `shell("gcal")`
    // and ended without proposing a slot.
    //
    // Post-I-0055-T-B fix: calendar tools are now in `ToolCategory::Calendar`
    // and gated by the `google_calendar` connected capability — NOT by user
    // message text. Once the integration is connected, the tool survives
    // every iteration regardless of what the user types. The two tests
    // below exercise this contract using the literal failing prompt from
    // the original UAT transcript.

    /// The exact scenario that failed in ARAWN-T-0394: filter activated
    /// (messages.len() > 2), no calendar keywords in the user message, small
    /// context window (32K, so T-E bypass doesn't fire). With
    /// `google_calendar` connected, `calendar_upcoming` MUST be in the
    /// filtered catalog.
    #[test]
    fn t_0394_calendar_tools_survive_filter_after_iter_1_when_calendar_capability_connected() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        registry.register(Box::new(CategorizedStub {
            name_: "weekly_run",
            category_: arawn_tool::ToolCategory::Ceremony,
        }));
        registry.register(Box::new(CategorizedStub {
            name_: "web_fetch",
            category_: arawn_tool::ToolCategory::Web,
        }));
        let all = vec![
            tool_def("calendar_upcoming"),
            tool_def("weekly_run"),
            tool_def("web_fetch"),
        ];
        // Literal user prompt from the failing UAT transcript.
        let session = session_past_iter_1(
            "Switch to `personal`. Bob replied to my catch-up email — he's open \
             Tue/Wed mornings or Thu after 2 next week. Pick a 30-min slot that \
             doesn't conflict with anything on my calendar and propose it to me. \
             Don't book it without asking.",
        );
        let connected = vec!["google_calendar".to_string()];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().any(|t| t.name == "calendar_upcoming"),
            "T-0394 regression: calendar_upcoming MUST be visible on iter-2+ \
             when google_calendar capability is connected, regardless of user message text"
        );
    }

    /// Companion: same scenario, capability not connected. The tool must
    /// NOT appear — confirms the capability gate works both ways. Without
    /// this, the positive test alone could pass via the early-return path
    /// or by some other accident; the negative pins down the contract.
    #[test]
    fn t_0394_calendar_tools_hidden_when_capability_absent() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(CategorizedStub {
            name_: "calendar_upcoming",
            category_: arawn_tool::ToolCategory::Calendar,
        }));
        registry.register(Box::new(CategorizedStub {
            name_: "weekly_run",
            category_: arawn_tool::ToolCategory::Ceremony,
        }));
        registry.register(Box::new(CategorizedStub {
            name_: "web_fetch",
            category_: arawn_tool::ToolCategory::Web,
        }));
        let all = vec![
            tool_def("calendar_upcoming"),
            tool_def("weekly_run"),
            tool_def("web_fetch"),
        ];
        let session = session_past_iter_1(
            "Switch to `personal`. Bob replied to my catch-up email — he's open \
             Tue/Wed mornings or Thu after 2 next week. Pick a 30-min slot that \
             doesn't conflict with anything on my calendar and propose it to me. \
             Don't book it without asking.",
        );
        let connected: Vec<String> = vec![];
        let filtered =
            filter_tools_for_context(&all, &session, &registry, &connected, &small_model_limits());
        assert!(
            filtered.iter().all(|t| t.name != "calendar_upcoming"),
            "T-0394 negative: calendar_upcoming MUST be dropped when google_calendar \
             capability is absent (otherwise the gate is vacuous)"
        );
    }
}

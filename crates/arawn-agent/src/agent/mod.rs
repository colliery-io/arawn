//! Core Agent implementation.
//!
//! The [`Agent`] struct is the brain of the system - it orchestrates the
//! conversation loop, handles tool execution, and manages context.

mod builder;
mod execution;
mod recall;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Instant;

use arawn_llm::{
    Message, SharedBackend, SharedEmbedder, ToolResultBlock,
    interaction_log::{InteractionLogger, InteractionRecord},
};
use arawn_memory::store::MemoryStore;
use arawn_types::{FsGateResolver, SharedHookDispatcher, SharedSecretResolver};
use tokio_util::sync::CancellationToken;

use crate::stream::{AgentStream, create_turn_stream};

use crate::error::Result;
use crate::prompt::SystemPromptBuilder;
use crate::tool::ToolRegistry;
use crate::types::{AgentConfig, AgentResponse, ResponseUsage, Session};

pub use builder::AgentBuilder;

// ─────────────────────────────────────────────────────────────────────────────
// Recall Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for active recall behavior in the agent turn loop.
#[derive(Debug, Clone)]
pub struct RecallConfig {
    /// Whether active recall is enabled.
    pub enabled: bool,
    /// Minimum similarity score threshold (0.0–1.0).
    pub threshold: f32,
    /// Maximum number of memories to recall per turn.
    pub limit: usize,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.6,
            limit: 5,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent
// ─────────────────────────────────────────────────────────────────────────────

/// The core agent that orchestrates LLM calls and tool execution.
pub struct Agent {
    /// LLM backend for completions.
    backend: SharedBackend,
    /// Registered tools.
    tools: Arc<ToolRegistry>,
    /// Agent configuration.
    config: AgentConfig,
    /// System prompt builder — rebuilt per-turn for fresh datetime, plugins, etc.
    prompt_builder: Option<SystemPromptBuilder>,
    /// Optional interaction logger for structured JSONL capture.
    interaction_logger: Option<Arc<InteractionLogger>>,
    /// Optional memory store for active recall.
    memory_store: Option<Arc<MemoryStore>>,
    /// Optional embedder for computing query embeddings.
    embedder: Option<SharedEmbedder>,
    /// Active recall configuration.
    recall_config: RecallConfig,
    /// Optional hook dispatcher for plugin lifecycle events.
    hook_dispatcher: Option<SharedHookDispatcher>,
    /// Optional resolver for filesystem access gates.
    fs_gate_resolver: Option<FsGateResolver>,
    /// Optional secret resolver for `${{secrets.*}}` handle resolution.
    secret_resolver: Option<SharedSecretResolver>,
}

impl Agent {
    /// Create a new agent with the given backend and tools.
    pub fn new(backend: SharedBackend, tools: ToolRegistry, config: AgentConfig) -> Self {
        Self {
            backend,
            tools: Arc::new(tools),
            config,
            prompt_builder: None,
            interaction_logger: None,
            memory_store: None,
            embedder: None,
            recall_config: RecallConfig::default(),
            hook_dispatcher: None,
            fs_gate_resolver: None,
            secret_resolver: None,
        }
    }

    /// Create an agent builder for fluent construction.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// Get the agent configuration.
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get the tool registry.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Get the LLM backend.
    pub fn backend(&self) -> SharedBackend {
        self.backend.clone()
    }

    /// Get the current system prompt (built dynamically if a builder is present).
    ///
    /// This is the prompt that would be sent to the LLM on the next turn,
    /// without any session context preamble.
    pub fn system_prompt(&self) -> Option<String> {
        self.build_system_prompt(None)
    }

    /// Execute a single turn of conversation.
    ///
    /// Takes a user message, potentially executes multiple tool calls,
    /// and returns the final response.
    pub async fn turn(
        &self,
        session: &mut Session,
        user_message: &str,
        workstream_id: Option<&str>,
    ) -> Result<AgentResponse> {
        // Start a new turn
        let turn = session.start_turn(user_message);
        let turn_id = turn.id;
        let session_id = session.id;

        tracing::info!(
            %session_id,
            %turn_id,
            message_len = user_message.len(),
            "Turn started"
        );

        // Build initial messages from session history
        let mut messages = self.build_messages(session);

        // Log initial context size
        let initial_context_tokens = self.estimate_messages_tokens(&messages);
        tracing::debug!(
            %session_id,
            %turn_id,
            message_count = messages.len(),
            estimated_tokens = initial_context_tokens,
            "Context: initial history loaded"
        );

        // Active recall: inject relevant memories before first LLM call
        if let Some(context_msg) = self.perform_recall(user_message).await {
            // Insert as second message (after first user message, or at start)
            let insert_pos = 1.min(messages.len());
            messages.insert(insert_pos, context_msg);
        }

        // Track usage and iterations
        let mut total_input_tokens = 0u32;
        let mut total_output_tokens = 0u32;
        let mut iterations = 0u32;
        let mut all_tool_calls = Vec::new();
        let mut all_tool_results = Vec::new();

        // Tool execution loop
        loop {
            iterations += 1;

            if iterations > self.config.max_iterations {
                tracing::debug!(%session_id, %turn_id, iterations, max = self.config.max_iterations, "Max iterations reached — truncating turn");
                // Mark turn as truncated
                if let Some(turn) = session.current_turn_mut() {
                    turn.complete("[Response truncated: max iterations exceeded]");
                    turn.tool_calls = all_tool_calls.clone();
                    turn.tool_results = all_tool_results.clone();
                }

                return Ok(AgentResponse {
                    text: "[Response truncated: max iterations exceeded]".to_string(),
                    tool_calls: all_tool_calls,
                    tool_results: all_tool_results,
                    iterations,
                    usage: ResponseUsage::new(total_input_tokens, total_output_tokens),
                    truncated: true,
                });
            }

            // Build completion request
            let request = self.build_request(&messages, session.context_preamble());

            tracing::debug!(
                %session_id,
                iteration = iterations,
                messages = messages.len(),
                tools = self.tools.names().len(),
                model = %request.model,
                "Calling LLM"
            );

            // Call LLM with timing
            let call_start = Instant::now();
            let response = match self.backend.complete(request.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    // Check if this is a tool validation error (LLM hallucinated a tool name)
                    // If so, inject feedback and retry instead of failing
                    if e.is_tool_validation_error() {
                        let invalid_tool = e.invalid_tool_name().unwrap_or("unknown");
                        let available_tools = self.tools.names().join(", ");

                        tracing::warn!(
                            %session_id,
                            %turn_id,
                            iteration = iterations,
                            invalid_tool = %invalid_tool,
                            "Tool validation error - injecting feedback and retrying"
                        );

                        // Add feedback as a user message so the LLM can correct itself
                        let feedback = format!(
                            "Error: The tool '{}' does not exist. Available tools are: {}. Please use the exact tool name from this list.",
                            invalid_tool, available_tools
                        );
                        messages.push(Message::user(feedback));

                        // Continue to retry (counts against iteration limit)
                        continue;
                    }

                    tracing::error!(%session_id, %turn_id, iteration = iterations, error = %e, "LLM call failed");
                    return Err(e.into());
                }
            };
            let duration_ms = call_start.elapsed().as_millis() as u64;

            // Update usage
            total_input_tokens += response.usage.input_tokens;
            total_output_tokens += response.usage.output_tokens;

            // Check token budget
            if let Some(max) = self.config.max_total_tokens {
                let total = (total_input_tokens + total_output_tokens) as usize;
                if total > max {
                    tracing::warn!(
                        %session_id, %turn_id, total, max,
                        "Token budget exceeded"
                    );
                    let text = response.text();
                    if let Some(turn) = session.current_turn_mut() {
                        turn.complete(&text);
                        turn.tool_calls = all_tool_calls.clone();
                        turn.tool_results = all_tool_results.clone();
                    }

                    return Ok(AgentResponse {
                        text,
                        tool_calls: all_tool_calls,
                        tool_results: all_tool_results,
                        iterations,
                        usage: ResponseUsage::new(total_input_tokens, total_output_tokens),
                        truncated: true,
                    });
                }
            }

            tracing::debug!(
                %session_id,
                iteration = iterations,
                input_tokens = response.usage.input_tokens,
                output_tokens = response.usage.output_tokens,
                stop_reason = ?response.stop_reason,
                has_tool_use = response.has_tool_use(),
                duration_ms,
                "LLM response received"
            );

            // Write structured interaction record
            if let Some(ref logger) = self.interaction_logger {
                let record = InteractionRecord::from_exchange(&request, &response, duration_ms);
                if let Err(e) = logger.log(&record) {
                    tracing::warn!(error = %e, "Failed to write interaction log");
                }
            }

            // Check for tool use
            if response.has_tool_use() {
                let tool_uses = response.tool_uses();
                tracing::info!(
                    %session_id,
                    iteration = iterations,
                    tool_count = tool_uses.len(),
                    tools = %tool_uses.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "),
                    "Executing tools"
                );

                // Execute tools
                let (tool_calls, tool_results) = self
                    .execute_tools(&response, session_id, turn_id, workstream_id)
                    .await?;

                // Record tool calls and results
                all_tool_calls.extend(tool_calls.clone());
                all_tool_results.extend(tool_results.clone());

                // Add assistant message with tool calls to history
                messages.push(Message::assistant_blocks(response.content.clone()));

                // Add tool results to history
                let tool_result_blocks: Vec<ToolResultBlock> = tool_results
                    .iter()
                    .map(|r| {
                        if r.success {
                            ToolResultBlock::success(&r.tool_call_id, &r.content)
                        } else {
                            ToolResultBlock::error(&r.tool_call_id, &r.content)
                        }
                    })
                    .collect();

                messages.push(Message::tool_results(tool_result_blocks));

                // Log context size after adding tool results
                let context_tokens = self.estimate_messages_tokens(&messages);
                tracing::debug!(
                    %session_id,
                    iteration = iterations,
                    message_count = messages.len(),
                    estimated_tokens = context_tokens,
                    "Context: after tool results"
                );

                // Continue loop for next LLM call
                continue;
            }

            // No tool use - we have the final response
            let text = response.text();

            tracing::info!(
                %session_id,
                %turn_id,
                iterations,
                total_input_tokens,
                total_output_tokens,
                tool_calls = all_tool_calls.len(),
                response_len = text.len(),
                "Turn completed"
            );

            // Complete the turn
            if let Some(turn) = session.current_turn_mut() {
                turn.complete(&text);
                turn.tool_calls = all_tool_calls.clone();
                turn.tool_results = all_tool_results.clone();
            }

            return Ok(AgentResponse {
                text,
                tool_calls: all_tool_calls,
                tool_results: all_tool_results,
                iterations,
                usage: ResponseUsage::new(total_input_tokens, total_output_tokens),
                truncated: false,
            });
        }
    }

    /// Execute a single turn of conversation with streaming output.
    ///
    /// Returns a stream of chunks that yield text deltas, tool execution events,
    /// and completion notifications as they occur.
    ///
    /// # Arguments
    /// * `session` - The session to operate on
    /// * `user_message` - The user's message
    /// * `cancellation` - Token to cancel the operation
    ///
    /// # Returns
    /// A stream of `StreamChunk` items
    pub fn turn_stream(
        &self,
        session: &mut Session,
        user_message: &str,
        cancellation: CancellationToken,
        workstream_id: Option<&str>,
    ) -> AgentStream {
        // Start a new turn
        let turn = session.start_turn(user_message);
        let turn_id = turn.id;
        let session_id = session.id;

        // Resolve filesystem gate for workstream sandbox enforcement
        let fs_gate = match (&self.fs_gate_resolver, workstream_id) {
            (Some(resolver), Some(ws_id)) => resolver(&session_id.to_string(), ws_id),
            _ => None,
        };

        // Build initial messages from session history
        let messages = self.build_messages(session);

        // Build a config snapshot with a fresh system prompt for this turn
        let mut config = self.config.clone();
        config.system_prompt = self.build_system_prompt(session.context_preamble());

        create_turn_stream(
            self.backend.clone(),
            self.tools.clone(),
            config,
            messages,
            session_id,
            turn_id,
            cancellation,
            fs_gate,
            self.secret_resolver.clone(),
        )
    }
}

//! Message building, request construction, and tool execution for the Agent.

use std::sync::Arc;

use arawn_llm::{CompletionRequest, CompletionResponse, ContentBlock, Message, ToolResultBlock};
use arawn_types::HookOutcome;

use crate::context::estimate_tokens;
use crate::error::Result;
use crate::tool::{ToolContext, ToolResult};
use crate::types::{ToolCall, ToolResultRecord};

use super::Agent;

impl Agent {
    /// Estimate total tokens for a list of messages.
    pub(super) fn estimate_messages_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum()
    }

    /// Estimate tokens for a single message.
    fn estimate_message_tokens(&self, message: &Message) -> usize {
        // Base overhead for message structure
        let mut tokens = 10;

        // Add content tokens
        for block in message.content.blocks() {
            tokens += match block {
                ContentBlock::Text { text, .. } => estimate_tokens(&text),
                ContentBlock::ToolUse { name, input, .. } => {
                    estimate_tokens(&name) + estimate_tokens(&input.to_string())
                }
                ContentBlock::ToolResult { content, .. } => {
                    if let Some(c) = content {
                        match c {
                            arawn_llm::ToolResultContent::Text(text) => estimate_tokens(&text),
                            arawn_llm::ToolResultContent::Blocks(blocks) => {
                                estimate_tokens(&serde_json::to_string(&blocks).unwrap_or_default())
                            }
                        }
                    } else {
                        0
                    }
                }
            };
        }

        tokens
    }

    /// Build messages from session history.
    pub(super) fn build_messages(&self, session: &crate::types::Session) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add previous turns (excluding current incomplete turn)
        for turn in session.all_turns() {
            // Skip if this turn has no response (current turn)
            if turn.assistant_response.is_none() && turn.tool_calls.is_empty() {
                // This is the current turn - add just the user message
                messages.push(Message::user(&turn.user_message));
                continue;
            }

            // Add user message
            messages.push(Message::user(&turn.user_message));

            // Build assistant content blocks
            let mut assistant_blocks: Vec<ContentBlock> = Vec::new();

            // Add tool calls as ToolUse blocks
            for tc in &turn.tool_calls {
                assistant_blocks.push(ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                    cache_control: None,
                });
            }

            // Add final text response if present
            if let Some(ref response) = turn.assistant_response
                && !response.is_empty()
            {
                assistant_blocks.push(ContentBlock::Text {
                    text: response.clone(),
                    cache_control: None,
                });
            }

            if !assistant_blocks.is_empty() {
                messages.push(Message::assistant_blocks(assistant_blocks));
            }

            // Add tool results
            if !turn.tool_results.is_empty() {
                let result_blocks: Vec<ToolResultBlock> = turn
                    .tool_results
                    .iter()
                    .map(|r| {
                        if r.success {
                            ToolResultBlock::success(&r.tool_call_id, &r.content)
                        } else {
                            ToolResultBlock::error(&r.tool_call_id, &r.content)
                        }
                    })
                    .collect();
                messages.push(Message::tool_results(result_blocks));
            }
        }

        messages
    }

    /// Build a completion request.
    ///
    /// # Arguments
    /// * `messages` - The conversation messages
    /// * `context_preamble` - Optional session context to prepend to the system prompt
    pub(super) fn build_request(
        &self,
        messages: &[Message],
        context_preamble: Option<&str>,
    ) -> CompletionRequest {
        let mut request = CompletionRequest::new(
            &self.config.model,
            messages.to_vec(),
            self.config.max_tokens,
        );

        // Build system prompt dynamically (fresh datetime, etc.)
        if let Some(ref prompt) = self.build_system_prompt(context_preamble) {
            request = request.with_system(prompt);
        }

        // Add temperature
        if let Some(temp) = self.config.temperature {
            request = request.with_temperature(temp);
        }

        // Add tools
        let tool_defs = self.tools.to_llm_definitions();
        if !tool_defs.is_empty() {
            request = request.with_tools(tool_defs);
        }

        request
    }

    /// Execute tool calls from an LLM response.
    pub(super) async fn execute_tools(
        &self,
        response: &CompletionResponse,
        session_id: crate::types::SessionId,
        turn_id: crate::types::TurnId,
        workstream_id: Option<&str>,
    ) -> Result<(Vec<ToolCall>, Vec<ToolResultRecord>)> {
        let mut tool_calls = Vec::new();
        let mut tool_results = Vec::new();

        let mut ctx = ToolContext::new(session_id, turn_id);

        // Resolve filesystem gate for workstream sandbox enforcement
        if let (Some(resolver), Some(ws_id)) = (&self.fs_gate_resolver, workstream_id)
            && let Some(gate) = resolver(&session_id.to_string(), ws_id)
        {
            ctx.fs_gate = Some(gate);
        }

        // Attach secret resolver for ${{secrets.*}} handle resolution
        if let Some(ref resolver) = self.secret_resolver {
            ctx.secret_resolver = Some(Arc::clone(resolver));
        }

        for tool_use in response.tool_uses() {
            let tool_call = ToolCall {
                id: tool_use.id.clone(),
                name: tool_use.name.clone(),
                arguments: tool_use.input.clone(),
            };
            tool_calls.push(tool_call);

            // Pre-tool hook: can block tool execution
            if let Some(ref dispatcher) = self.hook_dispatcher {
                match dispatcher
                    .dispatch_pre_tool_use(&tool_use.name, &tool_use.input)
                    .await
                {
                    HookOutcome::Block { reason } => {
                        tracing::info!(
                            tool = %tool_use.name,
                            reason = %reason,
                            "Tool blocked by hook"
                        );
                        tool_results.push(ToolResultRecord {
                            tool_call_id: tool_use.id.clone(),
                            success: false,
                            content: format!("Blocked by hook: {}", reason),
                        });
                        continue;
                    }
                    HookOutcome::Allow | HookOutcome::Info { .. } => {
                        // Proceed with tool execution
                    }
                }
            }

            // Log tool input
            let input_str = tool_use.input.to_string();
            let input_bytes = input_str.len();
            tracing::debug!(
                tool = %tool_use.name,
                tool_call_id = %tool_use.id,
                input_bytes,
                input_tokens = estimate_tokens(&input_str),
                "Tool: executing"
            );

            // Execute the tool with per-tool output limits
            let output_config = self.tools.output_config_for(&tool_use.name);
            let result = match self
                .tools
                .execute_with_config(&tool_use.name, tool_use.input.clone(), &ctx, &output_config)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(
                        tool = %tool_use.name,
                        error = %e,
                        "Tool execution failed"
                    );
                    ToolResult::error(e.to_string())
                }
            };

            // Log tool output size
            let output_content = result.to_llm_content();
            let output_bytes = output_content.len();
            let output_tokens = estimate_tokens(&output_content);
            tracing::debug!(
                tool = %tool_use.name,
                tool_call_id = %tool_use.id,
                success = result.is_success(),
                output_bytes,
                output_tokens,
                "Tool: completed"
            );

            // Post-tool hook: informational only
            if let Some(ref dispatcher) = self.hook_dispatcher {
                let result_json = serde_json::to_value(&result).unwrap_or_default();
                let _ = dispatcher
                    .dispatch_post_tool_use(&tool_use.name, &tool_use.input, &result_json)
                    .await;
            }

            tool_results.push(ToolResultRecord {
                tool_call_id: tool_use.id.clone(),
                success: result.is_success(),
                content: result.to_llm_content(),
            });
        }

        Ok((tool_calls, tool_results))
    }
}

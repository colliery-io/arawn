//! Active recall and system prompt building for the Agent.

use arawn_llm::Message;
use arawn_memory::store::RecallQuery;

use super::Agent;

/// Format recall matches into a concise context string for injection.
fn format_recall_context(matches: &[arawn_memory::store::RecallMatch]) -> String {
    let mut lines = Vec::new();
    for m in matches {
        let ts = m.memory.created_at.format("%Y-%m-%d %H:%M");
        lines.push(format!(
            "- [{}] ({:.0}%) {}",
            ts,
            m.score * 100.0,
            m.memory.content
        ));
    }
    lines.join("\n")
}

impl Agent {
    /// Build the system prompt dynamically.
    ///
    /// If a `SystemPromptBuilder` is stored, rebuilds the prompt fresh
    /// (giving current datetime, etc.). Falls back to the static
    /// `config.system_prompt` if no builder is present.
    pub(super) fn build_system_prompt(&self, context_preamble: Option<&str>) -> Option<String> {
        // Build from dynamic builder if present
        let base_prompt = if let Some(ref builder) = self.prompt_builder {
            let prompt = builder.build();
            if prompt.is_empty() {
                None
            } else {
                Some(prompt)
            }
        } else {
            self.config.system_prompt.clone()
        };

        // Merge with context preamble
        match (base_prompt, context_preamble) {
            (Some(prompt), Some(preamble)) => Some(format!(
                "[Session Context]\n{}\n\n---\n\n{}",
                preamble, prompt
            )),
            (Some(prompt), None) => Some(prompt),
            (None, Some(preamble)) => Some(format!("[Session Context]\n{}", preamble)),
            (None, None) => None,
        }
    }

    /// Perform active recall for a user message.
    ///
    /// Embeds the user message, queries the memory store, and returns
    /// a system message with relevant context if any matches are found.
    /// Returns `None` if recall is disabled, not configured, or finds nothing.
    pub(super) async fn perform_recall(&self, user_message: &str) -> Option<Message> {
        // Guard: recall must be enabled
        if !self.recall_config.enabled {
            return None;
        }

        // Guard: need both memory store and embedder
        let store = self.memory_store.as_ref()?;
        let embedder = self.embedder.as_ref()?;

        // Guard: skip empty/whitespace messages
        if user_message.trim().is_empty() {
            return None;
        }

        // Guard: vectors must be initialized
        if !store.has_vectors() {
            return None;
        }

        // Embed the user message
        let embedding = match embedder.embed(user_message).await {
            Ok(emb) => emb,
            Err(e) => {
                tracing::debug!(error = %e, "Recall: embedding failed, skipping");
                return None;
            }
        };

        // Build recall query
        let query = RecallQuery::new(embedding)
            .with_limit(self.recall_config.limit)
            .with_min_score(self.recall_config.threshold);

        // Execute recall
        let result = match store.recall(query) {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, "Recall: query failed, skipping");
                return None;
            }
        };

        if result.matches.is_empty() {
            return None;
        }

        let context = format_recall_context(&result.matches);

        tracing::debug!(
            matches = result.matches.len(),
            query_time_ms = result.query_time_ms,
            "Recall: injecting context"
        );

        Some(Message::user(format!(
            "[SYSTEM: Relevant memories recalled for context]\n{}",
            context
        )))
    }
}

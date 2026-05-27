use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::util::extract_json_block;

pub struct LensProposeOntologyTool {
    client: Arc<dyn arawn_llm::LlmClient>,
    model: String,
}

impl LensProposeOntologyTool {
    pub fn new(client: Arc<dyn arawn_llm::LlmClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }
}

#[async_trait]
impl Tool for LensProposeOntologyTool {
    fn name(&self) -> &str {
        "lens_propose_ontology"
    }

    fn description(&self) -> &str {
        "Given a free-text description of a lens, propose an initial \
         tag ontology (closed list of tag slugs the extractor will be allowed \
         to use). Returns `{ tags: [...], rationale: \"...\" }`. The agent \
         calls this during the create flow, shows the proposal to the user, \
         iterates if needed, then calls `lens_new` with the agreed list. \
         Tag slugs are short (`lowercase-with-dashes`), describe the kinds of \
         things you'll track (projects, people, processes), and should number \
         5–12 — keep it focused; new tags grow into the ontology via the \
         tag-promoter steward subroutine over time."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Free-text description of the lens — what it tracks, who's involved, what's in scope."
                }
            },
            "required": ["description"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let description = match params.get("description").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => {
                return Ok(ToolOutput::error("description is required".to_string()));
            }
        };

        let system = "You propose an initial tag ontology for a personal \
                      knowledge-base lens. Output ONLY a JSON object: \
                      {\"tags\": [array of 5–12 lowercase slug strings], \
                      \"rationale\": \"one short paragraph explaining what \
                      kinds of things each cluster of tags captures\"}.\n\n\
                      Tag slug rules:\n\
                      - lowercase letters, digits, '-' only (e.g. \
                      `on-call`, `rfc`, `falcon`, `house-rules`)\n\
                      - prefer concrete identifiers (project names, system \
                      names, person names, ritual names) over generic \
                      categories — these form natural clusters\n\
                      - include 1–2 broad categorical tags (e.g. \
                      `infrastructure`, `process`) so the ontology has \
                      buckets for content that doesn't fit a specific name\n\
                      - tags are a STARTING point. They grow over time via \
                      the tag-promoter subroutine. Don't try to anticipate \
                      everything — pick 5–12 that cover the obvious shape \
                      of this lens.";
        let user = format!(
            "Lens description:\n{description}\n\n\
             Propose the initial ontology.",
        );

        let raw = match propose_llm_call(&self.client, &self.model, system, &user).await {
            Ok(s) => s,
            Err(e) => {
                return Ok(ToolOutput::error(format!("LLM call failed: {e}")));
            }
        };
        let json_block = match extract_json_block(&raw) {
            Some(s) => s,
            None => {
                return Ok(ToolOutput::error(format!(
                    "no JSON block in LLM response: {raw}"
                )));
            }
        };
        #[derive(serde::Deserialize)]
        struct Proposal {
            tags: Vec<String>,
            #[serde(default)]
            rationale: String,
        }
        let mut proposal: Proposal = match serde_json::from_str(json_block) {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "couldn't parse LLM JSON: {e} — raw: {raw}"
                )));
            }
        };
        // Normalize tags (lowercase + trim), dedupe, drop empties.
        let mut seen = std::collections::HashSet::new();
        proposal.tags = proposal
            .tags
            .into_iter()
            .map(|t| arawn_memory::normalize_tag(&t))
            .filter(|t| !t.is_empty() && seen.insert(t.clone()))
            .collect();

        Ok(ToolOutput::success(
            json!({
                "tags": proposal.tags,
                "rationale": proposal.rationale,
            })
            .to_string(),
        ))
    }
}

/// Tiny streaming-drain helper. Mirrors `arawn-extractor::llm_text::complete_text`
/// and `arawn-steward::llm_text::complete_text`. There are now 3 consumers;
/// the right home is `arawn-llm` but consolidation is a separate cleanup pass.
async fn propose_llm_call(
    client: &Arc<dyn arawn_llm::LlmClient>,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    use futures::StreamExt;
    let req = arawn_llm::types::ChatRequest {
        model: model.to_string(),
        system_prompt: Some(system.to_string()),
        messages: vec![arawn_llm::types::ChatMessage {
            role: "user".to_string(),
            content: arawn_llm::types::ChatContent::Text(user.to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        max_tokens: None,
    };
    let _gate = arawn_llm::gate::acquire_local()
        .await
        .map_err(|e| format!("llm gate refused acquire: {e:?}"))?;
    let mut stream = client.stream(req).await.map_err(|e| e.to_string())?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if let arawn_llm::types::ChatChunk::TextDelta { text } = chunk {
            out.push_str(&text);
        }
    }
    Ok(out)
}

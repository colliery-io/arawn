//! LLM-based classifier that turns memory-write text into a structured
//! `PersonIntent` — *"Sarah is someone I manage"* → `Person(Sarah)` +
//! `relation_to_user=Manages`; *"Marcus reports to Sarah"* → two Persons
//! plus a `Manages` graph edge.
//!
//! ARAWN-I-0064 T-C. The classifier is optional on
//! `MemoryStoreTool` — when not configured, the tool stores the entity
//! as before with no relation inference. When configured, every
//! `memory_store` call with `entity_type=person` runs through the
//! classifier; the resulting intent is applied atomically alongside the
//! entity write.
//!
//! Pattern mirrors `propose_ontology::propose_llm_call`: streaming-drain
//! one-shot completion with a JSON-only prompt, then `serde_json` parse.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use arawn_llm::{ChatContent, ChatMessage, ChatRequest, LlmClient};
use arawn_memory::RelationToUser;

/// Two non-self people in a manager/report relationship the user has
/// described (e.g. *"Marcus reports to Sarah"*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetweenPeople {
    /// The person who manages — written as the source of a `Manages`
    /// graph edge in the org graph.
    pub manager_name: String,
    /// The person being managed — written as the target.
    pub report_name: String,
}

/// Structured intent extracted from a memory-store text by the LLM
/// classifier. All fields are optional; when nothing is detected, the
/// classifier returns a "Person, no relations" intent and the tool
/// behaves as if the classifier weren't configured.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonIntent {
    /// If the text describes a single person and how the *user* relates
    /// to them, this is the canonical name to use for the Person entity
    /// (e.g. "Sarah Lee" extracted from "Sarah Lee is my direct").
    /// `None` when no person is clearly named.
    #[serde(default)]
    pub primary_name: Option<String>,
    /// How the user relates to `primary_name`. `None` when the text
    /// doesn't describe a self-relation.
    #[serde(default)]
    pub relation_to_user: Option<RelationToUser>,
    /// When the text describes a relation between two non-self people
    /// (e.g. "Marcus reports to Sarah"), both names + which is the
    /// manager. Mutually exclusive with `relation_to_user` semantically;
    /// the classifier is instructed to set one or the other or neither.
    #[serde(default)]
    pub between_people: Option<BetweenPeople>,
}

const SYSTEM_PROMPT: &str = "\
You classify memory-write text describing people and their relationships \
to the user or to each other. Output strict JSON ONLY (no prose, no \
markdown fences).

The user is writing a memory note. Determine:
1. `primary_name`: if the text names a single person being introduced, \
   the canonical proper-noun name. Null otherwise.
2. `relation_to_user`: how that person relates to the *speaker (user)*:
   - \"manages\"          — user manages this person (e.g. \"X is my direct\", \
                            \"someone I manage\", \"X reports to me\")
   - \"reports_to_user\"  — user reports up to this person (e.g. \"X is my \
                            manager\", \"my boss\", \"my skip-level\")
   - \"peer_of_user\"     — peer at same level (e.g. \"X is a peer\", \"my \
                            counterpart\", \"works alongside me\")
   - null                  — no self-relation described
3. `between_people`: when the text describes a manager/report relationship \
   between two non-self people (e.g. \"Marcus reports to Sarah\", \"Sarah \
   manages Marcus\"), return `{\"manager_name\": \"Sarah\", \
   \"report_name\": \"Marcus\"}`. Null otherwise.

Output schema (strict):
{\"primary_name\": string|null, \
\"relation_to_user\": \"manages\"|\"reports_to_user\"|\"peer_of_user\"|null, \
\"between_people\": {\"manager_name\": string, \"report_name\": string}|null}

If the text isn't person-shaped at all, return \
{\"primary_name\":null,\"relation_to_user\":null,\"between_people\":null}.\
";

/// Classify a memory-write text into a structured `PersonIntent`. Returns
/// `Ok(default)` on transient LLM / parse failures so a classifier blip
/// never blocks a memory write — the caller stores the entity unchanged
/// and the user can amend later.
pub async fn classify_person_intent(
    text: &str,
    client: &Arc<dyn LlmClient>,
    model: &str,
) -> PersonIntent {
    let prompt = format!("Classify this memory-write text. Output JSON only.\n\nTEXT: {text}",);
    match run_classifier(&prompt, client, model).await {
        Ok(raw) => parse_intent(&raw),
        Err(e) => {
            warn!(error = %e, "person-intent classifier failed; falling back to empty intent");
            PersonIntent::default()
        }
    }
}

async fn run_classifier(
    prompt: &str,
    client: &Arc<dyn LlmClient>,
    model: &str,
) -> Result<String, String> {
    use futures::StreamExt;
    let req = ChatRequest {
        model: model.to_string(),
        system_prompt: Some(SYSTEM_PROMPT.to_string()),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(prompt.to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        max_tokens: Some(200),
    };
    let _gate = arawn_llm::gate::acquire_local()
        .await
        .map_err(|e| format!("llm gate refused: {e:?}"))?;
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

/// Lenient JSON extraction — strips common fence patterns (```json ... ```)
/// before parsing. Returns the empty intent on any parse failure.
fn parse_intent(raw: &str) -> PersonIntent {
    let trimmed = raw.trim();
    // Strip ```json ... ``` or ``` ... ``` fences if the model added them.
    let body = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.trim_start().trim_end_matches("```").trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.trim_start().trim_end_matches("```").trim()
    } else {
        trimmed
    };
    match serde_json::from_str::<PersonIntent>(body) {
        Ok(intent) => intent,
        Err(e) => {
            debug!(error = %e, raw = %body, "person-intent JSON parse failed");
            PersonIntent::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_intent_handles_plain_json() {
        let raw = r#"{"primary_name":"Sarah","relation_to_user":"manages","between_people":null}"#;
        let intent = parse_intent(raw);
        assert_eq!(intent.primary_name.as_deref(), Some("Sarah"));
        assert_eq!(intent.relation_to_user, Some(RelationToUser::Manages));
        assert!(intent.between_people.is_none());
    }

    #[test]
    fn parse_intent_handles_fenced_json() {
        let raw = "```json\n{\"primary_name\":\"Marcus\",\"relation_to_user\":null,\"between_people\":{\"manager_name\":\"Sarah\",\"report_name\":\"Marcus\"}}\n```";
        let intent = parse_intent(raw);
        assert_eq!(intent.primary_name.as_deref(), Some("Marcus"));
        assert!(intent.relation_to_user.is_none());
        let bp = intent.between_people.unwrap();
        assert_eq!(bp.manager_name, "Sarah");
        assert_eq!(bp.report_name, "Marcus");
    }

    #[test]
    fn parse_intent_handles_plain_fences_no_lang() {
        let raw =
            "```\n{\"primary_name\":null,\"relation_to_user\":null,\"between_people\":null}\n```";
        let intent = parse_intent(raw);
        assert_eq!(intent, PersonIntent::default());
    }

    #[test]
    fn parse_intent_returns_default_on_garbage() {
        let raw = "I cannot classify this.";
        let intent = parse_intent(raw);
        assert_eq!(intent, PersonIntent::default());
    }

    #[test]
    fn parse_intent_returns_default_on_partial_json() {
        let raw = r#"{"primary_name": "Sarah""#;
        let intent = parse_intent(raw);
        assert_eq!(intent, PersonIntent::default());
    }

    #[test]
    fn parse_intent_accepts_each_relation_to_user_variant() {
        for (json_val, expected) in [
            ("manages", RelationToUser::Manages),
            ("reports_to_user", RelationToUser::ReportsToUser),
            ("peer_of_user", RelationToUser::PeerOfUser),
        ] {
            let raw = format!(
                r#"{{"primary_name":"X","relation_to_user":"{json_val}","between_people":null}}"#
            );
            let intent = parse_intent(&raw);
            assert_eq!(intent.relation_to_user, Some(expected));
        }
    }
}

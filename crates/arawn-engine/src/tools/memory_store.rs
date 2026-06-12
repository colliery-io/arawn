use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, info};

use arawn_embed::Embedder;
use arawn_llm::LlmClient;
use arawn_memory::{
    ConfidenceSource, Entity, EntityType, PersonProfile, RelationToUser, RelationType, Scope,
    StoreFactResult,
};

use crate::lens_router::MemoryHandle;
use crate::person_intent::{BetweenPeople, PersonIntent, classify_person_intent};
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Tool that stores knowledge in the KB with search-before-create deduplication.
pub struct MemoryStoreTool {
    memory: MemoryHandle,
    embedder: Option<Arc<dyn Embedder>>,
    /// Optional LLM classifier — when configured (ARAWN-I-0064 T-C), any
    /// `entity_type=person` write runs through `classify_person_intent`
    /// and the resulting `PersonProfile` sidecar + Person↔Person graph
    /// edges are written atomically alongside the entity.
    classifier_llm: Option<Arc<dyn LlmClient>>,
    classifier_model: Option<String>,
}

impl MemoryStoreTool {
    pub fn new(memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>) -> Self {
        Self {
            memory: memory.into(),
            embedder,
            classifier_llm: None,
            classifier_model: None,
        }
    }

    /// Attach the LLM classifier used to detect social-relation phrasings
    /// in person memories. Without this, the tool behaves exactly as
    /// before — it stores the Person entity with no PersonProfile or
    /// relations written.
    pub fn with_classifier(mut self, client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self {
        self.classifier_llm = Some(client);
        self.classifier_model = Some(model.into());
        self
    }

    /// Run the LLM classifier on a Person memory text. Returns `None`
    /// when the entity type isn't Person, when the classifier isn't
    /// configured, or when the classifier itself decides nothing
    /// person-shaped is present (the empty intent — we treat that
    /// identically to "not configured" so the caller doesn't need to
    /// distinguish).
    async fn maybe_classify(
        &self,
        entity_type: EntityType,
        title: &str,
        content: Option<&str>,
    ) -> Option<PersonIntent> {
        if !matches!(entity_type, EntityType::Person) {
            return None;
        }
        let (client, model) = match (&self.classifier_llm, &self.classifier_model) {
            (Some(c), Some(m)) => (c, m),
            _ => return None,
        };
        let text = match content {
            Some(c) if !c.is_empty() => format!("{title}. {c}"),
            _ => title.to_string(),
        };
        let intent = classify_person_intent(&text, client, model).await;
        // Empty intent = nothing to apply. Don't pay downstream cost.
        if intent.between_people.is_none()
            && intent.relation_to_user.is_none()
            && intent.primary_name.is_none()
        {
            None
        } else {
            Some(intent)
        }
    }

    /// Apply a `between_people` intent: create / find both Person
    /// entities and write `Manages` + `ReportsTo` graph edges between
    /// them. Returns a tool output describing the org-relation write
    /// (no per-entity-id detail — the agent rarely needs those when the
    /// user's intent was the relation itself).
    async fn write_between_people_relation(
        &self,
        store: &arawn_memory::MemoryStore,
        bp: &BetweenPeople,
        session_id: uuid::Uuid,
    ) -> Result<ToolOutput, ToolError> {
        let manager_id = ensure_person(store, &bp.manager_name, session_id)?;
        let report_id = ensure_person(store, &bp.report_name, session_id)?;
        store
            .add_relation(manager_id, RelationType::Manages, report_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("write Manages edge: {e}")))?;
        store
            .add_relation(report_id, RelationType::ReportsTo, manager_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("write ReportsTo edge: {e}")))?;
        info!(
            manager = %bp.manager_name,
            report = %bp.report_name,
            "recorded org relation between two non-self people",
        );
        Ok(ToolOutput::success(format!(
            "Recorded org relation: {} manages {} (both Person entities ensured; \
             Manages + ReportsTo edges written).",
            bp.manager_name, bp.report_name
        )))
    }
}

/// Ensure a Person entity exists for the given name. Search-before-create
/// via `store_fact` returns the same id on subsequent calls.
fn ensure_person(
    store: &arawn_memory::MemoryStore,
    name: &str,
    session_id: uuid::Uuid,
) -> Result<uuid::Uuid, ToolError> {
    let entity = Entity::new(EntityType::Person, name)
        .with_confidence(ConfidenceSource::Stated)
        .with_session(session_id);
    let result = store
        .store_fact(&entity)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure_person `{name}`: {e}")))?;
    Ok(match result {
        StoreFactResult::Inserted { entity_id } => entity_id,
        StoreFactResult::Reinforced { entity_id, .. } => entity_id,
        StoreFactResult::Superseded { new_entity_id, .. } => new_entity_id,
    })
}

/// Merge a `relation_to_user` into the PersonProfile sidecar. Preserves
/// any pre-existing structured fields (role, growth_areas, etc.) — we
/// only flip the `relation_to_user` column + bump `updated_at`.
fn upsert_relation_to_user(
    store: &arawn_memory::MemoryStore,
    entity_id: uuid::Uuid,
    rel: RelationToUser,
) -> Result<(), arawn_memory::MemoryError> {
    let mut profile = store
        .get_person_profile(entity_id)?
        .unwrap_or_else(|| PersonProfile::new(entity_id));
    profile.relation_to_user = Some(rel);
    profile.updated_at = chrono::Utc::now();
    store.upsert_person_profile(&profile)
}

#[async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a memory — a global statement of fact or behavioral tuning that should be generally \
         known across every session and lens (e.g. \"Pat Collins is someone I manage\"). Memories are \
         always global; they are not filed into a lens. Uses search-before-create to avoid \
         duplicates — if the same fact already exists, it's reinforced instead of duplicated.\n\n\
         Entity types:\n\
         - **fact**: Known facts (\"project uses PostgreSQL 15\")\n\
         - **decision**: Choices made (\"went with microservices architecture\")\n\
         - **convention**: Patterns/rules (\"tests go inline, not in separate files\")\n\
         - **preference**: User preferences (\"prefers terse responses\")\n\
         - **person**: People (\"Alice — backend lead\")\n\
         - **note**: Freeform annotations\n\n\
         Use this when you learn something worth remembering across sessions. (Signals extracted \
         from feeds are a separate concern and are written by the extractor, not this tool.)"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Concise title for the knowledge entity"
                },
                "entity_type": {
                    "type": "string",
                    "enum": ["fact", "decision", "convention", "preference", "person", "note"],
                    "description": "Type of knowledge being stored"
                },
                "content": {
                    "type": "string",
                    "description": "Detailed content (markdown). Optional — title alone may suffice for simple facts."
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Categorization tags for filtering"
                }
            },
            "required": ["title", "entity_type"]
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'title' parameter".into()))?;

        let type_str = params
            .get("entity_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'entity_type' parameter".into()))?;

        let entity_type = EntityType::from_str(type_str).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("unknown entity_type: '{type_str}'"))
        })?;

        let content = params.get("content").and_then(|v| v.as_str());

        let tags: Vec<String> = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // ARAWN-I-0061: memory is always global. No lens write-target.
        let scope = Scope::Global;

        // Memory is global (ARAWN-I-0061): route to the global store, never a lens.
        let manager = self
            .memory
            .manager()
            .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))?;
        let store = manager.store_for(scope);

        // ARAWN-I-0064 T-C: when this is a Person write and the classifier
        // is configured, classify the text BEFORE storing. The classifier
        // may tell us the user is actually describing a relation between
        // two other people (e.g. "Marcus reports to Sarah") — in which
        // case we short-circuit, write two Person entities + a typed
        // graph edge, and skip creating a single mis-titled entity.
        let person_intent = self.maybe_classify(entity_type, title, content).await;

        if let Some(intent) = person_intent.as_ref()
            && let Some(bp) = intent.between_people.as_ref()
        {
            return self
                .write_between_people_relation(store, bp, ctx.session_id())
                .await;
        }

        // Build entity
        let mut entity = Entity::new(entity_type, title)
            .with_confidence(ConfidenceSource::Stated)
            .with_tags(tags)
            .with_session(ctx.session_id());

        if let Some(c) = content {
            entity = entity.with_content(c);
        }

        // Search-before-create via store_fact
        let result = store
            .store_fact(&entity)
            .map_err(|e| ToolError::ExecutionFailed(format!("memory store error: {e}")))?;

        // Embed if embedder available
        if let Some(ref embedder) = self.embedder {
            let text_to_embed = format!("{} {}", title, content.unwrap_or(""));
            match embedder.embed(&text_to_embed).await {
                Ok(embedding) => {
                    let entity_id = match &result {
                        StoreFactResult::Inserted { entity_id } => *entity_id,
                        StoreFactResult::Reinforced { entity_id, .. } => *entity_id,
                        StoreFactResult::Superseded { new_entity_id, .. } => *new_entity_id,
                    };
                    if let Err(e) = store.store_embedding(entity_id, &embedding) {
                        debug!(error = %e, "failed to store embedding (non-fatal)");
                    }
                }
                Err(e) => {
                    debug!(error = %e, "failed to embed entity (non-fatal)");
                }
            }
        }

        // Add extracted_from relation to session
        let entity_id = match &result {
            StoreFactResult::Inserted { entity_id } => *entity_id,
            StoreFactResult::Reinforced { entity_id, .. } => *entity_id,
            StoreFactResult::Superseded { new_entity_id, .. } => *new_entity_id,
        };

        // Create a session-reference entity ID from the session UUID
        // (We use the session ID directly as a relation target — it doesn't need
        // to be a stored entity, just a UUID for provenance tracking)
        let _ = store.add_relation(entity_id, RelationType::ExtractedFrom, ctx.session_id());

        // ARAWN-I-0064 T-C: if classifier detected the user's own relation
        // to this person, upsert the PersonProfile sidecar with it. We
        // merge with any existing profile so an earlier role/concerns row
        // isn't clobbered when only the relation is being captured.
        let mut relation_note = String::new();
        if let Some(intent) = person_intent.as_ref()
            && let Some(rel) = intent.relation_to_user
        {
            if let Err(e) = upsert_relation_to_user(store, entity_id, rel) {
                debug!(error = %e, "failed to upsert PersonProfile (non-fatal)");
            } else {
                relation_note = format!(" [relation_to_user={}]", rel.as_str());
            }
        }

        // Format output
        let scope_label = match scope {
            Scope::Global => "global",
            Scope::Lens => "lens",
        };

        match result {
            StoreFactResult::Inserted { entity_id } => {
                info!(id = %entity_id, title, scope = scope_label, "new entity stored");
                Ok(ToolOutput::success(format!(
                    "Stored new {type_str} in {scope_label} KB: \"{title}\" (id: {entity_id}){relation_note}"
                )))
            }
            StoreFactResult::Reinforced {
                entity_id,
                new_count,
            } => {
                info!(id = %entity_id, title, count = new_count, "entity reinforced");
                Ok(ToolOutput::success(format!(
                    "Reinforced existing {type_str}: \"{title}\" (now confirmed {new_count} times, id: {entity_id}){relation_note}"
                )))
            }
            StoreFactResult::Superseded {
                old_entity_id,
                new_entity_id,
            } => {
                info!(old = %old_entity_id, new = %new_entity_id, title, "entity superseded");
                Ok(ToolOutput::success(format!(
                    "Superseded old {type_str} with: \"{title}\" (old: {old_entity_id} → new: {new_entity_id}){relation_note}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Lens;
    use arawn_memory::MemoryManager;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (
        TempDir,
        Arc<MemoryManager>,
        crate::context::EngineToolContext,
    ) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("lenses/test-ws")).unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "test-ws", None).unwrap());
        let ws = Lens::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, ctx)
    }

    /// Mock LLM client that returns a canned JSON intent on every call.
    /// Mirrors the `MockLlm` pattern in `query_engine.rs` tests — kept
    /// inline so it doesn't leak into production exports.
    struct CannedIntentLlm {
        canned_json: String,
    }

    impl CannedIntentLlm {
        fn returning(json: impl Into<String>) -> Arc<Self> {
            Arc::new(Self {
                canned_json: json.into(),
            })
        }
    }

    #[async_trait]
    impl arawn_llm::LlmClient for CannedIntentLlm {
        async fn stream(
            &self,
            _request: arawn_llm::ChatRequest,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<
                            Item = Result<arawn_llm::ChatChunk, arawn_llm::error::LlmError>,
                        > + Send,
                >,
            >,
            arawn_llm::error::LlmError,
        > {
            let chunks = vec![
                Ok(arawn_llm::ChatChunk::TextDelta {
                    text: self.canned_json.clone(),
                }),
                Ok(arawn_llm::ChatChunk::Done {
                    usage: None,
                    finish_reason: None,
                }),
            ];
            Ok(Box::pin(futures::stream::iter(chunks)))
        }
    }

    #[tokio::test]
    async fn store_new_fact() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        let result = tool
            .execute(
                &ctx,
                json!({"title": "Rust is fast", "entity_type": "fact"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Stored new fact"));
        // ARAWN-I-0061: memory is global.
        assert_eq!(mgr.global.count_all().unwrap(), 1);
        assert_eq!(mgr.lens.count_all().unwrap(), 0);
    }

    #[tokio::test]
    async fn store_preference_goes_global() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(
            &ctx,
            json!({"title": "Prefers terse output", "entity_type": "preference"}),
        )
        .await
        .unwrap();

        assert_eq!(mgr.global.count_all().unwrap(), 1);
        assert_eq!(mgr.lens.count_all().unwrap(), 0);
    }

    #[tokio::test]
    async fn store_decision_goes_global() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(
            &ctx,
            json!({"title": "Use microservices", "entity_type": "decision"}),
        )
        .await
        .unwrap();

        // ARAWN-I-0061: all memory is global, including decisions.
        assert_eq!(mgr.global.count_all().unwrap(), 1);
        assert_eq!(mgr.lens.count_all().unwrap(), 0);
    }

    #[tokio::test]
    async fn store_reinforces_duplicate() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(
            &ctx,
            json!({"title": "Rust is fast", "entity_type": "fact"}),
        )
        .await
        .unwrap();

        let result = tool
            .execute(
                &ctx,
                json!({"title": "Rust is fast", "entity_type": "fact"}),
            )
            .await
            .unwrap();

        assert!(result.content.contains("Reinforced"));
        assert_eq!(mgr.global.count_all().unwrap(), 1);
    }

    #[tokio::test]
    async fn store_with_tags() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(
            &ctx,
            json!({"title": "Tagged fact", "entity_type": "fact", "tags": ["rust", "perf"]}),
        )
        .await
        .unwrap();

        let results = mgr.global.search_by_tags(&["rust".into()], 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    // === ARAWN-I-0064 T-C: classifier integration ===

    #[tokio::test]
    async fn classifier_skipped_when_entity_type_is_not_person() {
        // Classifier set, but entity_type=fact — classifier must NOT run
        // (and the mock would have no scripted call). Tool succeeds.
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None)
            .with_classifier(CannedIntentLlm::returning(""), "test-model");

        let result = tool
            .execute(
                &ctx,
                json!({"title": "Rust is fast", "entity_type": "fact"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("Stored new fact"));
        // No PersonProfile rows because no Person was written.
        assert_eq!(mgr.global.count_by_type(EntityType::Person).unwrap(), 0);
    }

    #[tokio::test]
    async fn classifier_writes_person_profile_when_relation_to_user_detected() {
        let (_tmp, mgr, ctx) = setup();
        let canned =
            r#"{"primary_name":"Sarah Lee","relation_to_user":"manages","between_people":null}"#;
        let tool = MemoryStoreTool::new(mgr.clone(), None)
            .with_classifier(CannedIntentLlm::returning(canned), "test-model");

        let result = tool
            .execute(
                &ctx,
                json!({
                    "title": "Sarah Lee",
                    "entity_type": "person",
                    "content": "Sarah is someone I manage."
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        assert!(result.content.contains("relation_to_user=manages"));

        // Find the Person, then fetch its profile.
        let people = mgr.global.list_by_type(EntityType::Person, 10).unwrap();
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].title, "Sarah Lee");
        let profile = mgr
            .global
            .get_person_profile(people[0].id)
            .unwrap()
            .unwrap();
        assert_eq!(profile.relation_to_user, Some(RelationToUser::Manages));
    }

    #[tokio::test]
    async fn classifier_short_circuits_for_between_people_phrasings() {
        // "Marcus reports to Sarah" — agent passed title="Marcus reports to Sarah"
        // with entity_type=person. Classifier returns between_people, so the
        // tool writes two Persons (Sarah, Marcus) + Manages/ReportsTo edges
        // and does NOT create a single mis-titled entity.
        let (_tmp, mgr, ctx) = setup();
        let canned = r#"{"primary_name":null,"relation_to_user":null,"between_people":{"manager_name":"Sarah","report_name":"Marcus"}}"#;
        let tool = MemoryStoreTool::new(mgr.clone(), None)
            .with_classifier(CannedIntentLlm::returning(canned), "test-model");

        let result = tool
            .execute(
                &ctx,
                json!({
                    "title": "Marcus reports to Sarah",
                    "entity_type": "person"
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        assert!(result.content.contains("Sarah manages Marcus"));

        // Two Persons exist; no entity named "Marcus reports to Sarah".
        let people = mgr.global.list_by_type(EntityType::Person, 10).unwrap();
        assert_eq!(people.len(), 2);
        assert!(people.iter().any(|p| p.title == "Sarah"));
        assert!(people.iter().any(|p| p.title == "Marcus"));
        assert!(
            people.iter().all(|p| p.title != "Marcus reports to Sarah"),
            "mis-titled stub leaked: {:?}",
            people.iter().map(|p| &p.title).collect::<Vec<_>>()
        );

        // Relations: manager has Manages → report; report has ReportsTo → manager.
        let sarah = people.iter().find(|p| p.title == "Sarah").unwrap();
        let marcus = people.iter().find(|p| p.title == "Marcus").unwrap();
        let manages_edges = mgr.global.get_relations(sarah.id).unwrap();
        assert!(
            manages_edges
                .iter()
                .any(|r| r.relation_type == RelationType::Manages && r.target_id == marcus.id)
        );
        let reports_edges = mgr.global.get_relations(marcus.id).unwrap();
        assert!(
            reports_edges
                .iter()
                .any(|r| r.relation_type == RelationType::ReportsTo && r.target_id == sarah.id)
        );
    }

    #[tokio::test]
    async fn classifier_empty_intent_falls_through_to_normal_store() {
        // Classifier returns the empty intent — tool stores the Person
        // entity as before, no profile, no edges.
        let (_tmp, mgr, ctx) = setup();
        let canned = r#"{"primary_name":null,"relation_to_user":null,"between_people":null}"#;
        let tool = MemoryStoreTool::new(mgr.clone(), None)
            .with_classifier(CannedIntentLlm::returning(canned), "test-model");

        let result = tool
            .execute(
                &ctx,
                json!({"title": "Pat Collins", "entity_type": "person"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(!result.content.contains("relation_to_user"));
        let people = mgr.global.list_by_type(EntityType::Person, 10).unwrap();
        assert_eq!(people.len(), 1);
        assert!(
            mgr.global
                .get_person_profile(people[0].id)
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn classifier_failure_falls_through_to_normal_store() {
        // Classifier returns garbage (parse fails) — falls back to default
        // (empty) intent which == no special handling. Memory write still
        // succeeds; the tool never blocks on a classifier blip.
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None)
            .with_classifier(CannedIntentLlm::returning("not json at all"), "test-model");

        let result = tool
            .execute(
                &ctx,
                json!({"title": "Pat Collins", "entity_type": "person"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert_eq!(mgr.global.count_by_type(EntityType::Person).unwrap(), 1);
    }
}

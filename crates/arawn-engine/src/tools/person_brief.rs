//! `person_brief` — "what's been going on with X" orchestrator.
//!
//! ARAWN-I-0065 T-A. The single highest-frequency exec ask gets a dedicated
//! tool that reads:
//!   1. The named `Person` entity (exact-title match first, FTS fallback)
//!   2. Its `PersonProfile` sidecar (role, relation_to_user, last_1on1,
//!      pronouns, time_zone, growth_areas, current_concerns)
//!   3. Other memory entries mentioning the person (FTS over global)
//!   4. Graph-related entities (Manages / ReportsTo / PeerOf neighbors plus
//!      generic RelatesTo / Mentions edges if any)
//!
//! Calendar / Gmail / Slack fanout is deliberately deferred to a follow-up
//! task — landing those needs integration-layer access at construction time
//! and inflates this task from M to L. The memory-only first cut is the
//! single highest-leverage delta for the daily-driver.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_embed::Embedder;
use arawn_memory::{Entity, EntityType, MemoryStore, PersonProfile, RelationToUser, RelationType};

use crate::lens_router::MemoryHandle;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

pub struct PersonBriefTool {
    memory: MemoryHandle,
    #[allow(dead_code)]
    embedder: Option<Arc<dyn Embedder>>,
}

impl PersonBriefTool {
    pub fn new(memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>) -> Self {
        Self {
            memory: memory.into(),
            embedder,
        }
    }
}

#[async_trait]
impl Tool for PersonBriefTool {
    fn name(&self) -> &str {
        "person_brief"
    }

    fn description(&self) -> &str {
        "Brief on a person — role, relation to you (manage / report to / peer), \
         last 1:1, current concerns, growth themes, and recent memory entries \
         mentioning them. Pass `name` to look them up (exact title match preferred, \
         FTS fallback for partial matches). Use this when someone surfaces in the \
         user's day — a Slack thread, calendar slot, decision point — and you need \
         fast context before responding."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Person's name (matches Entity title; FTS fallback if no exact match)"
                },
                "include_related": {
                    "type": "boolean",
                    "description": "Include graph-related entities (default: true)"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'name' parameter".into()))?;
        let include_related = params
            .get("include_related")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let manager = self
            .memory
            .manager()
            .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))?;
        let store = &manager.global;

        // 1. Resolve Person entity: exact-title match first, FTS-like
        //    fallback (case-insensitive substring) for partial names.
        let person = match resolve_person(store, name)? {
            Some(p) => p,
            None => {
                return Ok(ToolOutput::success(format!(
                    "No Person entity matching '{name}'. \
                     Try `memory_store(entity_type=\"person\", title=\"{name}\", \
                     content=\"…\")` to capture them — phrasings like \
                     \"{name} is someone I manage\" auto-populate the relation.",
                )));
            }
        };

        // 2. PersonProfile sidecar (optional — None means unstructured).
        let profile = store
            .get_person_profile(person.id)
            .map_err(|e| ToolError::ExecutionFailed(format!("read profile: {e}")))?;

        // 3. Other memory entries mentioning the person.
        let mut memory_hits: Vec<Entity> = store
            .search(name, 10)
            .map_err(|e| ToolError::ExecutionFailed(format!("FTS search: {e}")))?
            .into_iter()
            .filter(|e| e.id != person.id)
            .collect();
        memory_hits.truncate(5);

        // 4. Graph-related entities (optional).
        let graph_related: Vec<(RelationType, String)> = if include_related {
            store
                .get_relations(person.id)
                .ok()
                .map(|rels| {
                    rels.iter()
                        .filter_map(|rel| {
                            let neighbor_id = if rel.source_id == person.id {
                                rel.target_id
                            } else {
                                rel.source_id
                            };
                            store
                                .get_entity(neighbor_id)
                                .ok()
                                .flatten()
                                .map(|e| (rel.relation_type, e.title))
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(ToolOutput::success(format_brief(
            &person,
            profile.as_ref(),
            &memory_hits,
            &graph_related,
        )))
    }
}

/// Find the Person entity matching `name`. Exact title (case-insensitive)
/// wins; substring match is the fallback so "Sarah" finds "Sarah Lee".
fn resolve_person(store: &MemoryStore, name: &str) -> Result<Option<Entity>, ToolError> {
    let people = store
        .list_by_type(EntityType::Person, 200)
        .map_err(|e| ToolError::ExecutionFailed(format!("list persons: {e}")))?;
    if let Some(exact) = people.iter().find(|e| e.title.eq_ignore_ascii_case(name)) {
        return Ok(Some(exact.clone()));
    }
    let needle = name.to_lowercase();
    if let Some(partial) = people
        .iter()
        .find(|e| e.title.to_lowercase().contains(&needle))
    {
        return Ok(Some(partial.clone()));
    }
    Ok(None)
}

/// Render the brief as markdown so it threads into chat naturally.
fn format_brief(
    person: &Entity,
    profile: Option<&PersonProfile>,
    memory_hits: &[Entity],
    graph_related: &[(RelationType, String)],
) -> String {
    let mut out = format!("# {}\n\n", person.title);

    match profile {
        Some(p) => {
            // Identity line: role, relation, pronouns.
            let mut id_parts: Vec<String> = Vec::new();
            if let Some(role) = &p.role {
                id_parts.push(format!("**{role}**"));
            }
            if let Some(rel) = p.relation_to_user {
                let label = match rel {
                    RelationToUser::Manages => "you manage",
                    RelationToUser::ReportsToUser => "your manager",
                    RelationToUser::PeerOfUser => "peer",
                };
                id_parts.push(format!("(_{label}_)"));
            }
            if let Some(pn) = &p.pronouns {
                id_parts.push(format!("[{pn}]"));
            }
            if !id_parts.is_empty() {
                out.push_str(&id_parts.join(" "));
                out.push_str("\n\n");
            }

            // Cadence + locale.
            if let Some(ts) = p.last_1on1 {
                out.push_str(&format!("- Last 1:1: {}\n", ts.format("%Y-%m-%d")));
            }
            if let Some(tz) = &p.time_zone {
                out.push_str(&format!("- Timezone: {tz}\n"));
            }
            if let Some(hd) = p.hire_date {
                out.push_str(&format!("- Hired: {hd}\n"));
            }

            if !p.growth_areas.is_empty() {
                out.push_str("\n## Growth themes\n");
                for g in &p.growth_areas {
                    out.push_str(&format!("- {g}\n"));
                }
            }
            if !p.current_concerns.is_empty() {
                out.push_str("\n## Current concerns\n");
                for c in &p.current_concerns {
                    out.push_str(&format!("- {c}\n"));
                }
            }
        }
        None => {
            out.push_str(
                "_(No structured profile yet — say something like \
                 \"X is someone I manage\" via `memory_store` to populate role, \
                 relation, growth themes, etc.)_\n",
            );
        }
    }

    // Free-text notes on the Person entity itself.
    if let Some(content) = &person.content
        && !content.is_empty()
    {
        out.push_str("\n## Notes\n");
        out.push_str(content);
        if !content.ends_with('\n') {
            out.push('\n');
        }
    }

    if !memory_hits.is_empty() {
        out.push_str("\n## Recent memory mentioning them\n");
        for hit in memory_hits {
            let snippet = hit
                .content
                .as_deref()
                .map(|c| c.chars().take(80).collect::<String>())
                .filter(|s| !s.is_empty())
                .map(|s| format!(" — {s}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "- **[{}]** {}{}\n",
                hit.entity_type.as_str(),
                hit.title,
                snippet
            ));
        }
    }

    if !graph_related.is_empty() {
        out.push_str("\n## Connected entities\n");
        for (rel, title) in graph_related {
            out.push_str(&format!("- {} {}\n", rel.as_str(), title));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Lens;
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager, RelationType};
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

    #[tokio::test]
    async fn returns_helpful_message_when_person_unknown() {
        let (_tmp, mgr, ctx) = setup();
        let tool = PersonBriefTool::new(mgr, None);
        let result = tool
            .execute(&ctx, json!({"name": "Unknown Person"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("No Person entity"));
        assert!(result.content.contains("Unknown Person"));
        assert!(result.content.contains("memory_store"));
    }

    #[tokio::test]
    async fn returns_brief_with_profile_role_and_relation() {
        let (_tmp, mgr, ctx) = setup();
        let sarah =
            Entity::new(EntityType::Person, "Sarah Lee").with_confidence(ConfidenceSource::Stated);
        mgr.global.insert_entity(&sarah).unwrap();
        mgr.global
            .upsert_person_profile(
                &PersonProfile::new(sarah.id)
                    .with_role("Senior EM")
                    .with_relation_to_user(RelationToUser::Manages),
            )
            .unwrap();

        let tool = PersonBriefTool::new(mgr, None);
        let result = tool
            .execute(&ctx, json!({"name": "Sarah Lee"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(
            result.content.contains("Sarah Lee"),
            "got:\n{}",
            result.content
        );
        assert!(result.content.contains("Senior EM"));
        assert!(result.content.contains("you manage"));
    }

    #[tokio::test]
    async fn falls_back_to_partial_match_when_no_exact_title() {
        let (_tmp, mgr, ctx) = setup();
        let sarah = Entity::new(EntityType::Person, "Sarah Lee");
        mgr.global.insert_entity(&sarah).unwrap();
        let tool = PersonBriefTool::new(mgr, None);
        let result = tool.execute(&ctx, json!({"name": "Sarah"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("Sarah Lee"));
    }

    #[tokio::test]
    async fn case_insensitive_exact_match_wins_over_partial() {
        let (_tmp, mgr, ctx) = setup();
        // Two persons: "Anita" and "Anita Patel". Query for "anita" should
        // hit the exact-title (case-insensitive) "Anita", not the longer
        // one even though both contain the substring.
        let exact = Entity::new(EntityType::Person, "Anita");
        let longer = Entity::new(EntityType::Person, "Anita Patel");
        mgr.global.insert_entity(&exact).unwrap();
        mgr.global.insert_entity(&longer).unwrap();
        let tool = PersonBriefTool::new(mgr, None);
        let result = tool.execute(&ctx, json!({"name": "anita"})).await.unwrap();
        assert!(!result.is_error);
        // Header is `# Anita`, not `# Anita Patel`.
        assert!(
            result.content.starts_with("# Anita\n"),
            "expected exact-match header, got:\n{}",
            result.content
        );
    }

    #[tokio::test]
    async fn renders_growth_themes_and_concerns_when_present() {
        let (_tmp, mgr, ctx) = setup();
        let person = Entity::new(EntityType::Person, "Marcus");
        mgr.global.insert_entity(&person).unwrap();
        let mut profile = PersonProfile::new(person.id).with_role("Staff Eng");
        profile.growth_areas = vec!["cross-team influence".into(), "public speaking".into()];
        profile.current_concerns = vec!["burnout risk in Q3".into()];
        mgr.global.upsert_person_profile(&profile).unwrap();

        let tool = PersonBriefTool::new(mgr, None);
        let result = tool.execute(&ctx, json!({"name": "Marcus"})).await.unwrap();
        assert!(result.content.contains("## Growth themes"));
        assert!(result.content.contains("cross-team influence"));
        assert!(result.content.contains("## Current concerns"));
        assert!(result.content.contains("burnout risk in Q3"));
    }

    #[tokio::test]
    async fn surfaces_unstructured_profile_hint_when_no_sidecar() {
        let (_tmp, mgr, ctx) = setup();
        let person = Entity::new(EntityType::Person, "Pat Collins");
        mgr.global.insert_entity(&person).unwrap();
        let tool = PersonBriefTool::new(mgr, None);
        let result = tool
            .execute(&ctx, json!({"name": "Pat Collins"}))
            .await
            .unwrap();
        assert!(result.content.contains("No structured profile yet"));
        assert!(result.content.contains("memory_store"));
    }

    #[tokio::test]
    async fn surfaces_related_memory_entries_mentioning_the_person() {
        let (_tmp, mgr, ctx) = setup();
        let person = Entity::new(EntityType::Person, "David Chen");
        mgr.global.insert_entity(&person).unwrap();
        // Unrelated fact mentioning David — should appear in the brief.
        let fact = Entity::new(
            EntityType::Decision,
            "Approved staffing plan with David Chen",
        );
        mgr.global.insert_entity(&fact).unwrap();

        let tool = PersonBriefTool::new(mgr, None);
        let result = tool
            .execute(&ctx, json!({"name": "David Chen"}))
            .await
            .unwrap();
        assert!(result.content.contains("Recent memory mentioning"));
        assert!(result.content.contains("Approved staffing plan"));
    }

    #[tokio::test]
    async fn lists_graph_related_when_include_related_default_on() {
        let (_tmp, mgr, ctx) = setup();
        let manager = Entity::new(EntityType::Person, "Sarah");
        let report = Entity::new(EntityType::Person, "Marcus");
        mgr.global.insert_entity(&manager).unwrap();
        mgr.global.insert_entity(&report).unwrap();
        mgr.global
            .add_relation(manager.id, RelationType::Manages, report.id)
            .unwrap();

        let tool = PersonBriefTool::new(mgr, None);
        let result = tool.execute(&ctx, json!({"name": "Sarah"})).await.unwrap();
        assert!(result.content.contains("## Connected entities"));
        assert!(result.content.contains("manages Marcus"));
    }

    #[tokio::test]
    async fn include_related_false_omits_graph_section() {
        let (_tmp, mgr, ctx) = setup();
        let manager = Entity::new(EntityType::Person, "Sarah");
        let report = Entity::new(EntityType::Person, "Marcus");
        mgr.global.insert_entity(&manager).unwrap();
        mgr.global.insert_entity(&report).unwrap();
        mgr.global
            .add_relation(manager.id, RelationType::Manages, report.id)
            .unwrap();

        let tool = PersonBriefTool::new(mgr, None);
        let result = tool
            .execute(&ctx, json!({"name": "Sarah", "include_related": false}))
            .await
            .unwrap();
        assert!(!result.content.contains("## Connected entities"));
    }

    #[tokio::test]
    async fn missing_name_param_errors() {
        let (_tmp, mgr, ctx) = setup();
        let tool = PersonBriefTool::new(mgr, None);
        let err = tool.execute(&ctx, json!({})).await.unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("name"));
    }
}

//! Per-(subroutine, action) "forward" dispatch — the opposite of
//! `rollback::apply_inverse`. Drives the proposal-accept path
//! (`workstream_apply <id>`).

use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use arawn_memory::{AddedVia, Entity, MemoryManager, RelationType, TagOntologyStore};

use crate::error::StewardError;
use crate::journal::JournalRow;

/// Context handed to the accept dispatch. Some subroutines (notably
/// `tag-promoter`) need access to the workstream's ontology table,
/// which lives next to (not inside) `MemoryManager`. The KB handle
/// alone isn't enough.
pub struct AcceptCtx<'a> {
    pub kb: &'a Arc<MemoryManager>,
    /// Path to the workstream's root directory (e.g.
    /// `<data_dir>/workstreams/<name>`). Used to open
    /// `TagOntologyStore` for the promotion accept path.
    pub workstream_root: &'a Path,
}

/// Apply the forward mutation described by `row.outputs_json`.
///
/// - `dust/summarize` → insert the proposed summary entity, add a
///   SUMMARIZES edge to each source entity.
/// - `map/propose_relation` → add the relation.
/// - `tag-promoter/promote_tag` → insert the tag into the workstream's
///   ontology with `added_via = 'promotion'`.
/// - `doorwatch/propose_identity` → no graph change; the journal row's
///   `applied = true` flip *is* the acceptance record.
/// - reshelve actions are already applied at the moment they were
///   journaled — accepting them again is a no-op (they're not
///   proposals).
pub fn apply_forward(row: &JournalRow, ctx: &AcceptCtx<'_>) -> Result<(), StewardError> {
    match (row.subroutine.as_str(), row.action.as_str()) {
        ("dust", "summarize") => dust_summarize(row, ctx.kb),
        ("map", "propose_relation") => map_propose_relation(row, ctx.kb),
        ("tag-promoter", "promote_tag") => promote_tag(row, ctx),
        ("doorwatch", "propose_identity") => Ok(()),
        ("reshelve", _) | ("identity", _) => Ok(()),
        (sub, act) => Err(StewardError::Subroutine {
            name: sub.into(),
            message: format!("no forward apply for action `{act}`"),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct PromoteOutputs {
    tag: String,
}

fn promote_tag(row: &JournalRow, ctx: &AcceptCtx<'_>) -> Result<(), StewardError> {
    let payload: PromoteOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("tag-promoter/promote_tag payload: {e}")))?;
    let ontology = TagOntologyStore::open_at(ctx.workstream_root)
        .map_err(StewardError::from)?;
    ontology.add(&payload.tag, AddedVia::Promotion)?;
    debug!(
        tag = %payload.tag,
        ws_dir = ?ctx.workstream_root,
        "accept: tag promoted into ontology"
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct DustOutputs {
    summary: Entity,
    source_ids: Vec<Uuid>,
}

fn dust_summarize(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    let payload: DustOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("dust/summarize payload: {e}")))?;
    kb.workstream.insert_entity(&payload.summary)?;
    for src in &payload.source_ids {
        if let Err(e) = kb
            .workstream
            .add_relation(payload.summary.id, RelationType::Summarizes, *src)
        {
            // One bad src shouldn't drop the whole apply.
            debug!(error = %e, src = %src, "dust apply: SUMMARIZES edge skipped");
        }
    }
    debug!(
        summary = %payload.summary.id,
        sources = payload.source_ids.len(),
        "accept: dust summary applied"
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct MapOutputs {
    from_id: Uuid,
    rel: String,
    to_id: Uuid,
}

fn map_propose_relation(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    let payload: MapOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("map/propose_relation payload: {e}")))?;
    let rel = RelationType::from_str(payload.rel.to_lowercase().as_str())
        .ok_or_else(|| StewardError::Parse(format!("unknown rel: {}", payload.rel)))?;
    kb.workstream
        .add_relation(payload.from_id, rel, payload.to_id)?;
    debug!(
        from = %payload.from_id,
        rel = rel.as_str(),
        to = %payload.to_id,
        "accept: map relation applied"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_memory::EntityType;
    use chrono::Utc;
    use serde_json::json;

    fn setup_kb() -> (tempfile::TempDir, Arc<MemoryManager>) {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "ws", None).unwrap());
        (tmp, mgr)
    }

    /// `MemoryManager::open(data_dir, "ws", _)` actually creates the
    /// KB at `<data_dir>/workstreams/ws/memory.db`. The matching
    /// workstream root for the ontology table is that subdirectory.
    fn ws_root(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        tmp.path().join("workstreams").join("ws")
    }

    fn row(sub: &str, act: &str, outputs: serde_json::Value) -> JournalRow {
        JournalRow {
            id: 1,
            ts: Utc::now(),
            subroutine: sub.into(),
            action: act.into(),
            inputs_json: "{}".into(),
            outputs_json: outputs.to_string(),
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: false,
            reverted_at: None,
        }
    }

    #[test]
    fn map_apply_adds_relation() {
        let (tmp, kb) = setup_kb();
        let a = Entity::new(EntityType::Fact, "a");
        let b = Entity::new(EntityType::Fact, "b");
        kb.workstream.insert_entity(&a).unwrap();
        kb.workstream.insert_entity(&b).unwrap();
        let r = row(
            "map",
            "propose_relation",
            json!({"from_id": a.id, "rel": "relates_to", "to_id": b.id}),
        );
        let root = ws_root(&tmp);
        apply_forward(&r, &AcceptCtx { kb: &kb, workstream_root: &root }).unwrap();
        let rels = kb.workstream.get_relations(a.id).unwrap();
        assert!(rels.iter().any(|x| x.target_id == b.id
            && matches!(x.relation_type, RelationType::RelatesTo)));
    }

    #[test]
    fn dust_apply_inserts_summary_and_edges() {
        let (tmp, kb) = setup_kb();
        let a = Entity::new(EntityType::Note, "old project a");
        let b = Entity::new(EntityType::Note, "old project b");
        kb.workstream.insert_entity(&a).unwrap();
        kb.workstream.insert_entity(&b).unwrap();
        let summary = Entity::new(EntityType::Note, "summary of project x")
            .with_content("compressed gist");
        let r = row(
            "dust",
            "summarize",
            json!({"summary": summary, "source_ids": [a.id, b.id]}),
        );
        let root = ws_root(&tmp);
        apply_forward(&r, &AcceptCtx { kb: &kb, workstream_root: &root }).unwrap();
        let fetched = kb.workstream.get_entity(summary.id).unwrap().unwrap();
        assert_eq!(fetched.title, "summary of project x");
        let edges = kb.workstream.get_relations(summary.id).unwrap();
        let summarizes: Vec<_> = edges
            .iter()
            .filter(|e| matches!(e.relation_type, RelationType::Summarizes))
            .collect();
        assert_eq!(summarizes.len(), 2);
    }

    #[test]
    fn doorwatch_apply_is_noop() {
        let (tmp, kb) = setup_kb();
        let r = row("doorwatch", "propose_identity", json!({"x": 1}));
        let root = ws_root(&tmp);
        apply_forward(&r, &AcceptCtx { kb: &kb, workstream_root: &root }).unwrap();
    }

    #[test]
    fn tag_promoter_apply_adds_to_ontology() {
        let (tmp, kb) = setup_kb();
        let root = ws_root(&tmp);
        // Ontology starts empty for "ws".
        let ont = TagOntologyStore::open_at(&root).unwrap();
        assert!(!ont.contains("calidor").unwrap());

        let r = row(
            "tag-promoter",
            "promote_tag",
            json!({"tag": "calidor", "count": 5, "sample_entity_ids": []}),
        );
        apply_forward(&r, &AcceptCtx { kb: &kb, workstream_root: &root }).unwrap();

        // Re-open to read; should now contain the tag with
        // added_via=Promotion.
        let ont = TagOntologyStore::open_at(&root).unwrap();
        assert!(ont.contains("calidor").unwrap());
        let entry = ont.get("calidor").unwrap().unwrap();
        assert_eq!(entry.added_via, AddedVia::Promotion);
    }

    #[test]
    fn unknown_action_errors() {
        let (tmp, kb) = setup_kb();
        let r = row("dust", "obscure", json!({}));
        let root = ws_root(&tmp);
        let err = apply_forward(&r, &AcceptCtx { kb: &kb, workstream_root: &root })
            .unwrap_err();
        assert!(matches!(err, StewardError::Subroutine { .. }));
    }
}

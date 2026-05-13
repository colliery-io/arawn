//! Per-(subroutine, action) "forward" dispatch — the opposite of
//! `rollback::apply_inverse`. Drives the proposal-accept path
//! (`workstream_apply <id>`).

use std::sync::Arc;

use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use arawn_memory::{Entity, MemoryManager, RelationType};

use crate::error::StewardError;
use crate::journal::JournalRow;

/// Apply the forward mutation described by `row.outputs_json` to `kb`.
///
/// - `dust/summarize` → insert the proposed summary entity, add a
///   SUMMARIZES edge to each source entity.
/// - `map/propose_relation` → add the relation.
/// - `doorwatch/propose_identity` → no graph change; the journal row's
///   `applied = true` flip *is* the acceptance record.
/// - reshelve actions are already applied at the moment they were
///   journaled — accepting them again is a no-op (they're not
///   proposals).
pub fn apply_forward(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    match (row.subroutine.as_str(), row.action.as_str()) {
        ("dust", "summarize") => dust_summarize(row, kb),
        ("map", "propose_relation") => map_propose_relation(row, kb),
        ("doorwatch", "propose_identity") => Ok(()),
        ("reshelve", _) | ("identity", _) => Ok(()),
        (sub, act) => Err(StewardError::Subroutine {
            name: sub.into(),
            message: format!("no forward apply for action `{act}`"),
        }),
    }
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
        let (_tmp, kb) = setup_kb();
        let a = Entity::new(EntityType::Fact, "a");
        let b = Entity::new(EntityType::Fact, "b");
        kb.workstream.insert_entity(&a).unwrap();
        kb.workstream.insert_entity(&b).unwrap();
        let r = row(
            "map",
            "propose_relation",
            json!({"from_id": a.id, "rel": "relates_to", "to_id": b.id}),
        );
        apply_forward(&r, &kb).unwrap();
        let rels = kb.workstream.get_relations(a.id).unwrap();
        assert!(rels.iter().any(|x| x.target_id == b.id
            && matches!(x.relation_type, RelationType::RelatesTo)));
    }

    #[test]
    fn dust_apply_inserts_summary_and_edges() {
        let (_tmp, kb) = setup_kb();
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
        apply_forward(&r, &kb).unwrap();
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
        let (_tmp, kb) = setup_kb();
        let r = row("doorwatch", "propose_identity", json!({"x": 1}));
        apply_forward(&r, &kb).unwrap();
    }

    #[test]
    fn unknown_action_errors() {
        let (_tmp, kb) = setup_kb();
        let r = row("dust", "obscure", json!({}));
        let err = apply_forward(&r, &kb).unwrap_err();
        assert!(matches!(err, StewardError::Subroutine { .. }));
    }
}

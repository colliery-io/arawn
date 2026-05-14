//! Per-subroutine rollback logic.
//!
//! `Journal::revert` flips the metadata; `rollback::apply_inverse` is
//! where the *actual* inverse mutation lives. We dispatch on
//! `(subroutine, action)` so the contract stays in one place.

use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use arawn_memory::{Entity, MemoryManager, RelationType, TagOntologyStore};

use crate::error::StewardError;
use crate::journal::JournalRow;

/// Context handed to the rollback dispatch — mirrors `accept::AcceptCtx`.
/// Some inverses need ontology access (the tag-promoter promotion
/// reversal removes the tag from the workstream's ontology table).
pub struct RollbackCtx<'a> {
    pub kb: &'a Arc<MemoryManager>,
    pub workstream_root: &'a Path,
}

/// Apply the inverse mutation described by `row.outputs_json`.
///
/// Returns `Ok(())` for proposal-only actions (no KB mutation needed —
/// `Journal::revert` is responsible for the metadata flip).
pub fn apply_inverse(row: &JournalRow, ctx: &RollbackCtx<'_>) -> Result<(), StewardError> {
    if row.reverted_at.is_some() {
        debug!(id = row.id, "rollback: row already reverted; skipping");
        return Ok(());
    }
    match (row.subroutine.as_str(), row.action.as_str()) {
        ("reshelve", "merge") => reshelve_merge_inverse(row, ctx.kb),
        ("reshelve", "delete") => reshelve_delete_inverse(row, ctx.kb),
        // Dust mutates by inserting a summary entity (+ SUMMARIZES
        // edges to sources) at apply time; the inverse is to delete
        // the summary entity, which DETACH DELETEs its edges too.
        // Source entities were never touched.
        ("dust", "summarize") => dust_summarize_inverse(row, ctx.kb),
        // tag-promoter promotions insert into the ontology table;
        // the inverse removes the tag from it. KB graph is untouched.
        ("tag-promoter", "promote_tag") => tag_promoter_inverse(row, ctx),
        // Proposal-only subroutines mutate nothing — `Journal::revert`
        // alone is enough.
        ("map", "propose_relation") => Ok(()),
        ("doorwatch", "propose_identity") => Ok(()),
        // Identity / unknown subroutines: best-effort no-op.
        ("identity", _) => Ok(()),
        (sub, act) => Err(StewardError::Subroutine {
            name: sub.into(),
            message: format!("no inverse for action `{act}`"),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct PromoteTagOutputs {
    tag: String,
}

fn tag_promoter_inverse(
    row: &JournalRow,
    ctx: &RollbackCtx<'_>,
) -> Result<(), StewardError> {
    let payload: PromoteTagOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("tag-promoter/promote_tag payload: {e}")))?;
    let ontology = TagOntologyStore::open_at(ctx.workstream_root)
        .map_err(StewardError::from)?;
    let removed = ontology.remove(&payload.tag)?;
    debug!(tag = %payload.tag, removed, "rollback: tag promotion reverted");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct MergeOutputs {
    survivor_id: Uuid,
    deprecated_id: Uuid,
    pre_survivor: Entity,
    pre_deprecated: Entity,
}

fn reshelve_merge_inverse(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    let payload: MergeOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("reshelve/merge payload: {e}")))?;
    // 1. Restore survivor's pre-merge state.
    kb.workstream.update_entity(&payload.pre_survivor)?;
    // 2. Restore deprecated's pre-merge state (clears superseded).
    kb.workstream.update_entity(&payload.pre_deprecated)?;
    // 3. Remove the SUPERSEDES edge we added.
    kb.workstream
        .delete_relation(payload.survivor_id, RelationType::Supersedes, payload.deprecated_id)?;
    debug!(
        survivor = %payload.survivor_id,
        deprecated = %payload.deprecated_id,
        "rollback: reshelve merge reverted"
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct DeleteOutputs {
    entity: Entity,
}

/// `dust/summarize` writes its outputs as `{summary: Entity, source_ids: [...], ...}`.
/// Only the summary's id is needed for the inverse — `MemoryStore::delete_entity`
/// uses `DETACH DELETE` so the SUMMARIZES edges go with it.
#[derive(Debug, Deserialize)]
struct DustSummarizeOutputs {
    summary: Entity,
}

fn dust_summarize_inverse(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    let payload: DustSummarizeOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("dust/summarize payload: {e}")))?;
    let removed = kb.workstream.delete_entity(payload.summary.id)?;
    debug!(
        summary = %payload.summary.id,
        removed,
        "rollback: dust summary entity deleted"
    );
    Ok(())
}

fn reshelve_delete_inverse(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    let payload: DeleteOutputs = serde_json::from_str(&row.outputs_json)
        .map_err(|e| StewardError::Parse(format!("reshelve/delete payload: {e}")))?;
    kb.workstream.insert_entity(&payload.entity)?;
    debug!(id = %payload.entity.id, "rollback: reshelve delete reverted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_memory::EntityType;

    fn setup_kb() -> (tempfile::TempDir, Arc<MemoryManager>) {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "ws", None).unwrap());
        (tmp, mgr)
    }

    fn ws_root(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        tmp.path().join("workstreams").join("ws")
    }

    #[test]
    fn proposal_inverse_is_noop() {
        let (tmp, kb) = setup_kb();
        let row = JournalRow {
            id: 1,
            ts: chrono::Utc::now(),
            subroutine: "map".into(),
            action: "propose_relation".into(),
            inputs_json: "{}".into(),
            outputs_json: "{}".into(),
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: false,
            reverted_at: None,
        };
        let root = ws_root(&tmp);
        apply_inverse(&row, &RollbackCtx { kb: &kb, workstream_root: &root }).unwrap();
    }

    #[test]
    fn reshelve_delete_inverse_reinserts_entity() {
        let (tmp, kb) = setup_kb();
        let e = Entity::new(EntityType::Fact, "restore me")
            .with_content("important content");
        let payload = serde_json::json!({"entity": e}).to_string();
        let row = JournalRow {
            id: 1,
            ts: chrono::Utc::now(),
            subroutine: "reshelve".into(),
            action: "delete".into(),
            inputs_json: "{}".into(),
            outputs_json: payload,
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: true,
            reverted_at: None,
        };
        let root = ws_root(&tmp);
        apply_inverse(&row, &RollbackCtx { kb: &kb, workstream_root: &root }).unwrap();
        let fetched = kb.workstream.get_entity(e.id).unwrap().unwrap();
        assert_eq!(fetched.title, "restore me");
    }

    #[test]
    fn dust_summarize_inverse_deletes_summary() {
        let (tmp, kb) = setup_kb();
        let summary = Entity::new(EntityType::Note, "summary of falcon")
            .with_content("the falcon project, distilled");
        kb.workstream.insert_entity(&summary).unwrap();
        assert!(kb.workstream.get_entity(summary.id).unwrap().is_some());

        let payload = serde_json::json!({
            "cluster_key": "falcon",
            "summary": &summary,
            "source_ids": [],
        })
        .to_string();
        let row = JournalRow {
            id: 1,
            ts: chrono::Utc::now(),
            subroutine: "dust".into(),
            action: "summarize".into(),
            inputs_json: "{}".into(),
            outputs_json: payload,
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: true,
            reverted_at: None,
        };
        let root = ws_root(&tmp);
        apply_inverse(&row, &RollbackCtx { kb: &kb, workstream_root: &root }).unwrap();
        assert!(kb.workstream.get_entity(summary.id).unwrap().is_none());
    }

    #[test]
    fn tag_promoter_inverse_removes_from_ontology() {
        let (tmp, kb) = setup_kb();
        let root = ws_root(&tmp);
        // Seed the tag into the ontology so revert has something to remove.
        let ontology = TagOntologyStore::open_at(&root).unwrap();
        ontology
            .add("calidor", arawn_memory::AddedVia::Promotion)
            .unwrap();
        assert!(ontology.contains("calidor").unwrap());

        let row = JournalRow {
            id: 1,
            ts: chrono::Utc::now(),
            subroutine: "tag-promoter".into(),
            action: "promote_tag".into(),
            inputs_json: "{}".into(),
            outputs_json: serde_json::json!({"tag": "calidor", "count": 5}).to_string(),
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: true,
            reverted_at: None,
        };
        apply_inverse(&row, &RollbackCtx { kb: &kb, workstream_root: &root }).unwrap();
        let ontology = TagOntologyStore::open_at(&root).unwrap();
        assert!(!ontology.contains("calidor").unwrap());
    }

    #[test]
    fn unknown_action_returns_error() {
        let (tmp, kb) = setup_kb();
        let row = JournalRow {
            id: 1,
            ts: chrono::Utc::now(),
            subroutine: "reshelve".into(),
            action: "obscure".into(),
            inputs_json: "{}".into(),
            outputs_json: "{}".into(),
            model: "m".into(),
            prompt_hash: "h".into(),
            applied: true,
            reverted_at: None,
        };
        let root = ws_root(&tmp);
        let err = apply_inverse(&row, &RollbackCtx { kb: &kb, workstream_root: &root })
            .unwrap_err();
        assert!(matches!(err, StewardError::Subroutine { .. }));
    }
}

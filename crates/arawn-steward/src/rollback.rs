//! Per-subroutine rollback logic.
//!
//! `Journal::revert` flips the metadata; `rollback::apply_inverse` is
//! where the *actual* inverse mutation lives. We dispatch on
//! `(subroutine, action)` so the contract stays in one place.

use std::sync::Arc;

use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use arawn_memory::{Entity, MemoryManager, RelationType};

use crate::error::StewardError;
use crate::journal::JournalRow;

/// Apply the inverse mutation described by `row.outputs_json` to `kb`.
///
/// Returns `Ok(())` for proposal-only actions (no KB mutation needed —
/// `Journal::revert` is responsible for the metadata flip).
pub fn apply_inverse(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError> {
    if row.reverted_at.is_some() {
        debug!(id = row.id, "rollback: row already reverted; skipping");
        return Ok(());
    }
    match (row.subroutine.as_str(), row.action.as_str()) {
        ("reshelve", "merge") => reshelve_merge_inverse(row, kb),
        ("reshelve", "delete") => reshelve_delete_inverse(row, kb),
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

    #[test]
    fn proposal_inverse_is_noop() {
        let (_tmp, kb) = setup_kb();
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
        apply_inverse(&row, &kb).unwrap();
    }

    #[test]
    fn reshelve_delete_inverse_reinserts_entity() {
        let (_tmp, kb) = setup_kb();
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
        apply_inverse(&row, &kb).unwrap();
        let fetched = kb.workstream.get_entity(e.id).unwrap().unwrap();
        assert_eq!(fetched.title, "restore me");
    }

    #[test]
    fn unknown_action_returns_error() {
        let (_tmp, kb) = setup_kb();
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
        let err = apply_inverse(&row, &kb).unwrap_err();
        assert!(matches!(err, StewardError::Subroutine { .. }));
    }
}

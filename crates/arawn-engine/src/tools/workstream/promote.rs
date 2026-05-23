use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_core::SCRATCH_NAME;
use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};


pub struct WorkstreamPromoteTool {
    store: Arc<Mutex<Store>>,
    router: Arc<crate::workstream_router::WorkstreamMemoryRouter>,
}

impl WorkstreamPromoteTool {
    pub fn new(
        store: Arc<Mutex<Store>>,
        router: Arc<crate::workstream_router::WorkstreamMemoryRouter>,
    ) -> Self {
        Self { store, router }
    }
}

#[async_trait]
impl Tool for WorkstreamPromoteTool {
    fn name(&self) -> &str {
        "workstream_promote"
    }

    fn description(&self) -> &str {
        "Move an entity from the scratch workstream into a named workstream's KB. \
         The entity is removed from scratch and `store_fact`-merged into the target \
         (so existing duplicates reinforce). Use this to consolidate ad-hoc notes \
         once you know which workstream they belong to."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "entity_id": {"type": "string", "description": "UUID of the entity to promote"},
                "target": {"type": "string", "description": "Slug of the target workstream"}
            },
            "required": ["entity_id", "target"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let entity_id_str = params
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let target_name = params
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if entity_id_str.is_empty() || target_name.is_empty() {
            return Ok(ToolOutput::error(
                "entity_id and target are required".to_string(),
            ));
        }
        if target_name == SCRATCH_NAME {
            return Ok(ToolOutput::error(
                "target must be a real workstream, not scratch".to_string(),
            ));
        }
        let entity_id = match uuid::Uuid::parse_str(entity_id_str) {
            Ok(id) => id,
            Err(_) => {
                return Ok(ToolOutput::error(
                    "entity_id is not a valid UUID".to_string(),
                ));
            }
        };

        // Verify target workstream exists.
        {
            let store = self.store.lock().unwrap();
            match store.find_workstream_by_name(&target_name) {
                Ok(Some(ws)) if ws.archived => {
                    return Ok(ToolOutput::error(format!(
                        "target '{target_name}' is archived"
                    )));
                }
                Ok(Some(_)) => {}
                Ok(None) => {
                    return Ok(ToolOutput::error(format!(
                        "target workstream '{target_name}' not found"
                    )));
                }
                Err(e) => return Ok(ToolOutput::error(format!("lookup failed: {e}"))),
            }
        }

        // Resolve both managers via the router so caching is shared.
        let scratch_mgr = self
            .router
            .for_workstream(SCRATCH_NAME)
            .map_err(|e| ToolError::ExecutionFailed(format!("scratch open: {e}")))?;
        let target_mgr = self
            .router
            .for_workstream(&target_name)
            .map_err(|e| ToolError::ExecutionFailed(format!("target open: {e}")))?;

        // Try workstream tier first, then global. Either source is fine
        // for promotion — both belong to "scratch context."
        let entity = match scratch_mgr.workstream.get_entity(entity_id) {
            Ok(Some(e)) => e,
            Ok(None) => match scratch_mgr.global.get_entity(entity_id) {
                Ok(Some(e)) => e,
                Ok(None) => {
                    return Ok(ToolOutput::error(format!(
                        "entity {entity_id} not found in scratch"
                    )));
                }
                Err(e) => return Ok(ToolOutput::error(format!("scratch global lookup: {e}"))),
            },
            Err(e) => return Ok(ToolOutput::error(format!("scratch workstream lookup: {e}"))),
        };
        let scope = entity.entity_type.default_scope();
        let target_store = target_mgr.store_for(scope);

        // Write into target via store_fact (dedup-safe).
        let result = target_store
            .store_fact(&entity)
            .map_err(|e| ToolError::ExecutionFailed(format!("target store_fact: {e}")))?;

        // Remove from scratch's matching tier.
        let scratch_store = scratch_mgr.store_for(scope);
        let _ = scratch_store.delete_entity(entity_id);

        Ok(ToolOutput::success(
            json!({
                "promoted": entity_id.to_string(),
                "from": SCRATCH_NAME,
                "to": target_name,
                "scope": match scope {
                    arawn_memory::Scope::Global => "global",
                    arawn_memory::Scope::Workstream => "workstream",
                },
                "result": match result {
                    arawn_memory::StoreFactResult::Inserted { entity_id } =>
                        json!({"action": "inserted", "id": entity_id.to_string()}),
                    arawn_memory::StoreFactResult::Reinforced { entity_id, new_count } =>
                        json!({"action": "reinforced", "id": entity_id.to_string(), "count": new_count}),
                    arawn_memory::StoreFactResult::Superseded { new_entity_id, .. } =>
                        json!({"action": "superseded", "id": new_entity_id.to_string()}),
                },
            })
            .to_string(),
        ))
    }
}


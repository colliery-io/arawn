//! Operator tools for the extraction pipeline (ARAWN-T-0484).
//!
//! These let the agent (and, through it, the user) inspect and correct what
//! the per-lens extractor did, backed by the `extraction_log` table:
//! - `signal_explain` — why a projection row did or didn't become signal.
//! - `extract_rerun` — clear a lens's cursors so the next pass re-evaluates.
//! - `signal_dismiss` — mark a row's signal as garbage so it's not re-created.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::{ExtractionLogStore, ExtractorCursorStore, Store};
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

fn missing(field: &str) -> ToolError {
    ToolError::ExecutionFailed(format!("missing '{field}'"))
}

/// `signal_explain` — report the extractor's decision for one projection row.
pub struct SignalExplainTool {
    store: Arc<Mutex<Store>>,
}

impl SignalExplainTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for SignalExplainTool {
    fn name(&self) -> &str {
        "signal_explain"
    }

    fn description(&self) -> &str {
        "Explain why a specific feed/projection row did or didn't become a \
         signal in a lens. Returns the extractor's recorded outcome \
         (`ok` | `empty` | `skipped`), the classify rationale, the run id, and \
         whether the row has been dismissed. Use it when a signal you expected \
         is missing, or one you didn't expect showed up — pass the `lens` name \
         and the row's `projection_id` (the `id` from a `feed_search` hit)."
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
                "lens": {"type": "string", "description": "Lens name"},
                "projection_id": {"type": "string", "description": "The feed row id"}
            },
            "required": ["lens", "projection_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let lens = params
            .get("lens")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing("lens"))?;
        let pid = params
            .get("projection_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing("projection_id"))?;
        let store = self.store.lock().unwrap();
        let record = ExtractionLogStore::new(store.database())
            .get(lens, pid)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        match record {
            None => Ok(ToolOutput::success(
                json!({
                    "lens": lens,
                    "projection_id": pid,
                    "status": "not_extracted",
                    "explanation": "No extraction has been recorded for this row yet \
                                    (the extractor may not have reached it, or it \
                                    predates extraction logging)."
                })
                .to_string(),
            )),
            Some(r) => Ok(ToolOutput::success(
                json!({
                    "lens": r.lens_name,
                    "projection_id": r.projection_id,
                    "outcome": r.outcome,
                    "reason": r.reason,
                    "dismissed": r.dismissed,
                    "run_id": r.run_id,
                    "updated_at": r.updated_at,
                })
                .to_string(),
            )),
        }
    }
}

/// `extract_rerun` — clear a lens's extraction cursors so the next pass
/// re-evaluates every row (idempotent; dismissed rows stay dismissed).
pub struct ExtractRerunTool {
    store: Arc<Mutex<Store>>,
}

impl ExtractRerunTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for ExtractRerunTool {
    fn name(&self) -> &str {
        "extract_rerun"
    }

    fn description(&self) -> &str {
        "Re-run extraction for a lens: clears its extraction cursors so the \
         next background pass re-evaluates every feed row against the lens's \
         current scope/ontology. Use after editing a lens description or \
         ontology so older rows get reconsidered. Safe to call repeatedly — \
         re-extraction is idempotent (duplicate entities are de-duplicated) \
         and rows you've dismissed stay dismissed. Pass the `lens` name."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "lens": {"type": "string", "description": "Lens name"}
            },
            "required": ["lens"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let lens = params
            .get("lens")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing("lens"))?;
        let store = self.store.lock().unwrap();
        let cleared = ExtractorCursorStore::new(store.database())
            .clear_for_lens(lens)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolOutput::success(
            json!({
                "lens": lens,
                "cursors_cleared": cleared,
                "note": "the next extraction pass will re-evaluate this lens's rows"
            })
            .to_string(),
        ))
    }
}

/// `signal_dismiss` — mark a row's extraction as garbage so future passes
/// skip it (it won't be re-created).
pub struct SignalDismissTool {
    store: Arc<Mutex<Store>>,
}

impl SignalDismissTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for SignalDismissTool {
    fn name(&self) -> &str {
        "signal_dismiss"
    }

    fn description(&self) -> &str {
        "Dismiss a feed/projection row's signal in a lens: marks the row so \
         the extractor SKIPS it on every future pass (it won't be re-created). \
         Use when a row keeps producing a wrong or noisy signal. Pass the \
         `lens` name and the row's `projection_id`. Pass `undo: true` to \
         un-dismiss. Note: this stops *re-extraction*; to remove an entity \
         that's already in memory, use `forget`."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "lens": {"type": "string", "description": "Lens name"},
                "projection_id": {"type": "string", "description": "The feed row id"},
                "undo": {"type": "boolean", "description": "Un-dismiss instead (default false)"}
            },
            "required": ["lens", "projection_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let lens = params
            .get("lens")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing("lens"))?;
        let pid = params
            .get("projection_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| missing("projection_id"))?;
        let undo = params
            .get("undo")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let store = self.store.lock().unwrap();
        ExtractionLogStore::new(store.database())
            .set_dismissed(lens, pid, !undo)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolOutput::success(
            json!({
                "lens": lens,
                "projection_id": pid,
                "dismissed": !undo,
            })
            .to_string(),
        ))
    }
}

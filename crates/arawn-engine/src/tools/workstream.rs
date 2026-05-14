//! Workstream slash commands.
//!
//! Eight tools that cover the workstream lifecycle: new, list, switch,
//! show, describe, bind, unbind, delete. Each is a `Tool` impl with a
//! `workstream_*` name so the slash dispatcher routes naturally.
//!
//! Session-active workstream is held by `SessionWorkstream` (a shared
//! `Arc<Mutex<String>>`) — T-0250 replaces this with the real
//! `Session::workstream_name` field once sessions gain it. For now
//! the shim is enough to make `switch` / `show` work.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_core::{SCRATCH_NAME, Workstream};
use arawn_storage::Store;

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Holder for the session-active workstream name. Cheap to clone
/// (`Arc<Mutex<String>>`). T-0250 will retire this in favor of the
/// `Session::workstream_name` field.
#[derive(Clone, Debug)]
pub struct SessionWorkstream {
    inner: Arc<Mutex<String>>,
}

impl SessionWorkstream {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(initial.into())),
        }
    }

    pub fn scratch() -> Self {
        Self::new(SCRATCH_NAME)
    }

    pub fn current(&self) -> String {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, name: impl Into<String>) {
        *self.inner.lock().unwrap() = name.into();
    }
}

impl Default for SessionWorkstream {
    fn default() -> Self {
        Self::scratch()
    }
}

// ============================================================================
// workstream_new
// ============================================================================

pub struct WorkstreamCreateTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamCreateTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamCreateTool {
    fn name(&self) -> &str {
        "workstream_new"
    }

    fn description(&self) -> &str {
        "Create a new workstream with a declared tag ontology. Per ADR-0004 \
         the ontology is required at creation — the agent should propose it \
         via `workstream_propose_ontology(description)`, confirm with the \
         user, then call this tool with the agreed `tags_ontology`. Name \
         must be a slug (lowercase, digits, '-' and '_' only). Does not \
         switch into the new workstream."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Slug name (e.g. 'pat', 'auth-migration')"},
                "display_name": {"type": "string", "description": "Optional human label (defaults to name)"},
                "description": {"type": "string", "description": "Required free-text description — what this workstream tracks"},
                "tags_ontology": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Required non-empty list of initial ontology tags. These form the closed list of tags the extractor may attach to entities. Add more later via `workstream_apply` of `tag-promoter` proposals or directly with `workstream_tag add`."
                }
            },
            "required": ["name", "description", "tags_ontology"]
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        if name == SCRATCH_NAME {
            return Ok(ToolOutput::error(
                "the name 'scratch' is reserved — it always exists".to_string(),
            ));
        }
        let display_name = params
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| name.clone());
        let description = match params.get("description").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => {
                return Ok(ToolOutput::error(
                    "description is required — it shapes ontology proposals and seeds the extractor".to_string(),
                ));
            }
        };

        // Ontology is required and non-empty. Normalize + dedupe so a
        // caller that sends `["Falcon", "falcon ", "FALCON"]` lands a
        // single `falcon` tag.
        let tags_ontology: Vec<String> = match params.get("tags_ontology").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => {
                let mut seen = std::collections::HashSet::new();
                let mut out = Vec::new();
                for v in arr {
                    if let Some(s) = v.as_str() {
                        let canonical = arawn_memory::normalize_tag(s);
                        if !canonical.is_empty() && seen.insert(canonical.clone()) {
                            out.push(canonical);
                        }
                    }
                }
                if out.is_empty() {
                    return Ok(ToolOutput::error(
                        "tags_ontology contained no valid tags after normalization".to_string(),
                    ));
                }
                out
            }
            _ => {
                return Ok(ToolOutput::error(
                    "tags_ontology is required — pass a non-empty array of initial ontology tags (use `workstream_propose_ontology` to suggest)".to_string(),
                ));
            }
        };

        let data_dir = match ctx.data_dir() {
            Some(d) => d.to_path_buf(),
            None => {
                return Ok(ToolOutput::error(
                    "no data_dir available — required to materialize the workstream's KB + ontology".to_string(),
                ));
            }
        };
        let root_dir = data_dir.join("workstreams").join(&name);

        let mut ws = Workstream::new(&name, &root_dir);
        ws.display_name = display_name;
        ws.description = description;

        // Insert the workstream record first; if ontology seeding fails
        // afterwards the record stays — user can re-run `workstream_tag
        // add` to recover. The alternative (rollback the workstream)
        // doubles the failure modes for negligible benefit.
        {
            let store = self.store.lock().unwrap();
            if let Err(e) = store.create_workstream(&ws) {
                return Ok(ToolOutput::error(format!("failed to create workstream: {e}")));
            }
        }

        // Seed the ontology in the workstream's colocated table.
        let ontology = match arawn_memory::TagOntologyStore::open(&data_dir, &name) {
            Ok(o) => o,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "workstream record created but ontology open failed: {e}"
                )));
            }
        };
        if let Err(e) =
            ontology.add_many(tags_ontology.iter().cloned(), arawn_memory::AddedVia::Manual)
        {
            return Ok(ToolOutput::error(format!(
                "workstream record created but ontology seed failed: {e}"
            )));
        }

        Ok(ToolOutput::success(
            json!({
                "name": ws.name,
                "display_name": ws.display_name,
                "root_dir": ws.root_dir.display().to_string(),
                "tags_ontology": tags_ontology,
            })
            .to_string(),
        ))
    }
}

// ============================================================================
// workstream_list
// ============================================================================

pub struct WorkstreamListTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamListTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self {
            store,
            active: SessionWorkstream::default(),
        }
    }

    pub fn with_active(mut self, active: SessionWorkstream) -> Self {
        self.active = active;
        self
    }
}

#[async_trait]
impl Tool for WorkstreamListTool {
    fn name(&self) -> &str {
        "workstream_list"
    }

    fn description(&self) -> &str {
        "List active workstreams (newest update first). Pass `all: true` to include archived."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "all": {"type": "boolean", "description": "Include archived (soft-deleted) workstreams"}
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let include_archived = params
            .get("all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let active = self.active.current();
        let store = self.store.lock().unwrap();
        let workstreams = if include_archived {
            store.list_all_workstreams()
        } else {
            store.list_workstreams()
        }
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let items: Vec<Value> = workstreams
            .iter()
            .map(|ws| {
                json!({
                    "name": ws.name,
                    "display_name": ws.display_name,
                    "description": ws.description,
                    "bindings": ws.bindings,
                    "archived": ws.archived,
                    "active": ws.name == active,
                })
            })
            .collect();

        Ok(ToolOutput::success(
            json!({ "active": active, "workstreams": items }).to_string(),
        ))
    }
}

// ============================================================================
// workstream_switch
// ============================================================================

pub struct WorkstreamSwitchTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamSwitchTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamSwitchTool {
    fn name(&self) -> &str {
        "workstream_switch"
    }

    fn description(&self) -> &str {
        "Switch the session-active workstream. Subsequent memory operations \
         in this session route to that workstream's KB + global. Errors if \
         the named workstream doesn't exist."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "name": {"type": "string"} },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        let store = self.store.lock().unwrap();
        let ws = store
            .find_workstream_by_name(&name)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let Some(ws) = ws else {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' not found"
            )));
        };
        if ws.archived {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' is archived; un-archive it first"
            )));
        }
        let prev = self.active.current();
        // Update the in-memory shim first so any tool call running in
        // the same turn sees the new active workstream.
        self.active.set(&ws.name);
        // Persist on the session record too — `LocalService` re-reads
        // `meta.workstream_name` on every session-load (i.e. every
        // turn over WS) and pushes it back into the SessionWorkstream
        // shim. Without this write, the in-memory set above would be
        // overwritten by the stale persisted value on the next turn
        // and the switch would silently revert.
        let session_id = ctx.session_id();
        if let Err(e) = store.update_session_workstream_name(session_id, &ws.name) {
            tracing::warn!(
                error = %e,
                session = %session_id,
                workstream = %ws.name,
                "workstream_switch: failed to persist on session — switch is in-memory only, will revert on next turn"
            );
        }
        // Lead with the human-readable banner so the TUI / agent
        // surfaces the switch clearly; structured fields trail.
        let banner = format!(
            "→ now in workstream '{}' — next messages contribute to {}'s KB (was: {})",
            ws.name, ws.name, prev
        );
        Ok(ToolOutput::success(format!(
            "{banner}\n{}",
            json!({
                "switched_to": ws.name,
                "previous": prev,
            })
        )))
    }
}

// ============================================================================
// workstream_show
// ============================================================================

pub struct WorkstreamShowTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamShowTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamShowTool {
    fn name(&self) -> &str {
        "workstream_show"
    }

    fn description(&self) -> &str {
        "Show the active workstream's details (or a named one). \
         Includes display_name, description, bindings, and — crucially \
         for downstream tools — the workstream's declared tag ontology. \
         Use this before calling `workstream_dust` or `signal_query` \
         with a tag filter so you pick a tag that actually exists in \
         the ontology."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Defaults to the active workstream"}
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| self.active.current());
        let store = self.store.lock().unwrap();
        let ws = store
            .find_workstream_by_name(&name)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let Some(ws) = ws else {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' not found"
            )));
        };
        drop(store);

        // Surface the workstream's declared tag ontology — agents that
        // call this tool before `workstream_dust` / `signal_query` can
        // pick valid tags instead of guessing. Soft-fail to empty when
        // the data dir is unavailable or the ontology table is missing.
        let ontology_tags: Vec<String> = match ctx.data_dir() {
            Some(dir) => arawn_memory::TagOntologyStore::open(dir, &ws.name)
                .and_then(|s| s.tags())
                .unwrap_or_default(),
            None => Vec::new(),
        };

        Ok(ToolOutput::success(
            json!({
                "name": ws.name,
                "display_name": ws.display_name,
                "description": ws.description,
                "bindings": ws.bindings,
                "archived": ws.archived,
                "created_at": ws.created_at.to_rfc3339(),
                "updated_at": ws.updated_at.to_rfc3339(),
                "active": ws.name == self.active.current(),
                "tags_ontology": ontology_tags,
            })
            .to_string(),
        ))
    }
}

// ============================================================================
// workstream_describe
// ============================================================================

pub struct WorkstreamDescribeTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamDescribeTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamDescribeTool {
    fn name(&self) -> &str {
        "workstream_describe"
    }

    fn description(&self) -> &str {
        "Set or update a workstream's description. The description feeds the \
         per-workstream extractor in Phase 4."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "description": {"type": "string"}
            },
            "required": ["name", "description"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let store = self.store.lock().unwrap();
        match store.update_workstream_description(&name, &description) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "description": description}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

// ============================================================================
// workstream_bind / workstream_unbind
// ============================================================================

/// Side-channel that fires when `/workstream bind` lands a new
/// binding. Implementations spawn whatever work is needed (typically
/// an extractor backfill over the bound feed's projection rows).
pub trait BindBackfillHook: Send + Sync {
    fn on_bind(&self, workstream_name: &str, feed_id: &str);
}

pub struct WorkstreamBindTool {
    store: Arc<Mutex<Store>>,
    hook: Option<Arc<dyn BindBackfillHook>>,
}

impl WorkstreamBindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store, hook: None }
    }

    pub fn with_backfill_hook(mut self, hook: Arc<dyn BindBackfillHook>) -> Self {
        self.hook = Some(hook);
        self
    }
}

#[async_trait]
impl Tool for WorkstreamBindTool {
    fn name(&self) -> &str {
        "workstream_bind"
    }

    fn description(&self) -> &str {
        "Bind a feed to a workstream. Bindings hint to the Phase 4 extractor \
         which feed items should land in this workstream's KB. Idempotent."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "feed_id": {"type": "string"}
            },
            "required": ["name", "feed_id"]
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
            .unwrap_or_default()
            .to_string();
        let feed_id = params
            .get("feed_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || feed_id.is_empty() {
            return Ok(ToolOutput::error("name and feed_id are required".to_string()));
        }
        let result = {
            let store = self.store.lock().unwrap();
            store.add_workstream_binding(&name, &feed_id)
        };
        match result {
            Ok(()) => {
                // Fire backfill hook if wired. Drop store lock first
                // — the hook spawns its own task and shouldn't hold
                // our lock.
                if let Some(hook) = self.hook.as_ref() {
                    hook.on_bind(&name, &feed_id);
                }
                Ok(ToolOutput::success(
                    json!({"name": name, "feed_id": feed_id}).to_string(),
                ))
            }
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

pub struct WorkstreamUnbindTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamUnbindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamUnbindTool {
    fn name(&self) -> &str {
        "workstream_unbind"
    }

    fn description(&self) -> &str {
        "Remove a feed binding from a workstream. Silent no-op if not bound."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "feed_id": {"type": "string"}
            },
            "required": ["name", "feed_id"]
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
            .unwrap_or_default()
            .to_string();
        let feed_id = params
            .get("feed_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || feed_id.is_empty() {
            return Ok(ToolOutput::error("name and feed_id are required".to_string()));
        }
        let store = self.store.lock().unwrap();
        match store.remove_workstream_binding(&name, &feed_id) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "feed_id": feed_id}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

// ============================================================================
// workstream_promote
// ============================================================================

/// Move one entity from the `scratch` workstream into a named target.
/// The scratch tier accumulates loose facts that haven't yet been
/// associated with a real workstream; `promote` is the explicit
/// "this belongs to project X" pin.
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
            Err(_) => return Ok(ToolOutput::error("entity_id is not a valid UUID".to_string())),
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

// ============================================================================
// workstream_delete (soft)
// ============================================================================

pub struct WorkstreamDeleteTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamDeleteTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamDeleteTool {
    fn name(&self) -> &str {
        "workstream_delete"
    }

    fn description(&self) -> &str {
        "Soft-delete a workstream (sets archived = 1). On-disk KB is left intact. \
         Refuses 'scratch' and refuses the currently-active workstream."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "name": {"type": "string"} },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        if name == self.active.current() {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' is currently active; switch away before deleting"
            )));
        }
        let store = self.store.lock().unwrap();
        match store.soft_delete_workstream(&name) {
            Ok(()) => Ok(ToolOutput::success(
                json!({
                    "deleted": name,
                    "note": "soft delete; on-disk KB intact"
                })
                .to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

// ============================================================================
// workstream_propose_ontology
// ============================================================================

/// LLM-backed tool: take a workstream description, return a proposed
/// initial ontology (list of tag slugs) + rationale. The agent uses
/// this during the `/workstream-create` flow — see the
/// `workstream-create` skill for the playbook.
pub struct WorkstreamProposeOntologyTool {
    client: Arc<dyn arawn_llm::LlmClient>,
    model: String,
}

impl WorkstreamProposeOntologyTool {
    pub fn new(client: Arc<dyn arawn_llm::LlmClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }
}

#[async_trait]
impl Tool for WorkstreamProposeOntologyTool {
    fn name(&self) -> &str {
        "workstream_propose_ontology"
    }

    fn description(&self) -> &str {
        "Given a free-text description of a workstream, propose an initial \
         tag ontology (closed list of tag slugs the extractor will be allowed \
         to use). Returns `{ tags: [...], rationale: \"...\" }`. The agent \
         calls this during the create flow, shows the proposal to the user, \
         iterates if needed, then calls `workstream_new` with the agreed list. \
         Tag slugs are short (`lowercase-with-dashes`), describe the kinds of \
         things you'll track (projects, people, processes), and should number \
         5–12 — keep it focused; new tags grow into the ontology via the \
         tag-promoter steward subroutine over time."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Free-text description of the workstream — what it tracks, who's involved, what's in scope."
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
                return Ok(ToolOutput::error(
                    "description is required".to_string(),
                ));
            }
        };

        let system = "You propose an initial tag ontology for a personal \
                      knowledge-base workstream. Output ONLY a JSON object: \
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
                      of this workstream.";
        let user = format!(
            "Workstream description:\n{description}\n\n\
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

/// Same balanced-bracket scan as `arawn-extractor::llm_text::extract_json_block`.
fn extract_json_block(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    let mut open: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (open, b) {
            (None, b'{') | (None, b'[') => {
                start = Some(i);
                open = Some(b);
                depth = 1;
            }
            (Some(b'{'), b'{') | (Some(b'['), b'[') => depth += 1,
            (Some(b'{'), b'}') | (Some(b'['), b']') => {
                depth -= 1;
                if depth == 0 {
                    let end = i + 1;
                    return Some(&raw[start.unwrap()..end]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn setup() -> (tempfile::TempDir, Arc<Mutex<Store>>, SessionWorkstream) {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        (tmp, Arc::new(Mutex::new(store)), SessionWorkstream::scratch())
    }

    fn test_ctx(tmp: &tempfile::TempDir) -> crate::context::EngineToolContext {
        let ws = Workstream::scratch(tmp.path());
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
            .with_data_dir(tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn create_succeeds_with_valid_slug_description_and_ontology() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "Pat's day job",
                    "tags_ontology": ["postgres", "ledger"]
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        assert!(result.content.contains("pat"));
        // Ontology is materialized — both seed tags should be present.
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        assert_eq!(ont.count().unwrap(), 2);
        assert!(ont.contains("postgres").unwrap());
        assert!(ont.contains("ledger").unwrap());
    }

    #[tokio::test]
    async fn create_refuses_scratch() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "scratch",
                    "description": "x",
                    "tags_ontology": ["x"]
                }),
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn create_refuses_missing_description() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "tags_ontology": ["x"]}),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("description"));
    }

    #[tokio::test]
    async fn create_refuses_empty_ontology() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "x",
                    "tags_ontology": []
                }),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("tags_ontology"));
    }

    #[tokio::test]
    async fn create_dedupes_and_normalizes_ontology() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({
                    "name": "pat",
                    "description": "x",
                    "tags_ontology": ["Falcon", "falcon ", "FALCON", "ledger"]
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        assert_eq!(ont.count().unwrap(), 2);
        assert!(ont.contains("falcon").unwrap());
        assert!(ont.contains("ledger").unwrap());
    }

    #[tokio::test]
    async fn switch_updates_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("workstreams/pat")))
            .unwrap();
        let tool = WorkstreamSwitchTool::new(store.clone(), active.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert_eq!(active.current(), "pat");
    }

    #[tokio::test]
    async fn switch_unknown_errors() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamSwitchTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "ghost"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn show_defaults_to_active() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamShowTool::new(store.clone(), active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn describe_updates_description() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let tool = WorkstreamDescribeTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "description": "skip-level for pat"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.description, "skip-level for pat");
    }

    #[tokio::test]
    async fn bind_and_unbind_round_trip() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = WorkstreamBindTool::new(store.clone());
        bind.execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.bindings, vec!["f1"]);
        let unbind = WorkstreamUnbindTool::new(store.clone());
        unbind
            .execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert!(fetched.bindings.is_empty());
    }

    #[tokio::test]
    async fn delete_refuses_scratch() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "scratch"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active") || result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn delete_refuses_currently_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active"));
    }

    #[tokio::test]
    async fn delete_soft_marks_archived() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("temp", tmp.path().join("ws/temp")))
            .unwrap();
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "temp"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        // listed via list_all_workstreams should still show it as archived.
        let all = store.lock().unwrap().list_all_workstreams().unwrap();
        let found = all.iter().find(|w| w.name == "temp").unwrap();
        assert!(found.archived);
    }

    #[tokio::test]
    async fn promote_moves_entity_from_scratch_to_target() {
        use crate::workstream_router::WorkstreamMemoryRouter;
        use arawn_memory::{Entity, EntityType};

        let (tmp, store, _) = setup();
        // Create the target workstream.
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();

        // Seed scratch with a fact via the router so the promote tool
        // walks the same surface that prod uses.
        let scratch_session = SessionWorkstream::scratch();
        let router = Arc::new(WorkstreamMemoryRouter::new(
            tmp.path(),
            None,
            None,
            scratch_session.clone(),
        ));
        let scratch_mgr = router.for_workstream(SCRATCH_NAME).unwrap();
        let entity = Entity::new(EntityType::Fact, "pat 1on1 ran long today");
        scratch_mgr.workstream.store_fact(&entity).unwrap();

        let tool = WorkstreamPromoteTool::new(store.clone(), router.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"entity_id": entity.id.to_string(), "target": "pat"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);

        // Target now has it.
        let pat_mgr = router.for_workstream("pat").unwrap();
        assert!(pat_mgr.workstream.get_entity(entity.id).unwrap().is_some());
        // Scratch no longer.
        assert!(scratch_mgr.workstream.get_entity(entity.id).unwrap().is_none());
    }

    #[tokio::test]
    async fn promote_refuses_unknown_target() {
        use crate::workstream_router::WorkstreamMemoryRouter;
        let (tmp, store, _) = setup();
        let router = Arc::new(WorkstreamMemoryRouter::new(
            tmp.path(),
            None,
            None,
            SessionWorkstream::scratch(),
        ));
        let tool = WorkstreamPromoteTool::new(store.clone(), router);
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"entity_id": uuid::Uuid::new_v4().to_string(), "target": "ghost"}),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn show_includes_ontology() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("workstreams/pat")))
            .unwrap();
        // Seed the ontology table directly.
        let ont = arawn_memory::TagOntologyStore::open(tmp.path(), "pat").unwrap();
        ont.add("falcon", arawn_memory::AddedVia::Manual).unwrap();
        ont.add("ledger", arawn_memory::AddedVia::Manual).unwrap();

        active.set("pat");
        let tool = WorkstreamShowTool::new(store.clone(), active);
        let r = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!r.is_error, "got: {}", r.content);
        let v: serde_json::Value = serde_json::from_str(&r.content).unwrap();
        let tags: Vec<String> = v["tags_ontology"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        assert!(tags.contains(&"falcon".to_string()));
        assert!(tags.contains(&"ledger".to_string()));
    }

    #[tokio::test]
    async fn list_marks_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = WorkstreamListTool::new(store.clone()).with_active(active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        // Active workstream should be flagged.
        assert!(result.content.contains("\"active\":true"));
    }
}

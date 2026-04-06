use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

use arawn_core::{Message, Session, Workstream};
use arawn_engine::{
    BackgroundTaskManager, Compactor, PlanModeState, PermissionChecker, PermissionRule,
    QueryEngine, QueryEngineConfig, ToolContext, ToolRegistry,
};
use arawn_llm::LlmClient;
use arawn_service::{
    ArawnService, EngineEvent, ServiceError, SessionDetail, SessionInfo, WorkstreamInfo,
};
use arawn_storage::{JsonlMessageStore, Store, workstream_dir_name};

use crate::channel_prompt::{ChannelModalPrompt, PendingModals};

/// In-process implementation of ArawnService.
/// Wraps engine + store + tools and bridges to the EngineEvent stream.
/// Store is behind a std::sync::Mutex since rusqlite::Connection isn't Send.
pub struct LocalService {
    store: Arc<Mutex<Store>>,
    data_dir: PathBuf,
    llm: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    config: QueryEngineConfig,
    /// Shared permission rules — updated by ConfigWatcher on hot-reload.
    permission_rules: Arc<std::sync::RwLock<Vec<PermissionRule>>>,
    /// Shared skill registry — updated by plugin hot-reload.
    skill_registry: Option<Arc<arawn_engine::skills::SkillRegistry>>,
    /// Shared plugin registry — tracks loaded plugins.
    plugin_registry: Option<Arc<arawn_engine::plugins::PluginRegistry>>,
    /// Shared map for routing user input responses back to waiting tools.
    pub pending_modals: PendingModals,
    /// Shared plan mode state — persists across messages within the service lifetime.
    plan_state: Arc<PlanModeState>,
    /// Shared background task manager — tracks running background tasks.
    background_tasks: Arc<BackgroundTaskManager>,
    /// Shared memory manager — two-tier KB (global + workstream).
    memory_manager: Option<Arc<arawn_memory::MemoryManager>>,
}

impl LocalService {
    pub fn new(
        store: Store,
        data_dir: PathBuf,
        llm: Arc<dyn LlmClient>,
        registry: Arc<ToolRegistry>,
        config: QueryEngineConfig,
    ) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            data_dir,
            llm,
            registry,
            config,
            permission_rules: Arc::new(std::sync::RwLock::new(Vec::new())),
            skill_registry: None,
            plugin_registry: None,
            pending_modals: crate::new_pending_modals(),
            plan_state: Arc::new(PlanModeState::new()),
            background_tasks: Arc::new(BackgroundTaskManager::new()),
            memory_manager: None,
        }
    }

    pub fn with_permission_rules(mut self, rules: Vec<PermissionRule>) -> Self {
        *self.permission_rules.write().unwrap() = rules;
        self
    }

    /// Get a reference to the shared permission rules for hot-reload.
    pub fn shared_permission_rules(&self) -> Arc<std::sync::RwLock<Vec<PermissionRule>>> {
        Arc::clone(&self.permission_rules)
    }

    pub fn with_skill_registry(mut self, registry: Arc<arawn_engine::skills::SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    pub fn with_plugin_registry(mut self, registry: Arc<arawn_engine::plugins::PluginRegistry>) -> Self {
        self.plugin_registry = Some(registry);
        self
    }

    pub fn with_plan_state(mut self, state: Arc<PlanModeState>) -> Self {
        self.plan_state = state;
        self
    }

    pub fn with_background_tasks(mut self, manager: Arc<BackgroundTaskManager>) -> Self {
        self.background_tasks = manager;
        self
    }

    pub fn with_memory_manager(mut self, mgr: Arc<arawn_memory::MemoryManager>) -> Self {
        self.memory_manager = Some(mgr);
        self
    }

    /// Query available inventory for slash commands.
    /// Returns a JSON array of {name, description, source} items.
    pub fn query_inventory(&self, kind: &str) -> serde_json::Value {
        match kind {
            "tools" => {
                let tools = self.registry.tool_definitions();
                let items: Vec<serde_json::Value> = tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": first_sentence(&t.description),
                        })
                    })
                    .collect();
                serde_json::json!(items)
            }
            "skills" => {
                if let Some(ref reg) = self.skill_registry {
                    let items: Vec<serde_json::Value> = reg
                        .all()
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "name": s.name,
                                "description": first_sentence(&s.description),
                                "user_invocable": s.user_invocable,
                            })
                        })
                        .collect();
                    serde_json::json!(items)
                } else {
                    serde_json::json!([])
                }
            }
            "plugins" => {
                if let Some(ref reg) = self.plugin_registry {
                    let items: Vec<serde_json::Value> = reg
                        .all()
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "name": p.name(),
                                "description": p.manifest.description.as_deref().unwrap_or(""),
                                "enabled": p.enabled,
                            })
                        })
                        .collect();
                    serde_json::json!(items)
                } else {
                    serde_json::json!([])
                }
            }
            "agents" => {
                // Agent definitions are loaded into the AgentTool — list from agent_defs
                let agents = arawn_engine::agent_defs::built_in_agents();
                let items: Vec<serde_json::Value> = agents
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "name": a.name,
                            "description": first_sentence(&a.when_to_use),
                        })
                    })
                    .collect();
                serde_json::json!(items)
            }
            "mcp" => {
                // MCP servers aren't directly accessible from here — return empty for now
                // TODO: expose McpManager connected servers
                serde_json::json!([])
            }
            _ => serde_json::json!([]),
        }
    }

    /// List available commands (built-ins + user-invocable skills) for autocomplete cache.
    pub fn list_available_commands(&self) -> serde_json::Value {
        let mut commands: Vec<serde_json::Value> = Vec::new();

        // User-invocable skills become slash commands
        if let Some(ref reg) = self.skill_registry {
            for skill in reg.user_invocable() {
                commands.push(serde_json::json!({
                    "name": skill.name,
                    "description": first_sentence(&skill.description),
                    "kind": "skill",
                }));
            }
        }

        serde_json::json!(commands)
    }

    /// Store a fact in the KB via /remember command.
    pub fn remember_fact(&self, text: &str) -> serde_json::Value {
        use arawn_memory::{ConfidenceSource, Entity, EntityType};

        let memory = match self.memory_manager.as_ref() {
            Some(m) => m,
            None => return serde_json::json!({"error": "Memory system not available"}),
        };

        // Infer entity type from text patterns
        let (entity_type, title) = infer_entity_type(text);
        let mut entity = Entity::new(entity_type, &title)
            .with_confidence(ConfidenceSource::Stated);

        // If the text is longer than the title, store the full text as content
        if text.len() > title.len() + 5 {
            entity = entity.with_content(text);
        }

        let store = memory.store_for_type(entity_type);
        match store.store_fact(&entity) {
            Ok(arawn_memory::StoreFactResult::Inserted { entity_id }) => {
                serde_json::json!({
                    "status": "inserted",
                    "entity_id": entity_id.to_string(),
                    "title": title,
                    "entity_type": entity_type.as_str(),
                })
            }
            Ok(arawn_memory::StoreFactResult::Reinforced { entity_id, new_count }) => {
                serde_json::json!({
                    "status": "reinforced",
                    "entity_id": entity_id.to_string(),
                    "title": title,
                    "count": new_count,
                })
            }
            Ok(arawn_memory::StoreFactResult::Superseded { old_entity_id, new_entity_id }) => {
                serde_json::json!({
                    "status": "superseded",
                    "old_id": old_entity_id.to_string(),
                    "new_id": new_entity_id.to_string(),
                    "title": title,
                })
            }
            Err(e) => serde_json::json!({"error": e.to_string()}),
        }
    }

    /// Get KB summary for /memory command.
    pub fn memory_summary(&self) -> serde_json::Value {
        use arawn_memory::EntityType;

        let memory = match self.memory_manager.as_ref() {
            Some(m) => m,
            None => return serde_json::json!({"error": "Memory system not available"}),
        };

        let types = [
            EntityType::Fact,
            EntityType::Decision,
            EntityType::Convention,
            EntityType::Preference,
            EntityType::Person,
            EntityType::Note,
        ];

        let mut global_counts = Vec::new();
        let mut ws_counts = Vec::new();

        for et in &types {
            let g = memory.global.count_by_type(*et).unwrap_or(0);
            let w = memory.workstream.count_by_type(*et).unwrap_or(0);
            if g > 0 {
                global_counts.push(serde_json::json!({"type": et.as_str(), "count": g}));
            }
            if w > 0 {
                ws_counts.push(serde_json::json!({"type": et.as_str(), "count": w}));
            }
        }

        let global_total = memory.global.count_all().unwrap_or(0);
        let ws_total = memory.workstream.count_all().unwrap_or(0);

        serde_json::json!({
            "global": {"total": global_total, "by_type": global_counts},
            "workstream": {"total": ws_total, "by_type": ws_counts},
        })
    }

    /// Forget/delete an entity via /forget command.
    pub fn forget_entity(&self, query: &str) -> serde_json::Value {
        let memory = match self.memory_manager.as_ref() {
            Some(m) => m,
            None => return serde_json::json!({"error": "Memory system not available"}),
        };

        // Search both stores for matching entities
        let mut candidates = Vec::new();
        for (store, label) in [(&memory.global, "global"), (&memory.workstream, "workstream")] {
            if let Ok(results) = store.search(query, 5) {
                for e in results {
                    candidates.push((e, label));
                }
            }
        }

        if candidates.is_empty() {
            return serde_json::json!({"error": format!("No entities matching '{query}' found")});
        }

        // If exact single match, delete it
        if candidates.len() == 1 {
            let (entity, label) = &candidates[0];
            let store = if *label == "global" { &memory.global } else { &memory.workstream };
            match store.delete_entity(entity.id) {
                Ok(true) => serde_json::json!({
                    "status": "deleted",
                    "title": entity.title,
                    "entity_type": entity.entity_type.as_str(),
                    "scope": label,
                }),
                Ok(false) => serde_json::json!({"error": "Entity not found"}),
                Err(e) => serde_json::json!({"error": e.to_string()}),
            }
        } else {
            // Multiple matches — return candidates for user to choose
            let items: Vec<serde_json::Value> = candidates
                .iter()
                .map(|(e, label)| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "title": e.title,
                        "type": e.entity_type.as_str(),
                        "scope": label,
                    })
                })
                .collect();
            serde_json::json!({"status": "ambiguous", "candidates": items})
        }
    }
}

/// Infer entity type from text patterns.
fn infer_entity_type(text: &str) -> (arawn_memory::EntityType, String) {
    use arawn_memory::EntityType;
    let lower = text.to_lowercase();

    if lower.starts_with("i prefer") || lower.starts_with("prefer ") || lower.contains("preference") {
        (EntityType::Preference, text.to_string())
    } else if lower.starts_with("we decided") || lower.starts_with("decision:") || lower.contains("decided to") {
        (EntityType::Decision, text.to_string())
    } else if lower.starts_with("convention:") || lower.starts_with("the convention is") || lower.starts_with("always ") || lower.starts_with("never ") {
        (EntityType::Convention, text.to_string())
    } else {
        (EntityType::Fact, text.to_string())
    }
}

use async_trait::async_trait;

#[async_trait]
impl ArawnService for LocalService {
    async fn list_workstreams(&self) -> Result<Vec<WorkstreamInfo>, ServiceError> {
        let store = self.store.lock().unwrap();
        let workstreams = store
            .list_workstreams()
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(workstreams
            .into_iter()
            .map(|ws| WorkstreamInfo {
                id: ws.id,
                name: ws.name,
                root_dir: ws.root_dir,
                created_at: ws.created_at,
            })
            .collect())
    }

    async fn create_workstream(
        &self,
        name: String,
        root_dir: PathBuf,
    ) -> Result<WorkstreamInfo, ServiceError> {
        let ws = Workstream::new(&name, &root_dir);
        let store = self.store.lock().unwrap();
        store
            .create_workstream(&ws)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(WorkstreamInfo {
            id: ws.id,
            name: ws.name,
            root_dir: ws.root_dir,
            created_at: ws.created_at,
        })
    }

    async fn list_sessions(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<Vec<SessionInfo>, ServiceError> {
        let store = self.store.lock().unwrap();
        let metas = match workstream_id {
            Some(ws_id) => store.list_sessions_for_workstream(ws_id),
            None => store.list_scratch_sessions(),
        }
        .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(metas
            .into_iter()
            .map(|m| SessionInfo {
                id: m.id,
                workstream_id: m.workstream_id,
                created_at: m.created_at,
            })
            .collect())
    }

    async fn create_session(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<SessionInfo, ServiceError> {
        let session = match workstream_id {
            Some(ws_id) => Session::new(ws_id),
            None => Session::scratch(),
        };

        let store = self.store.lock().unwrap();
        store
            .create_session(&session)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        info!(session_id = %session.id, "session created via service");

        Ok(SessionInfo {
            id: session.id,
            workstream_id: session.workstream_id(),
            created_at: session.created_at,
        })
    }

    async fn load_session(&self, id: Uuid) -> Result<SessionDetail, ServiceError> {
        // Get metadata from SQLite (sync, hold lock briefly)
        let (meta, ws_dir) = {
            let store = self.store.lock().unwrap();
            let meta = store
                .get_session_meta(id)
                .map_err(|e| ServiceError::Storage(e.to_string()))?
                .ok_or_else(|| ServiceError::NotFound(format!("session {id}")))?;

            let ws_dir = resolve_ws_dir_from_store(&store, meta.workstream_id)?;
            (meta, ws_dir)
        };

        // Load messages from JSONL (async, no lock needed)
        let msg_store = JsonlMessageStore::new(&self.data_dir);
        let all_messages = msg_store
            .load(id, &ws_dir)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let messages = Session::load_compacted(all_messages);

        Ok(SessionDetail {
            id: meta.id,
            workstream_id: meta.workstream_id,
            created_at: meta.created_at,
            messages,
        })
    }

    async fn send_message(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = EngineEvent> + Send>>, ServiceError> {
        // Load session metadata and resolve workstream
        let (meta, workstream, ws_dir) = {
            let store = self.store.lock().unwrap();
            let meta = store
                .get_session_meta(session_id)
                .map_err(|e| ServiceError::Storage(e.to_string()))?
                .ok_or_else(|| ServiceError::NotFound(format!("session {session_id}")))?;

            let ws_dir = resolve_ws_dir_from_store(&store, meta.workstream_id)?;

            let workstream = if let Some(ws_id) = meta.workstream_id {
                store
                    .get_workstream(ws_id)
                    .map_err(|e| ServiceError::Storage(e.to_string()))?
                    .ok_or_else(|| ServiceError::NotFound(format!("workstream {ws_id}")))?
            } else {
                store
                    .find_workstream_by_name("scratch")
                    .map_err(|e| ServiceError::Storage(e.to_string()))?
                    .ok_or_else(|| ServiceError::NotFound("scratch workstream".into()))?
            };

            (meta, workstream, ws_dir)
        };

        // Load messages from JSONL
        let msg_store = JsonlMessageStore::new(&self.data_dir);
        let all_messages = msg_store
            .load(session_id, &ws_dir)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let messages = Session::load_compacted(all_messages);

        let mut session =
            Session::from_parts(meta.id, meta.workstream_id, meta.created_at, messages);

        // Add user message and persist
        let user_msg = Message::User {
            content: content.clone(),
        };
        session.add_message(user_msg.clone());

        let message_store = JsonlMessageStore::new(&self.data_dir);
        message_store
            .append(session_id, &ws_dir, &user_msg)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        // Resolve sandbox root for ToolContext
        // Scratch workstream sessions get per-session sandboxes
        let is_scratch = workstream.name == "scratch";
        let workspace_dir = msg_store.sandbox_dir(&ws_dir, session_id, is_scratch);

        // Ensure workspace directory exists
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        // Build ToolContext with the workspace as working directory
        let mut ws_for_ctx = workstream.clone();
        ws_for_ctx.root_dir = workspace_dir.clone();

        // Allow file tools to access arawn.md context files outside the sandbox
        let global_arawn_md = self.data_dir.join("arawn.md");
        let workstream_arawn_md = self
            .data_dir
            .join("workstreams")
            .join(&ws_dir)
            .join("arawn.md");
        let ctx = ToolContext::new(&ws_for_ctx, session.id)
            .with_allowed_paths(vec![global_arawn_md, workstream_arawn_md])
            .with_llm(self.llm.clone(), self.config.model.clone())
            .with_model_limits(self.config.model_limits.clone())
            .with_data_dir(self.data_dir.clone());

        // Build PromptContext per-session using the actual workspace
        let session_prompt_context =
            self.config
                .prompt_context
                .as_ref()
                .map(|pc| arawn_engine::PromptContext {
                    prompts_dir: pc.prompts_dir.clone(),
                    os: pc.os.clone(),
                    shell: pc.shell.clone(),
                    cwd: workspace_dir.clone(),
                    workstream_name: workstream.name.clone(),
                    workstream_root: workspace_dir.clone(),
                    context_files: arawn_engine::find_context_files(&workspace_dir, &self.data_dir),
                    memories: pc.memories.clone(),
                    session_context: pc.session_context.clone(),
                    plugin_prompts: pc.plugin_prompts.clone(),
                });

        // Build engine with compactor
        let compactor = Compactor::new(self.llm.clone(), self.config.model.clone());
        let mut engine = QueryEngine::with_config(
            self.llm.clone(),
            self.registry.clone(),
            QueryEngineConfig {
                model: self.config.model.clone(),
                max_iterations: self.config.max_iterations,
                system_prompt: self.config.system_prompt.clone(),
                max_tokens: self.config.max_tokens,
                model_limits: self.config.model_limits.clone(),
                data_dir: Some(self.data_dir.clone()),
                prompt_context: session_prompt_context,
            },
        )
        .with_compactor(compactor);

        if let Some(ref skill_reg) = self.skill_registry {
            engine = engine.with_skill_registry(Arc::clone(skill_reg));
        }
        if let Some(ref plugin_reg) = self.plugin_registry {
            engine = engine.with_plugin_registry(Arc::clone(plugin_reg));
        }

        let msgs_before = session.messages().len();
        let (tx, rx) = mpsc::channel::<EngineEvent>(64);

        // Create per-message permission checker with a prompt that sends
        // UserInputRequest events through the engine's tx channel.
        {
            let rules = self.permission_rules.read().unwrap().clone();
            if !rules.is_empty() {
                let prompt = ChannelModalPrompt::new(tx.clone(), self.pending_modals.clone());
                let checker = PermissionChecker::new(rules)
                    .with_prompter(Box::new(prompt));
                engine = engine.with_permission_checker(Arc::new(checker));
            }
        }

        engine = engine
            .with_plan_state(Arc::clone(&self.plan_state))
            .with_background_tasks(Arc::clone(&self.background_tasks));

        // Set up live progress channel so tool calls stream to the TUI
        // during the engine loop, not just after it completes.
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::channel::<arawn_engine::ProgressEvent>(64);
        engine = engine.with_progress_sender(progress_tx);

        let data_dir = self.data_dir.clone();
        let ws_dir_owned = ws_dir.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let msg_store = JsonlMessageStore::new(&data_dir);

            // Forward progress events inline (same task as engine, no race condition).
            // We drain the progress channel after each await point in the engine loop
            // by spawning a forwarder that the engine feeds into.
            let event_tx_progress = tx.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(event) = progress_rx.recv().await {
                    match event {
                        arawn_engine::ProgressEvent::AssistantText { content } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::StreamingText { text: content })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                        arawn_engine::ProgressEvent::ToolCallStart { id, name, input } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::ToolCallStart { id, name, input })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                        arawn_engine::ProgressEvent::ToolCallResult {
                            id,
                            content,
                            is_error,
                        } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::ToolCallResult {
                                    id,
                                    content,
                                    is_error,
                                })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                    }
                }
            });

            let engine_result = engine.run(&mut session, &ctx).await;

            // Engine dropped progress_tx on return → forwarder drains remaining events.
            // Wait for it to finish so all progress events are sent before Complete.
            let _ = forwarder.await;

            match engine_result {
                Ok(final_text) => {
                    // Tool call events were already streamed live via progress_tx.
                    // Only emit compaction events here (not streamed live).
                    let new_msgs = &session.messages()[msgs_before..];
                    for msg in new_msgs {
                        if let Message::Summary { original_count, .. } = msg {
                            let _ = tx
                                .send(EngineEvent::CompactionOccurred {
                                    messages_summarized: *original_count,
                                })
                                .await;
                            let _ = tx.send(EngineEvent::Flush).await;
                        }
                    }

                    // Persist new messages
                    for msg in &session.messages()[msgs_before..] {
                        if let Err(e) = msg_store.append(session_id, &ws_dir_owned, msg).await {
                            error!(error = %e, "failed to persist message");
                        }
                    }

                    // Persist stats
                    if let Ok(s) = store.lock() {
                        let _ = s.update_session_stats(session_id, &session.stats);
                    }

                    let _ = tx.send(EngineEvent::Usage {
                        input_tokens: session.stats.input_tokens,
                        output_tokens: session.stats.output_tokens,
                    }).await;
                    let _ = tx.send(EngineEvent::Complete { final_text }).await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
                Err(e) => {
                    for msg in &session.messages()[msgs_before..] {
                        let _ = msg_store.append(session_id, &ws_dir_owned, msg).await;
                    }
                    if let Ok(s) = store.lock() {
                        let _ = s.update_session_stats(session_id, &session.stats);
                    }
                    let _ = tx
                        .send(EngineEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn cancel(&self, _session_id: Uuid) -> Result<(), ServiceError> {
        // TODO: implement cancellation via CancellationToken
        debug!("cancel requested (not yet implemented)");
        Ok(())
    }
}

/// Resolve workstream directory name from store. Returns "scratch" for None.
fn resolve_ws_dir_from_store(store: &Store, ws_id: Option<Uuid>) -> Result<String, ServiceError> {
    match ws_id {
        Some(id) => {
            let ws = store
                .get_workstream(id)
                .map_err(|e| ServiceError::Storage(e.to_string()))?
                .ok_or_else(|| ServiceError::NotFound(format!("workstream {id}")))?;
            Ok(workstream_dir_name(&ws.name, ws.id))
        }
        None => Ok("scratch".to_string()),
    }
}

/// Extract the first sentence and sanitize for use in a markdown table cell.
/// Collapses whitespace, strips pipe chars, and cuts at the first sentence boundary.
fn first_sentence(s: &str) -> String {
    // Collapse all whitespace (newlines, tabs, multiple spaces) to single spaces
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    // Strip pipe chars which break markdown table cells
    let clean = collapsed.replace('|', "-");
    // Cut at first sentence boundary
    if let Some(pos) = clean.find(". ") {
        clean[..=pos].to_string()
    } else {
        clean
    }
}

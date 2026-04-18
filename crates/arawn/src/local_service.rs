use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use arawn_core::{Message, Session, Workstream};
use arawn_engine::{
    BackgroundTaskManager, Compactor, PlanModeState, PermissionChecker, PermissionRule,
    QueryEngine, QueryEngineConfig, ToolContext, ToolRegistry,
};
use tracing::instrument;
use arawn_llm::LlmClient;
use arawn_service::{
    ArawnService, CommandInfo, EngineEvent, ForgetCandidate, ForgetResult, InventoryItem,
    MemoryStoreResult, MemoryStoreSummary, MemorySummary, MemoryTypeCount, PermissionModeInfo,
    PromotionResult, ServiceError, SessionDetail, SessionInfo, WorkflowInfo, WorkstreamInfo,
};
use arawn_storage::{JsonlMessageStore, Store, workstream_dir_name};

use crate::channel_prompt::{ChannelModalPrompt, PendingModals};
use crate::llm_pool::LlmClientPool;

/// In-process implementation of ArawnService.
/// Wraps engine + store + tools and bridges to the EngineEvent stream.
/// Store is behind a std::sync::Mutex since rusqlite::Connection isn't Send.
pub struct LocalService {
    store: Arc<Mutex<Store>>,
    pub(crate) data_dir: PathBuf,
    /// Source of all LLM clients. The engine and compactor are resolved
    /// through here; tools and agents that adopt `LlmPreference` will
    /// resolve via the same pool.
    llm_pool: Arc<LlmClientPool>,
    registry: Arc<ToolRegistry>,
    config: QueryEngineConfig,
    /// Shared permission rules — updated by ConfigWatcher on hot-reload.
    permission_rules: Arc<std::sync::RwLock<Vec<PermissionRule>>>,
    /// Shared permission mode — toggled at runtime via /accept commands.
    permission_mode: Arc<std::sync::RwLock<arawn_engine::permissions::PermissionMode>>,
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
    /// Tracks sessions with active send_message calls to prevent concurrent access.
    active_sessions: Arc<Mutex<HashSet<Uuid>>>,
    /// Cancellation tokens for active engine runs, keyed by session ID.
    cancel_tokens: Arc<Mutex<HashMap<Uuid, tokio_util::sync::CancellationToken>>>,
}

impl LocalService {
    pub fn new(
        store: Store,
        data_dir: PathBuf,
        llm_pool: Arc<LlmClientPool>,
        registry: Arc<ToolRegistry>,
        config: QueryEngineConfig,
    ) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            data_dir,
            llm_pool,
            registry,
            config,
            permission_rules: Arc::new(std::sync::RwLock::new(Vec::new())),
            permission_mode: Arc::new(std::sync::RwLock::new(arawn_engine::permissions::PermissionMode::Default)),
            skill_registry: None,
            plugin_registry: None,
            pending_modals: crate::new_pending_modals(),
            plan_state: Arc::new(PlanModeState::new()),
            background_tasks: Arc::new(BackgroundTaskManager::new()),
            memory_manager: None,
            active_sessions: Arc::new(Mutex::new(HashSet::new())),
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_permission_rules(self, rules: Vec<PermissionRule>) -> Self {
        *self.permission_rules.write().unwrap() = rules;
        self
    }

    /// Get a reference to the shared permission rules for hot-reload.
    /// Get a shared reference to the store for tools that need direct access.
    pub fn shared_store(&self) -> Arc<Mutex<Store>> {
        Arc::clone(&self.store)
    }

    pub fn shared_llm(&self) -> Arc<dyn LlmClient> {
        self.llm_pool.engine()
    }

    /// Compactor LLM (separate client when `[compactor]` config selects a
    /// different `[llm.*]` entry; otherwise an `Arc` clone of the engine LLM).
    pub fn shared_compactor_llm(&self) -> Arc<dyn LlmClient> {
        self.llm_pool.compactor()
    }

    /// Model name used by the compactor.
    pub fn compactor_model(&self) -> &str {
        &self.llm_pool.compactor_config().model
    }

    /// Shared reference to the LLM pool — used by tools/agents that resolve
    /// via [`LlmPreference`].
    pub fn shared_llm_pool(&self) -> Arc<LlmClientPool> {
        Arc::clone(&self.llm_pool)
    }

    pub fn shared_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.registry)
    }

    pub fn engine_config(&self) -> &QueryEngineConfig {
        &self.config
    }

    pub fn shared_permission_rules(&self) -> Arc<std::sync::RwLock<Vec<PermissionRule>>> {
        Arc::clone(&self.permission_rules)
    }

    pub fn shared_permission_mode(&self) -> Arc<std::sync::RwLock<arawn_engine::permissions::PermissionMode>> {
        Arc::clone(&self.permission_mode)
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

    /// Load session metadata, resolve workstream, and load message history.
    #[instrument(skip_all, fields(%session_id))]
    fn load_session_state(
        &self,
        session_id: Uuid,
    ) -> Result<(arawn_storage::SessionMeta, Workstream, String, Vec<Message>), ServiceError> {
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

        Ok((meta, workstream, ws_dir, Vec::new()))
    }

    /// Build a ToolContext and per-session PromptContext for the engine.
    #[instrument(skip_all, fields(%session_id))]
    fn build_session_context(
        &self,
        session_id: Uuid,
        workstream: &Workstream,
        ws_dir: &str,
        workspace_dir: &std::path::Path,
        content: &str,
    ) -> (ToolContext, Option<arawn_engine::PromptContext>) {
        let mut ws_for_ctx = workstream.clone();
        ws_for_ctx.root_dir = workspace_dir.to_path_buf();

        let global_arawn_md = self.data_dir.join("arawn.md");
        let workstream_arawn_md = self
            .data_dir
            .join("workstreams")
            .join(ws_dir)
            .join("arawn.md");
        let pool: Arc<crate::LlmClientPool> = Arc::clone(&self.llm_pool);
        let resolver: Arc<dyn arawn_tool::LlmResolver> = pool;
        let ctx = ToolContext::new(&ws_for_ctx, session_id)
            .with_allowed_paths(vec![global_arawn_md, workstream_arawn_md])
            .with_llm(self.llm_pool.engine(), self.config.model.clone())
            .with_llm_resolver(resolver)
            .with_model_limits(self.config.model_limits.clone())
            .with_data_dir(self.data_dir.clone());

        let prompt_context = self.config.prompt_context.as_ref().map(|pc| {
            arawn_engine::PromptContext {
                prompts_dir: pc.prompts_dir.clone(),
                os: pc.os.clone(),
                shell: pc.shell.clone(),
                cwd: workspace_dir.to_path_buf(),
                workstream_name: workstream.name.clone(),
                workstream_root: workspace_dir.to_path_buf(),
                context_files: arawn_engine::find_context_files(workspace_dir, &self.data_dir),
                memories: self
                    .memory_manager
                    .as_ref()
                    .map(|mgr| {
                        let stack = arawn_memory::MemoryStack::new(mgr, &workstream.name);
                        let mut mems = vec![stack.wake_up(900)];

                        let keywords: Vec<String> = content
                            .split_whitespace()
                            .filter(|w| w.len() > 3)
                            .map(|w| {
                                w.trim_matches(|c: char| !c.is_alphanumeric())
                                    .to_lowercase()
                            })
                            .filter(|w| !w.is_empty())
                            .collect();
                        if !keywords.is_empty() {
                            let l1_titles = stack.l1_entity_titles();
                            if let Some(l2) = stack.topical_context(&keywords, &l1_titles, 400) {
                                mems.push(l2);
                            }
                        }

                        mems
                    })
                    .unwrap_or_else(|| pc.memories.clone()),
                session_context: pc.session_context.clone(),
                plugin_prompts: pc.plugin_prompts.clone(),
            }
        });

        (ctx, prompt_context)
    }

    /// Build a QueryEngine configured with compactor, skills, plugins, and plan state.
    #[instrument(skip_all)]
    fn build_engine(
        &self,
        prompt_context: Option<arawn_engine::PromptContext>,
        event_tx: &mpsc::Sender<EngineEvent>,
    ) -> QueryEngine {
        let compactor = Compactor::new(
            self.llm_pool.compactor(),
            self.llm_pool.compactor_config().model.clone(),
        );
        let mut engine = QueryEngine::with_config(
            self.llm_pool.engine(),
            self.registry.clone(),
            QueryEngineConfig {
                model: self.config.model.clone(),
                max_iterations: self.config.max_iterations,
                system_prompt: self.config.system_prompt.clone(),
                max_tokens: self.config.max_tokens,
                model_limits: self.config.model_limits.clone(),
                data_dir: Some(self.data_dir.clone()),
                prompt_context,
            },
        )
        .with_compactor(compactor);

        if let Some(ref skill_reg) = self.skill_registry {
            engine = engine.with_skill_registry(Arc::clone(skill_reg));
        }
        if let Some(ref plugin_reg) = self.plugin_registry {
            engine = engine.with_plugin_registry(Arc::clone(plugin_reg));
        }

        // Attach permission checker
        {
            let rules = self.permission_rules.read().unwrap().clone();
            if !rules.is_empty() {
                let prompt =
                    ChannelModalPrompt::new(event_tx.clone(), self.pending_modals.clone());
                let mode = *self.permission_mode.read().unwrap();
                let checker = PermissionChecker::new(rules)
                    .with_mode(mode)
                    .with_prompter(Box::new(prompt));
                engine = engine.with_permission_checker(Arc::new(checker));
            }
        }

        engine
            .with_plan_state(Arc::clone(&self.plan_state))
            .with_background_tasks(Arc::clone(&self.background_tasks))
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

    #[instrument(skip_all, fields(%session_id))]
    async fn send_message(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = EngineEvent> + Send>>, ServiceError> {
        // Prevent concurrent send_message calls to the same session
        {
            let mut active = self.active_sessions.lock().unwrap();
            if !active.insert(session_id) {
                return Err(ServiceError::InvalidOperation(
                    "Session is currently processing a message. Wait for the current request to complete.".into(),
                ));
            }
        }

        // Load session state
        let (meta, workstream, ws_dir, _) = self.load_session_state(session_id)?;

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

        // Resolve workspace directory
        let is_scratch = workstream.name == "scratch";
        let workspace_dir = msg_store.sandbox_dir(&ws_dir, session_id, is_scratch);
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        // Build context and engine
        let ws_dir_owned = ws_dir.clone();
        let (ctx, prompt_context) =
            self.build_session_context(session_id, &workstream, &ws_dir, &workspace_dir, &content);

        let msgs_before = session.messages().len();
        let (tx, rx) = mpsc::channel::<EngineEvent>(64);

        let mut engine = self.build_engine(prompt_context, &tx);

        // Set up live progress channel
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::channel::<arawn_engine::ProgressEvent>(64);
        engine = engine.with_progress_sender(progress_tx);

        // Create cancellation token for this session
        let cancel_token = tokio_util::sync::CancellationToken::new();
        engine = engine.with_cancel_token(cancel_token.clone());
        self.cancel_tokens.lock().unwrap().insert(session_id, cancel_token);

        let data_dir = self.data_dir.clone();
        let store = self.store.clone();
        let active_sessions = self.active_sessions.clone();
        let cancel_tokens = self.cancel_tokens.clone();

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

            // Drop the engine to release progress_tx — this closes the channel
            // so the forwarder can drain remaining events and exit.
            drop(engine);
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
                    let mut persist_errors = Vec::new();
                    for msg in &session.messages()[msgs_before..] {
                        if let Err(e) = msg_store.append(session_id, &ws_dir_owned, msg).await {
                            error!(error = %e, "failed to persist message");
                            persist_errors.push(e.to_string());
                        }
                    }

                    // Persist stats
                    if let Ok(s) = store.lock() {
                        if let Err(e) = s.update_session_stats(session_id, &session.stats) {
                            warn!(error = %e, "failed to update session stats");
                        }
                    }

                    let _ = tx.send(EngineEvent::Usage {
                        input_tokens: session.stats.input_tokens,
                        output_tokens: session.stats.output_tokens,
                    }).await;

                    // Surface persistence failures as warnings before Complete
                    if !persist_errors.is_empty() {
                        let _ = tx.send(EngineEvent::Warning {
                            message: format!(
                                "Some messages could not be saved to disk ({} error{}). Your conversation may not survive a restart.",
                                persist_errors.len(),
                                if persist_errors.len() == 1 { "" } else { "s" }
                            ),
                        }).await;
                    }

                    let _ = tx.send(EngineEvent::Complete { final_text }).await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
                Err(e) => {
                    for msg in &session.messages()[msgs_before..] {
                        if let Err(pe) = msg_store.append(session_id, &ws_dir_owned, msg).await {
                            error!(error = %pe, "failed to persist message in error path");
                        }
                    }
                    if let Ok(s) = store.lock() {
                        if let Err(se) = s.update_session_stats(session_id, &session.stats) {
                            warn!(error = %se, "failed to update session stats in error path");
                        }
                    }
                    let _ = tx
                        .send(EngineEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
            }

            // Release the session lock and cancel token so new messages can be sent
            active_sessions.lock().unwrap().remove(&session_id);
            cancel_tokens.lock().unwrap().remove(&session_id);
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn cancel(&self, session_id: Uuid) -> Result<(), ServiceError> {
        let token = self.cancel_tokens.lock().unwrap().get(&session_id).cloned();
        match token {
            Some(token) => {
                info!(%session_id, "cancelling engine run");
                token.cancel();
                Ok(())
            }
            None => {
                debug!(%session_id, "cancel requested but no active engine run");
                Ok(())
            }
        }
    }

    async fn promote_session(
        &self,
        session_id: Uuid,
        workstream_name: &str,
    ) -> Result<PromotionResult, ServiceError> {
        let (ws_id, ws_name, ws_dir, scratch_workspace, target_workspace) = {
            let store = self.store.lock().unwrap();
            let ws = store
                .find_workstream_by_name(workstream_name)
                .map_err(|e| ServiceError::Storage(e.to_string()))?
                .ok_or_else(|| {
                    ServiceError::NotFound(format!("workstream '{workstream_name}'"))
                })?;

            let ws_dir = arawn_storage::workstream_dir_name(&ws.name, ws.id);
            let scratch_ws = store.sandbox_for("scratch", session_id, true).join("workspace");
            let target_ws = store.sandbox_for(&ws_dir, session_id, false).join("workspace");

            (ws.id, ws.name, ws_dir, scratch_ws, target_ws)
        };

        let msg_store = arawn_storage::JsonlMessageStore::new(&self.data_dir);
        msg_store
            .move_session(session_id, "scratch", &ws_dir)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let sqlite_result = {
            let store = self.store.lock().unwrap();
            store.promote_session_metadata(session_id, ws_id)
        };
        if let Err(e) = sqlite_result {
            warn!(error = %e, "SQLite update failed during promotion, rolling back file move");
            let _ = msg_store.move_session(session_id, &ws_dir, "scratch").await;
            return Err(ServiceError::Storage(e.to_string()));
        }

        if scratch_workspace.exists() {
            let _ = tokio::fs::create_dir_all(
                target_workspace.parent().unwrap_or(&target_workspace),
            )
            .await;
            if let Err(e) = tokio::fs::rename(&scratch_workspace, &target_workspace).await {
                warn!(error = %e, "workspace rename failed during promotion, files remain in scratch");
            }
        }

        Ok(PromotionResult {
            workstream_id: ws_id.to_string(),
            workstream_name: ws_name,
        })
    }

    async fn resolve_user_input(
        &self,
        request_id: &str,
        selected_index: Option<usize>,
    ) -> Result<(), ServiceError> {
        let mut pending = self.pending_modals.lock().unwrap();
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(selected_index);
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!(
                "no pending modal for {request_id}"
            )))
        }
    }

    async fn query_inventory(&self, kind: &str) -> Result<Vec<InventoryItem>, ServiceError> {
        let items = match kind {
            "tools" => self
                .registry
                .tool_definitions()
                .iter()
                .map(|t| InventoryItem {
                    name: t.name.clone(),
                    description: first_sentence(&t.description),
                    kind: None,
                    enabled: None,
                    user_invocable: None,
                })
                .collect(),
            "skills" => {
                if let Some(ref reg) = self.skill_registry {
                    reg.all()
                        .iter()
                        .map(|s| InventoryItem {
                            name: s.name.clone(),
                            description: first_sentence(&s.description),
                            kind: None,
                            enabled: None,
                            user_invocable: Some(s.user_invocable),
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            "plugins" => {
                if let Some(ref reg) = self.plugin_registry {
                    reg.all()
                        .iter()
                        .map(|p| InventoryItem {
                            name: p.name().to_string(),
                            description: p
                                .manifest
                                .description
                                .as_deref()
                                .unwrap_or("")
                                .to_string(),
                            kind: None,
                            enabled: Some(p.enabled),
                            user_invocable: None,
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            "agents" => arawn_engine::agent_defs::built_in_agents()
                .iter()
                .map(|a| InventoryItem {
                    name: a.name.clone(),
                    description: first_sentence(&a.when_to_use),
                    kind: None,
                    enabled: None,
                    user_invocable: None,
                })
                .collect(),
            "mcp" => Vec::new(),
            _ => Vec::new(),
        };
        Ok(items)
    }

    async fn list_available_commands(&self) -> Result<Vec<CommandInfo>, ServiceError> {
        let mut commands = Vec::new();
        if let Some(ref reg) = self.skill_registry {
            for skill in reg.user_invocable() {
                commands.push(CommandInfo {
                    name: skill.name.clone(),
                    description: first_sentence(&skill.description),
                    kind: "skill".to_string(),
                });
            }
        }
        Ok(commands)
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowInfo>, ServiceError> {
        let workflows_dir = self.data_dir.join("workflows");
        let mut workflows = Vec::new();
        if workflows_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let pkg_toml = entry.path().join("package.toml");
                        let cron = pkg_toml
                            .exists()
                            .then(|| {
                                std::fs::read_to_string(&pkg_toml).ok().and_then(|s| {
                                    s.lines()
                                        .find(|l| l.contains("cron"))
                                        .map(|l| {
                                            l.split('=')
                                                .nth(1)
                                                .unwrap_or("")
                                                .trim()
                                                .trim_matches('"')
                                                .to_string()
                                        })
                                })
                            })
                            .flatten();
                        workflows.push(WorkflowInfo { name, cron });
                    }
                }
            }
        }
        Ok(workflows)
    }

    async fn remember_fact(&self, text: &str) -> Result<MemoryStoreResult, ServiceError> {
        use arawn_memory::{ConfidenceSource, Entity};

        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

        let (entity_type, title) = infer_entity_type(text);
        let mut entity =
            Entity::new(entity_type, &title).with_confidence(ConfidenceSource::Stated);
        if text.len() > title.len() + 5 {
            entity = entity.with_content(text);
        }

        // Use store_fact_embedded to auto-embed if embedder is available
        let result = memory
            .store_fact_embedded(&entity, None)
            .await
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        match result {
            arawn_memory::StoreFactResult::Inserted { entity_id } => {
                Ok(MemoryStoreResult::Inserted {
                    entity_id: entity_id.to_string(),
                    title,
                    entity_type: entity_type.as_str().to_string(),
                })
            }
            arawn_memory::StoreFactResult::Reinforced {
                entity_id,
                new_count,
            } => Ok(MemoryStoreResult::Reinforced {
                entity_id: entity_id.to_string(),
                title,
                count: new_count as u64,
            }),
            arawn_memory::StoreFactResult::Superseded {
                old_entity_id,
                new_entity_id,
            } => Ok(MemoryStoreResult::Superseded {
                old_id: old_entity_id.to_string(),
                new_id: new_entity_id.to_string(),
                title,
            }),
        }
    }

    async fn memory_summary(&self) -> Result<MemorySummary, ServiceError> {
        use arawn_memory::EntityType;

        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

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
                global_counts.push(MemoryTypeCount {
                    entity_type: et.as_str().to_string(),
                    count: g as u64,
                });
            }
            if w > 0 {
                ws_counts.push(MemoryTypeCount {
                    entity_type: et.as_str().to_string(),
                    count: w as u64,
                });
            }
        }

        Ok(MemorySummary {
            global: MemoryStoreSummary {
                total: memory.global.count_all().unwrap_or(0) as u64,
                by_type: global_counts,
            },
            workstream: MemoryStoreSummary {
                total: memory.workstream.count_all().unwrap_or(0) as u64,
                by_type: ws_counts,
            },
        })
    }

    async fn forget_entity(&self, query: &str) -> Result<ForgetResult, ServiceError> {
        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

        let mut candidates = Vec::new();
        for (store, label) in [(&memory.global, "global"), (&memory.workstream, "workstream")] {
            if let Ok(results) = store.search(query, 5) {
                for e in results {
                    candidates.push((e, label));
                }
            }
        }

        if candidates.is_empty() {
            return Err(ServiceError::NotFound(format!(
                "No entities matching '{query}' found"
            )));
        }

        if candidates.len() == 1 {
            let (entity, label) = &candidates[0];
            let store = if *label == "global" {
                &memory.global
            } else {
                &memory.workstream
            };
            match store.delete_entity(entity.id) {
                Ok(true) => Ok(ForgetResult::Deleted {
                    title: entity.title.clone(),
                    entity_type: entity.entity_type.as_str().to_string(),
                    scope: label.to_string(),
                }),
                Ok(false) => Err(ServiceError::NotFound("Entity not found".into())),
                Err(e) => Err(ServiceError::Storage(e.to_string())),
            }
        } else {
            Ok(ForgetResult::Ambiguous {
                candidates: candidates
                    .iter()
                    .map(|(e, label)| ForgetCandidate {
                        id: e.id.to_string(),
                        title: e.title.clone(),
                        entity_type: e.entity_type.as_str().to_string(),
                        scope: label.to_string(),
                    })
                    .collect(),
            })
        }
    }

    async fn get_permission_mode(&self) -> Result<PermissionModeInfo, ServiceError> {
        let mode = *self.permission_mode.read().unwrap();
        Ok(PermissionModeInfo {
            mode: serde_json::to_value(mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{mode:?}")),
        })
    }

    async fn set_permission_mode(&self, mode_str: &str) -> Result<PermissionModeInfo, ServiceError> {
        let mode: arawn_engine::permissions::PermissionMode =
            serde_json::from_value(serde_json::json!(mode_str)).map_err(|_| {
                ServiceError::InvalidOperation(format!(
                    "unknown mode '{mode_str}'. Valid: default, accept_edits, bypass, plan"
                ))
            })?;
        *self.permission_mode.write().unwrap() = mode;
        info!(mode = %mode_str, "permission mode updated");
        Ok(PermissionModeInfo {
            mode: mode_str.to_string(),
        })
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

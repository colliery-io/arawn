use crate::lock_ext::Recover;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use arawn_core::{Lens, Message};
use arawn_engine::{
    BackgroundTaskManager, Compactor, EngineToolContext, PermissionChecker, PermissionRule,
    PlanModeState, QueryEngine, QueryEngineConfig, ToolRegistry,
};
use arawn_llm::LlmClient;
use arawn_service::{
    ArawnService, CommandInfo, EngineEvent, ForgetResult, InventoryItem, LensInfo,
    MemoryStoreResult, MemorySummary, PermissionModeInfo, ServiceError, SessionDetail, SessionInfo,
    WorkflowInfo,
};
use arawn_storage::{Store, lens_dir_name};
use tracing::instrument;

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
    /// Shared memory manager — two-tier KB (global + lens).
    memory_manager: Option<Arc<arawn_memory::MemoryManager>>,
    /// Tracks sessions with active send_message calls to prevent concurrent access.
    active_sessions: Arc<Mutex<HashSet<Uuid>>>,
    /// Cancellation tokens for active engine runs, keyed by session ID.
    cancel_tokens: Arc<Mutex<HashMap<Uuid, tokio_util::sync::CancellationToken>>>,
    /// Persistent audit buffer for permission decisions. Each per-message
    /// PermissionChecker writes into this so the rolling history survives
    /// across messages and is exposed via `get_permissions_status`.
    permission_audit: arawn_engine::permissions::SharedAudit,
    /// Broadcast channel for server-wide notices (hot-reload outcomes,
    /// config changes). Each WS connection subscribes its own receiver and
    /// forwards events to the client. Capped at 64 — overflow drops oldest,
    /// which is the right behavior for "the user just wants to know
    /// something happened" notifications.
    notice_tx: tokio::sync::broadcast::Sender<arawn_service::ServerNotice>,
    /// Registered external integrations (Gmail, Calendar, Slack, ...). Keyed
    /// by stable service name. Populated at startup from the binary; tools
    /// look up their integration here at construction time.
    integration_registry:
        Arc<std::sync::RwLock<HashMap<String, Arc<dyn arawn_integrations::Integration>>>>,
    /// Live handle to the arawn-feeds runtime, set after the workflow
    /// runner comes up. `None` when feeds couldn't start (no workflow
    /// runner, no DB, etc.) — `/watch` and `/feeds` then return a
    /// clear "feeds runtime unavailable" error.
    feed_runtime: Arc<std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>>,
    /// Shared active-lens shim. Memory tools read this to route
    /// memory_store / memory_search to the right KB. Set on session
    /// resume from the persisted Session.lens_name so the
    /// active lens re-establishes without the user re-typing
    /// `/lens switch`.
    active_lens: Option<arawn_engine::SessionLens>,
    /// Shared ceremony service — wired by main.rs after the cloacina
    /// runtime is available. Set late (post-construction) via
    /// `set_ceremony_service`, mirroring `feed_runtime`'s lifecycle.
    /// `None` when the binary skipped ceremony wiring (workflow
    /// runner unavailable).
    ceremony_service: Arc<std::sync::RwLock<Option<Arc<arawn_ceremonies::CeremonyService>>>>,
    /// Broadcast sender for `TodoEvent`s. The WS layer hands a clone
    /// to every `todos.*` RPC handler that mutates state. A forwarder
    /// task in main.rs translates events onto `notice_tx` with
    /// `category="todo_event"`.
    todo_event_tx: arawn_storage::TodoEventSender,
    /// Optional hook runner — fires lifecycle events to user-configured
    /// shell commands. Wired by main.rs at startup via T-A's
    /// `startup::hooks::load_and_build_hook_runner`. `None` when no
    /// `settings.json` has been written; downstream fire sites no-op.
    hook_runner: Option<Arc<arawn_engine::hooks::HookRunner>>,
    /// One-way readiness flag (ARAWN-I-0068 P2-1). Set true by
    /// `mark_ready()` once the post-startup wiring
    /// (feeds/ceremonies/memory/storage) finishes. The `health` RPC reads
    /// it so a client knows when the server is safe to drive.
    ready: Arc<std::sync::atomic::AtomicBool>,
    /// Shared projection store, when wired — backs the embedding-backlog
    /// count in the `status` surface. `None` when projections failed to open.
    projections: Arc<std::sync::RwLock<Option<Arc<arawn_projections::ProjectionStore>>>>,
    /// True once the per-lens extractor runner is wired — backs the
    /// "extraction available" flag in `status`.
    extractor_available: Arc<std::sync::atomic::AtomicBool>,
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
            permission_mode: Arc::new(std::sync::RwLock::new(
                arawn_engine::permissions::PermissionMode::Ask,
            )),
            skill_registry: None,
            plugin_registry: None,
            pending_modals: crate::new_pending_modals(),
            plan_state: Arc::new(PlanModeState::new()),
            background_tasks: Arc::new(BackgroundTaskManager::new()),
            memory_manager: None,
            active_sessions: Arc::new(Mutex::new(HashSet::new())),
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            permission_audit: arawn_engine::permissions::new_shared_audit(),
            notice_tx: tokio::sync::broadcast::channel(64).0,
            integration_registry: Arc::new(std::sync::RwLock::new(HashMap::new())),
            feed_runtime: Arc::new(std::sync::RwLock::new(None)),
            active_lens: None,
            ceremony_service: Arc::new(std::sync::RwLock::new(None)),
            todo_event_tx: arawn_storage::todo_event_channel().0,
            hook_runner: None,
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            projections: Arc::new(std::sync::RwLock::new(None)),
            extractor_available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Flip the readiness flag to true (ARAWN-I-0068 P2-1). Called once by
    /// main.rs after all post-startup wiring completes, just before the WS
    /// server starts accepting connections.
    pub fn mark_ready(&self) {
        self.ready.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Whether the server has finished post-startup wiring. Backs the
    /// `health` RPC's readiness gate.
    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Wire the projection store so the status surface can report the
    /// embedding backlog. Called from main.rs when projections open.
    pub fn set_projections(&self, projections: Arc<arawn_projections::ProjectionStore>) {
        *self.projections.write().recover() = Some(projections);
    }

    /// Mark the per-lens extractor runner as wired — drives the
    /// "extraction available" flag in `status`.
    pub fn mark_extractor_available(&self) {
        self.extractor_available
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Attach a hook runner. Every `QueryEngine` built by this service
    /// will receive the same runner via `with_hook_runner`. Wired at
    /// startup from `~/.arawn/settings.json` merged with the project's
    /// `.arawn/settings.json` — see `startup::hooks`.
    pub fn with_hook_runner(mut self, runner: Arc<arawn_engine::hooks::HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }

    /// Get a clone of the optional hook runner. Main uses this to spawn
    /// the Notification-forwarder task (T-C).
    pub fn hook_runner_clone(&self) -> Option<Arc<arawn_engine::hooks::HookRunner>> {
        self.hook_runner.clone()
    }

    /// Sender for todo events — RPC handlers clone this when
    /// constructing a `TodoService` so mutations propagate.
    pub fn todo_event_sender(&self) -> arawn_storage::TodoEventSender {
        self.todo_event_tx.clone()
    }

    /// Subscribe to the todo event channel — main.rs spawns a
    /// forwarder task that wraps events into `ServerNotice`s.
    pub fn subscribe_todo_events(&self) -> arawn_storage::TodoEventReceiver {
        self.todo_event_tx.subscribe()
    }

    /// Wire the ceremony service. Called from main.rs after the
    /// cloacina runtime starts.
    pub fn set_ceremony_service(&self, svc: Arc<arawn_ceremonies::CeremonyService>) {
        *self.ceremony_service.write().recover() = Some(svc);
    }

    /// Shared reference to the ceremony service, if wired.
    pub fn ceremony_service(&self) -> Option<Arc<arawn_ceremonies::CeremonyService>> {
        self.ceremony_service.read().recover().clone()
    }

    /// Wire the shared `SessionLens` shim. Memory tools read
    /// this for routing; load_session_state restores it from the
    /// persisted name on resume.
    pub fn with_active_lens(mut self, ws: arawn_engine::SessionLens) -> Self {
        self.active_lens = Some(ws);
        self
    }

    /// Hand the live feed runtime to the service so `/watch` and
    /// `/feeds` can dispatch to it. Called from main.rs after
    /// `arawn_feeds::start` returns.
    pub fn set_feed_runtime(&self, runtime: Arc<arawn_feeds::FeedRuntime>) {
        *self.feed_runtime.write().recover() = Some(runtime);
    }

    fn feed_runtime_or_err(&self) -> Result<Arc<arawn_feeds::FeedRuntime>, ServiceError> {
        self.feed_runtime.read().recover().clone().ok_or_else(|| {
            ServiceError::Internal("feeds runtime unavailable — workflow runner not running".into())
        })
    }

    /// Register an external integration. Called from main.rs at startup
    /// for each integration the binary wants to expose.
    pub fn register_integration(&self, integration: Arc<dyn arawn_integrations::Integration>) {
        let name = integration.name().to_string();
        info!(name = %name, "registering integration");
        self.integration_registry
            .write()
            .recover()
            .insert(name, integration);
    }

    /// Shared reference to the integration registry — for tools that want
    /// to look up an integration at construction time.
    pub fn shared_integrations(
        &self,
    ) -> Arc<std::sync::RwLock<HashMap<String, Arc<dyn arawn_integrations::Integration>>>> {
        Arc::clone(&self.integration_registry)
    }

    /// Subscribe to server-wide notices (plugin/config hot-reload, etc.).
    /// Each call returns a fresh receiver — every subscriber gets every
    /// notice. Receivers that fall behind by more than 64 messages drop
    /// oldest first.
    pub fn subscribe_notices(
        &self,
    ) -> tokio::sync::broadcast::Receiver<arawn_service::ServerNotice> {
        self.notice_tx.subscribe()
    }

    /// Get a sender clone — used to wire watchers (plugin runtime, config
    /// watcher) into the broadcast at startup.
    pub fn notice_sender(&self) -> tokio::sync::broadcast::Sender<arawn_service::ServerNotice> {
        self.notice_tx.clone()
    }

    pub fn with_permission_rules(self, rules: Vec<PermissionRule>) -> Self {
        *self.permission_rules.write().recover() = rules;
        self
    }

    /// T-0347: override the starting permission mode (declared in
    /// `[permissions] autonomy` in arawn.toml). Defaults to
    /// `PermissionMode::Ask`.
    pub fn with_permission_mode(self, mode: arawn_engine::permissions::PermissionMode) -> Self {
        *self.permission_mode.write().recover() = mode;
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

    pub fn shared_permission_mode(
        &self,
    ) -> Arc<std::sync::RwLock<arawn_engine::permissions::PermissionMode>> {
        Arc::clone(&self.permission_mode)
    }

    pub fn with_skill_registry(
        mut self,
        registry: Arc<arawn_engine::skills::SkillRegistry>,
    ) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    pub fn with_plugin_registry(
        mut self,
        registry: Arc<arawn_engine::plugins::PluginRegistry>,
    ) -> Self {
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

    /// Load session metadata, resolve lens, and load message history.
    #[instrument(skip_all, fields(%session_id))]
    fn load_session_state(
        &self,
        session_id: Uuid,
    ) -> Result<(arawn_storage::SessionMeta, Lens, String, Vec<Message>), ServiceError> {
        let (meta, lens, ws_dir) = {
            let store = self.store.lock().recover();
            let meta = store
                .get_session_meta(session_id)?
                .ok_or_else(|| ServiceError::NotFound(format!("session {session_id}")))?;

            let ws_dir = resolve_ws_dir_from_store(&store, meta.lens_id)?;

            let lens = if let Some(ws_id) = meta.lens_id {
                store
                    .get_lens(ws_id)?
                    .ok_or_else(|| ServiceError::NotFound(format!("lens {ws_id}")))?
            } else {
                store
                    .find_lens_by_name("scratch")?
                    .ok_or_else(|| ServiceError::NotFound("scratch lens".into()))?
            };

            (meta, lens, ws_dir)
        };

        // Re-establish the active lens from the persisted
        // session record. Falls back gracefully when the shim isn't
        // wired (e.g. tests that bypass `with_active_lens`).
        if let Some(active) = self.active_lens.as_ref() {
            let name = if !meta.lens_name.is_empty() {
                meta.lens_name.clone()
            } else {
                lens.name.clone()
            };
            active.set(name);
        }

        Ok((meta, lens, ws_dir, Vec::new()))
    }

    /// Build a EngineToolContext and per-session PromptContext for the engine.
    #[instrument(skip_all, fields(%session_id))]
    fn build_session_context(
        &self,
        session_id: Uuid,
        lens: &Lens,
        ws_dir: &str,
        workspace_dir: &std::path::Path,
        content: &str,
    ) -> (EngineToolContext, Option<arawn_engine::PromptContext>) {
        let mut ws_for_ctx = lens.clone();
        ws_for_ctx.root_dir = workspace_dir.to_path_buf();

        let global_arawn_md = self.data_dir.join("arawn.md");
        let lens_arawn_md = self.data_dir.join("lenses").join(ws_dir).join("arawn.md");
        let pool = Arc::clone(&self.llm_pool);
        let resolver: Arc<arawn_tool::LlmResolverFn> = Arc::new(move |pref| pool.resolve(pref));
        let ctx = EngineToolContext::new(&ws_for_ctx, session_id)
            .with_allowed_paths(vec![global_arawn_md, lens_arawn_md])
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
                lens_name: lens.name.clone(),
                lens_root: workspace_dir.to_path_buf(),
                context_files: arawn_engine::find_context_files(workspace_dir, &self.data_dir),
                memories: self
                    .memory_manager
                    .as_ref()
                    .map(|mgr| {
                        let stack = arawn_memory::MemoryStack::new(mgr, &lens.name);
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
                // ARAWN-I-0060: lens-agnostic chat → always the default
                // `assistant` persona, never flipped by the active lens.
                identity_profile: arawn_core::IdentityProfile::Assistant,
                // Closure captures the registry Arc; queries it fresh each
                // turn so /connect and /disconnect reflect immediately.
                integration_capabilities: Some({
                    let registry = Arc::clone(&self.integration_registry);
                    Arc::new(move || -> Vec<String> {
                        let map = match registry.read() {
                            Ok(g) => g,
                            Err(_) => return Vec::new(),
                        };
                        let integrations: Vec<_> = map.values().map(Arc::clone).collect();
                        drop(map);
                        // capabilities_summary is async — but we promised
                        // the closure stays cheap & sync. Run on a fresh
                        // tokio runtime handle if available, else block.
                        // Practically: every impl is sync-disk-only (no
                        // network), so block_in_place + Handle::block_on
                        // is fine.
                        let mut out = Vec::new();
                        let handle = tokio::runtime::Handle::try_current().ok();
                        for integ in integrations {
                            let summary = match &handle {
                                Some(h) => tokio::task::block_in_place(|| {
                                    h.block_on(integ.capabilities_summary())
                                }),
                                None => {
                                    // No tokio context — should not happen in
                                    // the engine path, but bail safely.
                                    None
                                }
                            };
                            if let Some(s) = summary {
                                out.push(s);
                            }
                        }
                        out.sort();
                        out
                    }) as arawn_engine::IntegrationCapabilitiesFn
                }),
                connected_services: Some({
                    let registry = Arc::clone(&self.integration_registry);
                    Arc::new(move || -> Vec<String> {
                        // Returns canonical Integration::name() values for
                        // every integration that reports `is_connected() ==
                        // true`. Feeds the I-0055 capability-driven tool
                        // filter. Sync-disk-only impls — block_in_place is
                        // the same pattern integration_capabilities uses.
                        let map = match registry.read() {
                            Ok(g) => g,
                            Err(_) => return Vec::new(),
                        };
                        let integrations: Vec<_> = map.values().map(Arc::clone).collect();
                        drop(map);
                        let mut out = Vec::new();
                        let handle = tokio::runtime::Handle::try_current().ok();
                        for integ in integrations {
                            let connected = match &handle {
                                Some(h) => {
                                    tokio::task::block_in_place(|| h.block_on(integ.is_connected()))
                                }
                                None => false,
                            };
                            if connected {
                                out.push(integ.name().to_string());
                            }
                        }
                        out.sort();
                        out
                    }) as arawn_engine::ConnectedServicesFn
                }),
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
        // Resolve `hint:*` at the engine/compactor boundary. The pool
        // maps each hint to a concrete model via `[routing.hints]`.
        // Pure config-driven — capabilities are assigned to specific
        // models via configuration, not runtime dispatch policy.
        let (compactor_client, compactor_model) = self
            .llm_pool
            .resolve_hint(&arawn_llm::ModelHint::Medium.as_hint());
        let compactor = Compactor::new(compactor_client, compactor_model);
        let (engine_client, engine_model) = self.llm_pool.resolve_hint(&self.config.model);
        let mut engine = QueryEngine::with_config(
            engine_client,
            self.registry.clone(),
            QueryEngineConfig {
                model: engine_model,
                max_iterations: self.config.max_iterations,
                system_prompt: self.config.system_prompt.clone(),
                max_tokens: self.config.max_tokens,
                model_limits: self.config.model_limits.clone(),
                data_dir: Some(self.data_dir.clone()),
                prompt_context,
                tool_timeout_secs: self.config.tool_timeout_secs,
            },
        )
        .with_compactor(compactor);

        if let Some(ref skill_reg) = self.skill_registry {
            engine = engine.with_skill_registry(Arc::clone(skill_reg));
        }
        if let Some(ref plugin_reg) = self.plugin_registry {
            engine = engine.with_plugin_registry(Arc::clone(plugin_reg));
        }
        if let Some(ref hook_runner) = self.hook_runner {
            engine = engine.with_hook_runner(Arc::clone(hook_runner));
        }

        // Attach permission checker
        {
            let rules = self.permission_rules.read().recover().clone();
            if !rules.is_empty() {
                let prompt = ChannelModalPrompt::new(event_tx.clone(), self.pending_modals.clone());
                let mode = *self.permission_mode.read().recover();
                let mut checker = PermissionChecker::new(rules)
                    .with_mode(mode)
                    .with_prompter(Box::new(prompt))
                    .with_audit(Arc::clone(&self.permission_audit));
                if let Some(ref hook_runner) = self.hook_runner {
                    checker = checker.with_hook_runner(Arc::clone(hook_runner));
                }
                engine = engine.with_permission_checker(Arc::new(checker));
            }
        }

        engine
            .with_plan_state(Arc::clone(&self.plan_state))
            .with_background_tasks(Arc::clone(&self.background_tasks))
    }
}

/// Infer entity type from text patterns.
pub(super) fn infer_entity_type(text: &str) -> (arawn_memory::EntityType, String) {
    use arawn_memory::EntityType;
    let lower = text.to_lowercase();

    if lower.starts_with("i prefer") || lower.starts_with("prefer ") || lower.contains("preference")
    {
        (EntityType::Preference, text.to_string())
    } else if lower.starts_with("we decided")
        || lower.starts_with("decision:")
        || lower.contains("decided to")
    {
        (EntityType::Decision, text.to_string())
    } else if lower.starts_with("convention:")
        || lower.starts_with("the convention is")
        || lower.starts_with("always ")
        || lower.starts_with("never ")
    {
        (EntityType::Convention, text.to_string())
    } else {
        (EntityType::Fact, text.to_string())
    }
}

use async_trait::async_trait;

mod commands;
mod feeds;
mod integrations;
mod lenses;
mod memory;
mod permissions;
mod sessions;
mod status;

#[async_trait]
impl ArawnService for LocalService {
    async fn list_lenses(&self) -> Result<Vec<LensInfo>, ServiceError> {
        self.list_lenses_inner().await
    }
    async fn create_lens(&self, name: String, root_dir: PathBuf) -> Result<LensInfo, ServiceError> {
        self.create_lens_inner(name, root_dir).await
    }
    async fn list_sessions(&self, lens_id: Option<Uuid>) -> Result<Vec<SessionInfo>, ServiceError> {
        self.list_sessions_inner(lens_id).await
    }
    async fn create_session(&self, lens_id: Option<Uuid>) -> Result<SessionInfo, ServiceError> {
        self.create_session_inner(lens_id).await
    }
    async fn load_session(&self, id: Uuid) -> Result<SessionDetail, ServiceError> {
        self.load_session_inner(id).await
    }
    async fn truncate_session_at_user_message(
        &self,
        id: Uuid,
        user_message_index: usize,
    ) -> Result<SessionDetail, ServiceError> {
        self.truncate_session_at_user_message_inner(id, user_message_index)
            .await
    }
    async fn send_message(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = EngineEvent> + Send>>, ServiceError> {
        self.send_message_inner(session_id, content).await
    }
    async fn cancel(&self, session_id: Uuid) -> Result<(), ServiceError> {
        self.cancel_inner(session_id).await
    }
    async fn resolve_user_input(
        &self,
        request_id: &str,
        selected_index: Option<usize>,
    ) -> Result<(), ServiceError> {
        self.resolve_user_input_inner(request_id, selected_index)
            .await
    }
    async fn query_inventory(&self, kind: &str) -> Result<Vec<InventoryItem>, ServiceError> {
        self.query_inventory_inner(kind).await
    }
    async fn list_available_commands(&self) -> Result<Vec<CommandInfo>, ServiceError> {
        self.list_available_commands_inner().await
    }
    async fn list_workflows(&self) -> Result<Vec<WorkflowInfo>, ServiceError> {
        self.list_workflows_inner().await
    }
    async fn remember_fact(&self, text: &str) -> Result<MemoryStoreResult, ServiceError> {
        self.remember_fact_inner(text).await
    }
    async fn memory_summary(&self) -> Result<MemorySummary, ServiceError> {
        self.memory_summary_inner().await
    }
    async fn forget_entity(&self, query: &str) -> Result<ForgetResult, ServiceError> {
        self.forget_entity_inner(query).await
    }
    async fn get_permission_mode(&self) -> Result<PermissionModeInfo, ServiceError> {
        self.get_permission_mode_inner().await
    }
    async fn set_permission_mode(
        &self,
        mode_str: &str,
    ) -> Result<PermissionModeInfo, ServiceError> {
        self.set_permission_mode_inner(mode_str).await
    }
    async fn get_capabilities(&self) -> Result<arawn_service::ServerCapabilities, ServiceError> {
        self.get_capabilities_inner().await
    }
    async fn get_permissions_status(
        &self,
    ) -> Result<arawn_service::PermissionsStatus, ServiceError> {
        self.get_permissions_status_inner().await
    }
    async fn health(&self) -> Result<arawn_service::HealthStatus, ServiceError> {
        self.health_inner().await
    }
    async fn status(&self) -> Result<arawn_service::SystemStatus, ServiceError> {
        self.status_inner().await
    }
    async fn list_integrations(
        &self,
    ) -> Result<Vec<arawn_service::IntegrationStatus>, ServiceError> {
        self.list_integrations_inner().await
    }
    async fn start_oauth_flow(
        &self,
        service: &str,
    ) -> Result<arawn_service::OAuthFlowStarted, ServiceError> {
        self.start_oauth_flow_inner(service).await
    }
    async fn disconnect_integration(&self, service: &str) -> Result<(), ServiceError> {
        self.disconnect_integration_inner(service).await
    }
    async fn feed_register(
        &self,
        spec: arawn_service::FeedRegisterSpec,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        self.feed_register_inner(spec).await
    }
    async fn feed_list(&self) -> Result<Vec<arawn_service::FeedSummaryDto>, ServiceError> {
        self.feed_list_inner().await
    }
    async fn feed_pause(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        self.feed_pause_inner(feed_id).await
    }
    async fn feed_resume(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        self.feed_resume_inner(feed_id).await
    }
    async fn feed_run(&self, feed_id: &str) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        self.feed_run_inner(feed_id).await
    }
    async fn feed_discover(
        &self,
        template: &str,
    ) -> Result<arawn_service::FeedDiscoverDto, ServiceError> {
        self.feed_discover_inner(template).await
    }
    async fn feed_schema(
        &self,
        template: &str,
    ) -> Result<arawn_service::FeedSchemaDto, ServiceError> {
        self.feed_schema_inner(template).await
    }
    async fn feed_templates(&self) -> Result<Vec<arawn_service::FeedTemplateInfo>, ServiceError> {
        Ok(arawn_feeds::template_catalog()
            .into_iter()
            .map(|(name, description)| arawn_service::FeedTemplateInfo {
                name: name.to_string(),
                description: description.to_string(),
            })
            .collect())
    }
    async fn feed_remove(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedRemoveDto, ServiceError> {
        self.feed_remove_inner(feed_id).await
    }
}

/// Personal default feed registered automatically the first time
/// the user runs `/connect <service>`. None means "this integration
/// has no auto-feed; users must `/watch` explicitly."
///
/// Returns `(template_name, feed_id)`.
pub(super) fn default_feed_for_service(service: &str) -> Option<(&'static str, &'static str)> {
    match service {
        "slack" => Some(("slack/my-mentions", "me")),
        "gmail" => Some(("gmail/inbox-archive", "me")),
        "google_calendar" => Some(("calendar/upcoming-archive", "primary")),
        "google_drive" => Some(("drive/recent", "me")),
        "atlassian" => Some(("jira/assignee-tracker", "me")),
        _ => None,
    }
}

async fn current_summary(
    runtime: &arawn_feeds::FeedRuntime,
    feed_id: &str,
) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
    let summaries = runtime.list_summaries().await.map_err(feed_err)?;
    summaries
        .into_iter()
        .find(|s| s.id == feed_id)
        .map(feed_summary_to_dto)
        .ok_or_else(|| ServiceError::NotFound(format!("feed {feed_id}")))
}

pub(super) fn feed_err(e: arawn_feeds::FeedError) -> ServiceError {
    use arawn_feeds::FeedError;
    match e {
        FeedError::InvalidParams(msg) => ServiceError::InvalidOperation(msg),
        FeedError::Auth(msg) => ServiceError::InvalidOperation(format!("auth: {msg}")),
        other => ServiceError::Internal(other.to_string()),
    }
}

pub(super) fn feed_summary_to_dto(s: arawn_feeds::FeedSummary) -> arawn_service::FeedSummaryDto {
    arawn_service::FeedSummaryDto {
        id: s.id,
        template: s.template,
        cadence: s.cadence,
        enabled: s.enabled,
        created_at: s.created_at,
        updated_at: s.updated_at,
        last_run_at: s.last_run_at,
        last_status: s.last_status,
        run_count: s.run_count,
        data_size_bytes: s.data_size_bytes,
        data_dir: s.data_dir,
        last_run_items: None,
    }
}

/// Glue that lets `LocalService::start_oauth_flow` bridge the integration's
/// `connect()` callback to (a) the RPC's oneshot reply for the auth URL and
/// (b) the server-wide notice broadcast for progress messages.
pub(super) struct OAuthFlowCtx {
    service: String,
    url_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<url::Url>>>,
    notice_tx: tokio::sync::broadcast::Sender<arawn_service::ServerNotice>,
}

#[async_trait::async_trait]
impl arawn_integrations::ConnectContext for OAuthFlowCtx {
    fn service(&self) -> &str {
        &self.service
    }

    async fn publish_auth_url(&self, url: &url::Url) {
        // Take the sender so re-publishing is a no-op (integrations should
        // only publish once per flow).
        let mut guard = self.url_tx.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(url.clone());
        }
    }

    async fn publish_progress(&self, message: &str) {
        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "integration".into(),
            message: format!("{}: {message}", self.service),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}

/// Resolve lens directory name from store. Returns "scratch" for None.
pub(super) fn resolve_ws_dir_from_store(
    store: &Store,
    ws_id: Option<Uuid>,
) -> Result<String, ServiceError> {
    match ws_id {
        Some(id) => {
            let ws = store
                .get_lens(id)?
                .ok_or_else(|| ServiceError::NotFound(format!("lens {id}")))?;
            Ok(lens_dir_name(&ws.name, ws.id))
        }
        None => Ok("scratch".to_string()),
    }
}

/// Extract the first sentence and sanitize for use in a markdown table cell.
/// Collapses whitespace, strips pipe chars, and cuts at the first sentence boundary.
pub(super) fn first_sentence(s: &str) -> String {
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

#[cfg(test)]
mod feed_default_tests {
    use super::default_feed_for_service;

    #[test]
    fn known_services_each_have_a_default_feed() {
        // Sanity: every integration we ship a personal feed for is
        // reachable from the auto-create map. If we add a sixth
        // integration this test won't fail — it's a snapshot, not an
        // exhaustiveness check — but it fences the existing five
        // against typos.
        assert_eq!(
            default_feed_for_service("slack"),
            Some(("slack/my-mentions", "me"))
        );
        assert_eq!(
            default_feed_for_service("gmail"),
            Some(("gmail/inbox-archive", "me"))
        );
        assert_eq!(
            default_feed_for_service("google_calendar"),
            Some(("calendar/upcoming-archive", "primary"))
        );
        assert_eq!(
            default_feed_for_service("google_drive"),
            Some(("drive/recent", "me"))
        );
        assert_eq!(
            default_feed_for_service("atlassian"),
            Some(("jira/assignee-tracker", "me"))
        );
    }

    #[test]
    fn unknown_service_has_no_default_feed() {
        assert!(default_feed_for_service("zoom").is_none());
        assert!(default_feed_for_service("").is_none());
    }
}

/// P1-2 (ARAWN-T-0469): the service must survive a panic in a background
/// writer. All lock acquisitions in this crate recover from poisoning via the
/// `Recover` trait (`.recover()`) rather than `.unwrap()`, so a panic while one
/// task holds a write guard does NOT brick every later RPC. These tests
/// exercise the trait directly on the `feed_runtime: RwLock<Option<_>>` access
/// shape and fence against anyone reintroducing a poison-panicking `.unwrap()`.
#[cfg(test)]
mod poison_recovery_tests {
    use std::sync::{Arc, Mutex, PoisonError, RwLock};

    use crate::lock_ext::Recover;

    #[test]
    fn rwlock_read_recovers_after_writer_panic() {
        let lock: Arc<RwLock<Option<u32>>> = Arc::new(RwLock::new(Some(1)));

        // A background "writer" panics while holding the write guard,
        // poisoning the lock — exactly the scenario where a panicking
        // ceremony/feed task could otherwise brick the server.
        let l = Arc::clone(&lock);
        let handle = std::thread::spawn(move || {
            // Acquire via the std API here so the panic happens regardless of
            // our trait; recovery on the *read* side is what we assert below.
            let mut g = l.write().unwrap_or_else(PoisonError::into_inner);
            *g = Some(2);
            panic!("simulated background writer panic");
        });
        assert!(
            handle.join().is_err(),
            "the writer thread should have panicked"
        );
        assert!(lock.is_poisoned(), "the lock should now be poisoned");

        // The production access pattern (`.recover()`) must still work — no
        // panic, and the partially-applied write is recovered rather than lost.
        let read = lock.read().recover().clone();
        assert_eq!(read, Some(2));

        // And subsequent writes still succeed through the recovered guard.
        *lock.write().recover() = Some(3);
        assert_eq!(*lock.read().recover(), Some(3));
    }

    #[test]
    fn mutex_recovers_after_holder_panic() {
        let lock: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let l = Arc::clone(&lock);
        let handle = std::thread::spawn(move || {
            let mut g = l.lock().unwrap_or_else(PoisonError::into_inner);
            *g = 42;
            panic!("poison the mutex");
        });
        assert!(handle.join().is_err());
        assert!(lock.is_poisoned());
        // Recovered access via `.recover()` does not panic and sees the value.
        assert_eq!(*lock.lock().recover(), 42);
    }
}

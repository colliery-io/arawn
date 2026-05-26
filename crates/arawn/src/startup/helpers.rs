//! Small standalone helpers used by `main()` during startup.
//! Moved out of `main.rs` as part of I-0054 T-E to reduce the
//! main hotspot file.

use std::sync::Arc;

use anyhow::Result;
use tracing::info;

/// Build the appropriate LLM client based on provider config.
pub fn build_llm_client(config: &crate::LlmConfig) -> Result<Arc<dyn arawn_llm::LlmClient>> {
    let resolved_key = crate::ArawnConfig::resolve_api_key(config);
    // Log the resolved provider/endpoint so "why is it hitting <provider>?"
    // is answerable from the startup log. The endpoint is either the explicit
    // base_url override or the provider's compiled-in default.
    info!(
        provider = %config.provider,
        model = %config.model,
        endpoint = %config
            .base_url
            .clone()
            .unwrap_or_else(|| format!("<{} default>", config.provider)),
        api_key = if resolved_key.is_some() { "present" } else { "absent" },
        "building LLM client",
    );
    match config.provider.as_str() {
        "anthropic" => {
            let api_key = resolved_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "Anthropic provider requires an API key — set `api_key` in [llm.<name>] or export {}",
                    config.api_key_env
                )
            })?;
            Ok(Arc::new(arawn_llm::AnthropicClient::new(api_key)))
        }
        _ => {
            // All other providers use OpenAI-compatible client
            Ok(Arc::new(arawn_llm::OpenAICompatibleClient::from_config(
                &config.provider,
                config.base_url.as_deref(),
                resolved_key,
            )?))
        }
    }
}

/// Register all default tools into the registry.
pub fn register_default_tools(
    registry: &Arc<arawn_engine::ToolRegistry>,
    config: &crate::ArawnConfig,
    data_dir: &str,
    bg_manager: Arc<arawn_engine::BackgroundTaskManager>,
    plan_state: Arc<arawn_engine::PlanModeState>,
    hook_runner: Option<Arc<arawn_engine::hooks::HookRunner>>,
) {
    use arawn_engine::{
        AgentTool, AskUserTool, EnterPlanModeTool, ExitPlanModeTool, FileEditTool, FileReadTool,
        FileWriteTool, GlobTool, GrepTool, ShellTool, SleepTool, TaskGetTool, TaskListTool,
        TaskOutputTool, TaskStopTool, ThinkTool, WebFetchTool, WebSearchTool,
    };

    // I-0056 T-D: attach hook runner to subsystems that fire lifecycle
    // events via internal sites (BackgroundTaskManager for TaskCreated/
    // Completed; AgentTool for SubagentStart/Stop).
    if let Some(ref runner) = hook_runner {
        bg_manager.set_hook_runner(Arc::clone(runner));
    }

    registry.register(Box::new(ThinkTool));
    registry.register(Box::new(
        ShellTool::with_network_tools(config.sandbox.network_tools.clone())
            .with_background_manager(Arc::clone(&bg_manager)),
    ));
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(FileEditTool));
    registry.register(Box::new(GlobTool));
    registry.register(Box::new(GrepTool));
    registry.register(Box::new(WebFetchTool::new()));
    registry.register(Box::new(WebSearchTool));
    registry.register(Box::new(AskUserTool));

    let agents_dir = std::path::PathBuf::from(data_dir).join("agents");
    let agent_defs = arawn_engine::agent_defs::get_all_agents(Some(&agents_dir));
    let agent_tool =
        AgentTool::new(Arc::clone(registry), agent_defs).with_background_manager(Arc::clone(&bg_manager));
    if let Some(ref runner) = hook_runner {
        agent_tool.set_hook_runner(Arc::clone(runner));
    }
    registry.register(Box::new(agent_tool));

    registry.register(Box::new(SleepTool));
    registry.register(Box::new(TaskListTool::new(Arc::clone(&bg_manager))));
    registry.register(Box::new(TaskGetTool::new(Arc::clone(&bg_manager))));
    registry.register(Box::new(TaskOutputTool::new(Arc::clone(&bg_manager))));
    registry.register(Box::new(TaskStopTool::new(Arc::clone(&bg_manager))));

    registry.register(Box::new(EnterPlanModeTool::new(Arc::clone(&plan_state))));
    registry.register(Box::new(ExitPlanModeTool::new(Arc::clone(&plan_state))));
}

/// Connect to MCP servers from config and plugins.
pub async fn connect_mcp_servers(
    data_dir: &str,
    plugin_result: &arawn_engine::plugins::PluginLoadResult,
    registry: &Arc<arawn_engine::ToolRegistry>,
) -> arawn_mcp::McpManager {
    let mcp_config =
        arawn_mcp::load_mcp_config(&std::path::PathBuf::from(data_dir).join("arawn.toml"));
    let mut mcp_manager = arawn_mcp::McpManager::new();
    if !mcp_config.servers.is_empty() {
        info!(
            servers = mcp_config.servers.len(),
            "connecting to config MCP servers"
        );
        mcp_manager.connect_all(&mcp_config.servers, registry).await;
    }

    if !plugin_result.mcp_servers.is_empty() {
        let plugin_mcp_configs: Vec<arawn_mcp::McpServerConfig> = plugin_result
            .mcp_servers
            .iter()
            .map(|s| arawn_mcp::McpServerConfig {
                name: s.name.clone(),
                command: s.command.clone(),
                args: s.args.clone(),
                env: s.env.clone(),
                enabled: true,
            })
            .collect();
        info!(
            servers = plugin_mcp_configs.len(),
            "connecting to plugin MCP servers"
        );
        mcp_manager.connect_all(&plugin_mcp_configs, registry).await;
    }

    if mcp_manager.tool_count() > 0 {
        info!(
            tools = mcp_manager.tool_count(),
            servers = mcp_manager.connected_servers().len(),
            "MCP servers connected"
        );
    }

    mcp_manager
}

/// Register workflow management tools.
pub fn register_workflow_tools(
    registry: &Arc<arawn_engine::ToolRegistry>,
    workflows_dir: std::path::PathBuf,
    shared_runner: arawn_workflow::SharedWorkflowRunner,
) {
    registry.register(Box::new(arawn_workflow::WorkflowCreateTool::new(
        workflows_dir.clone(),
    )));
    registry.register(Box::new(arawn_workflow::WorkflowListTool::new(
        workflows_dir.clone(),
    )));
    registry.register(Box::new(arawn_workflow::WorkflowDeleteTool::new(
        workflows_dir,
    )));
    registry.register(Box::new(arawn_workflow::WorkflowStatusTool::new(
        shared_runner,
    )));
}

pub fn dirs_path() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| format!("{h}/.arawn"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("HOME").ok().map(|h| format!("{h}/.arawn"))
    }
}

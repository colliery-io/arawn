pub mod agent_defs;
pub mod background;
pub mod compact_prompt;
pub mod diff;
pub mod compactor;
pub mod context;
pub mod error;
pub mod hooks;
pub mod permissions;
pub mod plan;
pub mod plugin_adapter;
pub mod plugins;
pub mod plugin_loader;
pub mod plugin_watcher;
pub mod query_engine;
pub mod skills;
pub mod system_prompt;
pub mod testing;
pub mod token_estimator;
pub mod tool;
pub mod tool_result_limiter;
pub mod tools;

pub use background::{
    BackgroundTaskManager, BackgroundTaskKind, BackgroundTaskStatus, TaskNotification, TaskSummary,
    append_output,
};
pub use compactor::Compactor;
pub use context::ToolContext;
pub use error::EngineError;
pub use hooks::{
    HookConfig, HookEvent, HookFileWatcher, HookInput, HookRunner, load_hooks_from_file,
    load_merged_hooks,
};
pub use permissions::{
    CliModalPrompt, MockModalPrompt, ModalOption, ModalPrompt, ModalRequest,
    PermissionChecker, PermissionConfig, PermissionDecision, PermissionMode,
    PermissionResponse, PermissionRule, RuleKind, SessionGrants, ToolCategory, tool_category,
};
pub use plan::{PlanModeState, PlanModeSnapshot, generate_slug};
pub use plugin_adapter::PluginToolAdapter;
pub use plugin_loader::PluginLoader;
pub use plugin_watcher::PluginWatcher;
pub use query_engine::{ProgressEvent, PromptContext, QueryEngine, QueryEngineConfig};
pub use system_prompt::{ContextFile, SystemPromptBuilder, find_context_files};
pub use token_estimator::{ModelLimits, TokenEstimator};
pub use tool::{Tool, ToolOutput, ToolRegistry};
pub use skills::{SkillDefinition, SkillRegistry, format_skill_listing, load_merged_skills};
pub use tools::{
    AgentTool, AskUserTool, EnterPlanModeTool, ExitPlanModeTool, FileEditTool, FileReadTool,
    FileWriteTool, GlobTool, GrepTool, SessionTaskStore, ShellTool, SkillTool, SleepTool,
    MemorySearchTool, MemoryStoreTool, TaskCreateTool, TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool,
    TaskUpdateTool, ThinkTool, WebFetchTool, WebSearchTool,
};

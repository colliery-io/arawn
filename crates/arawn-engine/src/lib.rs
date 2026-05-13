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
pub mod plugins;
pub mod query_engine;
pub mod skills;
pub mod system_prompt;
pub mod testing;
pub mod token_estimator;
pub mod tool;
pub mod tool_result_limiter;
pub mod tools;
pub mod workstream_router;

pub use background::{
    BackgroundTaskManager, BackgroundTaskKind, BackgroundTaskStatus, TaskNotification, TaskSummary,
    append_output,
};
pub use compactor::Compactor;
pub use context::EngineToolContext;
/// Backward-compatible alias: downstream code that used `arawn_engine::ToolContext`
/// continues to work via this re-export.
pub use context::EngineToolContext as ToolContext;
pub use error::EngineError;
pub use hooks::{
    HookConfig, HookEvent, HookFileWatcher, HookInput, HookRunner, load_hooks_from_file,
    load_merged_hooks,
};
pub use permissions::{
    CliModalPrompt, MockModalPrompt, ModalOption, ModalPrompt, ModalRequest,
    PermissionChecker, PermissionConfig, PermissionDecision, PermissionMode,
    PermissionResponse, PermissionRule, RuleKind, SessionGrants,
};
// The top-level ToolCategory re-export below is tool::ToolCategory
// (Core/Task/Agent/Web/etc.) for context filtering. Permission-risk classes
// now live on the Tool trait itself as arawn_tool::PermissionCategory.
pub use plan::{PlanModeState, PlanModeSnapshot, generate_slug};
pub use query_engine::{
    IntegrationCapabilitiesFn, ProgressEvent, PromptContext, QueryEngine, QueryEngineConfig,
};
pub use system_prompt::{ContextFile, SystemPromptBuilder, find_context_files};
pub use token_estimator::{ModelLimits, TokenEstimator};
pub use workstream_router::{MemoryHandle, WorkstreamMemoryRouter};
pub use tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};
pub use skills::{SkillDefinition, SkillRegistry, format_skill_listing, load_merged_skills};
pub use tools::{
    AgentTool, AskUserTool, EnterPlanModeTool, ExitPlanModeTool, FileEditTool, FileReadTool,
    FileWriteTool, GlobTool, GrepTool, SessionTaskStore, ShellTool, SkillTool, SleepTool,
    FeedSearchTool, MemorySearchTool, MemoryStoreTool, TaskCreateTool, TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool,
    TaskUpdateTool, ThinkTool, WebFetchTool, WebSearchTool,
    BindBackfillHook, SessionWorkstream, WorkstreamBindTool, WorkstreamCreateTool,
    WorkstreamDeleteTool, WorkstreamDescribeTool, WorkstreamListTool, WorkstreamPromoteTool,
    WorkstreamShowTool, WorkstreamSwitchTool, WorkstreamUnbindTool,
};

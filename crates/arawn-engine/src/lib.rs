pub mod agent_defs;
pub mod approval;
pub mod background;
pub mod ceremony_sources;
pub mod compact_prompt;
pub mod compactor;
pub mod context;
pub mod diff;
pub mod error;
pub mod hooks;
pub mod permissions;
pub mod plan;
pub mod plugins;
pub mod prompt_injection;
pub mod query_engine;
pub mod skills;
pub mod system_prompt;
pub mod testing;
pub mod token_estimator;
pub mod tool_result_limiter;
pub mod tool_timeout;
pub mod tools;
pub mod workstream_router;

pub use background::{
    BackgroundTaskKind, BackgroundTaskManager, BackgroundTaskStatus, TaskNotification, TaskSummary,
    append_output,
};
pub use compactor::Compactor;
pub use context::EngineToolContext;
pub use error::EngineError;
pub use hooks::{
    HookConfig, HookEvent, HookFileWatcher, HookInput, HookRunner, load_hooks_from_file,
    load_merged_hooks,
};
pub use permissions::{
    CliModalPrompt, MockModalPrompt, ModalOption, ModalPrompt, ModalRequest, PermissionChecker,
    PermissionConfig, PermissionDecision, PermissionMode, PermissionResponse, PermissionRule,
    RuleKind, SessionGrants,
};
// The top-level `ToolCategory` re-export below is `arawn_tool::ToolCategory`
// (Core/Task/Agent/Web/etc.) for context filtering. Permission-risk classes
// live on the Tool trait itself as `arawn_tool::PermissionCategory`.
pub use ceremony_sources::{ProjectionsAttentionSource, ProjectionsCalendarSource};
pub use plan::{PlanModeSnapshot, PlanModeState, generate_slug};
pub use query_engine::{
    IntegrationCapabilitiesFn, ProgressEvent, PromptContext, QueryEngine, QueryEngineConfig,
};
pub use skills::{SkillDefinition, SkillRegistry, format_skill_listing, load_merged_skills};
pub use system_prompt::{ContextFile, SystemPromptBuilder, find_context_files};
pub use token_estimator::{ModelLimits, TokenEstimator};
pub use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};
pub use tools::{
    AgentTool, AskUserTool, BindBackfillHook, EnterPlanModeTool, ExitPlanModeTool, FeedSearchTool,
    FileEditTool, FileReadTool, FileWriteTool, GlobTool, GrepTool, MemorySearchTool,
    MemoryStoreTool, SessionWorkstream, ShellTool, SignalQueryTool,
    SignalSearchTool, SignalTimelineTool, SkillTool, SleepTool, TaskGetTool,
    TaskListTool, TaskOutputTool, TaskStopTool, ThinkTool, UnbindHook,
    WebFetchTool, WebSearchTool, WorkstreamApplyTool, WorkstreamBindTool, WorkstreamCreateTool,
    WorkstreamDeleteTool, WorkstreamDescribeTool, WorkstreamDustTool, WorkstreamJournalTool,
    WorkstreamListTool, WorkstreamPromoteTool, WorkstreamProposeOntologyTool, WorkstreamRefineTool,
    WorkstreamRollbackTool, WorkstreamShowTool, WorkstreamSwitchTool, WorkstreamTagTool,
    WorkstreamUnbindTool,
};
pub use tools::{
    DailyAddTodoTool, DailyCurrentTool, DailyListItemsTool, DailyPatchItemTool, DailyRunTool,
    RetroCurrentTool, RetroListItemsTool, RetroPatchItemTool, RetroRunTool, RetroSaveDiaryTool,
    RetroSetCadenceTool,
    TodoArchiveTool, TodoCreateTool, TodoDoneTool, TodoGetTool, TodoListTool, TodoPatchTool,
    TodoSearchTool, TodoUndoTool, WeeklyAddPriorityTool, WeeklyConfirmPriorityTool,
    WeeklyCurrentTool, WeeklyListItemsTool, WeeklyListPrioritiesTool, WeeklyRejectPriorityTool,
    WeeklyRunTool,
};
pub use workstream_router::{MemoryHandle, WorkstreamMemoryRouter};

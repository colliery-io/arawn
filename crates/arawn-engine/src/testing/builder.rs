use std::sync::Arc;

use arawn_core::Lens;
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_tool::{Tool, ToolRegistry};
use tempfile::TempDir;

use crate::hooks::HookRunner;
use crate::permissions::PermissionChecker;
use crate::plan::PlanModeState;
use crate::query_engine::QueryEngineConfig;
use crate::skills::SkillRegistry;

use super::TestHarness;

/// Builder for constructing a TestHarness.
pub struct TestHarnessBuilder {
    temp_dir: TempDir,
    files: Vec<(String, String)>,
    tools: Vec<Box<dyn Tool>>,
    script: Vec<MockResponse>,
    max_iterations: usize,
    permission_checker: Option<Arc<PermissionChecker>>,
    hook_runner: Option<Arc<HookRunner>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    plan_active: bool,
    with_progress: bool,
}

impl TestHarnessBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("failed to create temp dir"),
            files: vec![],
            tools: vec![],
            script: vec![],
            max_iterations: 20,
            permission_checker: None,
            hook_runner: None,
            skill_registry: None,
            plan_active: false,
            with_progress: false,
        }
    }

    /// Pre-populate a file in the lens directory.
    pub fn with_lens_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.files.push((path.into(), content.into()));
        self
    }

    /// Register a tool in the registry.
    pub fn with_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Register multiple tools.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Box<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set the scripted LLM responses.
    pub fn with_script(mut self, script: Vec<MockResponse>) -> Self {
        self.script = script;
        self
    }

    /// Set max iterations for the engine.
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Wire a permission checker into the engine.
    pub fn with_permission_checker(mut self, checker: Arc<PermissionChecker>) -> Self {
        self.permission_checker = Some(checker);
        self
    }

    /// Wire a hook runner into the engine.
    pub fn with_hook_runner(mut self, runner: Arc<HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }

    /// Wire a skill registry into the engine.
    pub fn with_skill_registry(mut self, registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    /// Enable plan mode on the engine (blocks write tools, allows read-only).
    pub fn with_plan_active(mut self) -> Self {
        self.plan_active = true;
        self
    }

    /// Enable progress event capture. Call `progress_rx()` on the built harness
    /// to get the receiver.
    pub fn with_progress_channel(mut self) -> Self {
        self.with_progress = true;
        self
    }

    /// Build the harness.
    pub fn build(self) -> TestHarness {
        // Create files in temp dir
        for (path, content) in &self.files {
            let full_path = self.temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).expect("failed to create parent dirs");
            }
            std::fs::write(&full_path, content).expect("failed to write file");
        }

        let lens = Lens::new("test", self.temp_dir.path());
        let registry = Arc::new(ToolRegistry::new());
        for tool in self.tools {
            registry.register(tool);
        }

        // Auto-register SkillTool when a skill registry is provided
        if let Some(ref skill_reg) = self.skill_registry {
            registry.register(Box::new(crate::tools::SkillTool::new(skill_reg.clone())));
        }

        let mock_llm = Arc::new(MockLlmClient::new(self.script));

        let config = QueryEngineConfig {
            max_iterations: self.max_iterations,
            system_prompt: "Test system prompt".into(),
            ..Default::default()
        };

        let plan_state = if self.plan_active {
            let ps = Arc::new(PlanModeState::new());
            ps.enter(
                crate::permissions::PermissionMode::Ask,
                "test-plan",
                self.temp_dir.path(),
            )
            .expect("failed to enter plan mode");
            Some(ps)
        } else {
            None
        };

        let (progress_tx, progress_rx) = if self.with_progress {
            let (tx, rx) = tokio::sync::mpsc::channel(256);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        TestHarness {
            _temp_dir: self.temp_dir,
            lens,
            registry,
            mock_llm,
            config,
            permission_checker: self.permission_checker,
            hook_runner: self.hook_runner,
            skill_registry: self.skill_registry,
            plan_state,
            progress_tx,
            progress_rx: std::sync::Mutex::new(progress_rx),
        }
    }
}

impl Default for TestHarnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

//! Engine config builder.

use arawn_engine::QueryEngineConfig;

pub fn build_engine_config(
    config: &crate::ArawnConfig,
    workstream: &arawn_core::Workstream,
    data_dir: &str,
) -> QueryEngineConfig {
    let engine_llm = config.engine_llm();
    QueryEngineConfig {
        // Emit a hint instead of a concrete model name. The pool resolves
        // this at engine-construction time in `local_service`. T-0278 will
        // promote this to per-call resolution.
        model: arawn_llm::ModelHint::Heavy.as_hint(),
        max_iterations: config.engine.max_iterations,
        system_prompt: String::new(),
        max_tokens: Some(engine_llm.max_tokens),
        model_limits: arawn_engine::ModelLimits::new(
            engine_llm.context_window,
            config.compactor.compaction_threshold,
        ),
        data_dir: Some(std::path::PathBuf::from(data_dir)),
        prompt_context: Some(arawn_engine::PromptContext {
            prompts_dir: Some(config.prompts_dir()),
            os: std::env::consts::OS.to_string(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "sh".into()),
            cwd: workstream.root_dir.clone(),
            workstream_name: workstream.name.clone(),
            workstream_root: workstream.root_dir.clone(),
            context_files: arawn_engine::find_context_files(
                &workstream.root_dir,
                &std::path::PathBuf::from(data_dir),
            ),
            memories: vec![],
            session_context: String::new(),
            plugin_prompts: vec![],
            // Overridden per-session in `LocalService` from the active
            // workstream's column; the template carries the boot workstream's
            // value so single-shot CLI flows pick the right persona too.
            identity_profile: workstream.identity_profile,
            // Filled in by LocalService per-query (it has access to the
            // integration registry); the template stays None.
            integration_capabilities: None,
            // Same as above — filled in by LocalService per-query.
            connected_services: None,
        }),
        tool_timeout_secs: config.engine.tool_timeout_secs,
    }
}

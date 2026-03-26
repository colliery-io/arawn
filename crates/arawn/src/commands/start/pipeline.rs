//! Pipeline engine initialization and runtime registration for the start command.

use std::path::PathBuf;
use std::sync::Arc;

use arawn_agent::ToolRegistry;
use arawn_agent_tools as tools;
use arawn_pipeline::sandbox::ScriptExecutor;
use arawn_pipeline::{
    CatalogEntry, PipelineConfig, PipelineEngine, RuntimeCatalog, RuntimeCategory, WorkflowEvent,
    WorkflowLoader, build_executor_factory,
};
use tokio::sync::RwLock;

use super::Context;

/// Phase 6: Initialize the pipeline engine.
pub(super) async fn init_pipeline(
    pipeline_cfg: &arawn_config::PipelineSection,
    data_dir: &std::path::Path,
    ctx: &Context,
) -> (
    Option<Arc<PipelineEngine>>,
    PathBuf,
    Option<arawn_pipeline::WatcherHandle>,
) {
    let resolve_path = |p: Option<PathBuf>, default: &str| -> PathBuf {
        let p = p.unwrap_or_else(|| PathBuf::from(default));
        if p.is_relative() { data_dir.join(p) } else { p }
    };

    let pipeline_db_path = resolve_path(pipeline_cfg.database.clone(), "pipeline.db");
    let pipeline_workflow_dir = resolve_path(pipeline_cfg.workflow_dir.clone(), "workflows");

    if !pipeline_cfg.enabled {
        if ctx.verbose {
            println!("Pipeline engine: disabled");
        }
        return (None, pipeline_workflow_dir, None);
    }

    let engine_config = PipelineConfig {
        max_concurrent_tasks: pipeline_cfg.max_concurrent_tasks,
        task_timeout_secs: pipeline_cfg.task_timeout_secs,
        pipeline_timeout_secs: pipeline_cfg.pipeline_timeout_secs,
        cron_enabled: pipeline_cfg.cron_enabled,
        triggers_enabled: pipeline_cfg.triggers_enabled,
    };

    if let Err(e) = std::fs::create_dir_all(&pipeline_workflow_dir) {
        tracing::warn!("failed to create workflow directory: {}", e);
    }

    match PipelineEngine::new(&pipeline_db_path, engine_config).await {
        Ok(engine) => {
            let engine = Arc::new(engine);
            if ctx.verbose {
                println!(
                    "Pipeline engine: enabled (db: {}, workflows: {})",
                    pipeline_db_path.display(),
                    pipeline_workflow_dir.display(),
                );
            }
            (Some(engine), pipeline_workflow_dir, None)
        }
        Err(e) => {
            tracing::warn!("failed to start pipeline engine: {}", e);
            (None, pipeline_workflow_dir, None)
        }
    }
}

/// Phase 9: Register pipeline tools (CatalogTool, WorkflowTool) and start workflow hot-reload watcher.
pub(super) async fn register_pipeline_tools(
    engine: &Arc<PipelineEngine>,
    pipeline_cfg: &arawn_config::PipelineSection,
    pipeline_workflow_dir: &std::path::Path,
    data_dir: &std::path::Path,
    tool_registry: &mut ToolRegistry,
    ctx: &Context,
) -> Option<arawn_pipeline::WatcherHandle> {
    // Load runtime catalog + script executor (with fallbacks)
    let (executor, catalog) = {
        let runtimes_dir = data_dir.join("runtimes");
        let catalog = match RuntimeCatalog::load(&runtimes_dir) {
            Ok(c) => {
                if ctx.verbose {
                    println!("Runtime catalog: {}", runtimes_dir.display());
                }
                Arc::new(RwLock::new(c))
            }
            Err(e) => {
                tracing::warn!(
                    " failed to load runtime catalog at {}: {}",
                    runtimes_dir.display(),
                    e
                );
                let fallback = std::env::temp_dir().join("arawn-runtimes");
                match RuntimeCatalog::load(&fallback) {
                    Ok(c) => {
                        tracing::warn!("using fallback catalog at {}", fallback.display());
                        Arc::new(RwLock::new(c))
                    }
                    Err(e2) => {
                        tracing::error!("failed to create fallback catalog: {}", e2);
                        return None;
                    }
                }
            }
        };

        let cache_dir = data_dir.join("wasm-cache");
        let executor = match ScriptExecutor::new(
            cache_dir.clone(),
            std::time::Duration::from_secs(pipeline_cfg.task_timeout_secs),
        ) {
            Ok(e) => {
                if ctx.verbose {
                    println!("Script executor: cache at {}", cache_dir.display());
                }
                Arc::new(e)
            }
            Err(e) => {
                tracing::warn!("failed to create script executor: {}", e);
                let fallback_cache = std::env::temp_dir().join("arawn-wasm-cache");
                match ScriptExecutor::new(
                    fallback_cache,
                    std::time::Duration::from_secs(pipeline_cfg.task_timeout_secs),
                ) {
                    Ok(e2) => {
                        tracing::warn!("using fallback WASM cache");
                        Arc::new(e2)
                    }
                    Err(e2) => {
                        tracing::error!("failed to create fallback executor: {}", e2);
                        return None;
                    }
                }
            }
        };

        (executor, catalog)
    };

    // Auto-compile built-in WASM runtimes
    let runtimes_src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(|p| p.join("runtimes"));
    if let Some(ref src_dir) = runtimes_src_dir
        && src_dir.is_dir()
    {
        register_builtin_runtimes(src_dir, &executor, &catalog, ctx.verbose).await;
    }

    // Register CatalogTool + WorkflowTool
    tool_registry.register(tools::CatalogTool::new(catalog.clone(), executor.clone()));
    tool_registry.register(tools::WorkflowTool::new(
        engine.clone(),
        pipeline_workflow_dir.to_path_buf(),
        executor.clone(),
        catalog.clone(),
    ));

    // Load existing workflows + start hot-reload watcher

    match WorkflowLoader::new(pipeline_workflow_dir) {
        Ok(loader) => {
            let factory = build_executor_factory(executor.clone(), catalog.clone());

            let events = loader.load_all().await;
            for event in &events {
                if let WorkflowEvent::Loaded { name, path } = event {
                    let wf = match arawn_pipeline::WorkflowFile::from_file(path) {
                        Ok(wf) => wf,
                        Err(e) => {
                            tracing::warn!(" failed to parse workflow {}: {}", path.display(), e);
                            continue;
                        }
                    };
                    match wf.workflow.to_dynamic_tasks(&factory) {
                        Ok(tasks) => {
                            if let Err(e) = engine
                                .register_dynamic_workflow(name, &wf.workflow.description, tasks)
                                .await
                            {
                                tracing::warn!(" failed to register workflow {}: {}", name, e);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(" failed to convert workflow {} tasks: {}", name, e)
                        }
                    }
                }
            }

            if ctx.verbose {
                let loaded = events
                    .iter()
                    .filter(|e| matches!(e, WorkflowEvent::Loaded { .. }))
                    .count();
                if loaded > 0 {
                    println!("Workflow loader: {} workflows loaded", loaded);
                }
            }

            match loader.watch() {
                Ok((mut event_rx, handle)) => {
                    let engine_w = engine.clone();
                    let factory_w = build_executor_factory(executor, catalog);
                    tokio::spawn(async move {
                        while let Some(event) = event_rx.recv().await {
                            match event {
                                WorkflowEvent::Loaded { name, path } => {
                                    let wf = match arawn_pipeline::WorkflowFile::from_file(&path) {
                                        Ok(wf) => wf,
                                        Err(e) => {
                                            tracing::warn!(
                                                "Hot-reload: failed to parse {}: {}",
                                                path.display(),
                                                e
                                            );
                                            continue;
                                        }
                                    };
                                    match wf.workflow.to_dynamic_tasks(&factory_w) {
                                        Ok(tasks) => {
                                            if let Err(e) = engine_w
                                                .register_dynamic_workflow(
                                                    &name,
                                                    &wf.workflow.description,
                                                    tasks,
                                                )
                                                .await
                                            {
                                                tracing::warn!(
                                                    "Hot-reload: failed to register {}: {}",
                                                    name,
                                                    e
                                                );
                                            } else {
                                                tracing::info!(
                                                    "Hot-reload: workflow {} reloaded",
                                                    name
                                                );
                                            }
                                        }
                                        Err(e) => tracing::warn!(
                                            "Hot-reload: failed to convert {} tasks: {}",
                                            name,
                                            e
                                        ),
                                    }
                                }
                                WorkflowEvent::Removed { name, .. } => {
                                    tracing::info!("Hot-reload: workflow {} removed", name)
                                }
                                WorkflowEvent::Error { path, error } => tracing::warn!(
                                    "Hot-reload: error processing {}: {}",
                                    path.display(),
                                    error
                                ),
                            }
                        }
                    });
                    if ctx.verbose {
                        println!("Workflow watcher: enabled");
                    }
                    Some(handle)
                }
                Err(e) => {
                    tracing::warn!("failed to start workflow watcher: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!("failed to create workflow loader: {}", e);
            None
        }
    }
}

/// Compile and register built-in WASM runtimes from source crate directories.
///
/// Scans `runtimes_src_dir` for subdirectories, each expected to be a Cargo crate.
/// For each, if the runtime isn't already in the catalog, compiles it to wasm32-wasip1
/// and registers the `.wasm` as a builtin entry.
pub(super) async fn register_builtin_runtimes(
    runtimes_src_dir: &std::path::Path,
    executor: &Arc<ScriptExecutor>,
    catalog: &Arc<RwLock<RuntimeCatalog>>,
    verbose: bool,
) {
    let entries = match std::fs::read_dir(runtimes_src_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("cannot read runtimes source dir: {e}");
            return;
        }
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() || !path.join("Cargo.toml").exists() {
            continue;
        }

        let runtime_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip if already registered
        {
            let cat = catalog.read().await;
            if cat.get(&runtime_name).is_some() {
                if verbose {
                    println!("Runtime '{}' already registered, skipping", runtime_name);
                }
                continue;
            }
        }

        if verbose {
            println!("Compiling runtime '{}'...", runtime_name);
        }

        let wasm_path = match executor.compile_crate(&path).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(" failed to compile runtime '{}': {}", runtime_name, e);
                continue;
            }
        };

        // Copy .wasm to catalog's builtin/ directory
        let mut cat = catalog.write().await;
        let builtin_dir = cat.root().join("builtin");
        if let Err(e) = std::fs::create_dir_all(&builtin_dir) {
            tracing::warn!("cannot create builtin dir: {e}");
            continue;
        }

        let dest = builtin_dir.join(format!("{runtime_name}.wasm"));
        if let Err(e) = std::fs::copy(&wasm_path, &dest) {
            tracing::warn!("failed to copy wasm for '{}': {}", runtime_name, e);
            continue;
        }

        if let Err(e) = cat.add(
            &runtime_name,
            CatalogEntry {
                description: format!("Built-in {runtime_name} runtime"),
                path: format!("builtin/{runtime_name}.wasm"),
                category: RuntimeCategory::Builtin,
            },
        ) {
            tracing::warn!(" failed to register runtime '{}': {}", runtime_name, e);
            continue;
        }

        if verbose {
            println!("Registered runtime '{}'", runtime_name);
        }
    }
}

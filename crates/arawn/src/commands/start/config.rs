//! Configuration loading and validation for the start command.

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;

use arawn_config::ResolvedLlm;

use super::{Context, StartArgs};

/// Resolved server settings from config + CLI args.
pub(super) type ServerSettings = (SocketAddr, Option<PathBuf>, Option<PathBuf>, Option<String>);

/// Phase 1-2: Load config from file or discovery, print warnings, validate.
pub(super) fn load_and_validate_config(
    args: &StartArgs,
    ctx: &Context,
) -> Result<arawn_config::LoadedConfig> {
    let loaded = if let Some(ref config_path) = args.config {
        let config = arawn_config::load_config_file(config_path)?;
        let source = arawn_config::discovery::ConfigSource {
            path: config_path.clone(),
            loaded: true,
        };
        arawn_config::LoadedConfig {
            config,
            sources: vec![source.clone()],
            source: Some(source),
            warnings: Vec::new(),
        }
    } else {
        arawn_config::load_config(args.workspace.as_deref())?
    };

    for warning in &loaded.warnings {
        tracing::warn!("{}", warning);
    }

    if ctx.verbose {
        let sources = loaded.loaded_from();
        if sources.is_empty() {
            println!("No config files found, using defaults + CLI args");
        } else {
            for source in sources {
                println!("Loaded config: {}", source.display());
            }
        }
    }

    validate_config(&loaded.config)?;
    Ok(loaded)
}

/// Phase 4: Resolve server bind address, workspace, bootstrap dir, and auth token.
pub(super) fn resolve_server_settings(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
    ctx: &Context,
) -> Result<ServerSettings> {
    let server_cfg = config.server.as_ref();
    let port = args
        .port
        .or_else(|| server_cfg.map(|s| s.port))
        .unwrap_or(arawn_types::config::defaults::DEFAULT_PORT);
    let bind = args
        .bind
        .clone()
        .or_else(|| server_cfg.map(|s| s.bind.clone()))
        .unwrap_or_else(|| arawn_types::config::defaults::DEFAULT_BIND.to_string());
    let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;

    let workspace = args
        .workspace
        .clone()
        .or_else(|| server_cfg.and_then(|s| s.workspace.clone()));
    let bootstrap_dir = args
        .bootstrap_dir
        .clone()
        .or_else(|| server_cfg.and_then(|s| s.bootstrap_dir.clone()));

    let explicit_token = args
        .token
        .clone()
        .or_else(|| std::env::var("ARAWN_API_TOKEN").ok());
    let auth_token: Option<String> = if let Some(token) = explicit_token {
        Some(token)
    } else if addr.ip().is_loopback() {
        None
    } else {
        let token = super::server::load_or_generate_server_token()?;
        println!("Server auth token: {}", token);
        Some(token)
    };

    if ctx.verbose {
        println!("Bind address: {}", addr);
        match &auth_token {
            Some(t) => println!("Auth token: {}...", &t[..8.min(t.len())]),
            None => println!("Auth: disabled (localhost)"),
        }
    }

    Ok((addr, workspace, bootstrap_dir, auth_token))
}

/// Resolve LLM config, applying CLI overrides on top of config file values.
pub(super) fn resolve_with_cli_overrides(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
) -> Result<ResolvedLlm> {
    // Try config-based resolution first
    let mut resolved = match arawn_config::resolve_for_agent(config, "default") {
        Ok(r) => r,
        Err(_) => {
            // No config — build from CLI args or fail
            let backend_str = args.backend.as_deref().unwrap_or("anthropic");
            let backend = super::llm::parse_backend(backend_str)?;
            let model = args
                .model
                .clone()
                .unwrap_or_else(|| super::llm::default_model(&backend));

            let api_key = args.api_key.clone().or_else(|| {
                let ref_name = backend.env_var();
                arawn_config::secrets::resolve_api_key_ref(ref_name).map(|r| r.value)
            });

            ResolvedLlm {
                backend,
                model,
                base_url: args.base_url.clone(),
                api_key,
                api_key_source: None,
                api_key_ref: Some(backend.env_var().to_string()),
                resolved_from: arawn_config::ResolvedFrom::GlobalDefault,
                retry_max: None,
                retry_backoff_ms: None,
            }
        }
    };

    // CLI overrides
    if let Some(ref backend_str) = args.backend {
        resolved.backend = super::llm::parse_backend(backend_str)?;
    }
    if let Some(ref model) = args.model {
        resolved.model = model.clone();
    }
    if let Some(ref base_url) = args.base_url {
        resolved.base_url = Some(base_url.clone());
    }
    if let Some(ref api_key) = args.api_key {
        resolved.api_key = Some(api_key.clone());
        resolved.api_key_source = None; // CLI override, no tracked source
    }

    Ok(resolved)
}

/// Validate configuration values at startup, failing fast with clear errors.
pub(super) fn validate_config(config: &arawn_config::ArawnConfig) -> Result<()> {
    if let Some(ref server) = config.server {
        if server.port == 0 {
            anyhow::bail!("Invalid config: server.port cannot be 0");
        }
        if server.api_rpm == 0 {
            anyhow::bail!("Invalid config: server.api_rpm must be > 0");
        }
    }

    if let Some(ref embedding) = config.embedding
        && embedding.dimensions == Some(0)
    {
        anyhow::bail!("Invalid config: embedding.dimensions must be > 0");
    }

    Ok(())
}

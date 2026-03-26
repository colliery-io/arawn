//! LLM backend initialization and resolution for the start command.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use arawn_config::{Backend, LlmConfig, ResolvedLlm};
use arawn_llm::{
    AnthropicBackend, AnthropicConfig, ApiKeyProvider, OpenAiBackend, OpenAiConfig, SharedBackend,
};

use super::Context;
use super::StartArgs;

/// Phase 3: Resolve LLM backends (default + named profiles).
pub(super) async fn init_llm_backends(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
    ctx: &Context,
) -> Result<(ResolvedLlm, SharedBackend, HashMap<String, SharedBackend>)> {
    let resolved = super::config::resolve_with_cli_overrides(config, args)?;

    if ctx.verbose {
        println!("Backend: {}", resolved.backend);
        println!("Model: {}", resolved.model);
        println!("Resolved via: {}", resolved.resolved_from);
        if let Some(ref source) = resolved.api_key_source {
            println!("API key from: {}", source);
        }
    }

    let backend = create_backend(&resolved, config.oauth.as_ref()).await?;

    let mut backends: HashMap<String, SharedBackend> = HashMap::new();
    backends.insert("default".to_string(), backend.clone());

    for (name, llm_config) in &config.llm_profiles {
        match resolve_profile(name, llm_config) {
            Ok(profile_resolved) => {
                match create_backend(&profile_resolved, config.oauth.as_ref()).await {
                    Ok(profile_backend) => {
                        if ctx.verbose {
                            println!(
                                "Backend '{}': {} / {}",
                                name, profile_resolved.backend, profile_resolved.model
                            );
                        }
                        backends.insert(name.clone(), profile_backend);
                    }
                    Err(e) => {
                        tracing::warn!("failed to create backend '{}': {}", name, e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("failed to resolve profile '{}': {}", name, e);
            }
        }
    }

    if ctx.verbose && backends.len() > 1 {
        println!(
            "Available backends: {}",
            backends
                .keys()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok((resolved, backend, backends))
}

/// Create an LLM backend from a resolved config.
pub(super) async fn create_backend(
    resolved: &ResolvedLlm,
    oauth_overrides: Option<&arawn_config::OAuthConfigOverride>,
) -> Result<SharedBackend> {
    match resolved.backend {
        Backend::Anthropic => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = AnthropicConfig::new("placeholder");
            config.api_key = provider;
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(AnthropicBackend::new(config)?))
        }
        Backend::Openai => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = OpenAiConfig::openai("placeholder");
            config.api_key = provider;
            if let Some(ref base_url) = resolved.base_url {
                config = config.with_base_url(base_url);
            }
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Groq => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = OpenAiConfig::groq("placeholder");
            config.api_key = provider;
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Ollama => {
            let mut config = OpenAiConfig::ollama();
            if let Some(ref base_url) = resolved.base_url {
                config = config.with_base_url(base_url);
            }
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Custom => {
            let base_url = resolved.base_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Custom backend requires base_url in config or --base-url")
            })?;
            let mut config = OpenAiConfig::openai("")
                .with_base_url(base_url)
                .with_name("custom")
                .with_model(&resolved.model);
            if let Some(ref api_key) = resolved.api_key {
                config.api_key = ApiKeyProvider::from_static(api_key);
            } else {
                config.api_key = ApiKeyProvider::None;
            }
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::ClaudeOauth => {
            // IMPORTANT: The claude-oauth backend proxies through Claude Code's OAuth
            // infrastructure. It supports conversation only — tool use is NOT available.
            // This backend cannot drive agentic workflows (tool calling, file operations,
            // etc.). For full agent capabilities, use the `anthropic` backend with an
            // API key.
            //
            // WARNING: Using this backend outside of Claude Code may violate Anthropic's
            // Terms of Service. Use at your own risk.
            //
            // Start the OAuth proxy on a random port, then point AnthropicBackend at it
            let data_dir = arawn_config::xdg_config_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

            let oauth_config = {
                let base = arawn_oauth::OAuthConfig::default();
                match oauth_overrides {
                    Some(o) => base.with_overrides(
                        o.client_id.as_deref(),
                        o.authorize_url.as_deref(),
                        o.token_url.as_deref(),
                        o.redirect_uri.as_deref(),
                        o.scope.as_deref(),
                    ),
                    None => base,
                }
            };

            let token_manager = arawn_oauth::token_manager::create_token_manager_with_config(
                &data_dir,
                oauth_config,
            );
            if !token_manager.has_tokens() {
                return Err(anyhow::anyhow!(
                    "No OAuth tokens found. Run 'arawn auth login' first to authenticate."
                ));
            }

            let proxy_config =
                arawn_oauth::ProxyConfig::default().with_token_manager(token_manager);

            let proxy = arawn_oauth::ProxyServer::new(proxy_config);
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let proxy_addr = proxy
                .run_with_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start OAuth proxy: {}", e))?;

            // Leak the shutdown sender so the proxy lives for the process lifetime
            std::mem::forget(shutdown_tx);

            let proxy_url = format!("http://{}", proxy_addr);
            println!("OAuth proxy running on {}", proxy_url);

            // Point Anthropic backend at the proxy — no API key needed,
            // proxy handles auth via OAuth tokens
            let config = AnthropicConfig::new("oauth-proxy-managed").with_base_url(&proxy_url);
            Ok(Arc::new(AnthropicBackend::new(config)?))
        }
    }
}

/// Build an `ApiKeyProvider` that re-resolves from the secret store on each request.
///
/// This enables hot-loading: secrets stored after server startup are picked up
/// automatically without a restart.
pub(super) fn make_api_key_provider(ref_name: String) -> ApiKeyProvider {
    ApiKeyProvider::dynamic(move || {
        arawn_config::secrets::resolve_api_key_ref(&ref_name).map(|r| r.value)
    })
}

/// Resolve a named LLM profile into a ResolvedLlm ready for backend creation.
pub(super) fn resolve_profile(name: &str, llm_config: &LlmConfig) -> Result<ResolvedLlm> {
    let backend = llm_config
        .backend
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' is missing 'backend' field", name))?;

    let model = llm_config
        .model
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' is missing 'model' field", name))?;

    let api_key_ref = llm_config
        .api_key_ref
        .as_deref()
        .unwrap_or(backend.env_var());
    let resolved_secret = arawn_config::secrets::resolve_api_key_ref(api_key_ref);

    let (api_key, api_key_source) = match resolved_secret {
        Some(s) => (Some(s.value), Some(s.source)),
        None => (None, None),
    };

    Ok(ResolvedLlm {
        backend,
        model,
        base_url: llm_config.base_url.clone(),
        api_key,
        api_key_source,
        api_key_ref: Some(api_key_ref.to_string()),
        resolved_from: arawn_config::ResolvedFrom::AgentSpecific {
            agent: "profile".to_string(),
            profile: name.to_string(),
        },
        retry_max: llm_config.retry_max,
        retry_backoff_ms: llm_config.retry_backoff_ms,
    })
}

pub(super) fn parse_backend(s: &str) -> Result<Backend> {
    match s.to_lowercase().as_str() {
        "anthropic" => Ok(Backend::Anthropic),
        "openai" => Ok(Backend::Openai),
        "groq" => Ok(Backend::Groq),
        "ollama" => Ok(Backend::Ollama),
        "custom" => Ok(Backend::Custom),
        "claude-oauth" | "claudeoauth" => Ok(Backend::ClaudeOauth),
        other => Err(anyhow::anyhow!(
            "Unknown backend '{}'. Valid: anthropic, openai, groq, ollama, custom, claude-oauth",
            other
        )),
    }
}

pub(super) fn default_model(backend: &Backend) -> String {
    match backend {
        Backend::Anthropic | Backend::ClaudeOauth => "claude-sonnet-4-20250514".to_string(),
        Backend::Openai => "gpt-4o".to_string(),
        Backend::Groq => "llama-3.1-70b-versatile".to_string(),
        Backend::Ollama => "llama3.2".to_string(),
        Backend::Custom => "default".to_string(),
    }
}

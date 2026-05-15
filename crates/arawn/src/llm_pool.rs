//! Eager pool of named LLM clients.
//!
//! `LlmClientPool` is the single authority for resolving LLM clients at
//! runtime. It is constructed once in `main.rs` from an [`ArawnConfig`],
//! eagerly instantiates every `[llm.*]` entry, wraps each in a
//! [`arawn_llm::RetryClient`], and exposes name-based and role-based lookups.
//!
//! Startup is **fail-fast**: a missing API key env var or unsupported provider
//! surfaces here, not mid-session.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use arawn_llm::{LlmClient, ModelHint};
use arawn_llm::routing::{
    IntelligentRoutingProvider, LocalHealthChecker, ProviderHandle, RoutingHints, SharedHealth,
};
use arawn_tool::{LlmPreference, LlmResolution, MatchQuality};

use crate::config::{ArawnConfig, HintRoutingConfig, LlmConfig, ProvidersRoutingConfig};

/// A pool of named LLM clients built from an [`ArawnConfig`].
pub struct LlmClientPool {
    clients: HashMap<String, Arc<dyn LlmClient>>,
    configs: HashMap<String, LlmConfig>,
    engine_name: String,
    compactor_name: String,
    /// Resolved Local/Remote profile names for the routing policy
    /// (T-0278). `None` means the role is unconfigured; the routing
    /// provider degrades to "always Remote" / "no fallback" in that
    /// case.
    local_provider_name: Option<String>,
    remote_provider_name: Option<String>,
    /// Shared health snapshot for the local provider. Stored even
    /// when no local profile is configured (it returns
    /// `NotConfigured` in that case).
    local_health: SharedHealth,
    /// Hint tier → `[llm.NAME]`. Each entry is a *resolved* profile name
    /// (one that exists in `clients`); unresolved hints fall back to
    /// `engine_name` at lookup time so the map can simply be empty.
    hint_names: HashMap<ModelHint, String>,
}

impl std::fmt::Debug for LlmClientPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClientPool")
            .field("entries", &self.clients.keys().collect::<Vec<_>>())
            .field("engine_name", &self.engine_name)
            .field("compactor_name", &self.compactor_name)
            .finish()
    }
}

impl LlmClientPool {
    /// Build the pool from the given config. Eagerly constructs every
    /// `[llm.*]` entry; if any entry fails to build, returns an error and
    /// no pool is created.
    pub fn from_config<F>(config: &ArawnConfig, build: F) -> Result<Self>
    where
        F: Fn(&LlmConfig) -> Result<Arc<dyn LlmClient>>,
    {
        let mut clients = HashMap::with_capacity(config.llm.len());
        let mut configs = HashMap::with_capacity(config.llm.len());

        for (name, llm_config) in &config.llm {
            let raw = build(llm_config).with_context(|| {
                format!("failed to build LLM client for [llm.{name}]")
            })?;
            // Layering: raw provider
            //   → RetryClient        (retries on transient errors)
            //   → UsageTrackingClient (records token usage to the
            //     process-wide tracker — T-0277)
            //   → WarmingClient      (TTL-cached warmup + cold-restart retry)
            let with_retry: Arc<dyn LlmClient> =
                Arc::new(arawn_llm::RetryClient::new(raw));
            let tracked: Arc<dyn LlmClient> = Arc::new(
                arawn_llm::usage::UsageTrackingClient::new(with_retry, llm_config.provider.clone()),
            );
            let warmed: Arc<dyn LlmClient> = Arc::new(
                arawn_llm::WarmingClient::new(tracked, llm_config.provider.clone()),
            );
            clients.insert(name.clone(), warmed);
            configs.insert(name.clone(), llm_config.clone());
        }

        // Resolve role names with the same fallback logic as `engine_llm` /
        // `compactor_llm` on `ArawnConfig`. Both methods are guaranteed to
        // return an entry that exists in the map.
        let engine_name = resolve_engine_name(config, &clients)?;
        let compactor_name = resolve_compactor_name(config, &engine_name);
        let hint_names = resolve_hint_names(&config.routing.hints, &clients);
        let (local_provider_name, remote_provider_name) =
            resolve_routing_provider_names(&config.routing.providers, &clients, &engine_name);
        let local_health: SharedHealth = Arc::new(if local_provider_name.is_some() {
            LocalHealthChecker::configured()
        } else {
            LocalHealthChecker::not_configured()
        });

        Ok(Self {
            clients,
            configs,
            engine_name,
            compactor_name,
            local_provider_name,
            remote_provider_name,
            local_health,
            hint_names,
        })
    }

    /// Construct a pool from a pre-built map of clients. Mostly useful for
    /// tests; production code should go through [`LlmClientPool::from_config`].
    pub fn from_clients(
        clients: HashMap<String, Arc<dyn LlmClient>>,
        configs: HashMap<String, LlmConfig>,
        engine_name: impl Into<String>,
        compactor_name: impl Into<String>,
    ) -> Self {
        let engine_name = engine_name.into();
        let compactor_name = compactor_name.into();
        debug_assert!(clients.contains_key(&engine_name));
        debug_assert!(clients.contains_key(&compactor_name));
        Self {
            clients,
            configs,
            engine_name,
            compactor_name,
            local_provider_name: None,
            remote_provider_name: None,
            local_health: Arc::new(LocalHealthChecker::not_configured()),
            hint_names: HashMap::new(),
        }
    }

    /// Build a single-entry pool wrapping `client` as both engine and
    /// compactor under the name "default". Convenience for tests.
    pub fn single(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self {
        let mut clients = HashMap::new();
        let mut configs = HashMap::new();
        let cfg = LlmConfig { model: model.into(), ..LlmConfig::default() };
        clients.insert("default".to_string(), client);
        configs.insert("default".to_string(), cfg);
        Self {
            clients,
            configs,
            engine_name: "default".to_string(),
            compactor_name: "default".to_string(),
            local_provider_name: None,
            remote_provider_name: None,
            local_health: Arc::new(LocalHealthChecker::not_configured()),
            hint_names: HashMap::new(),
        }
    }

    /// Look up a client by name (e.g., "default", "cheap", "judge").
    pub fn get(&self, name: &str) -> Option<Arc<dyn LlmClient>> {
        self.clients.get(name).cloned()
    }

    /// Get the [`LlmConfig`] for a named entry.
    pub fn config(&self, name: &str) -> Option<&LlmConfig> {
        self.configs.get(name)
    }

    /// Engine LLM — never fails; falls back to whatever `engine_llm()` resolved.
    pub fn engine(&self) -> Arc<dyn LlmClient> {
        Arc::clone(self.clients.get(&self.engine_name).expect("engine LLM resolved at construction"))
    }

    pub fn engine_config(&self) -> &LlmConfig {
        self.configs.get(&self.engine_name).expect("engine config resolved at construction")
    }

    pub fn engine_name(&self) -> &str {
        &self.engine_name
    }

    /// Compactor LLM — never fails; falls back to engine LLM if `[compactor]`
    /// names a missing entry or is absent.
    pub fn compactor(&self) -> Arc<dyn LlmClient> {
        Arc::clone(self.clients.get(&self.compactor_name).expect("compactor LLM resolved at construction"))
    }

    pub fn compactor_config(&self) -> &LlmConfig {
        self.configs.get(&self.compactor_name).expect("compactor config resolved at construction")
    }

    pub fn compactor_name(&self) -> &str {
        &self.compactor_name
    }

    /// Build an [`IntelligentRoutingProvider`] from the configured
    /// Local/Remote profiles. Returns `None` when no local profile
    /// is configured — without a Local target there is no routing
    /// decision to make, and callers should fall back to the
    /// existing engine client + `resolve_hint`.
    ///
    /// The returned provider is a cheap struct — fine to construct
    /// per-call when hints vary. The underlying clients and health
    /// checker are shared.
    pub fn routing_provider(
        &self,
        hints: RoutingHints,
    ) -> Option<IntelligentRoutingProvider> {
        // Routing exists to choose Local-vs-Remote. Without a Local
        // profile the call collapses to "always Remote" — which is
        // identical to passthrough through the engine client. Tell
        // the caller to skip the layer entirely in that case.
        self.local_provider_name.as_ref()?;
        let remote_name = self.remote_provider_name.as_ref()?;
        let remote = ProviderHandle {
            client: Arc::clone(self.clients.get(remote_name)?),
            model: self.configs.get(remote_name)?.model.clone(),
        };
        let local = self
            .local_provider_name
            .as_ref()
            .and_then(|n| {
                let client = Arc::clone(self.clients.get(n)?);
                let model = self.configs.get(n)?.model.clone();
                Some(ProviderHandle { client, model })
            });
        Some(IntelligentRoutingProvider::new(
            local,
            remote,
            Arc::clone(&self.local_health),
            hints,
        ))
    }

    /// Shared handle to the local-health checker. The pool itself is
    /// the only writer in production; tests use this to drive the
    /// `Healthy/Unhealthy` transitions.
    pub fn local_health(&self) -> SharedHealth {
        Arc::clone(&self.local_health)
    }

    /// Resolve a model-string at the call-site boundary.
    ///
    /// - `hint:lightweight|medium|heavy` → the client and concrete model
    ///   for the named profile (`[routing.hints]` → `[llm.NAME]`).
    /// - Unknown `hint:*` → falls back to the engine LLM. Logged via
    ///   tracing so misconfigurations surface.
    /// - Anything else → treated as a concrete model name. The pool
    ///   returns the engine client (it does not look the model up across
    ///   providers — that is intentional, the caller already chose the
    ///   client elsewhere). The string is passed back unchanged.
    ///
    /// The returned model string is what the caller should put into
    /// `ChatRequest.model`. The returned client is what should serve it.
    pub fn resolve_hint(&self, model_str: &str) -> (Arc<dyn LlmClient>, String) {
        if let Some(hint) = arawn_llm::classify_hint(model_str) {
            if let Some(name) = self.hint_names.get(&hint)
                && let (Some(client), Some(cfg)) = (self.clients.get(name), self.configs.get(name))
            {
                return (Arc::clone(client), cfg.model.clone());
            }
            // Configured profile vanished, or hint not in [routing.hints] —
            // fall through to engine.
            let engine_cfg = self.engine_config();
            return (self.engine(), engine_cfg.model.clone());
        }
        if arawn_llm::is_hint_shape(model_str) {
            // Hint-shaped but unparseable — log and fall back to engine.
            tracing::warn!(
                hint = %model_str,
                "unknown hint tier; falling back to engine LLM"
            );
            let engine_cfg = self.engine_config();
            return (self.engine(), engine_cfg.model.clone());
        }
        // Concrete model string — engine client, model as-is.
        (self.engine(), model_str.to_string())
    }

    /// Iterator over (name, config) pairs.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &LlmConfig)> {
        self.configs.iter()
    }

    /// Warm up every entry concurrently. Returns a vector of `(name, result)`
    /// so callers can log per-entry outcomes. Never fails as a whole — a bad
    /// model surfaces as `Err` for that entry while others may still succeed.
    pub async fn warmup_all(
        &self,
    ) -> Vec<(String, Result<(), arawn_llm::LlmError>)> {
        use futures::future::join_all;

        let probes = self.clients.iter().map(|(name, client)| {
            let name = name.clone();
            let client = Arc::clone(client);
            let model = self
                .configs
                .get(&name)
                .map(|c| c.model.clone())
                .unwrap_or_default();
            async move {
                let res = client.warmup(&model).await;
                (name, res)
            }
        });
        join_all(probes).await
    }

    /// Resolve an [`LlmPreference`] against the pool. Always succeeds — the
    /// engine LLM is the unconditional fallback.
    ///
    /// Resolution order:
    /// 1. Named match — `preference.named` is set and exists in the pool → `Exact`.
    /// 2. Provider+model match — both fields set and an entry matches → `Exact`.
    /// 3. Capability match — first entry satisfying `preference.capabilities` → `Capability`.
    /// 4. Fallback — engine default → `Fallback`.
    pub fn resolve(&self, preference: &LlmPreference) -> LlmResolution {
        // 1. Named match
        if let Some(name) = &preference.named
            && let (Some(client), Some(cfg)) = (self.clients.get(name), self.configs.get(name))
        {
            return LlmResolution {
                client: Arc::clone(client),
                info: cfg.to_resolved_info(),
                match_quality: MatchQuality::Exact,
            };
        }

        // 2. Provider+model exact match (both fields required for "exact")
        if let (Some(provider), Some(model)) = (&preference.provider, &preference.model) {
            for (name, cfg) in &self.configs {
                if cfg.provider == *provider && cfg.model == *model {
                    return LlmResolution {
                        client: Arc::clone(&self.clients[name]),
                        info: cfg.to_resolved_info(),
                        match_quality: MatchQuality::Exact,
                    };
                }
            }
        }

        // 3. Capability match (also catches provider-only or model-only requests)
        let want_provider = preference.provider.as_deref();
        let want_model = preference.model.as_deref();
        let need_caps = !preference.capabilities.is_empty()
            || want_provider.is_some()
            || want_model.is_some();
        if need_caps {
            for (name, cfg) in &self.configs {
                let info = cfg.to_resolved_info();
                if let Some(p) = want_provider
                    && cfg.provider != p
                {
                    continue;
                }
                if let Some(m) = want_model
                    && cfg.model != m
                {
                    continue;
                }
                if !preference.capabilities.satisfied_by(&info) {
                    continue;
                }
                return LlmResolution {
                    client: Arc::clone(&self.clients[name]),
                    info,
                    match_quality: MatchQuality::Capability,
                };
            }
        }

        // 4. Fallback to engine
        LlmResolution {
            client: self.engine(),
            info: self.engine_config().to_resolved_info(),
            match_quality: MatchQuality::Fallback,
        }
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

fn resolve_hint_names(
    cfg: &HintRoutingConfig,
    clients: &HashMap<String, Arc<dyn LlmClient>>,
) -> HashMap<ModelHint, String> {
    let mut out = HashMap::new();
    let mut accept = |tier: ModelHint, name: &Option<String>| {
        if let Some(n) = name
            && clients.contains_key(n)
        {
            out.insert(tier, n.clone());
        } else if let Some(n) = name {
            tracing::warn!(
                tier = %tier.as_str(),
                profile = %n,
                "[routing.hints] references unknown [llm.NAME]; falling back to engine"
            );
        }
    };
    accept(ModelHint::Lightweight, &cfg.lightweight);
    accept(ModelHint::Medium, &cfg.medium);
    accept(ModelHint::Heavy, &cfg.heavy);
    out
}

fn resolve_routing_provider_names(
    cfg: &ProvidersRoutingConfig,
    clients: &HashMap<String, Arc<dyn LlmClient>>,
    engine_name: &str,
) -> (Option<String>, Option<String>) {
    let local = cfg.local.as_ref().and_then(|n| {
        if clients.contains_key(n) {
            Some(n.clone())
        } else {
            tracing::warn!(
                profile = %n,
                "[routing.providers.local] references unknown [llm.NAME]; routing will run Remote-only"
            );
            None
        }
    });
    let remote = cfg
        .remote
        .as_ref()
        .and_then(|n| {
            if clients.contains_key(n) {
                Some(n.clone())
            } else {
                tracing::warn!(
                    profile = %n,
                    "[routing.providers.remote] references unknown [llm.NAME]; falling back to engine"
                );
                None
            }
        })
        .or_else(|| Some(engine_name.to_string()));
    (local, remote)
}

fn resolve_engine_name(
    config: &ArawnConfig,
    clients: &HashMap<String, Arc<dyn LlmClient>>,
) -> Result<String> {
    if clients.contains_key(&config.engine.llm) {
        return Ok(config.engine.llm.clone());
    }
    if clients.contains_key("default") {
        return Ok("default".to_string());
    }
    Err(anyhow!(
        "no LLM config found for engine: '{}' is not defined and no [llm.default] exists",
        config.engine.llm
    ))
}

fn resolve_compactor_name(config: &ArawnConfig, engine_name: &str) -> String {
    config
        .compactor
        .llm
        .as_ref()
        .filter(|name| config.llm.contains_key(name.as_str()))
        .cloned()
        .unwrap_or_else(|| engine_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_llm::MockLlmClient;

    fn mock_builder(_cfg: &LlmConfig) -> Result<Arc<dyn LlmClient>> {
        Ok(Arc::new(MockLlmClient::new(vec![])))
    }

    fn cfg_from_toml(toml_str: &str) -> ArawnConfig {
        toml::from_str(toml_str).expect("valid toml")
    }

    #[test]
    fn pool_builds_every_named_entry() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.cheap]
provider = "groq"
model = "llama-3.3-70b-versatile"

[engine]
llm = "default"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert_eq!(pool.len(), 2);
        assert!(pool.get("default").is_some());
        assert!(pool.get("cheap").is_some());
        assert!(pool.get("nonexistent").is_none());
    }

    #[test]
    fn engine_and_compactor_resolve_distinct_clients_when_configured() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.cheap]
provider = "groq"
model = "llama-3.3-70b-versatile"

[engine]
llm = "default"

[compactor]
llm = "cheap"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert_eq!(pool.engine_name(), "default");
        assert_eq!(pool.compactor_name(), "cheap");
        assert_eq!(pool.engine_config().model, "openai/gpt-oss-20b");
        assert_eq!(pool.compactor_config().model, "llama-3.3-70b-versatile");
        assert!(!Arc::ptr_eq(&pool.engine(), &pool.compactor()));
    }

    #[test]
    fn compactor_falls_back_to_engine_when_unconfigured() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert_eq!(pool.engine_name(), "default");
        assert_eq!(pool.compactor_name(), "default");
        assert!(Arc::ptr_eq(&pool.engine(), &pool.compactor()));
    }

    #[test]
    fn compactor_falls_back_to_engine_when_pointing_at_missing_entry() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[compactor]
llm = "nonexistent"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert_eq!(pool.compactor_name(), "default");
    }

    #[test]
    fn resolve_named_exact_match() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.cheap]
provider = "groq"
model = "llama-3.3-70b-versatile"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let res = pool.resolve(&LlmPreference::named("cheap"));
        assert_eq!(res.match_quality, MatchQuality::Exact);
        assert_eq!(res.info.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn resolve_named_missing_falls_back() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let res = pool.resolve(&LlmPreference::named("missing"));
        assert_eq!(res.match_quality, MatchQuality::Fallback);
        assert_eq!(res.info.model, "openai/gpt-oss-20b");
    }

    #[test]
    fn resolve_provider_model_exact() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.fast]
provider = "groq"
model = "llama-3.3-70b-versatile"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let res = pool.resolve(&LlmPreference::provider_model(
            "groq",
            "llama-3.3-70b-versatile",
        ));
        assert_eq!(res.match_quality, MatchQuality::Exact);
        assert_eq!(res.info.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn resolve_capability_match_when_no_exact() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
context_window = 128000

[llm.huge]
provider = "anthropic"
model = "claude-sonnet-4"
context_window = 200000
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let pref = LlmPreference {
            capabilities: arawn_tool::LlmCapabilities {
                min_context_window: Some(150_000),
                ..Default::default()
            },
            ..Default::default()
        };
        let res = pool.resolve(&pref);
        assert_eq!(res.match_quality, MatchQuality::Capability);
        assert_eq!(res.info.model, "claude-sonnet-4");
    }

    #[test]
    fn resolve_capability_too_strict_falls_back() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
context_window = 128000
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let pref = LlmPreference {
            capabilities: arawn_tool::LlmCapabilities {
                min_context_window: Some(1_000_000),
                ..Default::default()
            },
            ..Default::default()
        };
        let res = pool.resolve(&pref);
        assert_eq!(res.match_quality, MatchQuality::Fallback);
    }

    #[test]
    fn resolve_empty_preference_is_fallback() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let res = pool.resolve(&LlmPreference::any());
        assert_eq!(res.match_quality, MatchQuality::Fallback);
    }

    #[test]
    fn resolve_provider_only_uses_capability_path() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.anth]
provider = "anthropic"
model = "claude-sonnet-4"
"#,
        );
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let pref = LlmPreference {
            provider: Some("anthropic".into()),
            ..Default::default()
        };
        let res = pool.resolve(&pref);
        assert_eq!(res.match_quality, MatchQuality::Capability);
        assert_eq!(res.info.provider, "anthropic");
    }

    #[test]
    fn pool_construction_fails_fast_when_builder_errors() {
        let config = cfg_from_toml(
            r#"
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"

[llm.broken]
provider = "groq"
model = "x"
"#,
        );
        let result = LlmClientPool::from_config(&config, |cfg| {
            if cfg.model == "x" {
                Err(anyhow!("simulated failure"))
            } else {
                Ok(Arc::new(MockLlmClient::new(vec![])))
            }
        });
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("[llm.broken]"), "error should name the bad entry: {msg}");
    }

    fn build_two_profile_pool() -> LlmClientPool {
        // Two profiles, distinct concrete models, distinct clients.
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "heavy-model"

[llm.cheap]
provider = "groq"
model = "lightweight-model"

[routing.hints]
lightweight = "cheap"
medium = "default"
heavy = "default"
"#,
        )
        .unwrap();
        LlmClientPool::from_config(&config, mock_builder).unwrap()
    }

    #[test]
    fn resolve_hint_uses_configured_profile_for_lightweight() {
        let pool = build_two_profile_pool();
        let (_, model) = pool.resolve_hint("hint:lightweight");
        assert_eq!(model, "lightweight-model");
    }

    #[test]
    fn resolve_hint_uses_configured_profile_for_medium() {
        let pool = build_two_profile_pool();
        let (_, model) = pool.resolve_hint("hint:medium");
        assert_eq!(model, "heavy-model"); // medium → default in this config
    }

    #[test]
    fn resolve_hint_falls_back_to_engine_when_unconfigured() {
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "only-model"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let (_, model) = pool.resolve_hint("hint:lightweight");
        assert_eq!(model, "only-model");
    }

    #[test]
    fn resolve_hint_unknown_hint_falls_back_to_engine() {
        let pool = build_two_profile_pool();
        let (_, model) = pool.resolve_hint("hint:reaction");
        assert_eq!(model, "heavy-model");
    }

    #[test]
    fn resolve_hint_concrete_model_passes_through() {
        let pool = build_two_profile_pool();
        let (_, model) = pool.resolve_hint("custom-fine-tune");
        assert_eq!(model, "custom-fine-tune");
    }

    #[test]
    fn resolve_hint_with_missing_profile_falls_back_silently() {
        // [routing.hints] names a profile that doesn't exist in [llm.*].
        // The resolver should drop the entry at construction and the
        // hint should fall through to engine.
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "engine-model"

[routing.hints]
medium = "nonexistent"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let (_, model) = pool.resolve_hint("hint:medium");
        assert_eq!(model, "engine-model");
    }

    #[test]
    fn routing_provider_is_none_when_unconfigured() {
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "engine-model"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert!(pool.routing_provider(RoutingHints::default()).is_none());
    }

    #[test]
    fn routing_provider_resolves_local_and_remote_from_config() {
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "cloud-model"

[llm.local]
provider = "ollama"
model = "ollama-model"

[routing.providers]
local = "local"
remote = "default"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        let provider = pool.routing_provider(RoutingHints::default());
        assert!(provider.is_some(), "routing provider should be wired");
        // Health is configured (local profile exists) → starts Healthy.
        assert!(matches!(
            pool.local_health().snapshot(),
            arawn_llm::routing::LocalHealth::Healthy
        ));
    }

    #[test]
    fn routing_provider_local_only_still_works_with_remote_engine_fallback() {
        // No explicit remote → remote falls back to engine profile.
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "engine-model"

[llm.local]
provider = "ollama"
model = "ollama-model"

[routing.providers]
local = "local"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert!(pool.routing_provider(RoutingHints::default()).is_some());
    }

    #[test]
    fn missing_local_profile_disables_routing_local_path() {
        // Names a local that doesn't exist — health stays NotConfigured.
        let config: ArawnConfig = toml::from_str(
            r#"
[llm.default]
provider = "groq"
model = "engine-model"

[routing.providers]
local = "does-not-exist"
remote = "default"
"#,
        )
        .unwrap();
        let pool = LlmClientPool::from_config(&config, mock_builder).unwrap();
        assert!(matches!(
            pool.local_health().snapshot(),
            arawn_llm::routing::LocalHealth::NotConfigured
        ));
    }
}

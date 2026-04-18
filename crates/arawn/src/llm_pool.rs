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
use arawn_llm::LlmClient;
use arawn_tool::{LlmPreference, LlmResolution, LlmResolver, MatchQuality};

use crate::config::{ArawnConfig, LlmConfig};

/// A pool of named LLM clients built from an [`ArawnConfig`].
pub struct LlmClientPool {
    clients: HashMap<String, Arc<dyn LlmClient>>,
    configs: HashMap<String, LlmConfig>,
    engine_name: String,
    compactor_name: String,
}

impl LlmResolver for LlmClientPool {
    fn resolve(&self, preference: &LlmPreference) -> LlmResolution {
        LlmClientPool::resolve(self, preference)
    }
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
            let wrapped: Arc<dyn LlmClient> =
                Arc::new(arawn_llm::RetryClient::new(raw));
            clients.insert(name.clone(), wrapped);
            configs.insert(name.clone(), llm_config.clone());
        }

        // Resolve role names with the same fallback logic as `engine_llm` /
        // `compactor_llm` on `ArawnConfig`. Both methods are guaranteed to
        // return an entry that exists in the map.
        let engine_name = resolve_engine_name(config, &clients)?;
        let compactor_name = resolve_compactor_name(config, &engine_name);

        Ok(Self { clients, configs, engine_name, compactor_name })
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
        Self { clients, configs, engine_name, compactor_name }
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

    /// Iterator over (name, config) pairs.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &LlmConfig)> {
        self.configs.iter()
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
}

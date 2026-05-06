use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// A named LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    /// Direct API key value. Plaintext — keep this file out of version
    /// control. Takes precedence over `api_key_env` when set.
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    /// Override the default API base URL for this provider.
    /// If not set, uses the provider's default (e.g., Groq → api.groq.com).
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_context_window")]
    pub context_window: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Whether this model supports tool/function calling. Defaults to true —
    /// every modern model supported by arawn does.
    #[serde(default = "default_tool_use")]
    pub tool_use: bool,
    /// Whether this model supports vision/image input. Defaults to false —
    /// must be opted into per entry.
    #[serde(default)]
    pub vision: bool,
}

fn default_api_key_env() -> String {
    "GROQ_API_KEY".into()
}
fn default_context_window() -> u32 {
    128_000
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_tool_use() -> bool {
    true
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "groq".into(),
            model: "openai/gpt-oss-20b".into(),
            api_key: None,
            api_key_env: default_api_key_env(),
            base_url: None,
            context_window: default_context_window(),
            max_tokens: default_max_tokens(),
            tool_use: true,
            vision: false,
        }
    }
}

impl LlmConfig {
    /// Project this config into the capability metadata used by
    /// `LlmPreference` resolution.
    pub fn to_resolved_info(&self) -> arawn_tool::ResolvedLlmInfo {
        arawn_tool::ResolvedLlmInfo {
            provider: self.provider.clone(),
            model: self.model.clone(),
            context_window: self.context_window,
            tool_use: self.tool_use,
            vision: self.vision,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default = "default_engine_llm")]
    pub llm: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_max_result_size")]
    pub max_result_size: usize,
}

fn default_engine_llm() -> String {
    "default".into()
}
fn default_max_iterations() -> usize {
    20
}
fn default_max_result_size() -> usize {
    50_000
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            llm: default_engine_llm(),
            max_iterations: default_max_iterations(),
            max_result_size: default_max_result_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactorConfig {
    /// LLM name — if None/empty, falls back to engine's LLM.
    #[serde(default)]
    pub llm: Option<String>,
    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: f32,
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}

fn default_compaction_threshold() -> f32 {
    0.85
}
fn default_keep_recent() -> usize {
    6
}

impl Default for CompactorConfig {
    fn default() -> Self {
        Self {
            llm: None,
            compaction_threshold: default_compaction_threshold(),
            keep_recent: default_keep_recent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    3100
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_data_dir() -> String {
    "~/.arawn".into()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    #[serde(default = "default_prompt_token_budget")]
    pub token_budget: u32,
}

fn default_prompt_token_budget() -> u32 {
    6000
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            token_budget: default_prompt_token_budget(),
        }
    }
}

/// Sandbox configuration for shell command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Tools that are granted network access when detected in a shell command.
    /// If the command invokes any of these binaries, network restrictions are
    /// lifted for that execution.
    #[serde(default = "default_network_tools")]
    pub network_tools: Vec<String>,
}

fn default_network_tools() -> Vec<String> {
    [
        "gh",
        "kubectl",
        "gcloud",
        "aws",
        "az",
        "npm",
        "npx",
        "yarn",
        "pnpm",
        "cargo",
        "rustup",
        "pip",
        "pip3",
        "poetry",
        "uv",
        "gem",
        "bundle",
        "go",
        "docker",
        "podman",
        "terraform",
        "helm",
        "curl",
        "wget",
        "fetch",
        "git",
        "ssh",
        "scp",
        "rsync",
        "brew",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            network_tools: default_network_tools(),
        }
    }
}

/// OAuth client credentials for one integration. Stored in plaintext —
/// keep `arawn.toml` out of version control. Env vars
/// (`ARAWN_<SERVICE>_CLIENT_ID` / `_SECRET`) override these at startup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationCredentials {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

/// Per-integration credential blocks. Each is optional — leaving any of
/// them out (or omitting the whole `[integrations]` section) just means
/// that integration is configured via env vars, or skipped if neither
/// is set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationsConfig {
    /// Slack OAuth app credentials. One Slack app, multiple workspace
    /// installs share these.
    #[serde(default)]
    pub slack: IntegrationCredentials,
    /// Shared Google OAuth client used by both Gmail and Calendar
    /// when service-specific credentials aren't set. Most users have
    /// one Google Cloud project per arawn install.
    #[serde(default)]
    pub google: IntegrationCredentials,
    /// Gmail-specific OAuth client. Falls back to `google` when empty.
    #[serde(default)]
    pub gmail: IntegrationCredentials,
    /// Calendar-specific OAuth client. Falls back to `google` when empty.
    #[serde(default)]
    pub calendar: IntegrationCredentials,
    /// Drive-specific OAuth client. Falls back to `google` when empty.
    #[serde(default)]
    pub drive: IntegrationCredentials,
    /// Atlassian (Jira + Confluence) OAuth client. One Atlassian Cloud
    /// app covers both products.
    #[serde(default)]
    pub atlassian: IntegrationCredentials,
}

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArawnConfig {
    #[serde(default = "default_llm_configs")]
    pub llm: HashMap<String, LlmConfig>,
    #[serde(default)]
    pub engine: EngineConfig,
    #[serde(default)]
    pub compactor: CompactorConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub integrations: IntegrationsConfig,
}

fn default_llm_configs() -> HashMap<String, LlmConfig> {
    let mut map = HashMap::new();
    map.insert("default".into(), LlmConfig::default());
    map
}

impl Default for ArawnConfig {
    fn default() -> Self {
        Self {
            llm: default_llm_configs(),
            engine: EngineConfig::default(),
            compactor: CompactorConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            prompts: PromptsConfig::default(),
            sandbox: SandboxConfig::default(),
            integrations: IntegrationsConfig::default(),
        }
    }
}

impl ArawnConfig {
    /// Load config from `data_dir/arawn.toml`, merging with env var overrides and defaults.
    pub fn load(data_dir: &Path) -> Self {
        let config_path = data_dir.join("arawn.toml");
        let mut config = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str::<ArawnConfig>(&content) {
                    Ok(c) => {
                        info!(path = %config_path.display(), "loaded config");
                        c
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to parse arawn.toml, using defaults");
                        Self::default()
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read arawn.toml, using defaults");
                    Self::default()
                }
            }
        } else {
            debug!("no arawn.toml found, using defaults");
            Self::default()
        };

        // Ensure "default" LLM exists
        if !config.llm.contains_key("default") {
            config.llm.insert("default".into(), LlmConfig::default());
        }

        // Apply env var overrides
        config.apply_env_overrides();

        config
    }

    fn apply_env_overrides(&mut self) {
        // GROQ_MODEL overrides the engine's LLM model
        if let Ok(model) = std::env::var("GROQ_MODEL")
            && !model.is_empty()
        {
            let llm_name = self.engine.llm.clone();
            if let Some(llm) = self.llm.get_mut(&llm_name) {
                debug!(model = %model, "GROQ_MODEL overriding engine LLM model");
                llm.model = model;
            }
        }

        // ARAWN_DATA_DIR overrides storage
        if let Ok(dir) = std::env::var("ARAWN_DATA_DIR")
            && !dir.is_empty()
        {
            self.storage.data_dir = dir;
        }
    }

    /// Resolve the LLM config for the engine.
    pub fn engine_llm(&self) -> &LlmConfig {
        self.llm
            .get(&self.engine.llm)
            .or_else(|| self.llm.get("default"))
            .expect("no LLM config found — at least 'default' should exist")
    }

    /// Resolve the LLM config for the compactor. Falls back to engine's LLM.
    pub fn compactor_llm(&self) -> &LlmConfig {
        if let Some(ref name) = self.compactor.llm
            && let Some(llm) = self.llm.get(name)
        {
            return llm;
        }
        self.engine_llm()
    }

    /// Resolve the data directory with ~ expansion.
    pub fn data_dir(&self) -> PathBuf {
        expand_tilde(&self.storage.data_dir)
    }

    /// Resolve the prompts directory.
    pub fn prompts_dir(&self) -> PathBuf {
        self.data_dir().join("prompts")
    }

    /// Resolve API key for an LLM config. Order: explicit `api_key` field
    /// (plaintext in arawn.toml) → env var named by `api_key_env`.
    pub fn resolve_api_key(llm: &LlmConfig) -> Option<String> {
        if let Some(key) = llm.api_key.as_ref().filter(|s| !s.is_empty()) {
            return Some(key.clone());
        }
        std::env::var(&llm.api_key_env)
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// Generate a default config file string with comments.
    pub fn generate_default_toml() -> String {
        r##"# Arawn Configuration
# Edit this file to customize Arawn's behavior.
# Env vars override values here: GROQ_API_KEY, GROQ_MODEL, ARAWN_DATA_DIR
#
# Multi-model setup
# -----------------
# `[llm.*]` entries define every model you have access to. Other sections
# (`[engine]`, `[compactor]`, future per-tool slots) reference them by name.
# At startup, arawn builds an `LlmClientPool` containing one client per
# `[llm.*]` entry; misconfigured entries fail fast.
#
# Tools and sub-agents can request a specific model via `LlmPreference`. The
# pool resolves preferences in this order:
#   1. Named match — preference.named is in the pool        → MatchQuality::Exact
#   2. Provider+model match — exact pair found              → MatchQuality::Exact
#   3. Capability match — first entry meeting bounds        → MatchQuality::Capability
#   4. Fallback — engine LLM (always succeeds)              → MatchQuality::Fallback
#
# Tools can inspect MatchQuality and degrade gracefully (e.g., skip an
# expensive summarization step when only `Fallback` is available).

# Named LLM configurations — define models you have access to
[llm.default]
provider = "groq"
model = "openai/gpt-oss-20b"
api_key_env = "GROQ_API_KEY"
context_window = 128000
max_tokens = 4096
tool_use = true   # supports tool/function calling (default: true)
vision = false    # supports image input (default: false)

# Example: add a cheaper model used for context compaction
# [llm.cheap]
# provider = "groq"
# model = "llama-3.3-70b-versatile"
# api_key_env = "GROQ_API_KEY"
# context_window = 128000
# max_tokens = 4096

# Example: add a strong model reserved for judging / evals
# [llm.judge]
# provider = "anthropic"
# model = "claude-sonnet-4-20250514"
# api_key_env = "ANTHROPIC_API_KEY"
# context_window = 200000
# max_tokens = 8192

# Engine uses a named LLM config
[engine]
llm = "default"
max_iterations = 20
max_result_size = 50000

# Compactor can use a different (cheaper) model. Set `llm` to any [llm.*] name.
# When unset (or pointing at a missing entry), falls back to the engine's LLM.
[compactor]
# llm = "cheap"  # uncomment to route compaction to the cheap model above
compaction_threshold = 0.85
keep_recent = 6

[server]
host = "127.0.0.1"
port = 3100

[storage]
data_dir = "~/.arawn"

[prompts]
token_budget = 6000

# Shell sandbox — tools granted network access when detected in a command.
# All other shell commands run with no network access.

[sandbox]
network_tools = [
    "gh", "kubectl", "gcloud", "aws", "az",
    "npm", "npx", "yarn", "pnpm",
    "cargo", "rustup",
    "pip", "pip3", "poetry", "uv",
    "gem", "bundle",
    "go",
    "docker", "podman",
    "terraform", "helm",
    "curl", "wget", "fetch",
    "git",
    "ssh", "scp", "rsync",
    "brew",
]
"##
        .to_string()
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_working_values() {
        let config = ArawnConfig::default();
        let engine_llm = config.engine_llm();
        assert_eq!(engine_llm.provider, "groq");
        assert_eq!(engine_llm.model, "openai/gpt-oss-20b");
        assert_eq!(engine_llm.context_window, 128_000);
        assert_eq!(engine_llm.max_tokens, 4096);
        assert_eq!(config.engine.max_iterations, 20);
        assert_eq!(config.server.port, 3100);
    }

    #[test]
    fn load_from_toml_string() {
        let toml = r#"
[llm.fast]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
context_window = 128000
max_tokens = 2048

[engine]
llm = "fast"
max_iterations = 10
"#;
        let config: ArawnConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.engine.llm, "fast");
        assert_eq!(config.engine.max_iterations, 10);

        let llm = config.engine_llm();
        assert_eq!(llm.model, "llama-3.3-70b-versatile");
        assert_eq!(llm.max_tokens, 2048);
    }

    #[test]
    fn compactor_falls_back_to_engine_llm() {
        let config = ArawnConfig::default();
        let compactor_llm = config.compactor_llm();
        let engine_llm = config.engine_llm();
        assert_eq!(compactor_llm.model, engine_llm.model);
    }

    #[test]
    fn compactor_uses_own_llm_when_specified() {
        let toml = r#"
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
"#;
        let config: ArawnConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.engine_llm().model, "openai/gpt-oss-20b");
        assert_eq!(config.compactor_llm().model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn missing_llm_name_falls_back_to_default_via_load() {
        // When loaded via load(), "default" is always ensured
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("arawn.toml"),
            r#"
[engine]
llm = "nonexistent"
"#,
        )
        .unwrap();

        let config = ArawnConfig::load(tmp.path());
        // engine_llm() falls back to "default" when "nonexistent" not found
        let llm = config.engine_llm();
        assert_eq!(llm.model, "openai/gpt-oss-20b"); // got the default
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let config = ArawnConfig::load(Path::new("/nonexistent/path"));
        assert_eq!(config.engine_llm().model, "openai/gpt-oss-20b");
        assert_eq!(config.server.port, 3100);
    }

    #[test]
    fn load_from_tempdir() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("arawn.toml"),
            r#"
[llm.default]
provider = "groq"
model = "custom-model"

[server]
port = 9999
"#,
        )
        .unwrap();

        let config = ArawnConfig::load(tmp.path());
        assert_eq!(config.engine_llm().model, "custom-model");
        assert_eq!(config.server.port, 9999);
    }

    #[test]
    fn generate_default_toml_is_parseable() {
        let toml_str = ArawnConfig::generate_default_toml();
        let parsed: ArawnConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.engine_llm().model, "openai/gpt-oss-20b");
    }

    #[test]
    fn tilde_expansion() {
        let expanded = expand_tilde("~/.arawn");
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }
}

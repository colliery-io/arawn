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
    /// Override the model-warmup TTL (seconds) for this profile. When unset,
    /// the TTL is chosen per-provider (short for cold-capable providers like
    /// Ollama, effectively-off for hosted providers like Groq/OpenAI). Set
    /// this only to override that default for a specific profile.
    #[serde(default)]
    pub warmup_ttl_secs: Option<u64>,
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

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "groq".into(),
            model: "openai/gpt-oss-120b".into(),
            api_key: None,
            api_key_env: default_api_key_env(),
            base_url: None,
            context_window: default_context_window(),
            max_tokens: default_max_tokens(),
            warmup_ttl_secs: None,
        }
    }
}

impl LlmConfig {
    /// Project this config into the metadata used by `LlmPreference`
    /// resolution (provider + model name for logs/display).
    pub fn to_resolved_info(&self) -> arawn_tool::ResolvedLlmInfo {
        arawn_tool::ResolvedLlmInfo {
            provider: self.provider.clone(),
            model: self.model.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default = "default_engine_llm")]
    pub llm: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// No-progress breaker (ARAWN-T-0475): end a turn after this many
    /// consecutive iterations whose tool calls all errored. 0 disables it
    /// (the `max_iterations` cap is the ultimate backstop).
    #[serde(default = "default_max_no_progress_iterations")]
    pub max_no_progress_iterations: usize,
    #[serde(default = "default_max_result_size")]
    pub max_result_size: usize,
    /// Default wall-clock timeout for individual tool calls, in seconds.
    /// Overridable per call by the agent via the `timeout_secs` tool argument,
    /// and process-wide by the `ARAWN_TOOL_TIMEOUT_SECS` env var (env wins).
    /// When unset or zero, falls back to 120s.
    #[serde(default)]
    pub tool_timeout_secs: Option<u64>,
}

fn default_engine_llm() -> String {
    "default".into()
}
fn default_max_iterations() -> usize {
    20
}
fn default_max_no_progress_iterations() -> usize {
    5
}
fn default_max_result_size() -> usize {
    50_000
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            llm: default_engine_llm(),
            max_iterations: default_max_iterations(),
            max_no_progress_iterations: default_max_no_progress_iterations(),
            max_result_size: default_max_result_size(),
            tool_timeout_secs: None,
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

/// Configuration for the per-lens extractor (I-0040 phase 4).
///
/// `llm` names an entry in the `[llm.<name>]` map. If `None` or empty,
/// extraction falls through to the engine's LLM. This lets users wire
/// a free / inexpensive model for extraction without affecting the
/// main interaction LLM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// LLM name — if None/empty, falls back to engine's LLM.
    #[serde(default)]
    pub llm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Extra browser origins allowed to connect (WS `Origin` + CORS), beyond
    /// the loopback defaults derived from `host:port`. Empty by default —
    /// localhost-first. Add an entry (e.g. `"http://192.168.1.10:3100"`) only
    /// when serving the GUI to another device. See ARAWN-T-0492 / GUI-G2.
    #[serde(default)]
    pub allowed_origins: Vec<String>,
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
            allowed_origins: Vec::new(),
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

/// GitHub App credentials. Unlike OAuth-app integrations, GitHub uses
/// the App model: numeric app_id + RSA private key (PEM) + URL-slug.
/// Env overrides: `ARAWN_GITHUB_APP_ID`, `ARAWN_GITHUB_APP_SLUG`,
/// `ARAWN_GITHUB_PRIVATE_KEY_PATH` (preferred) or
/// `ARAWN_GITHUB_PRIVATE_KEY_PEM` (inline).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GithubAppCredentials {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_slug: String,
    /// Path to the App's RSA private key in PEM format. Read lazily
    /// at startup; the key body never lands in arawn.toml verbatim.
    #[serde(default)]
    pub private_key_path: String,
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
    /// GitHub App credentials (I-0045). Different shape from OAuth-app
    /// integrations — see [`GithubAppCredentials`].
    #[serde(default)]
    pub github: GithubAppCredentials,
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
    pub extraction: ExtractionConfig,
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
    #[serde(default)]
    pub routing: RoutingConfig,
    /// Per-ceremony override map. Absent table = every ceremony uses
    /// its `Ceremony::default_schedule()` and a `hint:medium` model.
    /// Keyed by ceremony `kind` (e.g. `"retro"`, `"daily"`).
    #[serde(default)]
    pub ceremonies: HashMap<String, CeremonyConfig>,
    /// Boot-time back-fill for ceremonies whose cron tick fired
    /// while arawn was offline. See `[backfill]` table in the
    /// configuration reference. Absent = the documented defaults
    /// (14-day cap, daily + weekly only).
    #[serde(default)]
    pub backfill: BackfillConfig,
    /// Declared lenses (`[[lenses]]`, ARAWN-T-0506). Reconciled at
    /// startup and after each connect: missing ones are created, nothing
    /// is deleted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lenses: Vec<LensDecl>,
    /// Declared feeds (`[[feeds]]`, ARAWN-T-0506). A feed whose
    /// integration is not connected yet waits until it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<FeedDecl>,
}

/// One `[[lenses]]` entry: a lens that should exist, with its starting
/// tag ontology and the feeds bound to it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LensDecl {
    /// Slug: lowercase letters, digits, `-` and `_`.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// What the lens tracks. Shapes extraction.
    pub description: String,
    /// Tag ontology. Required (non-empty) to create the lens (ADR-0004).
    /// Tags listed here are added if missing; tags added later at
    /// runtime are kept.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Feed ids bound to this lens.
    #[serde(default)]
    pub feeds: Vec<String>,
}

/// One `[[feeds]]` entry: a feed that should exist.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FeedDecl {
    /// Unique feed id, e.g. `gmail-inbox`.
    pub id: String,
    /// Feed template, e.g. `gmail/inbox-archive`.
    pub template: String,
    /// Template parameters. See the feed templates reference.
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    pub params: toml::Table,
    /// Cron cadence. Absent = the template's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cadence: Option<String>,
}

/// `[backfill]` table — boot-time ceremony recovery knobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillConfig {
    /// How far back to walk when looking for missed daily/weekly
    /// ceremonies on boot. `0` disables back-fill entirely.
    /// Default: 14 days — see ARAWN-I-0052 ("Why the 14-day cap")
    /// for the UX rationale.
    #[serde(default = "default_backfill_lookback")]
    pub ceremony_lookback_days: u32,
}

fn default_backfill_lookback() -> u32 {
    14
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            ceremony_lookback_days: default_backfill_lookback(),
        }
    }
}

/// One ceremony's runtime overrides. Every field is optional; an
/// empty `[ceremonies.<kind>]` table is equivalent to no table at
/// all (regression-safety).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CeremonyConfig {
    /// When `Some(false)`, the binary skips wiring this ceremony
    /// entirely — no plugin registered, no cron, no RPC routes, no
    /// agent tools. Defaults to enabled.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Cron expression overriding the plugin's
    /// `default_schedule()`. Invalid expressions log a warn and
    /// fall back to the default — startup does not abort.
    #[serde(default)]
    pub schedule: Option<String>,
    /// Timezone for the cron expression. `"local"` (default) or an
    /// IANA zone like `"America/Los_Angeles"`.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Model string for compose calls — hint shortcut
    /// (`"hint:medium"`, etc.) or a concrete model name resolved
    /// through `LlmClientPool::resolve_hint`.
    #[serde(default)]
    pub model: Option<String>,
    /// Cadence hint for retro: `"weekly"` (default), `"biweekly"`,
    /// or `"monthly"`. Only consulted by the retro plugin. Runtime
    /// overrides written by the `retro_set_cadence` agent tool win
    /// over this default.
    #[serde(default)]
    pub cadence: Option<String>,
}

impl CeremonyConfig {
    /// `enabled` field defaulting to `true`.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

/// Routing configuration. Holds the hint→profile map (T-0272) and
/// the local/remote provider names that drive the health-aware
/// Maps `ModelHint` tiers (lightweight/medium/heavy) to named `[llm.NAME]`
/// profiles. Pure config-driven role-to-model assignment — no runtime
/// dispatch policy. Add fields here if new hint tiers become useful.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default)]
    pub hints: HintRoutingConfig,
}

/// Maps each `ModelHint` tier to a named `[llm.NAME]` profile. Any field
/// left `None` falls back to the engine LLM, so the minimal config
/// (no `[routing.hints]` section at all) keeps current behaviour.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HintRoutingConfig {
    /// `[llm.NAME]` to use for `hint:lightweight`. Falls back to engine.
    #[serde(default)]
    pub lightweight: Option<String>,
    /// `[llm.NAME]` to use for `hint:medium`. Falls back to engine.
    #[serde(default)]
    pub medium: Option<String>,
    /// `[llm.NAME]` to use for `hint:heavy`. Falls back to engine.
    #[serde(default)]
    pub heavy: Option<String>,
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
            extraction: ExtractionConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            prompts: PromptsConfig::default(),
            sandbox: SandboxConfig::default(),
            integrations: IntegrationsConfig::default(),
            routing: RoutingConfig::default(),
            ceremonies: HashMap::new(),
            backfill: BackfillConfig::default(),
            lenses: Vec::new(),
            feeds: Vec::new(),
        }
    }
}

impl ArawnConfig {
    /// Load config from `data_dir/arawn.toml`, merging with env var overrides
    /// and defaults. **Fail-fast**: aborts the process on a file that exists
    /// but can't be read/parsed — at startup, silently using the Groq defaults
    /// is how "why is it hitting groq?" happens. For the hot-reload path (where
    /// exiting would kill a running daemon on a typo) use [`try_load`] instead.
    ///
    /// [`try_load`]: ArawnConfig::try_load
    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(config) => config,
            Err(e) => {
                // NOTE: this runs before tracing is initialized (see main.rs),
                // so the message MUST go to stderr to be visible.
                eprintln!(
                    "FATAL: {e}\n\
                     Refusing to start with the built-in defaults (which use the \
                     Groq provider). Fix the error above, or remove/rename the \
                     file to intentionally use defaults."
                );
                std::process::exit(1);
            }
        }
    }

    /// Like [`load`](ArawnConfig::load) but returns a user-facing error
    /// instead of exiting the process. The hot-reload path uses this so an
    /// invalid edit surfaces as a toast and leaves the running config intact,
    /// rather than crashing the daemon (or silently reverting to defaults).
    pub fn try_load(data_dir: &Path) -> Result<Self, String> {
        let config_path = data_dir.join("arawn.toml");
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("failed to read {}: {e}", config_path.display()))?;
            let parsed = toml::from_str::<ArawnConfig>(&content)
                .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?;
            info!(path = %config_path.display(), "loaded config");
            parsed
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

        Ok(config)
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

    /// Resolve the LLM config for the per-lens extractor.
    /// Falls back to engine's LLM if `[extraction]` is absent or names
    /// a missing entry.
    pub fn extraction_llm(&self) -> &LlmConfig {
        if let Some(ref name) = self.extraction.llm
            && let Some(llm) = self.llm.get(name)
        {
            return llm;
        }
        self.engine_llm()
    }

    /// The configured name of the extraction LLM (or the engine's
    /// name when no override is set). Used by `LlmClientPool::resolve`
    /// callers that want the fall-through resolved upfront.
    pub fn extraction_llm_name(&self) -> &str {
        match &self.extraction.llm {
            Some(name) if !name.is_empty() && self.llm.contains_key(name) => name,
            _ => &self.engine.llm,
        }
    }

    /// Resolve the data directory with ~ expansion.
    pub fn data_dir(&self) -> PathBuf {
        expand_tilde(&self.storage.data_dir)
    }

    /// Apply the `--data-dir` flag: a non-empty value replaces
    /// `[storage].data_dir` (ARAWN-T-0509).
    pub fn override_data_dir(&mut self, flag: Option<&str>) {
        if let Some(dir) = flag.filter(|d| !d.is_empty()) {
            self.storage.data_dir = dir.to_string();
        }
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
model = "openai/gpt-oss-120b"
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

# Ceremony overrides
# ------------------
# Each `[ceremonies.<kind>]` table overrides the compiled-in defaults for
# one ceremony plugin. All fields are optional; an absent table means the
# plugin uses its `default_schedule()` and `hint:medium` as the compose
# model. Config changes require a restart — hot-reload is not supported.
#
# [ceremonies.retro]
# enabled = true                          # set false to disable retro entirely
# schedule = "0 16 * * FRI"               # cron expression (5 fields)
# timezone = "Local"                      # "Local" or an IANA zone (e.g. "America/Los_Angeles")
# model = "hint:medium"                   # hint shortcut or concrete model name from [llm.*]
#
# [ceremonies.weekly]
# enabled = true                          # set false to disable weekly entirely
# schedule = "0 9 * * MON"                # cron expression (5 fields)
# timezone = "Local"                      # "Local" or an IANA zone (e.g. "America/Los_Angeles")
# model = "hint:medium"                   # hint shortcut or concrete model name from [llm.*]
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
    fn try_load_returns_err_on_invalid_toml_instead_of_exiting() {
        // Regression guard (T-0489): the hot-reload path must NOT crash the
        // daemon on a typo. try_load returns Err; the watcher keeps the old
        // config and surfaces the message.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("arawn.toml"), "this is = not valid = toml").unwrap();
        let err = ArawnConfig::try_load(dir.path()).expect_err("invalid toml must error");
        assert!(err.contains("failed to parse"), "got: {err}");
    }

    #[test]
    fn try_load_uses_defaults_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let config = ArawnConfig::try_load(dir.path()).expect("absent file → defaults");
        assert!(config.llm.contains_key("default"));
    }

    #[test]
    fn try_load_parses_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arawn.toml"),
            "[engine]\nmax_iterations = 99\n",
        )
        .unwrap();
        let config = ArawnConfig::try_load(dir.path()).expect("valid toml");
        assert_eq!(config.engine.max_iterations, 99);
    }

    #[test]
    fn first_chat_tutorial_matches_default_model() {
        // Anti-drift pin (ARAWN-T-0472): the first-chat tutorial's example
        // config must show the same model the code defaults to. Also fails if
        // the tutorial file is missing — fencing the README link target.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/src/tutorials/first-chat.md"
        );
        let doc = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("first-chat.md must exist at {path}: {e}"));
        let default_model = LlmConfig::default().model;
        assert!(
            doc.contains(&default_model),
            "first-chat.md doesn't mention the default model '{default_model}' — \
             update the tutorial when changing the default"
        );
    }

    #[test]
    fn data_dir_flag_overrides_storage_data_dir() {
        // ARAWN-T-0509: config read from --data-dir must also store there.
        let mut cfg: ArawnConfig = toml::from_str("").unwrap();
        assert!(cfg.data_dir().ends_with(".arawn"));
        cfg.override_data_dir(Some("/tmp/work-arawn"));
        assert_eq!(cfg.data_dir(), PathBuf::from("/tmp/work-arawn"));
        // No flag, or an empty one, keeps the configured value.
        let mut cfg: ArawnConfig =
            toml::from_str("[storage]\ndata_dir = \"/srv/arawn\"\n").unwrap();
        cfg.override_data_dir(None);
        cfg.override_data_dir(Some(""));
        assert_eq!(cfg.data_dir(), PathBuf::from("/srv/arawn"));
    }

    #[test]
    fn default_config_has_working_values() {
        let config = ArawnConfig::default();
        let engine_llm = config.engine_llm();
        assert_eq!(engine_llm.provider, "groq");
        assert_eq!(engine_llm.model, "openai/gpt-oss-120b");
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
model = "openai/gpt-oss-120b"

[llm.cheap]
provider = "groq"
model = "llama-3.3-70b-versatile"

[engine]
llm = "default"

[compactor]
llm = "cheap"
"#;
        let config: ArawnConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.engine_llm().model, "openai/gpt-oss-120b");
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
        assert_eq!(llm.model, "openai/gpt-oss-120b"); // got the default
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let config = ArawnConfig::load(Path::new("/nonexistent/path"));
        assert_eq!(config.engine_llm().model, "openai/gpt-oss-120b");
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
        assert_eq!(parsed.engine_llm().model, "openai/gpt-oss-120b");
    }

    #[test]
    fn tilde_expansion() {
        let expanded = expand_tilde("~/.arawn");
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn empty_config_has_no_ceremony_overrides() {
        // Regression-safety: a config file with no `[ceremonies]`
        // section parses to an empty map. The binary treats this
        // as "every ceremony uses its compiled-in defaults".
        let cfg: ArawnConfig = toml::from_str("").unwrap();
        assert!(cfg.ceremonies.is_empty());
    }

    #[test]
    fn ceremonies_table_parses_full_block() {
        let s = r#"
[ceremonies.retro]
enabled = true
schedule = "0 17 * * FRI"
timezone = "America/Los_Angeles"
model = "hint:heavy"
"#;
        let cfg: ArawnConfig = toml::from_str(s).unwrap();
        let retro = cfg.ceremonies.get("retro").expect("retro section");
        assert_eq!(retro.enabled, Some(true));
        assert_eq!(retro.schedule.as_deref(), Some("0 17 * * FRI"));
        assert_eq!(retro.timezone.as_deref(), Some("America/Los_Angeles"));
        assert_eq!(retro.model.as_deref(), Some("hint:heavy"));
        assert!(retro.is_enabled());
    }

    #[test]
    fn ceremonies_disabled_observed() {
        let s = r#"
[ceremonies.retro]
enabled = false
"#;
        let cfg: ArawnConfig = toml::from_str(s).unwrap();
        let retro = cfg.ceremonies.get("retro").expect("retro section");
        assert_eq!(retro.enabled, Some(false));
        assert!(!retro.is_enabled());
        // Unset fields stay None — the override path skips them.
        assert!(retro.schedule.is_none());
        assert!(retro.model.is_none());
    }

    #[test]
    fn ceremonies_partial_block_keeps_other_fields_none() {
        let s = r#"
[ceremonies.daily]
schedule = "0 7 * * MON-FRI"
"#;
        let cfg: ArawnConfig = toml::from_str(s).unwrap();
        let daily = cfg.ceremonies.get("daily").expect("daily section");
        assert_eq!(daily.schedule.as_deref(), Some("0 7 * * MON-FRI"));
        assert!(daily.is_enabled()); // unset → defaults to enabled
        assert!(daily.model.is_none());
        assert!(daily.timezone.is_none());
    }
}

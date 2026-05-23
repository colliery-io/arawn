---
id: configuration-system-arawn-toml
level: task
title: "Configuration system — arawn.toml with sensible defaults for Groq/OpenGPT-20B"
short_code: "ARAWN-T-0050"
created_at: 2026-04-02T00:59:45.804189+00:00
updated_at: 2026-04-02T12:35:23.819700+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Configuration system — arawn.toml with sensible defaults for Groq/OpenGPT-20B

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Replace the scattered env var configuration with a proper `~/.arawn/arawn.toml` config file. Currently model, API key, and other settings are scattered across env vars with inconsistent defaults. Several things are broken because of this: compactor sends empty model string, ModelLimits gets empty string for lookup, serve mode doesn't propagate model correctly.

Sensible defaults should target **Groq + openai/gpt-oss-20b** as the out-of-box experience.

### Priority
- P0 — currently broken: empty model string causes 404 errors on compaction

### Current problems
- `GROQ_MODEL` env var not set → empty string propagated to compactor, ModelLimits
- Compactor creates ChatRequest with `model: String::new()` → Groq 404
- Model config scattered: DEFAULT_MODEL const in main.rs, env var lookups in 4 places, ModelLimits::for_model called with env var not resolved model
- No single source of truth for configuration
- Serve mode and CLI mode have duplicated config construction

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Config Structure
- [ ] Named LLM configs: `[llm.<name>]` with provider, model, api_key_env, context_window, max_tokens
- [ ] Component configs reference LLM by name: `[engine] llm = "<name>"`, `[compactor] llm = "<name>"`
- [ ] Compactor falls back to engine's LLM if not specified
- [ ] `ArawnConfig` struct with `LlmConfig`, `EngineConfig`, `CompactorConfig`, `ServerConfig`, `StorageConfig`, `PromptsConfig`, `PluginsConfig`

### Default Config
- [ ] `~/.arawn/arawn.toml` loaded on startup via `toml` crate
- [ ] Sensible defaults (no config file needed to run):
  ```toml
  [llm.default]
  provider = "groq"
  model = "openai/gpt-oss-20b"
  api_key_env = "GROQ_API_KEY"
  context_window = 128000
  max_tokens = 4096

  [engine]
  llm = "default"
  max_iterations = 20
  max_result_size = 50000

  [compactor]
  # llm = "default"  # omit to use engine's
  compaction_threshold = 0.85
  keep_recent = 6

  [server]
  host = "127.0.0.1"
  port = 3100

  [storage]
  data_dir = "~/.arawn"

  [prompts]
  token_budget = 6000

  [plugins]
  # uses data_dir/plugins/tools and data_dir/plugins/build by default
  ```

### Resolution & Overrides
- [ ] Resolution order: env var > arawn.toml > compiled default
- [ ] Env var overrides: `GROQ_API_KEY`, `GROQ_MODEL` (overrides engine LLM model), `ARAWN_DATA_DIR`
- [ ] `ArawnConfig::load(data_dir)` — read toml, merge env vars, apply defaults
- [ ] Missing config file → all compiled defaults (works out of box)
- [ ] `arawn.toml` generated with commented defaults on first run

### Bug Fixes
- [ ] Compactor uses resolved model name from config (not empty string)
- [ ] ModelLimits uses resolved model name from config
- [ ] Single config construction in main.rs, passed to all components
- [ ] Serve mode and CLI mode share same config path (no duplication)

### Tests
- [ ] Test: default config (no file) produces working values with groq-gpt defaults
- [ ] Test: load config from toml file, values parsed correctly
- [ ] Test: env var overrides toml value
- [ ] Test: missing LLM name in engine config falls back to "default"
- [ ] Test: compactor inherits engine LLM when not specified
- [ ] Test: resolve_llm_config returns correct provider/model/context_window
- [ ] Test: generate_default_toml produces valid parseable toml

## Implementation Notes

- New `config.rs` in binary crate (only binary needs full config resolution)
- `LlmConfig` struct: provider, model, api_key_env, context_window, max_tokens
- `ArawnConfig` uses `HashMap<String, LlmConfig>` for named LLM configs
- `resolve_llm_for_component(config, component_llm_name)` → returns the resolved LlmConfig
- Compactor::new gains `model: String` parameter (fix the empty string bug)
- Use `toml` crate for parsing (already transitive dep)
- `~` expansion: replace leading `~` with `dirs::home_dir()` or `$HOME`
- Generate default toml: `ArawnConfig::default().to_toml_string()` with comments

## Status Updates
- **2026-04-02**: Implementation complete.
  - `config.rs` in binary crate: ArawnConfig with named LLM configs (HashMap<String, LlmConfig>), EngineConfig, CompactorConfig, ServerConfig, StorageConfig, PromptsConfig
  - Resolution: ArawnConfig::load(data_dir) reads arawn.toml, applies env var overrides (GROQ_MODEL, ARAWN_DATA_DIR), ensures "default" LLM always exists
  - engine_llm() and compactor_llm() resolve named configs with fallback chain
  - Compactor now takes model name (impl Into<String>) — fixes the empty model 404 bug
  - build_engine_config() helper shared between serve and CLI modes — no more duplication
  - Default arawn.toml generated on first run with commented sections
  - DataLayout adds prompts/ directory
  - ~ expansion for data_dir paths
  - 8 config tests: defaults, toml parsing, fallback, compactor inheritance, env override path, generate_default parseable, tilde expansion
  - 288 total workspace tests, clippy clean
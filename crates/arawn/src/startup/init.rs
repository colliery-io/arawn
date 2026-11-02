//! `arawn init` — scaffold a minimal `arawn.toml` for first-run users
//! (ARAWN-T-0472 / P1-5, absorbs T-0194).
//!
//! Flags-first: provider/model/key-env come from CLI args with sensible
//! defaults. The model defaults to the same value the code uses when no
//! config exists (`LlmConfig::default().model`), so the scaffold can never
//! drift from the built-in default.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::config::{ArawnConfig, LlmConfig};

/// Options for `arawn init`, mapped 1:1 from the CLI flags.
pub struct InitOptions {
    pub provider: String,
    pub model: Option<String>,
    pub api_key_env: Option<String>,
    pub force: bool,
}

impl InitOptions {
    /// True when an LLM flag differs from its default. (An explicit
    /// `--provider groq` cannot be told apart from the default.)
    pub fn llm_flags_given(&self) -> bool {
        self.provider != "groq" || self.model.is_some() || self.api_key_env.is_some()
    }
}

/// Conventional env var holding a provider's API key.
fn default_key_env(provider: &str) -> &'static str {
    match provider {
        "groq" => "GROQ_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "fireworks" => "FIREWORKS_API_KEY",
        // Local providers don't need one; give a generic fallback name.
        _ => "ARAWN_API_KEY",
    }
}

/// Render the minimal `[llm.default]` + `[engine]` config for these options.
/// Serializes through the real `LlmConfig` type so it stays in lockstep with
/// the schema (no hand-built TOML string).
fn render_config(opts: &InitOptions) -> Result<(String, String)> {
    let model = opts
        .model
        .clone()
        .unwrap_or_else(|| LlmConfig::default().model);
    let api_key_env = opts
        .api_key_env
        .clone()
        .unwrap_or_else(|| default_key_env(&opts.provider).to_string());

    let llm = LlmConfig {
        provider: opts.provider.clone(),
        model,
        api_key_env: api_key_env.clone(),
        ..LlmConfig::default()
    };

    let mut root = toml::Table::new();
    let mut llm_tbl = toml::Table::new();
    llm_tbl.insert("default".into(), toml::Value::try_from(&llm)?);
    root.insert("llm".into(), toml::Value::Table(llm_tbl));

    let mut engine_tbl = toml::Table::new();
    engine_tbl.insert("llm".into(), toml::Value::String("default".into()));
    root.insert("engine".into(), toml::Value::Table(engine_tbl));

    let rendered = toml::to_string_pretty(&root)?;

    // Validate what we generated actually parses back into the config schema.
    toml::from_str::<ArawnConfig>(&rendered)
        .map_err(|e| anyhow::anyhow!("internal error: generated config did not parse: {e}"))?;

    Ok((rendered, api_key_env))
}

/// Write `arawn.toml` into `data_dir` without printing. Returns the
/// path and the env var that must hold the API key. Used by `arawn
/// setup` when no config exists yet.
pub fn write_init(data_dir: &Path, opts: &InitOptions) -> Result<(PathBuf, String)> {
    let config_path = data_dir.join("arawn.toml");
    if config_path.exists() && !opts.force {
        bail!(
            "{} already exists — pass --force to overwrite it",
            config_path.display()
        );
    }

    let (rendered, api_key_env) = render_config(opts)?;

    std::fs::create_dir_all(data_dir)?;
    std::fs::write(&config_path, &rendered)?;
    Ok((config_path, api_key_env))
}

/// Write `arawn.toml` into `data_dir`, then print next steps.
pub fn run_init(data_dir: &Path, opts: InitOptions) -> Result<()> {
    run_init_with_profile(data_dir, opts, None)
}

/// `arawn init [--profile <name>]`. With a profile and an existing
/// arawn.toml (and no `--force`), only the profile's `[[lenses]]` /
/// `[[feeds]]` are added; the rest of the file is kept (ARAWN-T-0507).
pub fn run_init_with_profile(
    data_dir: &Path,
    opts: InitOptions,
    profile_name: Option<&str>,
) -> Result<()> {
    let profile = profile_name.map(super::profile::profile).transpose()?;
    let config_path = data_dir.join("arawn.toml");
    let additive = profile.is_some() && config_path.exists() && !opts.force;
    // In additive mode the LLM settings are not written; refuse rather
    // than drop the flags without a word.
    if additive && opts.llm_flags_given() {
        bail!(
            "{} already exists, so --provider, --model and --api-key-env are not applied. \
             Give --force to rewrite the file, or leave those flags out to only add the profile",
            config_path.display()
        );
    }

    let api_key_env = if additive {
        None
    } else {
        let (_, env) = write_init(data_dir, &opts)?;
        println!("Wrote {}", config_path.display());
        Some(env)
    };

    if let (Some((lenses, feeds)), Some(name)) = (&profile, profile_name) {
        let mut doc = super::setup::edit::load_doc(&config_path)?;
        let applied = super::profile::apply(&mut doc, lenses, feeds)?;
        super::setup::edit::write_config(&config_path, &doc)?;
        if applied.is_empty() {
            println!(
                "The `{name}` profile is already in {}. Nothing was added.",
                config_path.display()
            );
        } else {
            println!(
                "Added the `{name}` profile to {}: {} lens(es), {} feed(s).",
                config_path.display(),
                applied.added_lenses.len(),
                applied.added_feeds.len()
            );
            for (lens, feeds) in &applied.bound_to_kept {
                println!("Bound to your existing `{lens}` lens: {}", feeds.join(", "));
            }
            if !applied.kept.is_empty() {
                println!(
                    "Already in the file, not changed: {}",
                    applied.kept.join(", ")
                );
            }
            println!("Edit the lens description and tags in arawn.toml to match your work.");
        }
    }

    println!();
    println!("Next steps:");
    let mut n = 1;
    if let Some(env) = api_key_env {
        println!("  {n}. export {env}=<your-api-key>");
        n += 1;
    }
    if profile.is_some() {
        println!("  {n}. arawn setup        # set up Google, Slack, Atlassian, GitHub");
        n += 1;
    }
    println!("  {n}. arawn serve        # start the server (in this terminal)");
    n += 1;
    if profile.is_some() {
        println!(
            "  {n}. arawn connect --all  # in another terminal; feeds start as each service connects"
        );
    } else {
        println!("  {n}. arawn tui          # open the chat UI (in another terminal)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(provider: &str) -> InitOptions {
        InitOptions {
            provider: provider.into(),
            model: None,
            api_key_env: None,
            force: false,
        }
    }

    #[test]
    fn rendered_config_parses_and_uses_code_default_model() {
        let (rendered, env) = render_config(&opts("groq")).unwrap();
        let cfg: ArawnConfig = toml::from_str(&rendered).unwrap();
        let def = cfg.llm.get("default").expect("default llm entry");
        // The scaffold's model must equal the built-in default — this is the
        // anti-drift guarantee.
        assert_eq!(def.model, LlmConfig::default().model);
        assert_eq!(def.provider, "groq");
        assert_eq!(env, "GROQ_API_KEY");
        assert_eq!(cfg.engine.llm, "default");
    }

    #[test]
    fn provider_picks_conventional_key_env() {
        let (_, env) = render_config(&opts("openai")).unwrap();
        assert_eq!(env, "OPENAI_API_KEY");
        let (_, env) = render_config(&opts("anthropic")).unwrap();
        assert_eq!(env, "ANTHROPIC_API_KEY");
    }

    #[test]
    fn explicit_overrides_win() {
        let o = InitOptions {
            provider: "groq".into(),
            model: Some("custom-model".into()),
            api_key_env: Some("MY_KEY".into()),
            force: false,
        };
        let (rendered, env) = render_config(&o).unwrap();
        assert_eq!(env, "MY_KEY");
        let cfg: ArawnConfig = toml::from_str(&rendered).unwrap();
        assert_eq!(cfg.llm.get("default").unwrap().model, "custom-model");
    }

    #[test]
    fn profile_on_existing_config_adds_only_the_profile() {
        let dir = tempfile::tempdir().unwrap();
        run_init(dir.path(), opts("openai")).unwrap();
        run_init_with_profile(dir.path(), opts("groq"), Some("work")).unwrap();
        let cfg: ArawnConfig =
            toml::from_str(&std::fs::read_to_string(dir.path().join("arawn.toml")).unwrap())
                .unwrap();
        // The LLM block from the first init is untouched.
        assert_eq!(cfg.llm["default"].provider, "openai");
        assert_eq!(cfg.lenses[0].name, "work");
    }

    #[test]
    fn profile_with_llm_flags_on_existing_config_refuses_instead_of_ignoring() {
        // Review regression: the flags used to be dropped without a word.
        let dir = tempfile::tempdir().unwrap();
        run_init(dir.path(), opts("groq")).unwrap();
        let err = run_init_with_profile(dir.path(), opts("openai"), Some("work")).unwrap_err();
        assert!(format!("{err}").contains("--force"), "{err}");
        // Nothing was written.
        let cfg: ArawnConfig =
            toml::from_str(&std::fs::read_to_string(dir.path().join("arawn.toml")).unwrap())
                .unwrap();
        assert!(cfg.lenses.is_empty());
    }

    #[test]
    fn refuses_to_overwrite_without_force() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("arawn.toml"), "existing").unwrap();
        let err = run_init(dir.path(), opts("groq")).unwrap_err();
        assert!(format!("{err}").contains("--force"));
    }

    #[test]
    fn writes_then_force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        run_init(dir.path(), opts("groq")).unwrap();
        assert!(dir.path().join("arawn.toml").exists());
        // Second run without force fails; with force succeeds.
        assert!(run_init(dir.path(), opts("groq")).is_err());
        let forced = InitOptions {
            force: true,
            ..opts("groq")
        };
        assert!(run_init(dir.path(), forced).is_ok());
    }
}

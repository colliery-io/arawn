//! `arawn setup` — guided integration setup (ARAWN-T-0501).
//!
//! Walks the user through registering each provider's app, then writes
//! the `[integrations.*]` blocks into `arawn.toml`. Two modes:
//!
//! - **Interactive** (`arawn setup` or `arawn setup <provider>`): prints
//!   the console steps and scopes, then prompts for the credentials.
//! - **Flags** (`arawn setup <provider> --client-id … --client-secret …`):
//!   no prompts, for scripts.
//!
//! The flow runs over a [`Prompter`] and a writer so tests can drive it
//! with scripted answers.

pub mod catalog;
pub mod edit;

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::ArawnConfig;
use crate::oauth_clients::OAuthClientResolver;
use catalog::{SetupTarget, guide};

/// Source of answers for the interactive flow.
pub trait Prompter {
    /// A line of text, trimmed. Empty when the user just presses Enter.
    fn ask(&mut self, question: &str) -> Result<String>;
    /// A line of text that is not echoed.
    fn ask_secret(&mut self, question: &str) -> Result<String>;
    /// Yes or no. Enter gives `default`.
    fn confirm(&mut self, question: &str, default: bool) -> Result<bool>;
}

/// Prompts on the terminal. Secrets are read without echo.
pub struct TerminalPrompter;

impl Prompter for TerminalPrompter {
    fn ask(&mut self, question: &str) -> Result<String> {
        print!("{question}: ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line.trim().to_string())
    }

    fn ask_secret(&mut self, question: &str) -> Result<String> {
        Ok(
            rpassword::prompt_password(format!("{question} (hidden): "))?
                .trim()
                .to_string(),
        )
    }

    fn confirm(&mut self, question: &str, default: bool) -> Result<bool> {
        let hint = if default { "Y/n" } else { "y/N" };
        loop {
            let a = self.ask(&format!("{question} [{hint}]"))?;
            match a.to_ascii_lowercase().as_str() {
                "" => return Ok(default),
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => println!("Type y or n."),
            }
        }
    }
}

/// Options for `arawn setup`, mapped 1:1 from the CLI flags.
#[derive(Debug, Default, Clone)]
pub struct SetupOptions {
    pub target: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub secret_from_env: bool,
    pub app_id: Option<String>,
    pub app_slug: Option<String>,
    pub private_key_path: Option<String>,
    /// LLM provider for the starter config when `arawn.toml` is absent.
    pub llm_provider: String,
}

impl SetupOptions {
    fn has_credential_flags(&self) -> bool {
        self.client_id.is_some()
            || self.client_secret.is_some()
            || self.secret_from_env
            || self.app_id.is_some()
            || self.app_slug.is_some()
            || self.private_key_path.is_some()
    }
}

/// The `[integrations.*]` tables in `arawn.toml` that already carry an
/// id for this target, even when the secret is missing from this shell.
fn tables_with_ids(cfg: &ArawnConfig, target: SetupTarget) -> Vec<&'static str> {
    let i = &cfg.integrations;
    let candidates: Vec<(&'static str, &str)> = match target {
        SetupTarget::Google => vec![
            ("google", &i.google.client_id),
            ("gmail", &i.gmail.client_id),
            ("calendar", &i.calendar.client_id),
            ("drive", &i.drive.client_id),
        ],
        SetupTarget::Slack => vec![("slack", &i.slack.client_id)],
        SetupTarget::Atlassian => vec![("atlassian", &i.atlassian.client_id)],
        SetupTarget::Github => vec![("github", &i.github.app_id)],
    };
    candidates
        .into_iter()
        .filter(|(_, id)| !id.is_empty())
        .map(|(k, _)| k)
        .collect()
}

/// One target's state, as the resolver sees it. A table that has an id
/// but no resolvable secret still counts as configured, so setup never
/// replaces it without asking.
fn target_status(cfg: &ArawnConfig, target: SetupTarget) -> Option<String> {
    let resolver = OAuthClientResolver::new(&cfg.integrations);
    let in_file_only = || {
        let tables = tables_with_ids(cfg, target);
        (!tables.is_empty()).then(|| {
            let names: Vec<_> = tables
                .iter()
                .map(|t| format!("[integrations.{t}]"))
                .collect();
            let missing = target
                .secret_env_var()
                .map(|v| format!("the secret, from {v}, is not set in this shell"))
                .unwrap_or_else(|| "it is incomplete".into());
            format!("in {}, but {missing}", names.join(", "))
        })
    };
    if target == SetupTarget::Github {
        return resolver
            .resolve_github()
            .map(|g| format!("configured (app ID from {})", g.app_id_origin))
            .or_else(in_file_only);
    }
    let resolved: Vec<_> = target
        .covers()
        .iter()
        .filter_map(|p| resolver.resolve(*p))
        .collect();
    let Some(first) = resolved.first() else {
        return in_file_only();
    };
    let origin = format!("client ID from {}", first.client_id_origin);
    if resolved.len() == target.covers().len() {
        Some(format!("configured ({origin})"))
    } else {
        let names: Vec<_> = resolved.iter().map(|c| c.provider.service_name()).collect();
        Some(format!(
            "partly configured: {} ({origin})",
            names.join(", ")
        ))
    }
}

fn print_guide(out: &mut dyn Write, target: SetupTarget) -> Result<()> {
    let g = guide(target);
    writeln!(out)?;
    writeln!(out, "== {} ==", target.title())?;
    writeln!(out, "Console: {}", g.console_url)?;
    writeln!(out)?;
    for (i, step) in g.steps.iter().enumerate() {
        writeln!(out, "  {}. {step}", i + 1)?;
    }
    for group in &g.scope_groups {
        writeln!(out)?;
        writeln!(out, "  {}:", group.label)?;
        for item in &group.items {
            writeln!(out, "    {item}")?;
        }
    }
    writeln!(out)?;
    writeln!(out, "Full guide: {}", g.doc)?;
    writeln!(out)?;
    Ok(())
}

/// Expand a leading `~/` and make the path absolute.
fn expand_path(raw: &str) -> PathBuf {
    let p = match raw.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(rest))
            .unwrap_or_else(|| PathBuf::from(raw)),
        None => PathBuf::from(raw),
    };
    std::path::absolute(&p).unwrap_or(p)
}

/// Check that the GitHub App key loads and can sign a JWT.
fn check_github_key(app_id: &str, app_slug: &str, path: &Path) -> Result<()> {
    let pem = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the private key {}", path.display()))?;
    arawn_integrations::github::sign_app_jwt(&arawn_integrations::github::GithubAppConfig {
        app_id: app_id.to_string(),
        app_slug: app_slug.to_string(),
        private_key_pem: pem,
    })
    .map_err(|e| {
        anyhow::anyhow!(
            "the key {} is not a usable RSA private key: {e}",
            path.display()
        )
    })?;
    Ok(())
}

/// What one target's setup wrote.
struct Applied {
    target: SetupTarget,
    /// Env var the user must export before `arawn serve`, if any.
    export_needed: Option<&'static str>,
    /// Sources that come before the written table and so still win.
    shadowed_by: Vec<String>,
}

impl Applied {
    fn report(&self, out: &mut dyn Write, path: &Path) -> Result<()> {
        writeln!(
            out,
            "Wrote [integrations.{}] to {}",
            self.target.key(),
            path.display()
        )?;
        for s in &self.shadowed_by {
            writeln!(out, "Warning: {s}")?;
        }
        Ok(())
    }
}

/// After a write, find the services that still resolve to a different
/// client: an env var or a per-service table comes first in the
/// resolution order and hides the new one.
fn shadowing(cfg: &ArawnConfig, target: SetupTarget, written_id: &str) -> Vec<String> {
    let resolver = OAuthClientResolver::new(&cfg.integrations);
    if target == SetupTarget::Github {
        return resolver
            .resolve_github()
            .filter(|g| g.app_id != written_id)
            .map(|g| {
                vec![format!(
                    "github still uses the app ID from {}. Remove it to use the new app.",
                    g.app_id_origin
                )]
            })
            .unwrap_or_default();
    }
    target
        .covers()
        .iter()
        .filter_map(|p| resolver.resolve(*p))
        .filter(|c| c.client_id != written_id)
        .map(|c| {
            format!(
                "{} still uses the client ID from {}, which comes before \
                 [integrations.{}]. Remove it to use the new client.",
                c.provider.service_name(),
                c.client_id_origin,
                target.key()
            )
        })
        .collect()
}

fn apply_oauth(
    path: &Path,
    target: SetupTarget,
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<Applied> {
    let mut doc = edit::load_doc(path)?;
    edit::set_oauth_client(&mut doc, target.key(), client_id, client_secret)?;
    let cfg = edit::write_config(path, &doc)?;
    let export_needed = match client_secret {
        Some(_) => None,
        None => target
            .secret_env_var()
            .filter(|v| std::env::var(v).map(|s| s.is_empty()).unwrap_or(true)),
    };
    Ok(Applied {
        target,
        export_needed,
        shadowed_by: shadowing(&cfg, target, client_id),
    })
}

fn apply_github(path: &Path, app_id: &str, app_slug: &str, key_path: &Path) -> Result<Applied> {
    if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) {
        bail!("the GitHub App ID must be a number, not `{app_id}`");
    }
    if app_slug.trim().is_empty() {
        bail!("the GitHub App slug is empty. Find it in the app URL: github.com/apps/<slug>");
    }
    check_github_key(app_id, app_slug, key_path)?;
    let mut doc = edit::load_doc(path)?;
    edit::set_github_app(
        &mut doc,
        app_id,
        app_slug.trim(),
        &key_path.to_string_lossy(),
    )?;
    let cfg = edit::write_config(path, &doc)?;
    Ok(Applied {
        target: SetupTarget::Github,
        export_needed: None,
        shadowed_by: shadowing(&cfg, SetupTarget::Github, app_id),
    })
}

/// Flags mode: no prompts.
fn run_flags(path: &Path, target: SetupTarget, opts: &SetupOptions) -> Result<Applied> {
    if target == SetupTarget::Github {
        let (Some(id), Some(slug), Some(key)) =
            (&opts.app_id, &opts.app_slug, &opts.private_key_path)
        else {
            bail!("for github, give --app-id, --app-slug and --private-key-path");
        };
        return apply_github(path, id, slug, &expand_path(key));
    }
    let Some(id) = opts.client_id.as_deref().filter(|s| !s.is_empty()) else {
        bail!("give --client-id for {}", target.key());
    };
    let secret = match (&opts.client_secret, opts.secret_from_env) {
        (Some(_), true) => bail!("give --client-secret or --secret-from-env, not both"),
        (Some(s), false) if !s.is_empty() => Some(s.as_str()),
        (None, true) => None,
        _ => bail!(
            "give --client-secret, or --secret-from-env to read it from {}",
            target.secret_env_var().unwrap_or("the environment")
        ),
    };
    apply_oauth(path, target, id, secret)
}

/// Interactive mode for one target. `None` when the user skips it.
fn run_interactive_target(
    path: &Path,
    target: SetupTarget,
    p: &mut dyn Prompter,
    out: &mut dyn Write,
) -> Result<Option<Applied>> {
    print_guide(out, target)?;
    if target == SetupTarget::Github {
        let app_id = p.ask("App ID (empty to skip)")?;
        if app_id.is_empty() {
            return Ok(None);
        }
        let slug = p.ask("App slug (from github.com/apps/<slug>)")?;
        let key = expand_path(&p.ask("Path to the private key .pem file")?);
        return match apply_github(path, &app_id, &slug, &key) {
            Ok(a) => Ok(Some(a)),
            Err(e) => {
                writeln!(out, "GitHub was not configured: {e:#}")?;
                Ok(None)
            }
        };
    }

    let client_id = p.ask("Client ID (empty to skip)")?;
    if client_id.is_empty() {
        return Ok(None);
    }
    if target == SetupTarget::Google && !client_id.ends_with(".apps.googleusercontent.com") {
        writeln!(
            out,
            "Warning: a Google client ID usually ends with .apps.googleusercontent.com."
        )?;
    }
    let env_var = target.secret_env_var().unwrap_or("");
    let in_file = p.confirm(
        &format!("Keep the client secret in arawn.toml? (No: read it from {env_var})"),
        true,
    )?;
    let secret = if in_file {
        let s = p.ask_secret("Client secret")?;
        if s.is_empty() {
            writeln!(out, "No secret given. {} was not configured.", target.key())?;
            return Ok(None);
        }
        Some(s)
    } else {
        None
    };
    Ok(Some(apply_oauth(
        path,
        target,
        &client_id,
        secret.as_deref(),
    )?))
}

fn print_next_steps(
    out: &mut dyn Write,
    applied: &[Applied],
    llm_key_env: Option<&str>,
) -> Result<()> {
    writeln!(out)?;
    if applied.is_empty() {
        writeln!(out, "No integrations were changed.")?;
        if let Some(v) = llm_key_env {
            writeln!(out, "Before arawn serve: export {v}=<your-api-key>")?;
        }
        return Ok(());
    }
    writeln!(out, "Next steps:")?;
    let mut n = 1;
    let exports: Vec<_> = applied.iter().filter_map(|a| a.export_needed).collect();
    if llm_key_env.is_some() || !exports.is_empty() {
        writeln!(out, "  {n}. In the shell that runs the server, export:")?;
        if let Some(v) = llm_key_env {
            writeln!(out, "       export {v}=<your-api-key>")?;
        }
        for v in exports {
            writeln!(out, "       export {v}=<client secret>")?;
        }
        n += 1;
    }
    writeln!(out, "  {n}. Restart the server: arawn serve")?;
    n += 1;
    let names: Vec<&str> = applied
        .iter()
        .flat_map(|a| a.target.connect_names())
        .collect();
    writeln!(
        out,
        "  {n}. In a second terminal, connect: arawn connect {}",
        names.join(" ")
    )?;
    writeln!(
        out,
        "     (Or later, every set-up service at once: arawn connect --all)"
    )?;
    n += 1;
    writeln!(out, "  {n}. Run arawn doctor to check the setup.")?;
    Ok(())
}

fn load_config(path: &Path) -> Result<ArawnConfig> {
    let doc = edit::load_doc(path)?;
    edit::validate(&doc).with_context(|| format!("{} does not load", path.display()))
}

/// Run `arawn setup`.
pub fn run_setup(
    data_dir: &Path,
    opts: SetupOptions,
    p: &mut dyn Prompter,
    out: &mut dyn Write,
    interactive: bool,
) -> Result<()> {
    let path = data_dir.join("arawn.toml");

    // Argument checks come first, so a bad invocation writes nothing.
    let target = match opts.target.as_deref() {
        Some(k) => Some(SetupTarget::from_key(k).with_context(|| {
            let keys: Vec<_> = SetupTarget::ALL.iter().map(|t| t.key()).collect();
            format!("unknown provider `{k}`. Use one of: {}", keys.join(", "))
        })?),
        None => None,
    };
    let flags_mode = opts.has_credential_flags();
    if flags_mode && target.is_none() {
        bail!("credential flags need a provider, e.g. arawn setup google --client-id …");
    }
    if !flags_mode && !interactive {
        bail!(
            "arawn setup needs a terminal for prompts. For scripts, give the provider and \
             the credential flags (see arawn setup --help)."
        );
    }

    // Set when this run scaffolds the starter config: the LLM key env var
    // joins the next steps.
    let mut llm_key_env = None;
    if !path.exists() {
        let (written, key_env) = super::init::write_init(
            data_dir,
            &super::init::InitOptions {
                provider: if opts.llm_provider.is_empty() {
                    "groq".into()
                } else {
                    opts.llm_provider.clone()
                },
                model: None,
                api_key_env: None,
                force: false,
            },
        )?;
        writeln!(
            out,
            "No arawn.toml found. Wrote a starter config to {}.",
            written.display()
        )?;
        llm_key_env = Some(key_env);
    }

    let mut applied = Vec::new();

    if let (true, Some(target)) = (flags_mode, target) {
        let a = run_flags(&path, target, &opts)?;
        a.report(out, &path)?;
        applied.push(a);
        return print_next_steps(out, &applied, llm_key_env.as_deref());
    }

    let targets: Vec<SetupTarget> = match target {
        Some(t) => vec![t],
        None => {
            let cfg = load_config(&path)?;
            writeln!(out, "Integrations in {}:", path.display())?;
            for t in SetupTarget::ALL {
                let state = target_status(&cfg, t).unwrap_or_else(|| "not configured".into());
                writeln!(out, "  {:<32} {state}", t.title())?;
            }
            SetupTarget::ALL.to_vec()
        }
    };

    for t in targets {
        let cfg = load_config(&path)?;
        let status = target_status(&cfg, t);
        let question = match &status {
            Some(state) => format!("{} is {state}. Replace it?", t.title()),
            None => format!("Set up {}?", t.title()),
        };
        // A provider named on the command line needs no confirmation
        // unless it would replace an existing configuration. Enter never
        // replaces working credentials, and never opts into a provider.
        let ask_first = target.is_none() || status.is_some();
        if ask_first && !p.confirm(&question, false)? {
            continue;
        }
        if let Some(a) = run_interactive_target(&path, t, p, out)? {
            a.report(out, &path)?;
            applied.push(a);
        }
    }

    // The full walk offers the work profile once, if no lens is declared
    // yet (ARAWN-T-0507). Enter means no.
    if target.is_none() && load_config(&path)?.lenses.is_empty() {
        writeln!(out)?;
        if p.confirm(
            "Add the work profile? (a `work` lens bound to mail, calendar, Slack, Jira and \
             GitHub feeds; each feed starts when its service connects)",
            false,
        )? {
            let (lenses, feeds) = super::profile::profile("work")?;
            let mut doc = edit::load_doc(&path)?;
            let added = super::profile::apply(&mut doc, &lenses, &feeds)?;
            edit::write_config(&path, &doc)?;
            writeln!(
                out,
                "Added the work profile: {} lens(es), {} feed(s). Edit the lens description \
                 and tags in arawn.toml to match your work.",
                added.added_lenses.len(),
                added.added_feeds.len()
            )?;
        }
    }

    print_next_steps(out, &applied, llm_key_env.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Answers from a script, in order. Panics when the script runs out,
    /// so an unexpected extra question fails the test.
    struct Scripted(VecDeque<&'static str>);

    impl Scripted {
        fn new(answers: &[&'static str]) -> Self {
            Self(answers.iter().copied().collect())
        }
        fn next(&mut self, q: &str) -> String {
            self.0
                .pop_front()
                .unwrap_or_else(|| panic!("no scripted answer for: {q}"))
                .to_string()
        }
    }

    impl Prompter for Scripted {
        fn ask(&mut self, q: &str) -> Result<String> {
            Ok(self.next(q))
        }
        fn ask_secret(&mut self, q: &str) -> Result<String> {
            Ok(self.next(q))
        }
        fn confirm(&mut self, q: &str, default: bool) -> Result<bool> {
            Ok(match self.next(q).as_str() {
                "" => default,
                a => a == "y",
            })
        }
    }

    fn opts() -> SetupOptions {
        SetupOptions {
            llm_provider: "groq".into(),
            ..Default::default()
        }
    }

    fn config(dir: &Path) -> ArawnConfig {
        toml::from_str(&std::fs::read_to_string(dir.join("arawn.toml")).unwrap()).unwrap()
    }

    #[test]
    fn flags_mode_writes_google_block_and_scaffolds_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        let o = SetupOptions {
            target: Some("google".into()),
            client_id: Some("x.apps.googleusercontent.com".into()),
            client_secret: Some("GOCSPX-s".into()),
            ..opts()
        };
        run_setup(dir.path(), o, &mut Scripted::new(&[]), &mut out, false).unwrap();
        let cfg = config(dir.path());
        assert_eq!(
            cfg.integrations.google.client_id,
            "x.apps.googleusercontent.com"
        );
        assert_eq!(cfg.integrations.google.client_secret, "GOCSPX-s");
        // The starter LLM config was written too, and its key export is in
        // the one next-steps list.
        assert!(cfg.llm.contains_key("default"));
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.matches("Next steps:").count(), 1, "{text}");
        assert!(text.contains("export GROQ_API_KEY=<your-api-key>"));
        assert!(
            text.contains("arawn connect gmail google_calendar google_drive"),
            "{text}"
        );
    }

    #[test]
    fn flags_mode_secret_from_env_omits_secret() {
        let dir = tempfile::tempdir().unwrap();
        let o = SetupOptions {
            target: Some("slack".into()),
            client_id: Some("123.456".into()),
            secret_from_env: true,
            ..opts()
        };
        let mut out = Vec::new();
        run_setup(dir.path(), o, &mut Scripted::new(&[]), &mut out, false).unwrap();
        let cfg = config(dir.path());
        assert_eq!(cfg.integrations.slack.client_id, "123.456");
        assert_eq!(cfg.integrations.slack.client_secret, "");
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("export ARAWN_SLACK_CLIENT_SECRET=<client secret>"));
    }

    #[test]
    fn flags_mode_rejects_missing_secret_choice_and_unknown_provider() {
        let dir = tempfile::tempdir().unwrap();
        let o = SetupOptions {
            target: Some("slack".into()),
            client_id: Some("id".into()),
            ..opts()
        };
        let err = run_setup(
            dir.path(),
            o,
            &mut Scripted::new(&[]),
            &mut Vec::new(),
            false,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("--secret-from-env"));

        let o = SetupOptions {
            target: Some("linear".into()),
            ..opts()
        };
        let err = run_setup(
            dir.path(),
            o,
            &mut Scripted::new(&[]),
            &mut Vec::new(),
            true,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("unknown provider `linear`"));
    }

    #[test]
    fn no_terminal_and_no_flags_is_an_error_not_a_hang() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_setup(
            dir.path(),
            opts(),
            &mut Scripted::new(&[]),
            &mut Vec::new(),
            false,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("needs a terminal"));
        // A bad invocation writes nothing, not even the starter config.
        assert!(!dir.path().join("arawn.toml").exists());
    }

    #[test]
    fn interactive_walk_configures_chosen_providers_and_skips_others() {
        let dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        let mut p = Scripted::new(&[
            // Google: yes, id, keep secret in file, secret
            "y",
            "x.apps.googleusercontent.com",
            "y",
            "GOCSPX-s",
            // Slack: no
            "n",
            // Atlassian: yes, id, secret from env
            "y",
            "atl-id",
            "n",
            // GitHub: no
            "n",
            // Work profile: yes
            "y",
        ]);
        run_setup(dir.path(), opts(), &mut p, &mut out, true).unwrap();
        let cfg = config(dir.path());
        assert_eq!(cfg.lenses[0].name, "work");
        assert_eq!(cfg.feeds.len(), 6);
        assert_eq!(cfg.integrations.google.client_secret, "GOCSPX-s");
        assert_eq!(cfg.integrations.atlassian.client_id, "atl-id");
        assert_eq!(cfg.integrations.slack.client_id, "");
        let text = String::from_utf8(out).unwrap();
        // The guide printed the crate's scopes.
        assert!(text.contains("https://www.googleapis.com/auth/gmail.readonly"));
        assert!(text.contains("offline_access"));
        assert!(
            text.contains("arawn connect gmail google_calendar google_drive atlassian\n"),
            "{text}"
        );
        assert!(!text.contains(" slack"), "{text}");
    }

    #[test]
    fn named_provider_already_configured_asks_before_replacing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arawn.toml"),
            "[integrations.slack]\nclient_id = \"old\"\nclient_secret = \"oldsec\"\n",
        )
        .unwrap();
        let o = SetupOptions {
            target: Some("slack".into()),
            ..opts()
        };
        let mut out = Vec::new();
        // Press Enter at "Replace it?": the default keeps the existing
        // credentials, and no further prompts follow.
        run_setup(dir.path(), o, &mut Scripted::new(&[""]), &mut out, true).unwrap();
        assert_eq!(config(dir.path()).integrations.slack.client_id, "old");
        assert!(
            String::from_utf8(out)
                .unwrap()
                .contains("No integrations were changed")
        );
    }

    #[test]
    fn github_rejects_bad_key_and_keeps_config_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("bad.pem");
        std::fs::write(&key, "not a key").unwrap();
        let o = SetupOptions {
            target: Some("github".into()),
            app_id: Some("42".into()),
            app_slug: Some("arawn-me".into()),
            private_key_path: Some(key.to_string_lossy().into_owned()),
            ..opts()
        };
        let err = run_setup(
            dir.path(),
            o,
            &mut Scripted::new(&[]),
            &mut Vec::new(),
            false,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("not a usable RSA private key"));
        assert_eq!(config(dir.path()).integrations.github.app_id, "");
    }

    #[test]
    fn warns_when_a_per_service_table_hides_the_new_google_client() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arawn.toml"),
            "[integrations.gmail]\nclient_id = \"old-gmail\"\nclient_secret = \"s\"\n",
        )
        .unwrap();
        let o = SetupOptions {
            target: Some("google".into()),
            client_id: Some("new.apps.googleusercontent.com".into()),
            client_secret: Some("GOCSPX-n".into()),
            ..opts()
        };
        let mut out = Vec::new();
        run_setup(dir.path(), o, &mut Scripted::new(&[]), &mut out, false).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains(
                "Warning: gmail still uses the client ID from arawn.toml \
                 integrations.gmail.client_id"
            ),
            "{text}"
        );
        // Calendar and Drive pick up the new shared client: no warning.
        assert!(!text.contains("google_calendar still uses"), "{text}");
    }

    #[test]
    fn github_rejects_empty_slug_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let o = SetupOptions {
            target: Some("github".into()),
            app_id: Some("42".into()),
            app_slug: Some("  ".into()),
            private_key_path: Some("/nonexistent.pem".into()),
            ..opts()
        };
        let err = run_setup(
            dir.path(),
            o,
            &mut Scripted::new(&[]),
            &mut Vec::new(),
            false,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("slug is empty"), "{err}");
    }

    #[test]
    fn id_without_secret_in_this_shell_still_asks_before_replacing() {
        // Set up earlier with --secret-from-env; the env var is not
        // exported now. The resolver cannot complete the client, but the
        // table is real configuration and must not be replaced silently.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arawn.toml"),
            "[integrations.atlassian]\nclient_id = \"keep-me\"\n",
        )
        .unwrap();
        let cfg = config(dir.path());
        let status = target_status(&cfg, SetupTarget::Atlassian).unwrap();
        assert!(status.contains("ARAWN_ATLASSIAN_CLIENT_SECRET"), "{status}");

        let o = SetupOptions {
            target: Some("atlassian".into()),
            ..opts()
        };
        // Only one answer: Enter at "Replace it?". Any further prompt
        // would panic the scripted prompter.
        run_setup(
            dir.path(),
            o,
            &mut Scripted::new(&[""]),
            &mut Vec::new(),
            true,
        )
        .unwrap();
        assert_eq!(
            config(dir.path()).integrations.atlassian.client_id,
            "keep-me"
        );
    }

    #[test]
    fn status_reports_shared_google_and_partial_configuration() {
        let cfg: ArawnConfig =
            toml::from_str("[integrations.gmail]\nclient_id = \"g\"\nclient_secret = \"s\"\n")
                .unwrap();
        let s = target_status(&cfg, SetupTarget::Google).unwrap();
        assert!(s.starts_with("partly configured: gmail"), "{s}");
        assert!(target_status(&cfg, SetupTarget::Slack).is_none());
    }
}

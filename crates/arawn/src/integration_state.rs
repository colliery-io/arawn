//! Per-integration setup state (ARAWN-T-0502): not configured, missing
//! its secret, configured, connected, or broken — each with the one-line
//! fix. `arawn doctor`, the startup log and the `status` RPC all read this,
//! so they cannot disagree.
//!
//! Configuration comes from [`OAuthClientResolver`]; connection comes from
//! the encrypted token store. Nothing here talks to a provider.

use std::path::Path;

use crate::config::IntegrationsConfig;
use crate::oauth_clients::{OAuthClientResolver, OAuthProvider};

/// One integration's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationState {
    /// No credentials anywhere.
    NotConfigured,
    /// A client ID is in `arawn.toml`, but no secret resolves in this
    /// environment (typically: set up with `--secret-from-env`, and the
    /// env var is not exported here).
    MissingSecret {
        tables: Vec<String>,
        env_var: String,
    },
    /// Some settings are present but they are not usable (GitHub only:
    /// a missing slug or key, or an unreadable key file).
    Incomplete { reason: String },
    /// Credentials resolve; no token yet.
    Configured { origin: String },
    /// Credentials resolve and a token is stored.
    Connected { origin: String },
    /// The stored token cannot be read (decrypt failure, bad file).
    TokenError { error: String },
}

impl IntegrationState {
    /// Stable machine name, used by the `status` RPC.
    pub fn code(&self) -> &'static str {
        match self {
            IntegrationState::NotConfigured => "not_configured",
            IntegrationState::MissingSecret { .. } => "missing_secret",
            IntegrationState::Incomplete { .. } => "incomplete",
            IntegrationState::Configured { .. } => "configured",
            IntegrationState::Connected { .. } => "connected",
            IntegrationState::TokenError { .. } => "token_error",
        }
    }

    /// True when the user tried to set this up and it does not work.
    pub fn is_broken(&self) -> bool {
        matches!(
            self,
            IntegrationState::MissingSecret { .. }
                | IntegrationState::Incomplete { .. }
                | IntegrationState::TokenError { .. }
        )
    }
}

/// One integration, its state, and how to fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationReport {
    /// Registry / token-store name, what `/connect` takes.
    pub service: &'static str,
    /// The `arawn setup <target>` that configures it.
    pub setup_target: &'static str,
    pub state: IntegrationState,
}

impl IntegrationReport {
    /// What is true now, in a few words.
    pub fn describe(&self) -> String {
        match &self.state {
            IntegrationState::NotConfigured => "not configured".into(),
            IntegrationState::MissingSecret { tables, env_var } => format!(
                "client ID in {}, but no client secret ({env_var} is not set)",
                tables.join(", ")
            ),
            IntegrationState::Incomplete { reason } => reason.clone(),
            IntegrationState::Configured { origin } => {
                format!(
                    "configured ({} from {origin}), not connected",
                    self.credential_noun()
                )
            }
            IntegrationState::Connected { origin } => {
                format!("connected ({} from {origin})", self.credential_noun())
            }
            IntegrationState::TokenError { error } => {
                format!("the stored token cannot be read: {error}")
            }
        }
    }

    /// GitHub is configured by an App ID; the others by an OAuth client.
    fn credential_noun(&self) -> &'static str {
        if self.setup_target == "github" {
            "app ID"
        } else {
            "client"
        }
    }

    /// The one-line fix, or `None` when nothing is needed.
    pub fn hint(&self) -> Option<String> {
        let svc = self.service;
        let target = self.setup_target;
        match &self.state {
            IntegrationState::NotConfigured => Some(format!("run: arawn setup {target}")),
            IntegrationState::MissingSecret { env_var, .. } => Some(format!(
                "export {env_var} in the shell that runs the server, or run: arawn setup {target}"
            )),
            IntegrationState::Incomplete { .. } => Some(format!("run: arawn setup {target}")),
            IntegrationState::Configured { .. } => Some(format!("run: arawn connect {svc}")),
            IntegrationState::Connected { .. } => None,
            IntegrationState::TokenError { .. } => Some(format!(
                "run: arawn disconnect {svc}, then arawn connect {svc}"
            )),
        }
    }
}

/// Token-store lookup: `Ok(true)` when a token is stored for the service.
pub type TokenLookup<'a> = dyn Fn(&str) -> Result<bool, String> + 'a;

fn setup_target(p: OAuthProvider) -> &'static str {
    if p.uses_shared_google() {
        "google"
    } else {
        p.config_key()
    }
}

fn token_state(lookup: &TokenLookup<'_>, service: &str, origin: String) -> IntegrationState {
    match lookup(service) {
        Ok(true) => IntegrationState::Connected { origin },
        Ok(false) => IntegrationState::Configured { origin },
        Err(error) => IntegrationState::TokenError { error },
    }
}

/// The tables that carry a client ID for this provider, and the secret
/// env var the user most likely meant to export.
fn ids_without_secret(cfg: &IntegrationsConfig, p: OAuthProvider) -> Option<(Vec<String>, String)> {
    let own = match p {
        OAuthProvider::Gmail => &cfg.gmail,
        OAuthProvider::Calendar => &cfg.calendar,
        OAuthProvider::Drive => &cfg.drive,
        OAuthProvider::Atlassian => &cfg.atlassian,
        OAuthProvider::Slack => &cfg.slack,
    };
    if !own.client_id.is_empty() {
        return Some((
            vec![format!("[integrations.{}]", p.config_key())],
            p.secret_env_var(),
        ));
    }
    if p.uses_shared_google() && !cfg.google.client_id.is_empty() {
        return Some((
            vec!["[integrations.google]".into()],
            "ARAWN_GOOGLE_CLIENT_SECRET".into(),
        ));
    }
    None
}

/// Inspect every integration with the given resolver and token lookup.
pub fn inspect_with(
    cfg: &IntegrationsConfig,
    resolver: &OAuthClientResolver<'_>,
    tokens: &TokenLookup<'_>,
) -> Vec<IntegrationReport> {
    let mut out: Vec<IntegrationReport> = OAuthProvider::ALL
        .iter()
        .map(|&p| {
            let state = match resolver.resolve(p) {
                Some(c) => token_state(tokens, p.service_name(), c.client_id_origin.to_string()),
                None => match ids_without_secret(cfg, p) {
                    Some((tables, env_var)) => IntegrationState::MissingSecret { tables, env_var },
                    None => IntegrationState::NotConfigured,
                },
            };
            IntegrationReport {
                service: p.service_name(),
                setup_target: setup_target(p),
                state,
            }
        })
        .collect();

    let gh = &cfg.github;
    let github_state = match resolver.resolve_github() {
        Some(app) => match app.load_private_key() {
            Ok(_) => token_state(
                tokens,
                arawn_integrations::github::SERVICE_NAME,
                app.app_id_origin.to_string(),
            ),
            Err(e) => IntegrationState::Incomplete {
                reason: format!("the GitHub App private key cannot be read: {e}"),
            },
        },
        None if !(gh.app_id.is_empty()
            && gh.app_slug.is_empty()
            && gh.private_key_path.is_empty()) =>
        {
            let mut missing = Vec::new();
            if gh.app_id.is_empty() {
                missing.push("app_id");
            }
            if gh.app_slug.is_empty() {
                missing.push("app_slug");
            }
            if gh.private_key_path.is_empty() {
                missing.push("private_key_path");
            }
            IntegrationState::Incomplete {
                reason: format!("[integrations.github] has no {}", missing.join(", ")),
            }
        }
        None => IntegrationState::NotConfigured,
    };
    out.push(IntegrationReport {
        service: arawn_integrations::github::SERVICE_NAME,
        setup_target: "github",
        state: github_state,
    });
    out
}

/// Inspect every integration against the process environment and the
/// token store in `data_dir`. `Err` when the token store cannot be opened.
pub fn inspect(
    cfg: &IntegrationsConfig,
    data_dir: &Path,
) -> Result<Vec<IntegrationReport>, String> {
    let store = arawn_auth::TokenStore::open(data_dir)
        .map_err(|e| format!("cannot open the token store in {}: {e}", data_dir.display()))?;
    let lookup = |service: &str| -> Result<bool, String> {
        store
            .load(service)
            .map(|t| t.is_some())
            .map_err(|e| e.to_string())
    };
    let resolver = OAuthClientResolver::new(cfg);
    Ok(inspect_with(cfg, &resolver, &lookup))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GithubAppCredentials, IntegrationCredentials};
    use crate::oauth_clients::BundledClient;
    use std::collections::{HashMap, HashSet};

    fn creds(id: &str, secret: &str) -> IntegrationCredentials {
        IntegrationCredentials {
            client_id: id.into(),
            client_secret: secret.into(),
        }
    }

    fn no_bundled(_: OAuthProvider) -> Option<BundledClient> {
        None
    }

    fn run(
        cfg: &IntegrationsConfig,
        env: &[(&'static str, &'static str)],
        connected: &[&str],
        broken: &[&str],
    ) -> HashMap<&'static str, IntegrationReport> {
        let env: HashMap<_, _> = env.iter().copied().collect();
        let connected: HashSet<String> = connected.iter().map(|s| s.to_string()).collect();
        let broken: HashSet<String> = broken.iter().map(|s| s.to_string()).collect();
        let resolver = OAuthClientResolver::with_sources(
            cfg,
            move |k| env.get(k).map(|v| v.to_string()),
            no_bundled,
        );
        let lookup = move |s: &str| -> Result<bool, String> {
            if broken.contains(s) {
                Err("decrypt failed".into())
            } else {
                Ok(connected.contains(s))
            }
        };
        inspect_with(cfg, &resolver, &lookup)
            .into_iter()
            .map(|r| (r.service, r))
            .collect()
    }

    #[test]
    fn empty_config_reports_every_integration_not_configured_with_setup_hint() {
        let r = run(&IntegrationsConfig::default(), &[], &[], &[]);
        assert_eq!(r.len(), 6);
        assert!(
            r.values()
                .all(|x| x.state == IntegrationState::NotConfigured)
        );
        assert_eq!(r["gmail"].hint().unwrap(), "run: arawn setup google");
        assert_eq!(r["slack"].hint().unwrap(), "run: arawn setup slack");
        assert_eq!(r["github"].hint().unwrap(), "run: arawn setup github");
    }

    #[test]
    fn configured_and_connected_states_follow_the_token_store() {
        let cfg = IntegrationsConfig {
            google: creds("gid", "gsec"),
            ..Default::default()
        };
        let r = run(&cfg, &[], &["gmail"], &[]);
        assert_eq!(r["gmail"].state.code(), "connected");
        assert!(r["gmail"].hint().is_none());
        assert_eq!(r["google_calendar"].state.code(), "configured");
        assert_eq!(
            r["google_calendar"].hint().unwrap(),
            "run: arawn connect google_calendar"
        );
        assert!(
            r["google_drive"]
                .describe()
                .contains("arawn.toml integrations.google.client_id")
        );
    }

    #[test]
    fn id_without_secret_is_missing_secret_and_names_the_env_var() {
        let cfg = IntegrationsConfig {
            google: creds("gid", ""),
            atlassian: creds("aid", ""),
            ..Default::default()
        };
        let r = run(&cfg, &[], &[], &[]);
        assert_eq!(
            r["gmail"].state,
            IntegrationState::MissingSecret {
                tables: vec!["[integrations.google]".into()],
                env_var: "ARAWN_GOOGLE_CLIENT_SECRET".into(),
            }
        );
        assert!(r["gmail"].state.is_broken());
        assert!(
            r["atlassian"]
                .hint()
                .unwrap()
                .starts_with("export ARAWN_ATLASSIAN_CLIENT_SECRET")
        );
        // Exporting the secret fixes it.
        let r = run(&cfg, &[("ARAWN_ATLASSIAN_CLIENT_SECRET", "s")], &[], &[]);
        assert_eq!(r["atlassian"].state.code(), "configured");
    }

    #[test]
    fn unreadable_token_is_a_token_error_with_reconnect_hint() {
        let cfg = IntegrationsConfig {
            slack: creds("sid", "ssec"),
            ..Default::default()
        };
        let r = run(&cfg, &[], &[], &["slack"]);
        assert_eq!(r["slack"].state.code(), "token_error");
        assert_eq!(
            r["slack"].hint().unwrap(),
            "run: arawn disconnect slack, then arawn connect slack"
        );
    }

    #[test]
    fn partial_github_is_incomplete_and_names_missing_fields() {
        let cfg = IntegrationsConfig {
            github: GithubAppCredentials {
                app_id: "42".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let r = run(&cfg, &[], &[], &[]);
        assert_eq!(
            r["github"].describe(),
            "[integrations.github] has no app_slug, private_key_path"
        );
        assert!(r["github"].state.is_broken());
    }

    #[test]
    fn github_with_unreadable_key_is_incomplete() {
        let cfg = IntegrationsConfig {
            github: GithubAppCredentials {
                app_id: "42".into(),
                app_slug: "s".into(),
                private_key_path: "/definitely/not/here.pem".into(),
            },
            ..Default::default()
        };
        let r = run(&cfg, &[], &[], &[]);
        assert!(
            r["github"]
                .describe()
                .contains("private key cannot be read"),
            "{}",
            r["github"].describe()
        );
    }

    #[test]
    fn inspect_reads_a_real_token_store() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = IntegrationsConfig {
            slack: creds("sid", "ssec"),
            ..Default::default()
        };
        // ARAWN_SLACK_* env vars may be set on a dev machine; only assert
        // what the empty token store guarantees.
        let reports = inspect(&cfg, dir.path()).unwrap();
        assert!(
            reports
                .iter()
                .all(|r| !matches!(r.state, IntegrationState::Connected { .. }))
        );
    }
}

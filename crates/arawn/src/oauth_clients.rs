//! OAuth client resolution — one place that answers "which OAuth client
//! does arawn use for provider X, and where did it come from?"
//! (ARAWN-T-0504, COLLIERY-I-0523 Phase 1).
//!
//! Startup, `arawn doctor` and `arawn setup` all ask this resolver, so
//! they cannot disagree about whether a provider is configured.
//!
//! Resolution order per provider, first complete tier wins:
//!
//! 1. **Service tier** — `ARAWN_<SVC>_CLIENT_ID` / `_SECRET` env vars, each
//!    field falling back independently to `[integrations.<svc>]`.
//! 2. **Shared Google tier** (Gmail / Calendar / Drive only) — the same
//!    walk over `ARAWN_GOOGLE_*` and `[integrations.google]`.
//! 3. **Bundled tier** — a shared client compiled into the binary. Empty
//!    today; COLLIERY-I-0523 fills it provider by provider.
//! 4. Nothing → the provider is skipped.
//!
//! A tier is used only when it yields BOTH an id and a secret (bundled
//! public clients excepted). A service tier with only an id does not
//! borrow the shared Google secret — the whole pair falls through. Empty
//! strings count as unset at every step.

use crate::config::{GithubAppCredentials, IntegrationCredentials, IntegrationsConfig};

/// The OAuth-app integrations. GitHub uses the App model instead; see
/// [`OAuthClientResolver::resolve_github`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuthProvider {
    Gmail,
    Calendar,
    Drive,
    Atlassian,
    Slack,
}

impl OAuthProvider {
    pub const ALL: [OAuthProvider; 5] = [
        OAuthProvider::Gmail,
        OAuthProvider::Calendar,
        OAuthProvider::Drive,
        OAuthProvider::Atlassian,
        OAuthProvider::Slack,
    ];

    /// The integration's registry / token-store name (what `/connect`
    /// takes). Sourced from each integration crate so it cannot drift.
    pub fn service_name(self) -> &'static str {
        match self {
            OAuthProvider::Gmail => arawn_integrations::gmail::SERVICE_NAME,
            OAuthProvider::Calendar => arawn_integrations::calendar::SERVICE_NAME,
            OAuthProvider::Drive => arawn_integrations::drive::SERVICE_NAME,
            OAuthProvider::Atlassian => arawn_integrations::atlassian::SERVICE_NAME,
            OAuthProvider::Slack => arawn_integrations::slack::SERVICE_NAME,
        }
    }

    /// The `[integrations.<key>]` table name in `arawn.toml`.
    pub fn config_key(self) -> &'static str {
        match self {
            OAuthProvider::Gmail => "gmail",
            OAuthProvider::Calendar => "calendar",
            OAuthProvider::Drive => "drive",
            OAuthProvider::Atlassian => "atlassian",
            OAuthProvider::Slack => "slack",
        }
    }

    /// Env-var infix: `ARAWN_<infix>_CLIENT_ID`. Calendar and Drive use
    /// GCAL / GDRIVE, not a GOOGLE_ prefix (historical; documented in
    /// reference/integrations-config.md).
    fn env_infix(self) -> &'static str {
        match self {
            OAuthProvider::Gmail => "GMAIL",
            OAuthProvider::Calendar => "GCAL",
            OAuthProvider::Drive => "GDRIVE",
            OAuthProvider::Atlassian => "ATLASSIAN",
            OAuthProvider::Slack => "SLACK",
        }
    }

    /// The env var that holds this provider's own client secret.
    pub fn secret_env_var(self) -> String {
        format!("ARAWN_{}_CLIENT_SECRET", self.env_infix())
    }

    /// Whether this provider falls back to the shared Google client.
    pub fn uses_shared_google(self) -> bool {
        matches!(
            self,
            OAuthProvider::Gmail | OAuthProvider::Calendar | OAuthProvider::Drive
        )
    }

    fn credentials(self, cfg: &IntegrationsConfig) -> &IntegrationCredentials {
        match self {
            OAuthProvider::Gmail => &cfg.gmail,
            OAuthProvider::Calendar => &cfg.calendar,
            OAuthProvider::Drive => &cfg.drive,
            OAuthProvider::Atlassian => &cfg.atlassian,
            OAuthProvider::Slack => &cfg.slack,
        }
    }
}

/// Which tier supplied the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientTier {
    /// The provider's own env vars / `[integrations.<svc>]` block.
    Service,
    /// `ARAWN_GOOGLE_*` / `[integrations.google]`.
    SharedGoogle,
    /// Compiled into this build.
    Bundled,
}

/// Where one field (id or secret) was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldOrigin {
    /// An env var, by name.
    Env(String),
    /// An `arawn.toml` key, e.g. `integrations.google.client_secret`.
    Config(String),
    /// Compiled into this build.
    Bundled,
}

impl std::fmt::Display for FieldOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldOrigin::Env(v) => write!(f, "env {v}"),
            FieldOrigin::Config(k) => write!(f, "arawn.toml {k}"),
            FieldOrigin::Bundled => write!(f, "bundled"),
        }
    }
}

/// A resolved OAuth client for one provider. `Debug` redacts the secret.
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedClient {
    pub provider: OAuthProvider,
    pub tier: ClientTier,
    pub client_id: String,
    pub client_id_origin: FieldOrigin,
    /// `None` only for a bundled public (PKCE) client.
    pub client_secret: Option<String>,
    pub client_secret_origin: Option<FieldOrigin>,
}

impl std::fmt::Debug for ResolvedClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedClient")
            .field("provider", &self.provider)
            .field("tier", &self.tier)
            .field("client_id", &self.client_id)
            .field("client_id_origin", &self.client_id_origin)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "<redacted>"),
            )
            .field("client_secret_origin", &self.client_secret_origin)
            .finish()
    }
}

/// A shared OAuth client compiled into the binary. A public client
/// ships no secret and relies on PKCE.
#[derive(Debug, Clone, Copy)]
pub struct BundledClient {
    pub client_id: &'static str,
    pub client_secret: Option<&'static str>,
}

/// The bundled-client registry for this build. Empty until a provider's
/// shared app is registered (COLLIERY-I-0523 Phase 2+).
pub fn bundled_client(provider: OAuthProvider) -> Option<BundledClient> {
    match provider {
        OAuthProvider::Gmail
        | OAuthProvider::Calendar
        | OAuthProvider::Drive
        | OAuthProvider::Atlassian
        | OAuthProvider::Slack => None,
    }
}

/// Where the GitHub App private key comes from. `Debug` redacts an
/// inline PEM body.
#[derive(Clone, PartialEq, Eq)]
pub enum GithubKeySource {
    /// Inline PEM from `ARAWN_GITHUB_PRIVATE_KEY_PEM`.
    InlinePem(String),
    /// A PEM file path (env or config). Read by the caller.
    Path { path: String, origin: FieldOrigin },
}

impl std::fmt::Debug for GithubKeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GithubKeySource::InlinePem(_) => f.write_str("InlinePem(<redacted>)"),
            GithubKeySource::Path { path, origin } => f
                .debug_struct("Path")
                .field("path", path)
                .field("origin", origin)
                .finish(),
        }
    }
}

/// GitHub App settings with their origins. Resolution does not read the
/// key file; [`ResolvedGithubApp::load_private_key`] does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGithubApp {
    pub app_id: String,
    pub app_id_origin: FieldOrigin,
    pub app_slug: String,
    pub app_slug_origin: FieldOrigin,
    pub key: GithubKeySource,
}

impl ResolvedGithubApp {
    /// The PEM body, reading the key file when the source is a path.
    pub fn load_private_key(&self) -> std::io::Result<String> {
        match &self.key {
            GithubKeySource::InlinePem(pem) => Ok(pem.clone()),
            GithubKeySource::Path { path, .. } => std::fs::read_to_string(path),
        }
    }
}

type EnvLookup<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// Resolves OAuth clients from env vars, `arawn.toml` and the bundled
/// registry.
pub struct OAuthClientResolver<'a> {
    config: &'a IntegrationsConfig,
    env: EnvLookup<'a>,
    bundled: fn(OAuthProvider) -> Option<BundledClient>,
}

impl<'a> OAuthClientResolver<'a> {
    /// Resolver over the process environment and this build's bundled
    /// clients.
    pub fn new(config: &'a IntegrationsConfig) -> Self {
        Self::with_sources(
            config,
            |name: &str| std::env::var(name).ok(),
            bundled_client,
        )
    }

    /// Resolver with injected env + bundled sources (tests).
    pub fn with_sources(
        config: &'a IntegrationsConfig,
        env: impl Fn(&str) -> Option<String> + 'a,
        bundled: fn(OAuthProvider) -> Option<BundledClient>,
    ) -> Self {
        Self {
            config,
            env: Box::new(env),
            bundled,
        }
    }

    fn env_nonempty(&self, name: &str) -> Option<String> {
        (self.env)(name).filter(|s| !s.is_empty())
    }

    /// One field: env var first, then the TOML value.
    fn field(
        &self,
        env_var: &str,
        toml_value: &str,
        toml_key: String,
    ) -> Option<(String, FieldOrigin)> {
        if let Some(v) = self.env_nonempty(env_var) {
            return Some((v, FieldOrigin::Env(env_var.to_string())));
        }
        if !toml_value.is_empty() {
            return Some((toml_value.to_string(), FieldOrigin::Config(toml_key)));
        }
        None
    }

    /// One env+TOML tier. Needs both id and secret.
    fn tier(
        &self,
        provider: OAuthProvider,
        tier: ClientTier,
        env_infix: &str,
        toml_table: &str,
        creds: &IntegrationCredentials,
    ) -> Option<ResolvedClient> {
        let (client_id, client_id_origin) = self.field(
            &format!("ARAWN_{env_infix}_CLIENT_ID"),
            &creds.client_id,
            format!("integrations.{toml_table}.client_id"),
        )?;
        let (secret, secret_origin) = self.field(
            &format!("ARAWN_{env_infix}_CLIENT_SECRET"),
            &creds.client_secret,
            format!("integrations.{toml_table}.client_secret"),
        )?;
        Some(ResolvedClient {
            provider,
            tier,
            client_id,
            client_id_origin,
            client_secret: Some(secret),
            client_secret_origin: Some(secret_origin),
        })
    }

    /// Resolve one provider's OAuth client, or `None` when no tier is
    /// complete.
    pub fn resolve(&self, provider: OAuthProvider) -> Option<ResolvedClient> {
        if let Some(c) = self.tier(
            provider,
            ClientTier::Service,
            provider.env_infix(),
            provider.config_key(),
            provider.credentials(self.config),
        ) {
            return Some(c);
        }
        if provider.uses_shared_google()
            && let Some(c) = self.tier(
                provider,
                ClientTier::SharedGoogle,
                "GOOGLE",
                "google",
                &self.config.google,
            )
        {
            return Some(c);
        }
        (self.bundled)(provider)
            .filter(|b| !b.client_id.is_empty())
            .map(|b| ResolvedClient {
                provider,
                tier: ClientTier::Bundled,
                client_id: b.client_id.to_string(),
                client_id_origin: FieldOrigin::Bundled,
                client_secret: b.client_secret.map(str::to_string),
                client_secret_origin: b.client_secret.map(|_| FieldOrigin::Bundled),
            })
    }

    /// Resolve every OAuth provider; unresolved providers are omitted.
    pub fn resolve_all(&self) -> Vec<ResolvedClient> {
        OAuthProvider::ALL
            .iter()
            .filter_map(|p| self.resolve(*p))
            .collect()
    }

    /// Resolve the GitHub App settings. `None` when the id, the slug or
    /// a key source is missing. An inline PEM beats a key path.
    pub fn resolve_github(&self) -> Option<ResolvedGithubApp> {
        let cfg: &GithubAppCredentials = &self.config.github;
        let (app_id, app_id_origin) = self.field(
            "ARAWN_GITHUB_APP_ID",
            &cfg.app_id,
            "integrations.github.app_id".into(),
        )?;
        let (app_slug, app_slug_origin) = self.field(
            "ARAWN_GITHUB_APP_SLUG",
            &cfg.app_slug,
            "integrations.github.app_slug".into(),
        )?;
        let key = if let Some(pem) = self.env_nonempty("ARAWN_GITHUB_PRIVATE_KEY_PEM") {
            GithubKeySource::InlinePem(pem)
        } else {
            let (path, origin) = self.field(
                "ARAWN_GITHUB_PRIVATE_KEY_PATH",
                &cfg.private_key_path,
                "integrations.github.private_key_path".into(),
            )?;
            GithubKeySource::Path { path, origin }
        };
        Some(ResolvedGithubApp {
            app_id,
            app_id_origin,
            app_slug,
            app_slug_origin,
            key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn creds(id: &str, secret: &str) -> IntegrationCredentials {
        IntegrationCredentials {
            client_id: id.into(),
            client_secret: secret.into(),
        }
    }

    fn no_bundled(_: OAuthProvider) -> Option<BundledClient> {
        None
    }

    fn atlassian_bundled(p: OAuthProvider) -> Option<BundledClient> {
        (p == OAuthProvider::Atlassian).then_some(BundledClient {
            client_id: "bundled-atl",
            client_secret: None,
        })
    }

    fn resolver<'a>(
        cfg: &'a IntegrationsConfig,
        env: &'a HashMap<&'static str, &'static str>,
        bundled: fn(OAuthProvider) -> Option<BundledClient>,
    ) -> OAuthClientResolver<'a> {
        OAuthClientResolver::with_sources(cfg, move |k| env.get(k).map(|v| v.to_string()), bundled)
    }

    #[test]
    fn nothing_configured_resolves_nothing() {
        let cfg = IntegrationsConfig::default();
        let env = HashMap::new();
        let r = resolver(&cfg, &env, no_bundled);
        assert!(r.resolve_all().is_empty());
        assert!(r.resolve_github().is_none());
    }

    #[test]
    fn service_toml_block_resolves() {
        let cfg = IntegrationsConfig {
            slack: creds("sid", "ssec"),
            ..Default::default()
        };
        let env = HashMap::new();
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Slack)
            .unwrap();
        assert_eq!(c.tier, ClientTier::Service);
        assert_eq!(c.client_id, "sid");
        assert_eq!(c.client_secret.as_deref(), Some("ssec"));
        assert_eq!(
            c.client_id_origin,
            FieldOrigin::Config("integrations.slack.client_id".into())
        );
    }

    #[test]
    fn env_beats_toml_per_field() {
        let cfg = IntegrationsConfig {
            gmail: creds("toml-id", "toml-secret"),
            ..Default::default()
        };
        let env = HashMap::from([("ARAWN_GMAIL_CLIENT_SECRET", "env-secret")]);
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Gmail)
            .unwrap();
        assert_eq!(c.client_id, "toml-id");
        assert_eq!(c.client_secret.as_deref(), Some("env-secret"));
        assert_eq!(
            c.client_secret_origin,
            Some(FieldOrigin::Env("ARAWN_GMAIL_CLIENT_SECRET".into()))
        );
    }

    #[test]
    fn empty_env_falls_through_to_toml() {
        let cfg = IntegrationsConfig {
            atlassian: creds("aid", "asec"),
            ..Default::default()
        };
        let env = HashMap::from([("ARAWN_ATLASSIAN_CLIENT_ID", "")]);
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Atlassian)
            .unwrap();
        assert_eq!(c.client_id, "aid");
    }

    #[test]
    fn google_services_fall_back_to_shared_block() {
        let cfg = IntegrationsConfig {
            google: creds("gid", "gsec"),
            ..Default::default()
        };
        let env = HashMap::new();
        let r = resolver(&cfg, &env, no_bundled);
        for p in [
            OAuthProvider::Gmail,
            OAuthProvider::Calendar,
            OAuthProvider::Drive,
        ] {
            let c = r.resolve(p).unwrap();
            assert_eq!(c.tier, ClientTier::SharedGoogle);
            assert_eq!(c.client_id, "gid");
        }
        // Non-Google providers never borrow the shared Google client.
        assert!(r.resolve(OAuthProvider::Slack).is_none());
        assert!(r.resolve(OAuthProvider::Atlassian).is_none());
    }

    #[test]
    fn service_block_beats_shared_google() {
        let cfg = IntegrationsConfig {
            google: creds("gid", "gsec"),
            calendar: creds("cid", "csec"),
            ..Default::default()
        };
        let env = HashMap::new();
        let r = resolver(&cfg, &env, no_bundled);
        assert_eq!(r.resolve(OAuthProvider::Calendar).unwrap().client_id, "cid");
        assert_eq!(r.resolve(OAuthProvider::Gmail).unwrap().client_id, "gid");
    }

    #[test]
    fn incomplete_service_tier_falls_through_as_a_pair() {
        // Gmail has only an id: the pair is incomplete, so the WHOLE
        // pair comes from the shared Google tier — the gmail id is not
        // mixed with the google secret. Matches pre-resolver behavior.
        let cfg = IntegrationsConfig {
            gmail: creds("gmail-id", ""),
            google: creds("gid", "gsec"),
            ..Default::default()
        };
        let env = HashMap::new();
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Gmail)
            .unwrap();
        assert_eq!(c.tier, ClientTier::SharedGoogle);
        assert_eq!(c.client_id, "gid");
    }

    #[test]
    fn shared_google_env_vars_resolve() {
        let cfg = IntegrationsConfig::default();
        let env = HashMap::from([
            ("ARAWN_GOOGLE_CLIENT_ID", "eid"),
            ("ARAWN_GOOGLE_CLIENT_SECRET", "esec"),
        ]);
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Drive)
            .unwrap();
        assert_eq!(c.tier, ClientTier::SharedGoogle);
        assert_eq!(
            c.client_id_origin,
            FieldOrigin::Env("ARAWN_GOOGLE_CLIENT_ID".into())
        );
    }

    #[test]
    fn bundled_client_is_last_resort_and_byo_overrides_it() {
        let env = HashMap::new();
        let empty = IntegrationsConfig::default();
        let c = resolver(&empty, &env, atlassian_bundled)
            .resolve(OAuthProvider::Atlassian)
            .unwrap();
        assert_eq!(c.tier, ClientTier::Bundled);
        assert_eq!(c.client_id, "bundled-atl");
        assert_eq!(c.client_secret, None);

        let byo = IntegrationsConfig {
            atlassian: creds("mine", "mysecret"),
            ..Default::default()
        };
        let c = resolver(&byo, &env, atlassian_bundled)
            .resolve(OAuthProvider::Atlassian)
            .unwrap();
        assert_eq!(c.tier, ClientTier::Service);
        assert_eq!(c.client_id, "mine");
    }

    #[test]
    fn this_build_bundles_no_clients() {
        for p in OAuthProvider::ALL {
            assert!(bundled_client(p).is_none(), "{p:?} unexpectedly bundled");
        }
    }

    #[test]
    fn service_names_match_integration_crates() {
        let names: Vec<_> = OAuthProvider::ALL
            .iter()
            .map(|p| p.service_name())
            .collect();
        assert_eq!(
            names,
            [
                "gmail",
                "google_calendar",
                "google_drive",
                "atlassian",
                "slack"
            ]
        );
    }

    #[test]
    fn github_resolves_path_from_config_and_inline_pem_wins() {
        let cfg = IntegrationsConfig {
            github: GithubAppCredentials {
                app_id: "42".into(),
                app_slug: "arawn-me".into(),
                private_key_path: "/k.pem".into(),
            },
            ..Default::default()
        };
        let env = HashMap::new();
        let g = resolver(&cfg, &env, no_bundled).resolve_github().unwrap();
        assert_eq!(g.app_id, "42");
        assert_eq!(
            g.key,
            GithubKeySource::Path {
                path: "/k.pem".into(),
                origin: FieldOrigin::Config("integrations.github.private_key_path".into()),
            }
        );

        let env = HashMap::from([("ARAWN_GITHUB_PRIVATE_KEY_PEM", "PEMBODY")]);
        let g = resolver(&cfg, &env, no_bundled).resolve_github().unwrap();
        assert_eq!(g.key, GithubKeySource::InlinePem("PEMBODY".into()));
        assert_eq!(g.load_private_key().unwrap(), "PEMBODY");
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let cfg = IntegrationsConfig {
            slack: creds("sid", "TOPSECRET"),
            ..Default::default()
        };
        let env = HashMap::new();
        let c = resolver(&cfg, &env, no_bundled)
            .resolve(OAuthProvider::Slack)
            .unwrap();
        assert!(!format!("{c:?}").contains("TOPSECRET"));
        let pem = GithubKeySource::InlinePem("-----BEGIN KEY-----".into());
        assert!(!format!("{pem:?}").contains("BEGIN"));
    }

    #[test]
    fn github_without_key_source_is_unresolved() {
        let cfg = IntegrationsConfig {
            github: GithubAppCredentials {
                app_id: "42".into(),
                app_slug: "arawn-me".into(),
                private_key_path: String::new(),
            },
            ..Default::default()
        };
        let env = HashMap::new();
        assert!(resolver(&cfg, &env, no_bundled).resolve_github().is_none());
    }
}

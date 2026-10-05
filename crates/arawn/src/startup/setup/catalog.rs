//! Provider guides for `arawn setup` (ARAWN-T-0501).
//!
//! Each guide is the short form of the matching how-to page under
//! `docs/src/how-to/connect-*.md`. Scope strings and redirect ports come
//! from the integration crates' constants, never retyped, so the guide
//! cannot drift from what `/connect` actually requests.

use crate::oauth_clients::OAuthProvider;

/// What `arawn setup` can configure. Google is one target: one OAuth
/// client covers Gmail, Calendar and Drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupTarget {
    Google,
    Slack,
    Atlassian,
    Github,
}

impl SetupTarget {
    pub const ALL: [SetupTarget; 4] = [
        SetupTarget::Google,
        SetupTarget::Slack,
        SetupTarget::Atlassian,
        SetupTarget::Github,
    ];

    /// The CLI argument and the `[integrations.<key>]` table name.
    pub fn key(self) -> &'static str {
        match self {
            SetupTarget::Google => "google",
            SetupTarget::Slack => "slack",
            SetupTarget::Atlassian => "atlassian",
            SetupTarget::Github => "github",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            SetupTarget::Google => "Google (Gmail, Calendar, Drive)",
            SetupTarget::Slack => "Slack",
            SetupTarget::Atlassian => "Atlassian (Jira, Confluence)",
            SetupTarget::Github => "GitHub",
        }
    }

    pub fn from_key(key: &str) -> Option<SetupTarget> {
        let key = key.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|t| t.key() == key)
            .or(match key.as_str() {
                "gmail" | "calendar" | "google_calendar" | "drive" | "google_drive" => {
                    Some(SetupTarget::Google)
                }
                "jira" | "confluence" => Some(SetupTarget::Atlassian),
                _ => None,
            })
    }

    /// The OAuth integrations this target enables. Empty for GitHub,
    /// which uses the App model.
    pub fn covers(self) -> &'static [OAuthProvider] {
        match self {
            SetupTarget::Google => &[
                OAuthProvider::Gmail,
                OAuthProvider::Calendar,
                OAuthProvider::Drive,
            ],
            SetupTarget::Slack => &[OAuthProvider::Slack],
            SetupTarget::Atlassian => &[OAuthProvider::Atlassian],
            SetupTarget::Github => &[],
        }
    }

    /// The env var that holds the client secret when the user keeps it
    /// out of `arawn.toml`. `None` for GitHub.
    pub fn secret_env_var(self) -> Option<&'static str> {
        match self {
            SetupTarget::Google => Some("ARAWN_GOOGLE_CLIENT_SECRET"),
            SetupTarget::Slack => Some("ARAWN_SLACK_CLIENT_SECRET"),
            SetupTarget::Atlassian => Some("ARAWN_ATLASSIAN_CLIENT_SECRET"),
            SetupTarget::Github => None,
        }
    }

    /// Service names to give to `/connect` after setup.
    pub fn connect_names(self) -> Vec<&'static str> {
        match self {
            SetupTarget::Github => vec![arawn_integrations::github::SERVICE_NAME],
            t => t.covers().iter().map(|p| p.service_name()).collect(),
        }
    }
}

/// A labelled list of scopes or permissions to add in the console.
#[derive(Debug, Clone)]
pub struct ScopeGroup {
    pub label: &'static str,
    pub items: Vec<String>,
}

/// The console walkthrough for one target.
#[derive(Debug, Clone)]
pub struct Guide {
    pub console_url: &'static str,
    pub steps: Vec<String>,
    pub scope_groups: Vec<ScopeGroup>,
    /// The full how-to, relative to the repository root.
    pub doc: &'static str,
}

fn redirect_uri(port: u16) -> String {
    // The callback server emits `localhost`, not 127.0.0.1 — Slack and
    // Atlassian string-match the URI (arawn-auth/src/server.rs).
    format!("http://localhost:{port}/oauth/callback")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

pub fn guide(target: SetupTarget) -> Guide {
    match target {
        SetupTarget::Google => {
            let mut scopes = strings(arawn_integrations::gmail::GMAIL_OAUTH_SCOPES);
            scopes.push(arawn_integrations::calendar::CALENDAR_OAUTH_SCOPE.into());
            scopes.push(arawn_integrations::drive::DRIVE_OAUTH_SCOPE.into());
            Guide {
                console_url: "https://console.cloud.google.com/",
                steps: vec![
                    "Create a Google Cloud project, or select one.".into(),
                    "Enable the Gmail API, the Google Calendar API and the Google Drive API \
                     (APIs & Services > Library). Enable only the services that you use."
                        .into(),
                    "Go to Google Auth Platform > Branding. Set the user type to External. \
                     Give an app name and your email."
                        .into(),
                    "Go to Google Auth Platform > Data Access > Add or Remove Scopes. Add the \
                     scopes below. If a scope is not in the list, paste it into \
                     \"Manually add scopes\"."
                        .into(),
                    "Go to Google Auth Platform > Audience > Test users. Add the Google \
                     account that you will connect."
                        .into(),
                    "Go to APIs & Services > Credentials > Create Credentials > OAuth client \
                     ID. Set the application type to Desktop app. Copy the client ID and the \
                     client secret."
                        .into(),
                ],
                scope_groups: vec![ScopeGroup {
                    label: "OAuth scopes",
                    items: scopes,
                }],
                doc: "docs/src/how-to/connect-google.md",
            }
        }
        SetupTarget::Slack => Guide {
            console_url: "https://api.slack.com/apps",
            steps: vec![
                "Click Create New App > From scratch. Select your workspace.".into(),
                "Go to OAuth & Permissions. Add the bot token scopes and the user token \
                 scopes below."
                    .into(),
                format!(
                    "In OAuth & Permissions > Redirect URLs, add exactly {}. Use localhost, \
                     not 127.0.0.1.",
                    redirect_uri(arawn_integrations::slack::DEFAULT_SLACK_REDIRECT_PORT)
                ),
                "Go to Install App > Install to Workspace. If the button is not available, \
                 a workspace admin must approve the app."
                    .into(),
                "Go to Basic Information > App Credentials. Copy the client ID and the client \
                 secret."
                    .into(),
            ],
            scope_groups: vec![
                ScopeGroup {
                    label: "Bot token scopes",
                    items: strings(arawn_integrations::slack::SLACK_OAUTH_SCOPES),
                },
                ScopeGroup {
                    label: "User token scopes",
                    items: strings(arawn_integrations::slack::SLACK_OAUTH_USER_SCOPES),
                },
            ],
            doc: "docs/src/how-to/connect-slack.md",
        },
        SetupTarget::Atlassian => Guide {
            console_url: "https://developer.atlassian.com/console/myapps/",
            steps: vec![
                "Click Create > OAuth 2.0 integration. Give a name.".into(),
                "Go to Permissions. Add the Jira API, the Confluence API and the User \
                 identity API."
                    .into(),
                "In the Configure page of each API, add the scopes below. For Confluence, \
                 add the classic scopes and the granular scopes."
                    .into(),
                format!(
                    "Go to Authorization > OAuth 2.0 (3LO). Set the callback URL to exactly \
                     {}.",
                    redirect_uri(arawn_integrations::atlassian::DEFAULT_ATLASSIAN_REDIRECT_PORT)
                ),
                "Go to Settings. Copy the client ID and the secret.".into(),
            ],
            scope_groups: vec![ScopeGroup {
                label: "OAuth scopes",
                items: strings(arawn_integrations::atlassian::ATLASSIAN_OAUTH_SCOPES),
            }],
            doc: "docs/src/how-to/connect-atlassian.md",
        },
        SetupTarget::Github => Guide {
            console_url: "https://github.com/settings/apps/new",
            steps: vec![
                "Create a GitHub App. For an organization app, use Settings > Developer \
                 settings > GitHub Apps > New GitHub App in the organization."
                    .into(),
                "Set the Setup URL to http://localhost/oauth/callback. Turn off the webhook."
                    .into(),
                "Give the repository and account permissions below. Subscribe to no events.".into(),
                "Click Create GitHub App. Record the App ID and the slug from the app URL \
                 (github.com/apps/<slug>)."
                    .into(),
                "Click Generate a private key. Keep the downloaded .pem file in a safe \
                 location."
                    .into(),
            ],
            scope_groups: vec![ScopeGroup {
                label: "Permissions (all read-only)",
                items: strings(&[
                    "Repository > Contents",
                    "Repository > Issues",
                    "Repository > Pull requests",
                    "Repository > Metadata (mandatory)",
                    "Account > Email addresses (optional)",
                ]),
            }],
            doc: "docs/src/how-to/connect-github.md",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_aliases_resolve() {
        for t in SetupTarget::ALL {
            assert_eq!(SetupTarget::from_key(t.key()), Some(t));
        }
        assert_eq!(SetupTarget::from_key("Gmail"), Some(SetupTarget::Google));
        assert_eq!(SetupTarget::from_key("jira"), Some(SetupTarget::Atlassian));
        assert_eq!(SetupTarget::from_key("linear"), None);
    }

    #[test]
    fn google_guide_lists_every_scope_the_integrations_request() {
        let g = guide(SetupTarget::Google);
        let scopes = &g.scope_groups[0].items;
        for s in arawn_integrations::gmail::GMAIL_OAUTH_SCOPES {
            assert!(scopes.iter().any(|x| x == s), "missing {s}");
        }
        assert!(scopes.contains(&arawn_integrations::calendar::CALENDAR_OAUTH_SCOPE.into()));
        assert!(scopes.contains(&arawn_integrations::drive::DRIVE_OAUTH_SCOPE.into()));
    }

    #[test]
    fn slack_and_atlassian_guides_use_crate_scopes_and_localhost_redirect() {
        let s = guide(SetupTarget::Slack);
        assert_eq!(
            s.scope_groups[0].items,
            strings(arawn_integrations::slack::SLACK_OAUTH_SCOPES)
        );
        assert_eq!(
            s.scope_groups[1].items,
            strings(arawn_integrations::slack::SLACK_OAUTH_USER_SCOPES)
        );
        assert!(
            s.steps
                .iter()
                .any(|l| l.contains("http://localhost:8080/oauth/callback"))
        );

        let a = guide(SetupTarget::Atlassian);
        assert_eq!(
            a.scope_groups[0].items,
            strings(arawn_integrations::atlassian::ATLASSIAN_OAUTH_SCOPES)
        );
        assert!(
            a.steps
                .iter()
                .any(|l| l.contains("http://localhost:8080/oauth/callback"))
        );
    }

    #[test]
    fn guide_docs_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for t in SetupTarget::ALL {
            let doc = root.join(guide(t).doc);
            assert!(doc.exists(), "{} missing", doc.display());
        }
    }

    #[test]
    fn connect_names_match_service_names() {
        assert_eq!(
            SetupTarget::Google.connect_names(),
            ["gmail", "google_calendar", "google_drive"]
        );
        assert_eq!(SetupTarget::Github.connect_names(), ["github"]);
    }
}

//! OAuth integration registration. Extracted from `main.rs` as part
//! of I-0054 T-E. Each provider is resolved from env-var + config and
//! registered into `service` + `registry`; the function returns handles
//! for the integrations the continual-feeds block needs downstream.

use std::sync::Arc;
use std::sync::RwLock;

use tracing::{debug, info, warn};

use crate::{ArawnConfig, LocalService};

/// Handles to the integrations that the continual-feeds setup needs.
/// `None` means the corresponding OAuth credentials were absent or
/// resolution failed; downstream feed templates will be skipped.
pub struct IntegrationsForFeeds {
    pub atlassian: Option<Arc<arawn_integrations::atlassian::AtlassianIntegration>>,
    pub calendar: Option<Arc<arawn_integrations::calendar::GoogleCalendarIntegration>>,
    pub drive: Option<Arc<arawn_integrations::drive::GoogleDriveIntegration>>,
    pub github: Option<Arc<arawn_integrations::github::GithubIntegration>>,
    pub gmail: Option<Arc<arawn_integrations::gmail::GmailIntegration>>,
    pub slack: Option<Arc<arawn_integrations::slack::SlackIntegration>>,
}

/// Register Gmail, Calendar, Drive, Atlassian, GitHub, and Slack integrations.
/// Each provider is skipped silently if credentials are not configured.
pub fn wire_integrations(
    config: &ArawnConfig,
    data_dir: &str,
    service: &mut LocalService,
    registry: &Arc<arawn_engine::ToolRegistry>,
    github_for_bind_hook: &Arc<RwLock<Option<Arc<arawn_integrations::github::GithubIntegration>>>>,
) -> IntegrationsForFeeds {
    // Resolve OAuth credentials with precedence:
    //   env var → arawn.toml `[integrations.<service>]` → empty (skip).
    // This lets users persist creds in config without exporting env
    // vars on every shell, while keeping env-var override for ad-hoc
    // testing (different OAuth client per run, etc.).
    let resolve = |env_id: &str,
                   env_secret: &str,
                   cfg: &crate::config::IntegrationCredentials|
     -> Option<(String, String)> {
        let id = std::env::var(env_id)
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(cfg.client_id.clone()).filter(|s| !s.is_empty()))?;
        let secret = std::env::var(env_secret)
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(cfg.client_secret.clone()).filter(|s| !s.is_empty()))?;
        Some((id, secret))
    };

    // Register Gmail integration if creds are present (env or config).
    // Skipped silently otherwise — users without Gmail credentials still
    // get a working server. See docs/src/integrations/gmail.md.
    let gmail_creds = resolve(
        "ARAWN_GMAIL_CLIENT_ID",
        "ARAWN_GMAIL_CLIENT_SECRET",
        &config.integrations.gmail,
    )
    .or_else(|| {
        // Fall back to the shared Google credentials.
        resolve(
            "ARAWN_GOOGLE_CLIENT_ID",
            "ARAWN_GOOGLE_CLIENT_SECRET",
            &config.integrations.google,
        )
    });
    let gmail_integration_for_feeds: Option<Arc<arawn_integrations::gmail::GmailIntegration>>;
    if let Some((client_id, client_secret)) = gmail_creds {
        let gmail = Arc::new(arawn_integrations::gmail::GmailIntegration::new(
            std::path::PathBuf::from(&data_dir),
            client_id,
            client_secret,
        ));
        service
            .register_integration(Arc::clone(&gmail) as Arc<dyn arawn_integrations::Integration>);
        registry.register(Box::new(
            arawn_integrations::gmail::GmailInboxReadTool::new(Arc::clone(&gmail)),
        ));
        registry.register(Box::new(arawn_integrations::gmail::GmailSearchTool::new(
            Arc::clone(&gmail),
        )));
        registry.register(Box::new(
            arawn_integrations::gmail::GmailGetMessageTool::new(Arc::clone(&gmail)),
        ));
        registry.register(Box::new(arawn_integrations::gmail::GmailSendTool::new(
            Arc::clone(&gmail),
        )));
        registry.register(Box::new(arawn_integrations::gmail::GmailMarkReadTool::new(
            Arc::clone(&gmail),
        )));
        info!("Gmail integration registered (5 tools)");
        gmail_integration_for_feeds = Some(gmail);
    } else {
        gmail_integration_for_feeds = None;
        debug!(
            "Gmail integration skipped — set ARAWN_GMAIL_CLIENT_ID + \
             ARAWN_GMAIL_CLIENT_SECRET (env) or [integrations.gmail] (config) \
             to enable. See docs/src/integrations/gmail.md."
        );
    }

    // Register Google Calendar. Service-specific creds first; falls back
    // to the shared Google credentials so one OAuth project covers both.
    let gcal_creds = resolve(
        "ARAWN_GCAL_CLIENT_ID",
        "ARAWN_GCAL_CLIENT_SECRET",
        &config.integrations.calendar,
    )
    .or_else(|| {
        resolve(
            "ARAWN_GOOGLE_CLIENT_ID",
            "ARAWN_GOOGLE_CLIENT_SECRET",
            &config.integrations.google,
        )
    });
    let calendar_integration_for_feeds: Option<
        Arc<arawn_integrations::calendar::GoogleCalendarIntegration>,
    >;
    if let Some((client_id, client_secret)) = gcal_creds {
        let calendar = Arc::new(
            arawn_integrations::calendar::GoogleCalendarIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ),
        );
        service.register_integration(
            Arc::clone(&calendar) as Arc<dyn arawn_integrations::Integration>
        );
        registry.register(Box::new(
            arawn_integrations::calendar::CalendarUpcomingTool::new(Arc::clone(&calendar)),
        ));
        registry.register(Box::new(
            arawn_integrations::calendar::CalendarCreateEventTool::new(Arc::clone(&calendar)),
        ));
        registry.register(Box::new(
            arawn_integrations::calendar::CalendarFindConflictsTool::new(Arc::clone(&calendar)),
        ));
        info!("Google Calendar integration registered (3 tools)");
        calendar_integration_for_feeds = Some(calendar);
    } else {
        calendar_integration_for_feeds = None;
        debug!(
            "Google Calendar integration skipped — set ARAWN_GCAL_CLIENT_ID + \
             ARAWN_GCAL_CLIENT_SECRET (env) or [integrations.calendar] / \
             [integrations.google] (config) to enable."
        );
    }

    // Register Google Drive. Same fallback chain as Calendar — service-specific
    // creds first, then the shared Google credentials.
    let drive_creds = resolve(
        "ARAWN_GDRIVE_CLIENT_ID",
        "ARAWN_GDRIVE_CLIENT_SECRET",
        &config.integrations.drive,
    )
    .or_else(|| {
        resolve(
            "ARAWN_GOOGLE_CLIENT_ID",
            "ARAWN_GOOGLE_CLIENT_SECRET",
            &config.integrations.google,
        )
    });
    let drive_integration_for_feeds: Option<
        Arc<arawn_integrations::drive::GoogleDriveIntegration>,
    >;
    if let Some((client_id, client_secret)) = drive_creds {
        let drive = Arc::new(arawn_integrations::drive::GoogleDriveIntegration::new(
            std::path::PathBuf::from(&data_dir),
            client_id,
            client_secret,
        ));
        service
            .register_integration(Arc::clone(&drive) as Arc<dyn arawn_integrations::Integration>);
        registry.register(Box::new(arawn_integrations::drive::DriveSearchTool::new(
            Arc::clone(&drive),
        )));
        registry.register(Box::new(arawn_integrations::drive::DriveListTool::new(
            Arc::clone(&drive),
        )));
        registry.register(Box::new(
            arawn_integrations::drive::DriveGetMetadataTool::new(Arc::clone(&drive)),
        ));
        registry.register(Box::new(arawn_integrations::drive::DriveReadTool::new(
            Arc::clone(&drive),
        )));
        registry.register(Box::new(arawn_integrations::drive::DriveUploadTool::new(
            Arc::clone(&drive),
        )));
        registry.register(Box::new(arawn_integrations::drive::DriveUpdateTool::new(
            Arc::clone(&drive),
        )));
        registry.register(Box::new(arawn_integrations::drive::DriveDeleteTool::new(
            Arc::clone(&drive),
        )));
        info!("Google Drive integration registered (7 tools)");
        drive_integration_for_feeds = Some(drive);
    } else {
        drive_integration_for_feeds = None;
        debug!(
            "Google Drive integration skipped — set ARAWN_GDRIVE_CLIENT_ID + \
             ARAWN_GDRIVE_CLIENT_SECRET (env) or [integrations.drive] / \
             [integrations.google] (config) to enable."
        );
    }

    // Register Atlassian (Jira + Confluence). One OAuth client, one
    // token; both tool families register together.
    let atlassian_integration_for_feeds: Option<
        Arc<arawn_integrations::atlassian::AtlassianIntegration>,
    >;
    if let Some((client_id, client_secret)) = resolve(
        "ARAWN_ATLASSIAN_CLIENT_ID",
        "ARAWN_ATLASSIAN_CLIENT_SECRET",
        &config.integrations.atlassian,
    ) {
        let atlassian = Arc::new(arawn_integrations::atlassian::AtlassianIntegration::new(
            std::path::PathBuf::from(&data_dir),
            client_id,
            client_secret,
        ));
        service.register_integration(
            Arc::clone(&atlassian) as Arc<dyn arawn_integrations::Integration>
        );
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraSearchTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraGetIssueTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraCreateIssueTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraUpdateIssueTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraAddCommentTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::JiraTransitionIssueTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::ConfluenceSearchTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::ConfluenceGetPageTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::ConfluenceCreatePageTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::ConfluenceUpdatePageTool::new(Arc::clone(&atlassian)),
        ));
        registry.register(Box::new(
            arawn_integrations::atlassian::ConfluenceListSpacesTool::new(Arc::clone(&atlassian)),
        ));
        info!("Atlassian integration registered (11 tools — 6 Jira, 5 Confluence)");
        // If the persisted token was minted by an older arawn
        // build that requested fewer scopes, surface that now —
        // confluence feeds will 401 with "scope does not match"
        // until the user re-runs /connect atlassian.
        if let Some(missing) = atlassian.missing_scopes() {
            warn!(
                missing = ?missing,
                "Atlassian token is missing scopes from the current build. \
                 Run `/disconnect atlassian` then `/connect atlassian` to \
                 mint a fresh token. Affected feeds (e.g. confluence/space-archive) \
                 will fail with 401 'scope does not match' until then."
            );
        }
        atlassian_integration_for_feeds = Some(atlassian);
    } else {
        atlassian_integration_for_feeds = None;
        debug!(
            "Atlassian integration skipped — set ARAWN_ATLASSIAN_CLIENT_ID + \
             ARAWN_ATLASSIAN_CLIENT_SECRET (env) or [integrations.atlassian] (config) \
             to enable."
        );
    }

    // Register GitHub (I-0045). Read-only v1 — the connect flow
    // captures an installation_id; tools/feed templates downstream
    // mint short-lived access tokens via the cached App config.
    let github_integration_for_feeds: Option<Arc<arawn_integrations::github::GithubIntegration>>;
    let resolve_github = || -> Option<arawn_integrations::github::GithubAppConfig> {
        let cfg = &config.integrations.github;
        let app_id = std::env::var("ARAWN_GITHUB_APP_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(cfg.app_id.clone()).filter(|s| !s.is_empty()))?;
        let app_slug = std::env::var("ARAWN_GITHUB_APP_SLUG")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(cfg.app_slug.clone()).filter(|s| !s.is_empty()))?;
        let private_key_pem = std::env::var("ARAWN_GITHUB_PRIVATE_KEY_PEM")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                let path = std::env::var("ARAWN_GITHUB_PRIVATE_KEY_PATH")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(cfg.private_key_path.clone()).filter(|s| !s.is_empty()))?;
                match std::fs::read_to_string(&path) {
                    Ok(pem) => Some(pem),
                    Err(e) => {
                        warn!(path = %path, error = %e,
                            "GitHub App private key path unreadable; skipping integration");
                        None
                    }
                }
            })?;
        Some(arawn_integrations::github::GithubAppConfig {
            app_id,
            app_slug,
            private_key_pem,
        })
    };
    if let Some(app_cfg) = resolve_github() {
        let github = Arc::new(arawn_integrations::github::GithubIntegration::new(
            std::path::PathBuf::from(&data_dir),
            app_cfg,
        ));
        service
            .register_integration(Arc::clone(&github) as Arc<dyn arawn_integrations::Integration>);
        info!("GitHub integration registered (read-only — no tools yet, feeds land in T-0319+)");
        // I-0050 T-0327 — wire the late-bound cell so the bind hook
        // can run list_org_repos expansion when github:org:owner
        // bindings land.
        *github_for_bind_hook.write().unwrap() = Some(Arc::clone(&github));
        github_integration_for_feeds = Some(github);
    } else {
        github_integration_for_feeds = None;
        debug!(
            "GitHub integration skipped — set ARAWN_GITHUB_APP_ID + \
             ARAWN_GITHUB_APP_SLUG + ARAWN_GITHUB_PRIVATE_KEY_PATH (env) or \
             [integrations.github] (config) to enable. See \
             docs/src/integrations/github.md."
        );
    }
    // Register Slack. No sharing with Google — different OAuth ecosystem.
    let slack_integration_for_feeds: Option<Arc<arawn_integrations::slack::SlackIntegration>>;
    if let Some((client_id, client_secret)) = resolve(
        "ARAWN_SLACK_CLIENT_ID",
        "ARAWN_SLACK_CLIENT_SECRET",
        &config.integrations.slack,
    ) {
        let slack = Arc::new(arawn_integrations::slack::SlackIntegration::new(
            std::path::PathBuf::from(&data_dir),
            client_id,
            client_secret,
        ));
        service
            .register_integration(Arc::clone(&slack) as Arc<dyn arawn_integrations::Integration>);
        registry.register(Box::new(
            arawn_integrations::slack::SlackListChannelsTool::new(Arc::clone(&slack)),
        ));
        registry.register(Box::new(arawn_integrations::slack::SlackHistoryTool::new(
            Arc::clone(&slack),
        )));
        registry.register(Box::new(arawn_integrations::slack::SlackPostTool::new(
            Arc::clone(&slack),
        )));
        registry.register(Box::new(arawn_integrations::slack::SlackReactTool::new(
            Arc::clone(&slack),
        )));
        registry.register(Box::new(
            arawn_integrations::slack::SlackUsersListTool::new(Arc::clone(&slack)),
        ));
        registry.register(Box::new(arawn_integrations::slack::SlackOpenDmTool::new(
            Arc::clone(&slack),
        )));
        info!("Slack integration registered (6 tools)");
        slack_integration_for_feeds = Some(slack);
    } else {
        slack_integration_for_feeds = None;
        debug!(
            "Slack integration skipped — set ARAWN_SLACK_CLIENT_ID + \
             ARAWN_SLACK_CLIENT_SECRET (env) or [integrations.slack] (config) \
             to enable. See docs/src/integrations/slack.md."
        );
    }

    IntegrationsForFeeds {
        atlassian: atlassian_integration_for_feeds,
        calendar: calendar_integration_for_feeds,
        drive: drive_integration_for_feeds,
        github: github_integration_for_feeds,
        gmail: gmail_integration_for_feeds,
        slack: slack_integration_for_feeds,
    }
}

/// ARAWN-I-0062 T-A: register lifecycle-level `UatMockIntegration`s based on
/// the `ARAWN_UAT_MOCK_INTEGRATIONS` env var (comma-separated service slugs).
/// This flips `LocalService::connected_services` for the listed services so
/// `query_engine::filter_tools_for_context` will include their categories.
///
/// ARAWN-I-0062 T-B: for services with projection-backed UAT tool families
/// (currently `google_calendar`), this also registers the UAT tool impls into
/// the engine's tool registry — so the agent can actually call them. Each
/// provider grows its own set of UAT tools under
/// `arawn_integrations::<service>::uat_tools`.
pub fn wire_uat_mock_integrations(
    data_dir: &str,
    service: &mut LocalService,
    registry: &Arc<arawn_engine::ToolRegistry>,
) {
    let Ok(list) = std::env::var("ARAWN_UAT_MOCK_INTEGRATIONS") else {
        return;
    };
    let data_dir_path = std::path::PathBuf::from(data_dir);
    for raw in list.split(',') {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        let mock = Arc::new(arawn_integrations::UatMockIntegration::new(name));
        service.register_integration(mock as Arc<dyn arawn_integrations::Integration>);
        info!(service = %name, "registered UAT mock integration");

        // Per-service UAT tool wiring. Production tools are constructed from
        // typed integrations + credentials; the UAT variants are credential-
        // free and read from the seeded projection store.
        match name {
            "google_calendar" => {
                for tool in arawn_integrations::calendar::uat_calendar_tools(data_dir_path.clone())
                {
                    registry.register(tool);
                }
                info!("registered UAT calendar tools (1)");
            }
            "gmail" => {
                for tool in arawn_integrations::gmail::uat_gmail_tools(data_dir_path.clone()) {
                    registry.register(tool);
                }
                info!("registered UAT gmail tools (2)");
            }
            "slack" => {
                for tool in arawn_integrations::slack::uat_slack_tools(data_dir_path.clone()) {
                    registry.register(tool);
                }
                info!("registered UAT slack tools (2)");
            }
            _ => {
                debug!(
                    service = %name,
                    "no projection-backed UAT tools for this service yet",
                );
            }
        }
    }
}

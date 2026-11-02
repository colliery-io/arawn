//! `arawn connect` / `arawn disconnect` — run an integration's OAuth flow
//! from the CLI (ARAWN-T-0503). The complement to the TUI's `/connect`
//! that ADR ARAWN-A-0001 left room for.
//!
//! The CLI talks to the running server: it calls `start_oauth_flow`,
//! opens the returned URL in the browser, then waits for the server's
//! `integration` notice that says the flow succeeded or failed. The OAuth
//! callback, token exchange and token storage all stay on the server.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;

use crate::integration_state::{IntegrationState, inspect};
use crate::oauth_clients::OAuthProvider;
use crate::startup::setup::catalog::SetupTarget;

/// One row of the server's integration registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    pub name: String,
    pub connected: bool,
}

/// A server notice: `category` + `message`.
#[derive(Debug, Clone)]
pub struct Notice {
    pub category: String,
    pub level: String,
    pub message: String,
}

/// What `arawn connect` needs from the server. Implemented over the
/// WebSocket client; tests use a scripted fake.
#[async_trait(?Send)]
pub trait ConnectTransport {
    async fn list_integrations(&mut self) -> Result<Vec<Registered>>;
    /// Start the flow; returns the URL the user must open.
    async fn start_oauth_flow(&mut self, service: &str) -> Result<String>;
    async fn disconnect(&mut self, service: &str) -> Result<()>;
    /// The next server notice, or `None` when the connection ends.
    async fn next_notice(&mut self) -> Option<Notice>;
}

/// Options for `arawn connect`.
pub struct ConnectOptions {
    /// Service names or setup targets (`google`, `jira`, …).
    pub services: Vec<String>,
    /// Connect every loaded, unconnected integration.
    pub all: bool,
    /// How long to wait for each browser flow.
    pub timeout: Duration,
    /// After a success, how long to wait for the default-feed notice.
    /// [`FEED_NOTICE_GRACE`] in production.
    pub feed_grace: Duration,
}

/// Where the CLI finds things: `arawn.toml` is read from `config`; the
/// server's token and token store are in `data` (they differ when
/// `[storage].data_dir` points elsewhere).
#[derive(Debug, Clone, Copy)]
pub struct Dirs<'a> {
    pub config: &'a Path,
    pub data: &'a Path,
}

/// Every integration name the server can register.
fn known_service_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = OAuthProvider::ALL
        .iter()
        .map(|p| p.service_name())
        .collect();
    v.push(arawn_integrations::github::SERVICE_NAME);
    v
}

/// Turn the user's names into registry names. An exact service name
/// (`gmail`) always means only that service, loaded or not. `google`
/// expands to the three Google services, `calendar`/`drive` to one, and
/// `jira`/`confluence` mean `atlassian`.
fn expand(
    requested: &[String],
    all: bool,
    registry: &[Registered],
) -> std::result::Result<Vec<String>, Vec<String>> {
    if all {
        return Ok(registry
            .iter()
            .filter(|r| !r.connected)
            .map(|r| r.name.clone())
            .collect());
    }
    let known = known_service_names();
    let mut out = Vec::new();
    let mut unknown = Vec::new();
    for raw in requested {
        let name = raw.trim().to_ascii_lowercase();
        let names: Vec<String> =
            if registry.iter().any(|r| r.name == name) || known.contains(&name.as_str()) {
                vec![name.clone()]
            } else if name == "calendar" {
                vec![arawn_integrations::calendar::SERVICE_NAME.into()]
            } else if name == "drive" {
                vec![arawn_integrations::drive::SERVICE_NAME.into()]
            } else if let Some(t) = SetupTarget::from_key(&name) {
                t.connect_names().into_iter().map(String::from).collect()
            } else {
                unknown.push(raw.clone());
                continue;
            };
        for n in names {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    if unknown.is_empty() {
        Ok(out)
    } else {
        Err(unknown)
    }
}

/// Why a service the user asked for is not loaded by the server, with the
/// fix. Reads `arawn.toml` + env of this shell.
fn not_loaded_reason(dirs: Dirs<'_>, service: &str) -> String {
    let cfg = match crate::ArawnConfig::try_load(dirs.config) {
        Ok(c) => c,
        Err(e) => return format!("arawn.toml does not load ({e})"),
    };
    let report = inspect(&cfg.integrations, dirs.data)
        .ok()
        .and_then(|rs| rs.into_iter().find(|r| r.service == service));
    match report {
        Some(r) => match r.state {
            IntegrationState::Configured { .. } | IntegrationState::Connected { .. } => {
                "it is set up in arawn.toml, but the running server did not load it. \
                 Restart arawn serve"
                    .into()
            }
            _ => match r.hint() {
                Some(h) => format!("{}. Fix: {h}", r.describe()),
                None => r.describe(),
            },
        },
        None => "the server does not know it".into(),
    }
}

/// How long to wait, after a successful connect, for the notice about the
/// auto-created default feed.
pub const FEED_NOTICE_GRACE: Duration = Duration::from_secs(2);

/// Wait for the `integration` notice about `service`. Returns the error
/// text on failure. Other notices are printed when they matter.
async fn await_outcome(
    t: &mut dyn ConnectTransport,
    service: &str,
    opts: &ConnectOptions,
    out: &mut dyn Write,
) -> Result<std::result::Result<(), String>> {
    let timeout = opts.timeout;
    let ok = format!("{service} connected");
    let failed = format!("{service} connection FAILED: ");
    let wait = async {
        while let Some(n) = t.next_notice().await {
            match n.category.as_str() {
                "integration" if n.message == ok => return Some(Ok(())),
                "integration" => {
                    if let Some(err) = n.message.strip_prefix(&failed) {
                        return Some(Err(err.to_string()));
                    }
                }
                // Default feed auto-created after the connect.
                "feeds" => {
                    let _ = writeln!(out, "  {}", n.message);
                }
                _ => {}
            }
        }
        None
    };
    match tokio::time::timeout(timeout, wait).await {
        Ok(Some(Ok(()))) => {
            // The server auto-creates the service's default feed right
            // AFTER the "connected" notice. Listen briefly so it is shown.
            let _ = tokio::time::timeout(opts.feed_grace, async {
                while let Some(n) = t.next_notice().await {
                    if n.category == "feeds" && n.message.contains(service) {
                        let _ = writeln!(out, "  {}", n.message);
                        break;
                    }
                }
            })
            .await;
            Ok(Ok(()))
        }
        Ok(Some(r)) => Ok(r),
        Ok(None) => bail!("the server closed the connection before {service} finished"),
        Err(_) => Ok(Err(format!(
            "no answer after {}s. Complete the browser step, then run: arawn connect {service}",
            timeout.as_secs()
        ))),
    }
}

/// Run `arawn connect`. `open_url` opens a URL in the browser and
/// returns false when it cannot.
pub async fn run_connect(
    t: &mut dyn ConnectTransport,
    dirs: Dirs<'_>,
    opts: &ConnectOptions,
    open_url: &dyn Fn(&str) -> bool,
    out: &mut dyn Write,
) -> Result<()> {
    if opts.services.is_empty() && !opts.all {
        bail!("name a service (for example: arawn connect gmail), or give --all");
    }
    let registry = t.list_integrations().await?;
    let services = expand(&opts.services, opts.all, &registry).map_err(|unknown| {
        let mut known: Vec<_> = registry.iter().map(|r| r.name.as_str()).collect();
        known.sort();
        anyhow!(
            "unknown service: {}. The server has: {}",
            unknown.join(", "),
            if known.is_empty() {
                "none".to_string()
            } else {
                known.join(", ")
            }
        )
    })?;

    if services.is_empty() {
        if registry.is_empty() {
            writeln!(out, "No integrations are set up. Run: arawn setup")?;
        } else {
            writeln!(out, "Every loaded integration is already connected.")?;
        }
        return Ok(());
    }

    let mut failures = Vec::new();
    for service in &services {
        let Some(row) = registry.iter().find(|r| &r.name == service) else {
            let why = not_loaded_reason(dirs, service);
            writeln!(out, "{service}: not available — {why}.")?;
            failures.push(service.clone());
            continue;
        };
        if row.connected {
            writeln!(
                out,
                "{service}: already connected. To reconnect: arawn disconnect {service}, then \
                 arawn connect {service}"
            )?;
            continue;
        }

        // A failed start is recorded like any other failure, so --all
        // still tries the remaining services.
        let url = match t.start_oauth_flow(service).await {
            Ok(u) => u,
            Err(e) => {
                writeln!(out, "{service}: FAILED — cannot start the flow: {e:#}")?;
                failures.push(service.clone());
                continue;
            }
        };
        writeln!(out, "{service}: approve access in the browser.")?;
        if !open_url(&url) {
            writeln!(out, "  Open this URL in a browser:")?;
        } else {
            writeln!(out, "  If the browser did not open, open this URL:")?;
        }
        writeln!(out, "  {url}")?;
        out.flush()?;

        match await_outcome(t, service, opts, out).await? {
            Ok(()) => writeln!(out, "{service}: connected.")?,
            Err(e) => {
                writeln!(out, "{service}: FAILED — {e}")?;
                failures.push(service.clone());
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        bail!("not connected: {}", failures.join(", "))
    }
}

/// Run `arawn disconnect <service>`.
pub async fn run_disconnect(
    t: &mut dyn ConnectTransport,
    service: &str,
    out: &mut dyn Write,
) -> Result<()> {
    let registry = t.list_integrations().await?;
    let names = expand(&[service.to_string()], false, &registry)
        .map_err(|_| anyhow!("unknown service: {service}"))?;
    for name in names {
        if !registry.iter().any(|r| r.name == name) {
            writeln!(
                out,
                "{name}: not loaded by the server; nothing to disconnect."
            )?;
            continue;
        }
        t.disconnect(&name).await?;
        writeln!(out, "{name}: disconnected. The stored token is deleted.")?;
    }
    Ok(())
}

/// Open a URL with the platform's opener. False when there is none or it
/// fails to start.
pub fn open_in_browser(url: &str) -> bool {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/c", "start", ""]);
        c
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open")
    } else {
        return false;
    };
    cmd.arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

/// [`ConnectTransport`] over the server's WebSocket.
pub struct WsTransport {
    client: arawn_tui::ws_client::WsClient,
    events: tokio::sync::mpsc::Receiver<arawn_tui::ws_client::WsEvent>,
}

impl WsTransport {
    /// Connect with the token from `<data_dir>/server.token`.
    pub async fn connect(url: &str, data_dir: &Path) -> Result<Self> {
        let token = crate::startup::helpers::read_server_token(data_dir);
        let mut client = arawn_tui::ws_client::WsClient::connect_with_token(url, token.as_deref())
            .await
            .map_err(|e| {
                anyhow!(
                    "cannot connect to the arawn server at {url}: {e}. \
                         Start it first: arawn serve"
                )
            })?;
        let events = client
            .events_take()
            .ok_or_else(|| anyhow!("internal error: event channel already taken"))?;
        Ok(Self { client, events })
    }

    async fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let resp = self
            .client
            .request_response(method, params)
            .await
            .map_err(|e| anyhow!("{method}: {e}"))?;
        if let Some(err) = resp.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            bail!("{msg}");
        }
        Ok(resp
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
}

#[async_trait(?Send)]
impl ConnectTransport for WsTransport {
    async fn list_integrations(&mut self) -> Result<Vec<Registered>> {
        let v = self
            .call("list_integrations", serde_json::json!({}))
            .await?;
        let rows: Vec<arawn_service::IntegrationStatus> = serde_json::from_value(v)?;
        Ok(rows
            .into_iter()
            .map(|r| Registered {
                name: r.name,
                connected: r.connected,
            })
            .collect())
    }

    async fn start_oauth_flow(&mut self, service: &str) -> Result<String> {
        let v = self
            .call(
                "start_oauth_flow",
                serde_json::json!({ "service": service }),
            )
            .await?;
        v.get("auth_url")
            .and_then(|u| u.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow!("the server returned no auth URL"))
    }

    async fn disconnect(&mut self, service: &str) -> Result<()> {
        self.call(
            "disconnect_integration",
            serde_json::json!({ "service": service }),
        )
        .await?;
        Ok(())
    }

    async fn next_notice(&mut self) -> Option<Notice> {
        use arawn_tui::ws_client::WsEvent;
        loop {
            match self.events.recv().await? {
                WsEvent::Text(text) => {
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                        continue;
                    };
                    if v.get("event").and_then(|e| e.as_str()) != Some("SystemNotice") {
                        continue;
                    }
                    let d = &v["data"];
                    let s = |k: &str| d.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    return Some(Notice {
                        category: s("category"),
                        level: s("level"),
                        message: s("message"),
                    });
                }
                WsEvent::Closed | WsEvent::Error(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Scripted server: a fixed registry, and for each started flow the
    /// notices it will emit.
    struct Fake {
        registry: Vec<Registered>,
        outcomes: std::collections::HashMap<String, Vec<Notice>>,
        queued: VecDeque<Notice>,
        started: Vec<String>,
        disconnected: Vec<String>,
        /// Services whose `start_oauth_flow` RPC errors.
        fail_start: Vec<String>,
    }

    fn notice(category: &str, message: &str) -> Notice {
        Notice {
            category: category.into(),
            level: "info".into(),
            message: message.into(),
        }
    }

    impl Fake {
        fn new(registry: &[(&str, bool)]) -> Self {
            Self {
                registry: registry
                    .iter()
                    .map(|(n, c)| Registered {
                        name: n.to_string(),
                        connected: *c,
                    })
                    .collect(),
                outcomes: Default::default(),
                queued: Default::default(),
                started: vec![],
                disconnected: vec![],
                fail_start: vec![],
            }
        }
        fn on(mut self, service: &str, notices: Vec<Notice>) -> Self {
            self.outcomes.insert(service.into(), notices);
            self
        }
    }

    #[async_trait(?Send)]
    impl ConnectTransport for Fake {
        async fn list_integrations(&mut self) -> Result<Vec<Registered>> {
            Ok(self.registry.clone())
        }
        async fn start_oauth_flow(&mut self, service: &str) -> Result<String> {
            self.started.push(service.into());
            if self.fail_start.iter().any(|s| s == service) {
                bail!("integration '{service}' did not publish an auth URL within 5s");
            }
            // A notice for a different service arrives first: it must be
            // ignored, not taken as this service's outcome.
            self.queued
                .push_back(notice("integration", "other connected"));
            self.queued
                .extend(self.outcomes.get(service).cloned().unwrap_or_default());
            Ok(format!("https://auth.example/{service}"))
        }
        async fn disconnect(&mut self, service: &str) -> Result<()> {
            self.disconnected.push(service.into());
            Ok(())
        }
        async fn next_notice(&mut self) -> Option<Notice> {
            match self.queued.pop_front() {
                Some(n) => Some(n),
                // Nothing more: hang like a quiet server, so timeouts work.
                None => std::future::pending().await,
            }
        }
    }

    fn opts(services: &[&str], all: bool) -> ConnectOptions {
        ConnectOptions {
            services: services.iter().map(|s| s.to_string()).collect(),
            all,
            timeout: Duration::from_millis(200),
            feed_grace: Duration::from_millis(20),
        }
    }

    async fn run(
        fake: &mut Fake,
        o: ConnectOptions,
        opened: &std::cell::RefCell<Vec<String>>,
    ) -> (Result<()>, String) {
        let dir = tempfile::tempdir().unwrap();
        let open = |u: &str| {
            opened.borrow_mut().push(u.to_string());
            true
        };
        let mut out = Vec::new();
        let dirs = Dirs {
            config: dir.path(),
            data: dir.path(),
        };
        let r = run_connect(fake, dirs, &o, &open, &mut out).await;
        (r, String::from_utf8(out).unwrap())
    }

    #[tokio::test]
    async fn connects_a_named_service_and_opens_the_browser() {
        let mut fake = Fake::new(&[("slack", false)]).on(
            "slack",
            // The server's real order: "connected" first, then the
            // auto-created default feed.
            vec![
                notice("integration", "slack connected"),
                notice(
                    "feeds",
                    "auto-registered slack/my-mentions as `slack-mentions` after /connect slack",
                ),
            ],
        );
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["slack"], false), &opened).await;
        r.unwrap();
        assert_eq!(fake.started, ["slack"]);
        assert_eq!(*opened.borrow(), ["https://auth.example/slack"]);
        assert!(out.contains("slack: connected."), "{out}");
        assert!(out.contains("auto-registered slack/my-mentions"), "{out}");
    }

    #[tokio::test]
    async fn google_expands_and_all_skips_connected_services() {
        let reg = [
            ("gmail", true),
            ("google_calendar", false),
            ("google_drive", false),
        ];
        let mut fake = Fake::new(&reg)
            .on(
                "google_calendar",
                vec![notice("integration", "google_calendar connected")],
            )
            .on(
                "google_drive",
                vec![notice("integration", "google_drive connected")],
            );
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["google"], false), &opened).await;
        r.unwrap();
        assert!(out.contains("gmail: already connected"), "{out}");
        assert_eq!(fake.started, ["google_calendar", "google_drive"]);

        let mut fake = Fake::new(&reg)
            .on(
                "google_calendar",
                vec![notice("integration", "google_calendar connected")],
            )
            .on(
                "google_drive",
                vec![notice("integration", "google_drive connected")],
            );
        let (r, _) = run(&mut fake, opts(&[], true), &opened).await;
        r.unwrap();
        assert_eq!(fake.started, ["google_calendar", "google_drive"]);
    }

    #[tokio::test]
    async fn failure_notice_fails_the_command_with_the_server_message() {
        let mut fake = Fake::new(&[("atlassian", false)]).on(
            "atlassian",
            vec![notice(
                "integration",
                "atlassian connection FAILED: redirect_uri_mismatch",
            )],
        );
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["jira"], false), &opened).await;
        assert!(format!("{}", r.unwrap_err()).contains("not connected: atlassian"));
        assert!(
            out.contains("atlassian: FAILED — redirect_uri_mismatch"),
            "{out}"
        );
    }

    #[tokio::test]
    async fn no_answer_times_out_with_a_retry_hint() {
        let mut fake = Fake::new(&[("slack", false)]);
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["slack"], false), &opened).await;
        assert!(r.is_err());
        assert!(out.contains("no answer after 0s"), "{out}");
        assert!(out.contains("arawn connect slack"), "{out}");
    }

    #[tokio::test]
    async fn service_not_loaded_says_why_and_fails() {
        let mut fake = Fake::new(&[("slack", false)]);
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["github"], false), &opened).await;
        assert!(r.is_err());
        assert!(fake.started.is_empty());
        assert!(out.contains("github: not available"), "{out}");
        assert!(out.contains("arawn setup github"), "{out}");
    }

    #[tokio::test]
    async fn unknown_name_lists_what_the_server_has() {
        let mut fake = Fake::new(&[("slack", false), ("gmail", false)]);
        let opened = Default::default();
        let (r, _) = run(&mut fake, opts(&["linear"], false), &opened).await;
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("unknown service: linear"), "{msg}");
        assert!(msg.contains("gmail, slack"), "{msg}");
    }

    #[tokio::test]
    async fn no_names_and_no_all_is_an_error() {
        let mut fake = Fake::new(&[]);
        let opened = Default::default();
        let (r, _) = run(&mut fake, opts(&[], false), &opened).await;
        assert!(format!("{}", r.unwrap_err()).contains("--all"));
    }

    #[tokio::test]
    async fn a_failed_start_does_not_stop_the_other_services() {
        let mut fake = Fake::new(&[("gmail", false), ("slack", false)])
            .on("slack", vec![notice("integration", "slack connected")]);
        fake.fail_start = vec!["gmail".into()];
        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&[], true), &opened).await;
        assert!(format!("{}", r.unwrap_err()).contains("not connected: gmail"));
        assert_eq!(fake.started, ["gmail", "slack"]);
        assert!(
            out.contains("gmail: FAILED — cannot start the flow"),
            "{out}"
        );
        assert!(out.contains("slack: connected."), "{out}");
    }

    #[tokio::test]
    async fn an_exact_service_name_never_expands_to_its_siblings() {
        // Review regression: gmail not loaded must not mean "all Google".
        let mut fake = Fake::new(&[("google_calendar", true), ("google_drive", true)]);
        let mut out = Vec::new();
        run_disconnect(&mut fake, "gmail", &mut out).await.unwrap();
        assert!(fake.disconnected.is_empty(), "{:?}", fake.disconnected);
        assert!(
            String::from_utf8(out)
                .unwrap()
                .contains("gmail: not loaded")
        );

        let opened = Default::default();
        let (r, out) = run(&mut fake, opts(&["gmail"], false), &opened).await;
        assert!(r.is_err());
        assert!(fake.started.is_empty(), "{:?}", fake.started);
        assert!(out.contains("gmail: not available"), "{out}");
        // Short aliases map to one service.
        let reg = [("google_calendar", false), ("google_drive", false)];
        assert_eq!(
            expand(&["calendar".into()], false, &Fake::new(&reg).registry).unwrap(),
            ["google_calendar"]
        );
    }

    #[tokio::test]
    async fn disconnect_expands_names_and_calls_the_server() {
        let mut fake = Fake::new(&[("gmail", true), ("google_drive", true)]);
        let mut out = Vec::new();
        run_disconnect(&mut fake, "google", &mut out).await.unwrap();
        assert_eq!(fake.disconnected, ["gmail", "google_drive"]);
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("google_calendar: not loaded"), "{out}");
    }
}

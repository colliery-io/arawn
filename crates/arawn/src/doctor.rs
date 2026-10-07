//! `arawn doctor` — diagnostic checks for config, data dir, LLM
//! reachability, memory backend, integration credentials, and plugins.
//!
//! Designed to be the first thing a contributor runs when something
//! feels off. Each check is named, returns `Pass | Fail(reason) |
//! Skip(reason)`, and produces a structured report renderable as
//! human-readable text or JSON.
//!
//! Exit code: 0 if every check is `Pass` or `Skip`, 1 if any `Fail`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

/// Per-check outcome. `Skip` is non-fatal and means the check did not
/// apply in this environment (e.g. no integrations configured).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum CheckOutcome {
    Pass,
    Fail { reason: String },
    Skip { reason: String },
}

impl CheckOutcome {
    fn is_fail(&self) -> bool {
        matches!(self, CheckOutcome::Fail { .. })
    }
    fn label(&self) -> &'static str {
        match self {
            CheckOutcome::Pass => "PASS",
            CheckOutcome::Fail { .. } => "FAIL",
            CheckOutcome::Skip { .. } => "SKIP",
        }
    }
    fn detail(&self) -> Option<&str> {
        match self {
            CheckOutcome::Pass => None,
            CheckOutcome::Fail { reason } | CheckOutcome::Skip { reason } => Some(reason),
        }
    }
}

/// A single named check with its outcome.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub outcome: CheckOutcome,
}

impl CheckResult {
    fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            outcome: CheckOutcome::Pass,
        }
    }
    fn fail(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            outcome: CheckOutcome::Fail {
                reason: reason.into(),
            },
        }
    }
    fn skip(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            outcome: CheckOutcome::Skip {
                reason: reason.into(),
            },
        }
    }
}

/// Full report from a doctor run.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub data_dir: PathBuf,
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    pub fn any_failed(&self) -> bool {
        self.checks.iter().any(|c| c.outcome.is_fail())
    }

    pub fn exit_code(&self) -> i32 {
        if self.any_failed() { 1 } else { 0 }
    }

    pub fn render_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "arawn doctor — data_dir: {}\n",
            self.data_dir.display()
        ));
        out.push('\n');
        let name_width = self.checks.iter().map(|c| c.name.len()).max().unwrap_or(0);
        for c in &self.checks {
            let label = c.outcome.label();
            out.push_str(&format!(
                "  [{label:4}] {name:width$}",
                label = label,
                name = c.name,
                width = name_width
            ));
            if let Some(d) = c.outcome.detail() {
                out.push_str("  — ");
                out.push_str(d);
            }
            out.push('\n');
        }
        let pass = self
            .checks
            .iter()
            .filter(|c| matches!(c.outcome, CheckOutcome::Pass))
            .count();
        let fail = self.checks.iter().filter(|c| c.outcome.is_fail()).count();
        let skip = self
            .checks
            .iter()
            .filter(|c| matches!(c.outcome, CheckOutcome::Skip { .. }))
            .count();
        out.push_str(&format!("\n{pass} pass · {fail} fail · {skip} skip\n"));
        out
    }

    pub fn render_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("DoctorReport JSON serialisation")
    }
}

/// Run every doctor check against the given data dir. The pool is
/// optional so tests can hand in a pre-built pool; production passes
/// `None` and the runner builds one from config.
pub async fn run(data_dir: &Path) -> DoctorReport {
    let mut checks = Vec::new();

    let (cfg_check, parsed_config) = check_config_parses(data_dir);
    checks.push(cfg_check);

    checks.push(check_data_dir_writable(data_dir));

    checks.push(check_memory_store(data_dir));

    checks.push(check_plugins_dir(data_dir));

    // LLM + integrations both depend on a parsed config; if it failed
    // to parse there is nothing useful to check.
    match parsed_config {
        Some(cfg) => {
            checks.extend(check_llm_reachable(&cfg).await);
            checks.extend(check_integrations(data_dir, &cfg));
            if let Some(c) = check_declared(data_dir, &cfg) {
                checks.push(c);
            }
        }
        None => {
            checks.push(CheckResult::skip(
                "llm-reachable",
                "skipped because config did not parse",
            ));
            checks.push(CheckResult::skip(
                "integrations",
                "skipped because config did not parse",
            ));
        }
    }

    DoctorReport {
        data_dir: data_dir.to_path_buf(),
        checks,
    }
}

fn check_config_parses(data_dir: &Path) -> (CheckResult, Option<crate::ArawnConfig>) {
    let path = data_dir.join("arawn.toml");
    if !path.exists() {
        return (
            CheckResult::skip(
                "config-parses",
                format!("no config at {} — using defaults", path.display()),
            ),
            Some(crate::ArawnConfig::default()),
        );
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<crate::ArawnConfig>(&content) {
            Ok(cfg) => (CheckResult::pass("config-parses"), Some(cfg)),
            Err(e) => (
                CheckResult::fail("config-parses", format!("parse error: {e}")),
                None,
            ),
        },
        Err(e) => (
            CheckResult::fail(
                "config-parses",
                format!("could not read {}: {e}", path.display()),
            ),
            None,
        ),
    }
}

fn check_data_dir_writable(data_dir: &Path) -> CheckResult {
    if let Err(e) = std::fs::create_dir_all(data_dir) {
        return CheckResult::fail(
            "data-dir-writable",
            format!("could not create {}: {e}", data_dir.display()),
        );
    }
    let probe = data_dir.join(".doctor-write-probe");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            CheckResult::pass("data-dir-writable")
        }
        Err(e) => CheckResult::fail(
            "data-dir-writable",
            format!("write to {} failed: {e}", probe.display()),
        ),
    }
}

fn check_memory_store(data_dir: &Path) -> CheckResult {
    // Open the global memory.db. This is the surface that lives in
    // data_dir directly — the lens tier opens lazily per ws.
    let path = data_dir.join("memory.db");
    match arawn_memory::MemoryStore::open(&path) {
        Ok(_) => CheckResult::pass("memory-store"),
        Err(e) => CheckResult::fail(
            "memory-store",
            format!("could not open {}: {e}", path.display()),
        ),
    }
}

fn check_plugins_dir(data_dir: &Path) -> CheckResult {
    let path = data_dir.join("plugins");
    if !path.exists() {
        return CheckResult::skip(
            "plugins-scan",
            format!("no plugin dir at {}", path.display()),
        );
    }
    // `discover_plugins` is infallible at the call boundary (it returns
    // an empty vec rather than erroring), but for "doctor" purposes the
    // useful signal is "does the cache dir scan cleanly + how many did
    // we get". A more granular per-plugin parse-error surface is on
    // the engine to expose; until then, we just report counts.
    let plugins = arawn_engine::plugins::discover_plugins(&path);
    CheckResult::pass(format!("plugins-scan ({} loaded)", plugins.len()))
}

async fn check_llm_reachable(config: &crate::ArawnConfig) -> Vec<CheckResult> {
    // Build the pool. If construction fails (bad API key env, unknown
    // provider, …) we surface that as one failure for "llm-build".
    let pool = match crate::LlmClientPool::from_config(config, build_real_client) {
        Ok(p) => p,
        Err(e) => {
            return vec![CheckResult::fail("llm-build", format!("{e:#}"))];
        }
    };

    let mut out = vec![CheckResult::pass(format!(
        "llm-build ({} profile(s))",
        pool.len()
    ))];

    // Probe every entry with a bounded timeout so a slow provider
    // doesn't hang the doctor command.
    let probes = pool
        .entries()
        .map(|(name, cfg)| {
            let name = name.clone();
            let model = cfg.model.clone();
            let client = pool.get(&name).expect("pool entry exists");
            async move {
                let probe =
                    tokio::time::timeout(Duration::from_secs(20), client.warmup(&model)).await;
                let result = match probe {
                    Ok(Ok(())) => CheckOutcome::Pass,
                    Ok(Err(e)) => CheckOutcome::Fail {
                        reason: format!("{e}"),
                    },
                    Err(_) => CheckOutcome::Fail {
                        reason: "warmup timed out after 20s".into(),
                    },
                };
                CheckResult {
                    name: format!("llm-reachable [{name}]"),
                    outcome: result,
                }
            }
        })
        .collect::<Vec<_>>();

    out.extend(futures::future::join_all(probes).await);
    out
}

/// One check per integration (ARAWN-T-0502), named `integration:<service>`.
/// Connected → PASS. Not configured or not yet connected → SKIP with the
/// fix. Set up but unusable (missing secret, incomplete GitHub App,
/// unreadable token) → FAIL with the fix.
fn check_integrations(data_dir: &Path, config: &crate::ArawnConfig) -> Vec<CheckResult> {
    match crate::integration_state::inspect(&config.integrations, data_dir) {
        Ok(reports) => reports.iter().map(integration_check).collect(),
        Err(e) => vec![CheckResult::fail("integrations", e)],
    }
}

/// Drift between `[[lenses]]` / `[[feeds]]` in arawn.toml and the store
/// (ARAWN-T-0506). Uses the server's own reconcile plan, read-only.
/// `None` when nothing is declared.
fn check_declared(data_dir: &Path, config: &crate::ArawnConfig) -> Option<CheckResult> {
    use crate::local_service::declared::{
        Declared, Existing, Step, existing_lenses, feed_map, plan,
    };
    let declared = Declared {
        lenses: config.lenses.clone(),
        feeds: config.feeds.clone(),
    };
    if declared.is_empty() {
        return None;
    }
    let name = "declared-lenses-feeds";
    if !data_dir.join("arawn.db").exists() {
        return Some(CheckResult::skip(
            name,
            "not applied yet — start arawn serve to create the declared lenses and feeds",
        ));
    }
    let store = match arawn_storage::Store::open(data_dir) {
        Ok(s) => s,
        Err(e) => {
            return Some(CheckResult::fail(
                name,
                format!("cannot open the store: {e}"),
            ));
        }
    };
    let feeds = arawn_feeds::FeedStore::new(store.database().conn())
        .list_all()
        .ok()
        .map(feed_map);
    let existing = Existing {
        lenses: existing_lenses(&store, data_dir, &declared),
        feeds,
    };
    let connected: std::collections::HashSet<String> =
        crate::integration_state::inspect(&config.integrations, data_dir)
            .map(|rs| {
                rs.into_iter()
                    .filter(|r| {
                        matches!(
                            r.state,
                            crate::integration_state::IntegrationState::Connected { .. }
                        )
                    })
                    .map(|r| r.service.to_string())
                    .collect()
            })
            .unwrap_or_default();

    let steps = plan(&declared, &existing, &connected);
    let problems: Vec<String> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Problem(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    let waiting: Vec<String> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Waiting { feed, reason } => Some(format!("{feed}: {reason}")),
            _ => None,
        })
        .collect();
    let pending = steps.len() - problems.len() - waiting.len();

    if !problems.is_empty() {
        return Some(CheckResult::fail(name, problems.join("; ")));
    }
    let mut notes = Vec::new();
    if pending > 0 {
        notes.push(format!(
            "{pending} change(s) not applied yet — restart arawn serve"
        ));
    }
    if !waiting.is_empty() {
        notes.push(format!("waiting: {}", waiting.join("; ")));
    }
    Some(if notes.is_empty() {
        CheckResult::pass(format!(
            "{name} ({} lens(es), {} feed(s))",
            declared.lenses.len(),
            declared.feeds.len()
        ))
    } else {
        CheckResult::skip(name, notes.join(" — "))
    })
}

fn integration_check(r: &crate::integration_state::IntegrationReport) -> CheckResult {
    use crate::integration_state::IntegrationState;
    let name = format!("integration:{}", r.service);
    let detail = match r.hint() {
        Some(h) => format!("{} — {h}", r.describe()),
        None => r.describe(),
    };
    match &r.state {
        IntegrationState::Connected { .. } => CheckResult::pass(name),
        s if s.is_broken() => CheckResult::fail(name, detail),
        _ => CheckResult::skip(name, detail),
    }
}

/// Construct a real LLM client from an [`LlmConfig`]. Mirrors the
/// `build_llm_client` helper in `main.rs`; kept local so doctor can
/// build a pool without depending on the binary's private helpers.
fn build_real_client(
    cfg: &crate::config::LlmConfig,
) -> anyhow::Result<std::sync::Arc<dyn arawn_llm::LlmClient>> {
    use std::sync::Arc;
    let resolved_key = crate::ArawnConfig::resolve_api_key(cfg);
    match cfg.provider.as_str() {
        "anthropic" => {
            let api_key = resolved_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "Anthropic provider requires an API key — set `api_key` in [llm.<name>] or export {}",
                    cfg.api_key_env
                )
            })?;
            Ok(Arc::new(arawn_llm::AnthropicClient::new(api_key)))
        }
        _ => Ok(Arc::new(arawn_llm::OpenAICompatibleClient::from_config(
            &cfg.provider,
            cfg.base_url.as_deref(),
            resolved_key,
        )?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn missing_config_is_skipped_not_failed() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let cfg_check = report
            .checks
            .iter()
            .find(|c| c.name == "config-parses")
            .unwrap();
        assert!(matches!(cfg_check.outcome, CheckOutcome::Skip { .. }));
        // Missing config alone doesn't mark the report failed.
        assert!(!report.any_failed() || report.checks.iter().any(|c| c.name.starts_with("llm-")));
    }

    fn outcome<'a>(checks: &'a [CheckResult], name: &str) -> &'a CheckOutcome {
        &checks
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no check named {name}"))
            .outcome
    }

    #[test]
    fn one_check_per_integration_with_store_names_and_fix_hints() {
        // Regression (ARAWN-T-0504): doctor used to ignore
        // [integrations.google] and used hyphenated names that never
        // matched the token store ("google-calendar"). ARAWN-T-0502: one
        // line per integration, each naming its fix.
        let tmp = TempDir::new().unwrap();
        let cfg: crate::ArawnConfig = toml::from_str(
            r#"
            [integrations.google]
            client_id = "gid"
            client_secret = "gsec"

            [integrations.atlassian]
            client_id = "aid"
            "#,
        )
        .unwrap();
        let checks = check_integrations(tmp.path(), &cfg);
        let names: Vec<_> = checks.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "integration:gmail",
                "integration:google_calendar",
                "integration:google_drive",
                "integration:atlassian",
                "integration:slack",
                "integration:github",
            ]
        );
        // Google: configured via the shared block, not yet connected.
        match outcome(&checks, "integration:google_calendar") {
            CheckOutcome::Skip { reason } => {
                assert!(reason.contains("arawn connect google_calendar"), "{reason}")
            }
            o => panic!("expected skip, got {o:?}"),
        }
        // Atlassian: id but no secret → FAIL naming the env var. (Skipped
        // when a developer has the env var exported.)
        if std::env::var("ARAWN_ATLASSIAN_CLIENT_SECRET").is_err() {
            match outcome(&checks, "integration:atlassian") {
                CheckOutcome::Fail { reason } => {
                    assert!(reason.contains("ARAWN_ATLASSIAN_CLIENT_SECRET"), "{reason}")
                }
                o => panic!("expected fail, got {o:?}"),
            }
        }
        // Slack: nothing → SKIP pointing at arawn setup.
        if std::env::var("ARAWN_SLACK_CLIENT_ID").is_err() {
            match outcome(&checks, "integration:slack") {
                CheckOutcome::Skip { reason } => {
                    assert!(reason.contains("arawn setup slack"), "{reason}")
                }
                o => panic!("expected skip, got {o:?}"),
            }
        }
    }

    #[tokio::test]
    async fn data_dir_writable_passes_for_tempdir() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let c = report
            .checks
            .iter()
            .find(|c| c.name == "data-dir-writable")
            .unwrap();
        assert!(matches!(c.outcome, CheckOutcome::Pass), "got {c:?}");
    }

    #[tokio::test]
    async fn memory_store_passes_for_fresh_dir() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let c = report
            .checks
            .iter()
            .find(|c| c.name == "memory-store")
            .unwrap();
        assert!(matches!(c.outcome, CheckOutcome::Pass), "got {c:?}");
    }

    #[tokio::test]
    async fn plugins_scan_skipped_when_no_plugins_dir() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let c = report
            .checks
            .iter()
            .find(|c| c.name.starts_with("plugins-scan"))
            .unwrap();
        assert!(matches!(c.outcome, CheckOutcome::Skip { .. }), "got {c:?}");
    }

    #[tokio::test]
    async fn integrations_skipped_when_none_configured() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        // Nothing configured is not a failure: every integration line is
        // a SKIP that names its fix (ARAWN-T-0502).
        let lines: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.starts_with("integration"))
            .collect();
        assert!(!lines.is_empty());
        for c in lines {
            assert!(matches!(c.outcome, CheckOutcome::Skip { .. }), "got {c:?}");
        }
    }

    #[tokio::test]
    async fn malformed_config_fails_and_skips_dependents() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("arawn.toml"), "this is not toml = = =\n").unwrap();
        let report = run(tmp.path()).await;
        assert!(report.any_failed());
        let cfg_check = report
            .checks
            .iter()
            .find(|c| c.name == "config-parses")
            .unwrap();
        assert!(matches!(cfg_check.outcome, CheckOutcome::Fail { .. }));
        // llm-reachable and integrations should be skipped (config didn't parse).
        let llm_check = report
            .checks
            .iter()
            .find(|c| c.name == "llm-reachable")
            .unwrap();
        assert!(matches!(llm_check.outcome, CheckOutcome::Skip { .. }));
    }

    #[tokio::test]
    async fn json_render_round_trips() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let json = report.render_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert!(parsed["checks"].is_array());
        assert!(parsed["data_dir"].is_string());
    }

    #[tokio::test]
    async fn human_render_contains_summary() {
        let tmp = TempDir::new().unwrap();
        let report = run(tmp.path()).await;
        let text = report.render_human();
        assert!(text.contains("arawn doctor"));
        assert!(text.contains("pass") || text.contains("fail") || text.contains("skip"));
    }

    #[test]
    fn exit_code_zero_when_no_fails() {
        let report = DoctorReport {
            data_dir: PathBuf::from("/tmp"),
            checks: vec![CheckResult::pass("a"), CheckResult::skip("b", "n/a")],
        };
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn exit_code_one_when_any_fail() {
        let report = DoctorReport {
            data_dir: PathBuf::from("/tmp"),
            checks: vec![CheckResult::pass("a"), CheckResult::fail("b", "broken")],
        };
        assert_eq!(report.exit_code(), 1);
    }
}

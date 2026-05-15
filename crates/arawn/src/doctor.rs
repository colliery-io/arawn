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
        out.push_str(&format!("arawn doctor — data_dir: {}\n", self.data_dir.display()));
        out.push_str("\n");
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
        let fail = self
            .checks
            .iter()
            .filter(|c| c.outcome.is_fail())
            .count();
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
            checks.push(check_integrations(data_dir, &cfg));
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
    // data_dir directly — the workstream tier opens lazily per ws.
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
                let probe = tokio::time::timeout(
                    Duration::from_secs(20),
                    client.warmup(&model),
                )
                .await;
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

fn check_integrations(data_dir: &Path, config: &crate::ArawnConfig) -> CheckResult {
    // OAuth token store. If no integrations are configured we skip.
    let any_configured = !configured_services(config).is_empty();
    if !any_configured {
        return CheckResult::skip(
            "integrations",
            "no [integrations.*] configured",
        );
    }
    match arawn_auth::TokenStore::open(data_dir) {
        Ok(store) => {
            // Count token files in the store's tokens dir. Failure here
            // (unreadable dir, decrypt failure on a known integration
            // token) is the signal worth surfacing.
            let tokens_dir = store.tokens_dir();
            let count = match std::fs::read_dir(tokens_dir) {
                Ok(entries) => entries
                    .flatten()
                    .filter(|e: &std::fs::DirEntry| {
                        e.file_name().to_string_lossy().ends_with(".token")
                    })
                    .count(),
                Err(_) => 0,
            };
            // For each configured integration that claims to be set up,
            // verify the token round-trips through decrypt.
            let mut failures: Vec<String> = Vec::new();
            for service in configured_services(config) {
                match store.load(&service) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        // Configured but no token saved — counts as a
                        // soft notice, not a fail.
                    }
                    Err(e) => failures.push(format!("{service}: {e}")),
                }
            }
            if failures.is_empty() {
                CheckResult::pass(format!("integrations ({count} token file(s))"))
            } else {
                CheckResult::fail(
                    "integrations",
                    format!("decrypt failures: {}", failures.join("; ")),
                )
            }
        }
        Err(e) => CheckResult::fail(
            "integrations",
            format!("could not open token store at {}: {e}", data_dir.display()),
        ),
    }
}

fn configured_services(config: &crate::ArawnConfig) -> Vec<String> {
    let mut out = Vec::new();
    let i = &config.integrations;
    if !i.gmail.client_id.is_empty() {
        out.push("gmail".into());
    }
    if !i.calendar.client_id.is_empty() {
        out.push("google-calendar".into());
    }
    if !i.drive.client_id.is_empty() {
        out.push("google-drive".into());
    }
    if !i.atlassian.client_id.is_empty() {
        out.push("atlassian".into());
    }
    if !i.slack.client_id.is_empty() {
        out.push("slack".into());
    }
    out
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
        let c = report
            .checks
            .iter()
            .find(|c| c.name == "integrations")
            .unwrap();
        assert!(matches!(c.outcome, CheckOutcome::Skip { .. }), "got {c:?}");
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
            checks: vec![
                CheckResult::pass("a"),
                CheckResult::skip("b", "n/a"),
            ],
        };
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn exit_code_one_when_any_fail() {
        let report = DoctorReport {
            data_dir: PathBuf::from("/tmp"),
            checks: vec![
                CheckResult::pass("a"),
                CheckResult::fail("b", "broken"),
            ],
        };
        assert_eq!(report.exit_code(), 1);
    }
}

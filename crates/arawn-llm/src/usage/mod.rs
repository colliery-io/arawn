//! Process-wide token usage tracker.
//!
//! Records token counts — *not* dollars — for every completed LLM
//! call. Pricing changes silently, varies by tier, and is sometimes
//! negotiated; shipping an inaccurate cost number is worse than
//! shipping none. Anyone who wants a dollar figure can multiply our
//! tokens by their negotiated rate themselves.
//!
//! # Architecture
//!
//! - [`TokenUsageRecord`] is one line written every time the LLM
//!   stream emits a `Done { usage: Some(_) }` chunk. Records persist
//!   to `<data_dir>/token_usage.jsonl` (append-only).
//! - [`UsageTracker`] is a process-wide singleton, installed at
//!   startup with the data dir. Calls that happen before the
//!   tracker is installed are dropped silently (with a tracing
//!   warning) — the tracker is best-effort observability, never
//!   load-bearing.
//! - [`tracking_client::UsageTrackingClient`] is an `LlmClient`
//!   decorator that snoops every stream for `Done { usage }` chunks
//!   and records them. Production layering in `arawn`'s
//!   `LlmClientPool::from_config`: `raw → Retry → UsageTracking →
//!   Warming`.
//!
//! # Reading
//!
//! [`UsageTracker::summary`] walks the log and produces rollups by
//! `(provider, model)`, optionally filtered by `UsagePeriod` and
//! optionally grouped by `call_site`. The CLI command `arawn usage`
//! is the user-facing surface; T-0278's routing policy will read
//! recent usage to drive the `UsagePressure` hint.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub mod tracking_client;

pub use tracking_client::UsageTrackingClient;

/// One token-usage event. Persisted as one JSON line per call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    /// Unix epoch seconds.
    pub ts: u64,
    /// Provider name (`groq`, `anthropic`, etc.).
    pub provider: String,
    /// Concrete model string passed in `ChatRequest.model`.
    pub model: String,
    /// Tokens billed against the input/prompt side.
    pub prompt_tokens: u32,
    /// Tokens billed against the completion/output side.
    pub completion_tokens: u32,
    /// Optional caller tag — e.g. `"workflow.morning_brief"`,
    /// `"steward.reshelve"`. Diagnostic only.
    pub call_site: Option<String>,
}

/// Rollup of `TokenUsageRecord`s for one `(provider, model)` group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub provider: String,
    pub model: String,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub call_count: u64,
}

impl ModelUsageStats {
    pub fn total_tokens(&self) -> u64 {
        self.total_prompt_tokens + self.total_completion_tokens
    }
}

/// Window selector for rollups. `Day`/`Week`/`Month` are bounded by
/// **UTC** day boundaries — keeps cross-host comparisons stable and
/// avoids local-time foot-guns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsagePeriod {
    /// Records with `ts` within the last 24h.
    Day,
    /// Records with `ts` within the last 7 days.
    Week,
    /// Records with `ts` within the last 30 days.
    Month,
    /// All records, regardless of age.
    All,
}

impl UsagePeriod {
    /// Minimum `ts` for this window, given a reference `now`. `All`
    /// returns 0.
    pub fn since(self, now: u64) -> u64 {
        match self {
            UsagePeriod::Day => now.saturating_sub(24 * 3600),
            UsagePeriod::Week => now.saturating_sub(7 * 24 * 3600),
            UsagePeriod::Month => now.saturating_sub(30 * 24 * 3600),
            UsagePeriod::All => 0,
        }
    }
}

/// Summary report produced by [`UsageTracker::summary`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub period: String,
    pub since_ts: u64,
    pub generated_ts: u64,
    /// Rollups keyed by `(provider, model)`.
    pub models: Vec<ModelUsageStats>,
    /// Per-`call_site` rollup. Only populated when `by_site` is
    /// requested.
    pub by_site: Vec<CallSiteStats>,
    pub total_calls: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSiteStats {
    pub call_site: String,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub call_count: u64,
}

/// Process-wide token usage tracker. Holds the data-dir path and a
/// mutex for serialised appends; the actual log is on disk.
pub struct UsageTracker {
    log_path: PathBuf,
    write_lock: Mutex<()>,
}

impl UsageTracker {
    /// Build a tracker writing to `<data_dir>/token_usage.jsonl`.
    /// Best-effort: creates the parent dir, but failures are logged
    /// and the tracker still works (read paths return empty).
    pub fn open(data_dir: &Path) -> Self {
        if let Err(e) = std::fs::create_dir_all(data_dir) {
            tracing::warn!(error = %e, dir = %data_dir.display(), "token usage: could not create data dir");
        }
        Self {
            log_path: data_dir.join("token_usage.jsonl"),
            write_lock: Mutex::new(()),
        }
    }

    /// Append one record. Failures log but never bubble.
    pub fn record(&self, record: &TokenUsageRecord) {
        use std::io::Write;
        let _g = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let line = match serde_json::to_string(record) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "token usage: could not serialise record");
                return;
            }
        };
        let result = (|| -> std::io::Result<()> {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)?;
            writeln!(f, "{line}")?;
            Ok(())
        })();
        if let Err(e) = result {
            tracing::warn!(error = %e, path = %self.log_path.display(), "token usage: append failed");
        }
    }

    /// Read every record in the log. Skips lines that fail to parse
    /// with a tracing warning. Returns empty Vec if the file is
    /// missing.
    pub fn read_all(&self) -> Vec<TokenUsageRecord> {
        let Ok(content) = std::fs::read_to_string(&self.log_path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (lineno, raw) in content.lines().enumerate() {
            if raw.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<TokenUsageRecord>(raw) {
                Ok(rec) => out.push(rec),
                Err(e) => tracing::warn!(line = lineno + 1, error = %e, "skipped malformed usage record"),
            }
        }
        out
    }

    /// Compute a [`UsageSummary`] over the log.
    pub fn summary(&self, period: UsagePeriod, model_filter: Option<&str>, by_site: bool) -> UsageSummary {
        let now = now_secs();
        let since = period.since(now);
        let mut records: Vec<TokenUsageRecord> = self
            .read_all()
            .into_iter()
            .filter(|r| r.ts >= since)
            .collect();
        if let Some(m) = model_filter {
            records.retain(|r| r.model == m);
        }

        use std::collections::HashMap;
        let mut by_model: HashMap<(String, String), ModelUsageStats> = HashMap::new();
        let mut sites: HashMap<String, CallSiteStats> = HashMap::new();
        let (mut total_p, mut total_c, mut total_n) = (0u64, 0u64, 0u64);

        for r in &records {
            let entry = by_model
                .entry((r.provider.clone(), r.model.clone()))
                .or_insert_with(|| ModelUsageStats {
                    provider: r.provider.clone(),
                    model: r.model.clone(),
                    total_prompt_tokens: 0,
                    total_completion_tokens: 0,
                    call_count: 0,
                });
            entry.total_prompt_tokens += r.prompt_tokens as u64;
            entry.total_completion_tokens += r.completion_tokens as u64;
            entry.call_count += 1;

            total_p += r.prompt_tokens as u64;
            total_c += r.completion_tokens as u64;
            total_n += 1;

            if by_site {
                let key = r.call_site.clone().unwrap_or_else(|| "<unknown>".into());
                let site = sites
                    .entry(key.clone())
                    .or_insert_with(|| CallSiteStats {
                        call_site: key,
                        total_prompt_tokens: 0,
                        total_completion_tokens: 0,
                        call_count: 0,
                    });
                site.total_prompt_tokens += r.prompt_tokens as u64;
                site.total_completion_tokens += r.completion_tokens as u64;
                site.call_count += 1;
            }
        }

        let mut models: Vec<ModelUsageStats> = by_model.into_values().collect();
        models.sort_by(|a, b| b.total_tokens().cmp(&a.total_tokens()));
        let mut by_site_v: Vec<CallSiteStats> = sites.into_values().collect();
        by_site_v.sort_by(|a, b| {
            (b.total_prompt_tokens + b.total_completion_tokens)
                .cmp(&(a.total_prompt_tokens + a.total_completion_tokens))
        });

        UsageSummary {
            period: format!("{period:?}").to_ascii_lowercase(),
            since_ts: since,
            generated_ts: now,
            models,
            by_site: by_site_v,
            total_calls: total_n,
            total_prompt_tokens: total_p,
            total_completion_tokens: total_c,
        }
    }
}

// --- Process-wide singleton ---

static GLOBAL: OnceLock<Arc<UsageTracker>> = OnceLock::new();

/// Install the process-wide tracker. Idempotent on the *first* call;
/// later calls are ignored (the tracker is sticky once set, like a
/// `OnceLock`). Returns the installed instance either way.
pub fn install(tracker: Arc<UsageTracker>) -> Arc<UsageTracker> {
    if let Some(existing) = GLOBAL.get() {
        return existing.clone();
    }
    GLOBAL.set(tracker.clone()).ok();
    GLOBAL.get().cloned().unwrap_or(tracker)
}

/// Fetch the active tracker, if any has been installed.
pub fn global() -> Option<Arc<UsageTracker>> {
    GLOBAL.get().cloned()
}

/// Record a single event. Drops silently with a tracing log when no
/// tracker is installed; the LLM call path must not depend on the
/// tracker being live.
pub fn record(record: TokenUsageRecord) {
    if let Some(tracker) = GLOBAL.get() {
        tracker.record(&record);
    } else {
        tracing::debug!(
            provider = %record.provider,
            model = %record.model,
            "token usage event dropped — no tracker installed"
        );
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample(model: &str, p: u32, c: u32, ts: u64, site: Option<&str>) -> TokenUsageRecord {
        TokenUsageRecord {
            ts,
            provider: "groq".into(),
            model: model.into(),
            prompt_tokens: p,
            completion_tokens: c,
            call_site: site.map(|s| s.into()),
        }
    }

    #[test]
    fn record_round_trips() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        t.record(&sample("m1", 10, 5, now_secs(), None));
        t.record(&sample("m1", 20, 7, now_secs(), Some("steward.dust")));
        let all = t.read_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].model, "m1");
        assert_eq!(all[1].call_site.as_deref(), Some("steward.dust"));
    }

    #[test]
    fn summary_rolls_up_by_model() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        let now = now_secs();
        t.record(&sample("a", 100, 50, now, None));
        t.record(&sample("a", 200, 100, now, None));
        t.record(&sample("b", 30, 15, now, None));
        let s = t.summary(UsagePeriod::All, None, false);
        assert_eq!(s.models.len(), 2);
        assert_eq!(s.total_calls, 3);
        assert_eq!(s.total_prompt_tokens, 330);
        assert_eq!(s.total_completion_tokens, 165);
        // models sorted by total tokens desc — "a" has 450 vs "b" 45.
        assert_eq!(s.models[0].model, "a");
        assert_eq!(s.models[0].total_prompt_tokens, 300);
        assert_eq!(s.models[0].call_count, 2);
    }

    #[test]
    fn summary_filters_by_model() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        let now = now_secs();
        t.record(&sample("a", 10, 5, now, None));
        t.record(&sample("b", 20, 10, now, None));
        let s = t.summary(UsagePeriod::All, Some("a"), false);
        assert_eq!(s.models.len(), 1);
        assert_eq!(s.models[0].model, "a");
        assert_eq!(s.total_calls, 1);
    }

    #[test]
    fn summary_respects_day_window() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        let now = now_secs();
        // One record from 25h ago, one from now.
        t.record(&sample("m", 10, 5, now - 25 * 3600, None));
        t.record(&sample("m", 100, 50, now, None));
        let s = t.summary(UsagePeriod::Day, None, false);
        assert_eq!(s.total_calls, 1);
        assert_eq!(s.total_prompt_tokens, 100);
    }

    #[test]
    fn summary_by_site_groups_correctly() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        let now = now_secs();
        t.record(&sample("m", 10, 5, now, Some("agent.loop")));
        t.record(&sample("m", 20, 10, now, Some("agent.loop")));
        t.record(&sample("m", 5, 2, now, Some("steward.dust")));
        t.record(&sample("m", 1, 1, now, None));
        let s = t.summary(UsagePeriod::All, None, true);
        assert_eq!(s.by_site.len(), 3);
        let by_site = s.by_site;
        let agent = by_site.iter().find(|r| r.call_site == "agent.loop").unwrap();
        assert_eq!(agent.call_count, 2);
        assert_eq!(agent.total_prompt_tokens, 30);
        let unknown = by_site.iter().find(|r| r.call_site == "<unknown>").unwrap();
        assert_eq!(unknown.call_count, 1);
    }

    #[test]
    fn summary_empty_log_is_empty() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        let s = t.summary(UsagePeriod::All, None, true);
        assert_eq!(s.total_calls, 0);
        assert!(s.models.is_empty());
    }

    #[test]
    fn read_all_skips_malformed_lines() {
        let tmp = TempDir::new().unwrap();
        let t = UsageTracker::open(tmp.path());
        t.record(&sample("m", 10, 5, now_secs(), None));
        // Hand-write a bad line.
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(tmp.path().join("token_usage.jsonl"))
            .unwrap();
        writeln!(f, "this is not json").unwrap();
        t.record(&sample("m", 20, 10, now_secs(), None));
        let all = t.read_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn record_global_without_install_does_not_panic() {
        // The static GLOBAL may be set or unset depending on test
        // ordering; either way this must not panic.
        record(sample("m", 1, 1, now_secs(), None));
    }
}

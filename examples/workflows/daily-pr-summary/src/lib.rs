//! Linear pipeline example: fetch → process → save.
//!
//! Three tasks chained in a strict order:
//!
//!   fetch_prs ─▶ summarize_prs ─▶ save_briefing
//!
//! The trigger fires every weekday at 8:00 AM. Each task reads/writes the
//! shared `Context<Value>` to pass data downstream.
//!
//! Task bodies here are illustrative — real `gh` CLI calls and file writes
//! are replaced with stubs marked TODO. Swap them for your environment.

use cloacina_workflow::{workflow, task, Context, TaskError};
use cloacina_workflow::trigger;
use serde_json::{json, Value};

/// Tiny helper — collapses cloacina's struct-shaped TaskError variants into
/// the one this example actually uses, avoiding `task_id` / `timestamp`
/// boilerplate at every call site.
fn fail(task_id: &str, message: impl Into<String>) -> TaskError {
    TaskError::Unknown {
        task_id: task_id.to_string(),
        message: message.into(),
    }
}

#[workflow(name = "daily_pr_summary", description = "Daily PR briefing for a configured GitHub org.")]
pub mod daily_pr_summary {
    use super::*;

    /// Fetch open PRs from the configured GitHub org.
    ///
    /// Stub: returns a fixed payload. Real version would shell out to
    /// `gh pr list --state open --json number,title,author,createdAt`.
    #[task(id = "fetch_prs", dependencies = [])]
    pub async fn fetch_prs(context: &mut Context<Value>) -> Result<(), TaskError> {
        // TODO: real fetch
        // let output = tokio::process::Command::new("gh")
        //     .args(["pr", "list", "--repo", "colliery-io/arawn",
        //            "--state", "open", "--json", "number,title,author,createdAt"])
        //     .output().await
        //     .map_err(|e| fail("fetch_prs", format!("gh: {e}")))?;
        // let prs: Value = serde_json::from_slice(&output.stdout)
        //     .map_err(|e| fail("fetch_prs", format!("parse: {e}")))?;

        let prs = json!([
            {"number": 42, "title": "Add warmup TTL", "author": "alice", "createdAt": "2026-05-01"},
            {"number": 43, "title": "Fix permission audit", "author": "bob", "createdAt": "2026-05-02"},
        ]);

        context
            .insert("open_prs", prs)
            .map_err(|e| fail("fetch_prs", format!("insert: {e}")))?;
        Ok(())
    }

    /// Summarize the fetched PRs into markdown sections.
    ///
    /// Reads `open_prs` from context, formats one bullet per PR, writes
    /// `briefing_markdown` for the next task.
    #[task(id = "summarize_prs", dependencies = ["fetch_prs"])]
    pub async fn summarize_prs(context: &mut Context<Value>) -> Result<(), TaskError> {
        let prs = context
            .get("open_prs")
            .ok_or_else(|| fail("summarize_prs", "open_prs missing from context"))?;

        let mut md = String::from("# Open PRs\n\n");
        if let Some(arr) = prs.as_array() {
            for pr in arr {
                let n = pr["number"].as_i64().unwrap_or(0);
                let title = pr["title"].as_str().unwrap_or("?");
                let author = pr["author"].as_str().unwrap_or("?");
                md.push_str(&format!("- #{n}  **{title}**  _by {author}_\n"));
            }
        }
        if md == "# Open PRs\n\n" {
            md.push_str("(none)\n");
        }

        context
            .insert("briefing_markdown", json!(md))
            .map_err(|e| fail("summarize_prs", format!("insert: {e}")))?;
        Ok(())
    }

    /// Persist the briefing to disk.
    ///
    /// Stub: logs the byte count instead of writing. Real version would write
    /// to a configured path (e.g. `~/Documents/briefings/YYYY-MM-DD.md`).
    #[task(id = "save_briefing", dependencies = ["summarize_prs"])]
    pub async fn save_briefing(context: &mut Context<Value>) -> Result<(), TaskError> {
        let md = context
            .get("briefing_markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| fail("save_briefing", "briefing_markdown missing"))?;

        // TODO: real write
        // let path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        //     .join("Documents")
        //     .join("briefings")
        //     .join(format!("{}.md", chrono::Utc::now().format("%Y-%m-%d")));
        // tokio::fs::create_dir_all(path.parent().unwrap()).await
        //     .map_err(|e| fail("save_briefing", format!("mkdir: {e}")))?;
        // tokio::fs::write(&path, md).await
        //     .map_err(|e| fail("save_briefing", format!("write: {e}")))?;

        eprintln!("[daily_pr_summary] would write {} bytes of briefing", md.len());
        Ok(())
    }
}

/// Cron trigger — every weekday at 8:00 AM, server's local timezone.
#[trigger(on = "daily_pr_summary", cron = "0 8 * * 1-5", timezone = "UTC")]
pub async fn scheduled() {}

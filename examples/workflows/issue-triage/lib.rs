//! Decision-task example — uses the arawn agent to classify issue severity,
//! then conditionally fires an action task.
//!
//! See README.md for how to turn this source listing into a buildable crate
//! by copying boilerplate from ../daily-pr-summary/.

use cloacina_workflow::{workflow, task, Context, TaskError};
use serde_json::{json, Value};

fn fail(task_id: &str, message: impl Into<String>) -> TaskError {
    TaskError::Unknown {
        task_id: task_id.to_string(),
        message: message.into(),
    }
}

#[workflow(
    name = "issue_triage",
    description = "Classify open issues by severity using the agent and notify when any are P0."
)]
pub mod issue_triage {
    use super::*;

    /// Pull open issues from a GitHub repo. Stub returns three made-up issues
    /// with different shapes so the decision task has something to chew on.
    #[task(id = "fetch_open_issues", dependencies = [])]
    pub async fn fetch_open_issues(context: &mut Context<Value>) -> Result<(), TaskError> {
        // TODO: real fetch — `gh issue list --state open --json
        // number,title,body,labels,createdAt`.
        let issues = json!([
            {
                "number": 100,
                "title": "API returns 500 on /users/me when token is expired",
                "body": "Repro: expire a token, hit the endpoint. Expected 401, got 500. Production traffic affected.",
                "labels": ["api", "auth"],
            },
            {
                "number": 101,
                "title": "Typo in onboarding email",
                "body": "Says 'wlcome' instead of 'welcome'.",
                "labels": ["docs"],
            },
            {
                "number": 102,
                "title": "Add dark mode",
                "body": "Users have asked for a dark theme. Low priority.",
                "labels": ["enhancement"],
            },
        ]);
        context
            .insert("open_issues", issues)
            .map_err(|e| fail("fetch_open_issues", format!("insert: {e}")))?;
        Ok(())
    }

    /// Decision task — asks the agent to classify each issue's severity.
    ///
    /// **Stubbed.** The real call goes through cloacina-workflow-plugin's
    /// agent service, which arawn injects at workflow-load time. The shape
    /// is something like:
    ///
    /// ```ignore
    /// let agent = cloacina_workflow_plugin::agent::current()
    ///     .ok_or_else(|| fail("classify_severity", "no agent available"))?;
    /// let prompt = format!(
    ///     "Classify each issue as P0 (production-down), P1 (urgent), \
    ///      P2 (normal), or P3 (cosmetic). Return JSON: \
    ///      [{{\"number\": N, \"severity\": \"P0|P1|P2|P3\"}}, ...]\n\n{}",
    ///     serde_json::to_string_pretty(&issues).unwrap()
    /// );
    /// let response = agent.complete(&prompt).await
    ///     .map_err(|e| fail("classify_severity", format!("agent: {e}")))?;
    /// let classifications: Value = serde_json::from_str(&response)
    ///     .map_err(|e| fail("classify_severity", format!("parse: {e}")))?;
    /// ```
    ///
    /// Until the integration API stabilizes, this stub returns deterministic
    /// classifications based on the issue title — enough to demonstrate the
    /// downstream conditional action.
    #[task(id = "classify_severity", dependencies = ["fetch_open_issues"])]
    pub async fn classify_severity(context: &mut Context<Value>) -> Result<(), TaskError> {
        let issues = context
            .get("open_issues")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // TODO: replace this stub with a real agent.complete() call (see
        // doc-comment above for the shape).
        let classifications: Vec<Value> = issues
            .iter()
            .map(|issue| {
                let title = issue["title"].as_str().unwrap_or("");
                let severity = if title.contains("500") || title.to_lowercase().contains("production") {
                    "P0"
                } else if title.to_lowercase().contains("urgent") {
                    "P1"
                } else if title.to_lowercase().contains("typo") {
                    "P3"
                } else {
                    "P2"
                };
                json!({"number": issue["number"], "severity": severity})
            })
            .collect();

        context
            .insert("classifications", json!(classifications))
            .map_err(|e| fail("classify_severity", format!("insert: {e}")))?;
        Ok(())
    }

    /// Action task — only does work if classifications include at least one P0.
    ///
    /// Cloacina has no native way to *skip* a task based on upstream data
    /// (you'd use `trigger_rules` for stricter gating, out of scope here).
    /// The pattern shown is "task always runs, but early-returns when there's
    /// nothing to do."
    #[task(id = "notify_if_p0", dependencies = ["classify_severity"])]
    pub async fn notify_if_p0(context: &mut Context<Value>) -> Result<(), TaskError> {
        let classifications = context
            .get("classifications")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let p0s: Vec<&Value> = classifications
            .iter()
            .filter(|c| c["severity"].as_str() == Some("P0"))
            .collect();

        if p0s.is_empty() {
            eprintln!("[issue_triage] no P0 issues — no notification sent");
            return Ok(());
        }

        // TODO: real notification — Slack webhook, email, push, etc. Stub logs.
        eprintln!(
            "[issue_triage] {} P0 issue(s) flagged: {:?}",
            p0s.len(),
            p0s.iter()
                .map(|c| c["number"].as_i64().unwrap_or(0))
                .collect::<Vec<_>>()
        );
        Ok(())
    }
}

// No #[trigger] — this workflow is invoked on demand. Either the agent calls
// it via workflow_status / workflow_run, or you can add a cron trigger here:
//
// #[trigger(on = "issue_triage", cron = "0 9 * * 1-5", timezone = "UTC")]
// pub async fn scheduled() {}

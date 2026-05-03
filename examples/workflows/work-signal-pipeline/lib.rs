//! DAG-with-parallel-ingestion example.
//!
//! Three independent ingestion tasks run in parallel; an aggregator waits
//! for all three; prioritization and briefing run sequentially after.
//!
//! See README.md for how to turn this source listing into a buildable crate
//! by copying boilerplate from ../daily-pr-summary/.

use cloacina_workflow::{workflow, task, Context, TaskError};
use cloacina_workflow::trigger;
use serde_json::{json, Value};

fn fail(task_id: &str, message: impl Into<String>) -> TaskError {
    TaskError::Unknown {
        task_id: task_id.to_string(),
        message: message.into(),
    }
}

#[workflow(
    name = "work_signal_pipeline",
    description = "Daily briefing from three signal sources: meetings, Slack, Jira."
)]
pub mod work_signal_pipeline {
    use super::*;

    // ── Parallel ingestion ────────────────────────────────────────────────
    // All three of these have `dependencies = []`. The runner schedules them
    // concurrently. Each writes a distinct context key for the aggregator
    // to pick up.

    #[task(id = "fetch_meeting_notes", dependencies = [])]
    pub async fn fetch_meeting_notes(context: &mut Context<Value>) -> Result<(), TaskError> {
        // TODO: real fetch — e.g. read transcripts from a known directory,
        // or call a meeting-bot API. Stub returns one fake meeting.
        let signals = json!([
            {"source": "meeting", "topic": "alpha launch", "decision": "ship friday"},
        ]);
        context
            .insert("meeting_signals", signals)
            .map_err(|e| fail("fetch_meeting_notes", format!("insert: {e}")))?;
        Ok(())
    }

    #[task(id = "fetch_slack_digest", dependencies = [])]
    pub async fn fetch_slack_digest(context: &mut Context<Value>) -> Result<(), TaskError> {
        // TODO: real fetch — e.g. Slack API search, or read a digest file
        // produced by a separate ETL. Stub returns one fake message.
        let signals = json!([
            {"source": "slack", "channel": "#eng", "summary": "perf regression in API v3"},
        ]);
        context
            .insert("slack_signals", signals)
            .map_err(|e| fail("fetch_slack_digest", format!("insert: {e}")))?;
        Ok(())
    }

    #[task(id = "fetch_jira_updates", dependencies = [])]
    pub async fn fetch_jira_updates(context: &mut Context<Value>) -> Result<(), TaskError> {
        // TODO: real fetch — e.g. Jira REST `search` for issues updated in
        // the last 24h. Stub returns one fake issue.
        let signals = json!([
            {"source": "jira", "key": "ENG-1234", "status": "in_review", "owner": "alice"},
        ]);
        context
            .insert("jira_signals", signals)
            .map_err(|e| fail("fetch_jira_updates", format!("insert: {e}")))?;
        Ok(())
    }

    // ── Fan-in ────────────────────────────────────────────────────────────
    // Depends on all three. Cloacina waits for all upstream tasks to finish
    // (success only — failures abort the downstream by default) before
    // invoking this.

    #[task(
        id = "aggregate_signals",
        dependencies = ["fetch_meeting_notes", "fetch_slack_digest", "fetch_jira_updates"]
    )]
    pub async fn aggregate_signals(context: &mut Context<Value>) -> Result<(), TaskError> {
        let mut all = Vec::new();
        for key in ["meeting_signals", "slack_signals", "jira_signals"] {
            if let Some(arr) = context.get(key).and_then(|v| v.as_array()) {
                all.extend(arr.iter().cloned());
            }
        }
        context
            .insert("unified_signals", json!(all))
            .map_err(|e| fail("aggregate_signals", format!("insert: {e}")))?;
        Ok(())
    }

    // ── Sequential post-processing ────────────────────────────────────────

    #[task(id = "prioritize_signals", dependencies = ["aggregate_signals"])]
    pub async fn prioritize_signals(context: &mut Context<Value>) -> Result<(), TaskError> {
        let signals = context
            .get("unified_signals")
            .cloned()
            .unwrap_or_else(|| json!([]));
        // TODO: real prioritization — score by urgency / impact / staleness.
        // Stub: keep them in arrival order.
        context
            .insert("prioritized_signals", signals)
            .map_err(|e| fail("prioritize_signals", format!("insert: {e}")))?;
        Ok(())
    }

    #[task(id = "write_briefing", dependencies = ["prioritize_signals"])]
    pub async fn write_briefing(context: &mut Context<Value>) -> Result<(), TaskError> {
        let signals = context
            .get("prioritized_signals")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut md = String::from("# Daily briefing\n\n");
        for s in &signals {
            md.push_str(&format!("- {}\n", s));
        }
        // TODO: write to ~/Documents/briefings/YYYY-MM-DD.md (see linear example).
        eprintln!("[work_signal_pipeline] briefing built: {} signals", signals.len());
        Ok(())
    }
}

#[trigger(on = "work_signal_pipeline", cron = "0 8 * * 1-5", timezone = "UTC")]
pub async fn scheduled() {}

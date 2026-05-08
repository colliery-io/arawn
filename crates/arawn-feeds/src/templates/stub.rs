//! `stub/echo` — runtime-only template used by tests.
//!
//! Param: `{ "message": "..." }` (defaults to empty if absent).
//! On each run: appends one JSONL line `{ ts, run, message }` to
//! `feed_dir/log.jsonl` and increments the cursor's `run_count`.
//!
//! No provider access. Lets us exercise the runtime + cloacina
//! integration without involving any real provider client.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};

use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct EchoTemplate;

const NAME: &str = "stub/echo";

#[async_trait]
impl FeedTemplate for EchoTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        // No required fields — `message` is optional and any
        // template-specific shape is accepted.
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/15 * * * *".into(),
            initial_cursor: json!({ "run_count": 0 }),
        }
    }

    async fn run(
        &self,
        _ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let prev_run = cursor.get("run_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let next_run = prev_run + 1;

        let message = params.get_str("message").unwrap_or("");
        let line = json!({
            "ts": Utc::now().to_rfc3339(),
            "run": next_run,
            "message": message,
        });

        let log_path = feed_dir.join("log.jsonl");
        let serialized = serde_json::to_string(&line)
            .map_err(|e| FeedError::Storage(format!("serialize log line: {e}")))?;
        let body = format!("{serialized}\n");

        // Append-only — preserves prior runs' lines.
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| FeedError::Storage(format!("open {}: {e}", log_path.display())))?;
        f.write_all(body.as_bytes())
            .map_err(|e| FeedError::Storage(format!("write {}: {e}", log_path.display())))?;

        Ok(RunOutcome {
            cursor: json!({ "run_count": next_run }),
            summary: RunSummary {
                items_written: 1,
                bytes_written: body.len() as u64,
                duration: started.elapsed(),
            },
            status: "ok".into(),
        })
    }
}

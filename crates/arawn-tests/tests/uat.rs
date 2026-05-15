//! End-to-end UAT: starts an isolated arawn server, drives multi-turn
//! conversations via WebSocket, collects artifacts, runs mechanical checks.
//!
//! Requires a real LLM (Ollama Cloud / Groq). Gated behind #[ignore].
//!
//! Run: cargo test -p arawn-tests --test uat -- --ignored --nocapture
//! Or via angreal: angreal test uat --model gemma4

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use uuid::Uuid;

// ============================================================================
// Scenario Definition
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub objective: String,
    pub turns: Vec<ScenarioTurn>,
    pub mechanical: MechanicalThresholds,
    /// Optional path to a JSON fixture (relative to `arawn-tests`'s
    /// CARGO_MANIFEST_DIR) loaded by `uat_fixture::apply` before the
    /// server starts. Used to pre-populate projections (and drive the
    /// extractor) so the agent sees a warm KB on turn 1.
    #[serde(default)]
    pub seed_fixture: Option<String>,
    /// When true, the harness runs `drive_tag_promoter` after seed
    /// extraction so promotion proposals exist before turn 1. Opt-in
    /// because the side effect (a stray promote_tag journal row) can
    /// pollute scenarios whose turns expect *only* dust proposals on
    /// the journal — UAT 23:31's signal-extraction-e2e regression
    /// surfaced exactly this when the agent flaked on its dust retry
    /// and then `workstream_refine` returned only the tag-promoter
    /// proposal as a confused fallback target.
    #[serde(default)]
    pub seed_tag_promoter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTurn {
    pub user_message: String,
    pub judge_expectation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicalThresholds {
    pub min_files_created: usize,
    #[serde(default)]
    pub min_workflows_created: usize,
    pub min_memory_entities: usize,
    pub max_tool_errors: usize,
}

// ============================================================================
// Turn Result (collected during execution)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct TurnResult {
    pub turn_number: usize,
    pub user_message: String,
    pub assistant_text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub tool_results: Vec<ToolResultRecord>,
    pub engine_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub completed: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResultRecord {
    pub id: String,
    pub content: String,
    pub is_error: bool,
}

// ============================================================================
// Scenario Result
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub model: String,
    pub turns: Vec<TurnResult>,
    pub mechanical: MechanicalCheckResult,
    pub workspace_files: Vec<String>,
    pub total_duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct MechanicalCheckResult {
    pub all_turns_completed: bool,
    pub no_errors: bool,
    pub tool_use_occurred: bool,
    pub files_created: usize,
    pub workflows_created: usize,
    pub tool_errors: usize,
    pub pass: bool,
}

// ============================================================================
// Event Handling (extracted for testability)
// ============================================================================

/// State accumulated while consuming engine events for a single turn.
#[derive(Debug, Default)]
struct TurnAccumulator {
    assistant_text: String,
    tool_calls: Vec<ToolCallRecord>,
    tool_results: Vec<ToolResultRecord>,
    engine_error: bool,
    error_message: Option<String>,
    completed: bool,
}

/// Count subdirectories of `dir`. Each `workflow_create` install lands as
/// `<dir>/<name>/{libname.dylib, package.toml}`, so subdir count == installed workflow count.
fn count_workflows_in(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count()
}

/// Apply one engine event JSON value to the accumulator.
/// Returns `true` if this event terminates the turn (Complete or Error).
fn apply_event(event: &Value, acc: &mut TurnAccumulator) -> bool {
    // Skip the RPC ack response (it has both `id` and `result`, no `event`).
    if event.get("id").is_some() && event.get("result").is_some() {
        return false;
    }

    match event.get("event").and_then(|e| e.as_str()) {
        Some("StreamingText") => {
            if let Some(t) = event["data"]["text"].as_str() {
                acc.assistant_text.push_str(t);
            }
            false
        }
        Some("ToolCallStart") => {
            acc.tool_calls.push(ToolCallRecord {
                id: event["data"]["id"].as_str().unwrap_or("").to_string(),
                name: event["data"]["name"].as_str().unwrap_or("").to_string(),
                input: event["data"]["input"].clone(),
            });
            false
        }
        Some("ToolCallResult") => {
            let is_err = event["data"]["is_error"].as_bool().unwrap_or(false);
            acc.tool_results.push(ToolResultRecord {
                id: event["data"]["id"].as_str().unwrap_or("").to_string(),
                content: event["data"]["content"].as_str().unwrap_or("").to_string(),
                is_error: is_err,
            });
            false
        }
        Some("Complete") => {
            if let Some(t) = event["data"]["final_text"].as_str() {
                acc.assistant_text = t.to_string();
            }
            acc.completed = true;
            true
        }
        Some("Error") => {
            acc.engine_error = true;
            acc.error_message = event["data"]["message"]
                .as_str()
                .map(|s| s.to_string());
            true
        }
        _ => false, // Flush, Usage, Warning, etc.
    }
}

// ============================================================================
// UAT Harness
// ============================================================================

pub struct UatHarness {
    data_dir: PathBuf,
    port: u16,
    server_process: Option<Child>,
}

impl UatHarness {
    /// Create a new harness with an isolated data directory.
    pub fn new(base_dir: &Path, model: &str, provider: &str, api_key_env: &str) -> Self {
        let data_dir = base_dir.to_path_buf();
        let port = 3100 + (std::process::id() % 1000) as u16; // semi-random port

        // Create data dir and write config
        std::fs::create_dir_all(&data_dir).expect("create data dir");

        // api_key_env line is omitted if empty (local ollama doesn't need one)
        let api_key_line = if api_key_env.is_empty() {
            String::from("api_key_env = \"\"")
        } else {
            format!("api_key_env = \"{api_key_env}\"")
        };

        let config = format!(
            r#"[llm.default]
provider = "{provider}"
model = "{model}"
{api_key_line}
context_window = 128000
max_tokens = 8192

[engine]
llm = "default"
max_iterations = 30

[compactor]
compaction_threshold = 0.85

[server]
host = "127.0.0.1"
port = {port}

[storage]
data_dir = "{data_dir}"

[sandbox]
network_tools = ["gh", "curl"]
"#,
            provider = provider,
            model = model,
            api_key_line = api_key_line,
            port = port,
            data_dir = data_dir.display(),
        );

        std::fs::write(data_dir.join("arawn.toml"), &config).expect("write config");

        Self {
            data_dir,
            port,
            server_process: None,
        }
    }

    /// Start the arawn server process.
    pub fn start_server(&mut self) -> Result<(), String> {
        let binary = std::env::var("ARAWN_BINARY").unwrap_or_else(|_| {
            // Find the binary relative to the workspace root
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            PathBuf::from(manifest_dir)
                .parent() // crates/
                .unwrap()
                .parent() // workspace root
                .unwrap()
                .join("target/debug/arawn")
                .to_string_lossy()
                .to_string()
        });

        let child = Command::new(&binary)
            .args(["--data-dir", &self.data_dir.to_string_lossy(), "serve", "--port", &self.port.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to start server: {e}"))?;

        self.server_process = Some(child);
        Ok(())
    }

    /// Wait for the server to be ready by polling the WebSocket endpoint.
    pub async fn wait_for_ready(&self, timeout: Duration) -> Result<(), String> {
        let start = Instant::now();

        // Wait for server.token to appear (server writes it before binding)
        let token_path = self.data_dir.join("server.token");
        while !token_path.exists() && start.elapsed() < timeout {
            sleep(Duration::from_millis(200)).await;
        }

        // Then poll the WS endpoint with the token
        while start.elapsed() < timeout {
            let url = self.ws_url();
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws, _)) => {
                    // Close cleanly so the server doesn't log a disconnect error
                    drop(ws);
                    sleep(Duration::from_millis(200)).await;
                    return Ok(());
                }
                Err(_) => sleep(Duration::from_millis(500)).await,
            }
        }

        Err(format!("server not ready after {:?}", timeout))
    }

    pub fn ws_url(&self) -> String {
        // Read token from data dir
        let token = std::fs::read_to_string(self.data_dir.join("server.token"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let base = format!("ws://127.0.0.1:{}/ws", self.port);
        match token {
            Some(t) => format!("{base}?token={t}"),
            None => base,
        }
    }

    /// Run a scenario: create session, drive all turns, collect results.
    pub async fn run_scenario(&self, scenario: &Scenario, model: &str) -> ScenarioResult {
        let start = Instant::now();
        let url = self.ws_url();

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect to server");
        let (mut write, mut read) = ws_stream.split();

        // Create session
        let session_id = self.rpc_create_session(&mut write, &mut read).await;

        // Drive each turn
        let mut turns = Vec::new();
        for (i, turn) in scenario.turns.iter().enumerate() {
            let turn_start = Instant::now();
            let result = self.drive_turn(
                &mut write,
                &mut read,
                session_id,
                i + 1,
                &turn.user_message,
            ).await;
            let mut result = result;
            result.duration_ms = turn_start.elapsed().as_millis() as u64;
            turns.push(result);
        }

        // Collect workspace files
        let workspace_files = self.list_workspace_files();
        let workflows_created = self.count_installed_workflows();

        // Mechanical checks
        let all_completed = turns.iter().all(|t| t.completed);
        let no_engine_errors = turns.iter().all(|t| !t.engine_error);
        let tool_use = turns.iter().any(|t| !t.tool_calls.is_empty());
        let tool_errors = turns.iter().flat_map(|t| &t.tool_results).filter(|r| r.is_error).count();

        let mech_pass = all_completed
            && no_engine_errors
            && workspace_files.len() >= scenario.mechanical.min_files_created
            && workflows_created >= scenario.mechanical.min_workflows_created;

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            model: model.to_string(),
            turns,
            mechanical: MechanicalCheckResult {
                all_turns_completed: all_completed,
                no_errors: no_engine_errors,
                tool_use_occurred: tool_use,
                files_created: workspace_files.len(),
                workflows_created,
                tool_errors,
                pass: mech_pass,
            },
            workspace_files,
            total_duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    async fn rpc_create_session(
        &self,
        write: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            WsMessage,
        >,
        read: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    ) -> Uuid {
        use futures_util::SinkExt;
        let req = json!({"id": 1, "method": "create_session", "params": {"workstream_id": null}});
        write.send(WsMessage::Text(req.to_string().into())).await.unwrap();

        // Read response
        while let Some(Ok(msg)) = read.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(resp) = serde_json::from_str::<Value>(&text) {
                    if let Some(result) = resp.get("result") {
                        let id_str = result["id"].as_str().unwrap();
                        return Uuid::parse_str(id_str).unwrap();
                    }
                }
            }
        }
        panic!("failed to create session");
    }

    async fn drive_turn(
        &self,
        write: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            WsMessage,
        >,
        read: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
        session_id: Uuid,
        turn_number: usize,
        user_message: &str,
    ) -> TurnResult {
        use futures_util::SinkExt;

        // Send message
        let req = json!({
            "id": turn_number as u64 + 100,
            "method": "send_message",
            "params": {"session_id": session_id.to_string(), "content": user_message}
        });
        write.send(WsMessage::Text(req.to_string().into())).await.unwrap();

        let mut acc = TurnAccumulator::default();

        // Collect events until Complete or Error
        while let Some(Ok(msg)) = read.next().await {
            let text = match msg {
                WsMessage::Text(t) => t,
                _ => continue,
            };

            let event: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if apply_event(&event, &mut acc) {
                break;
            }
        }

        TurnResult {
            turn_number,
            user_message: user_message.to_string(),
            assistant_text: acc.assistant_text,
            tool_calls: acc.tool_calls,
            tool_results: acc.tool_results,
            engine_error: acc.engine_error,
            error_message: acc.error_message,
            completed: acc.completed,
            duration_ms: 0, // filled by caller
        }
    }

    fn list_workspace_files(&self) -> Vec<String> {
        let ws_dir = self.data_dir.join("workstreams");
        let mut files = Vec::new();
        if let Ok(entries) = walkdir(&ws_dir) {
            for entry in entries {
                if entry.is_file() && !entry.to_string_lossy().contains("memory.db") {
                    if let Ok(relative) = entry.strip_prefix(&ws_dir) {
                        files.push(relative.to_string_lossy().to_string());
                    }
                }
            }
        }
        files
    }

    /// Count installed workflows under `<data_dir>/workflows/`.
    fn count_installed_workflows(&self) -> usize {
        count_workflows_in(&self.data_dir.join("workflows"))
    }

    /// Write all artifacts to the results directory.
    pub fn write_artifacts(&self, result: &ScenarioResult, scenario: &Scenario) {
        let results_dir = self.data_dir
            .join("uat-results")
            .join(&result.scenario_name)
            .join(&result.model);
        std::fs::create_dir_all(&results_dir).expect("create results dir");

        // transcript.jsonl
        let transcript_path = results_dir.join("transcript.jsonl");
        let mut transcript = String::new();
        for turn in &result.turns {
            transcript.push_str(&serde_json::to_string(turn).unwrap());
            transcript.push('\n');
        }
        std::fs::write(&transcript_path, &transcript).unwrap();

        // mechanical.json
        let mech_path = results_dir.join("mechanical.json");
        std::fs::write(&mech_path, serde_json::to_string_pretty(&result.mechanical).unwrap()).unwrap();

        // scenario.md (rubric for judge)
        let mut rubric = format!("# {}\n\n## Objective\n{}\n\n## Per-Turn Expectations\n",
            scenario.name, scenario.objective);
        for (i, turn) in scenario.turns.iter().enumerate() {
            rubric.push_str(&format!("\n### Turn {}\n**User**: {}\n**Expectation**: {}\n",
                i + 1, turn.user_message, turn.judge_expectation));
        }
        std::fs::write(results_dir.join("scenario.md"), &rubric).unwrap();

        // workspace/ snapshot
        let ws_snapshot_dir = results_dir.join("workspace");
        std::fs::create_dir_all(&ws_snapshot_dir).ok();
        let ws_dir = self.data_dir.join("workstreams");
        if let Ok(entries) = walkdir(&ws_dir) {
            for entry in entries {
                if entry.is_file() {
                    if let Ok(relative) = entry.strip_prefix(&ws_dir) {
                        let dest = ws_snapshot_dir.join(relative);
                        if let Some(parent) = dest.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }
                        std::fs::copy(&entry, &dest).ok();
                    }
                }
            }
        }

        println!("  Artifacts written to: {}", results_dir.display());
    }

    /// Stop the server process.
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.server_process {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.server_process = None;
    }
}

impl Drop for UatHarness {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Recursively list all files under a directory.
fn walkdir(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

// ============================================================================
// Scenarios
// ============================================================================

fn github_monitor_scenario() -> Scenario {
    Scenario {
        name: "github-monitor".to_string(),
        objective: "Design and implement a daily process for monitoring the colliery-io GitHub organization. Track new PRs, reported issues, and release activity. Produce scripts and a report template.".to_string(),
        turns: vec![
            ScenarioTurn {
                user_message: "I need you to design a daily process for monitoring the colliery-io GitHub organization. I want to track new PRs, reported issues, and any release activity across all repos. Think through the approach first, then outline the steps.".to_string(),
                judge_expectation: "Agent should use the think tool to plan. Response should outline a multi-step monitoring process covering PRs, issues, and releases with a daily cadence.".to_string(),
            },
            ScenarioTurn {
                user_message: "Great. Now implement the intake step — write a script that uses the GitHub CLI (gh) to fetch open PRs and issues from the last 24 hours for the colliery-io org.".to_string(),
                judge_expectation: "Agent should create a file using file_write. The file should contain a script using 'gh api' or 'gh pr list'/'gh issue list' commands targeting colliery-io repositories.".to_string(),
            },
            ScenarioTurn {
                user_message: "Add a prioritization step that categorizes issues by severity based on labels and age. Write this as a separate module or function.".to_string(),
                judge_expectation: "Agent should create or modify a file. The code should implement prioritization logic referencing labels (bug, enhancement, critical, etc.) and time-based aging.".to_string(),
            },
            ScenarioTurn {
                user_message: "Now write a summary report template in markdown that I'd review each morning. It should have sections for critical items, new PRs needing review, and a stats summary.".to_string(),
                judge_expectation: "Agent should create a markdown template file with structured sections. The template should cover critical/high-priority items, PR review queue, and aggregate statistics.".to_string(),
            },
        ],
        mechanical: MechanicalThresholds {
            min_files_created: 2,
            min_workflows_created: 0,
            min_memory_entities: 0,
            max_tool_errors: 2,
        },
        seed_fixture: None,
        seed_tag_promoter: false,
    }
}

fn work_signal_pipeline_scenario() -> Scenario {
    Scenario {
        name: "work-signal-pipeline".to_string(),
        objective: "Build a daily work signal processing pipeline as an arawn workflow using the workflow_create tool. The workflow should intake meeting transcripts, Slack exports, and task updates, extract action items, and produce a prioritized daily briefing on a cron schedule.".to_string(),
        turns: vec![
            ScenarioTurn {
                user_message: "I need a daily work signal processing pipeline built as an arawn workflow — use the workflows skill to learn how, then use workflow_create to build it. Every morning it should intake signals from multiple sources — meeting transcripts, Slack channel exports, and Jira updates — then analyze, extract action items, and produce a prioritized daily briefing. Think through the architecture first.".to_string(),
                judge_expectation: "Agent should invoke skill('workflows') to load the workflow authoring guide, then use think/plan mode to design the pipeline architecture as a cloacina DAG with data tasks, decision tasks, and action tasks.".to_string(),
            },
            ScenarioTurn {
                user_message: "Let's start with the transcript processor. Write the ingestion task that fetches and parses meeting transcripts to extract: attendees, key decisions, action items with owners, and follow-up dates.".to_string(),
                judge_expectation: "Agent should create code for a workflow task (Rust function body) that processes transcripts. May use file_write for supporting modules or workflow_create for the task definition.".to_string(),
            },
            ScenarioTurn {
                user_message: "Now write the signal aggregator task that combines outputs from transcript processing, Slack digests, and task tracker updates into a unified daily signal feed.".to_string(),
                judge_expectation: "Agent should create an aggregator — either as a workflow task body or supporting module. Should handle multiple input sources and merge into a unified structure.".to_string(),
            },
            ScenarioTurn {
                user_message: "Add a prioritization step that ranks signals by urgency (time-sensitive items first), impact (cross-team items higher), and staleness (older unresolved items bubble up).".to_string(),
                judge_expectation: "Agent should create prioritization logic with three scoring dimensions. May be a workflow decision task that uses the arawn agent for LLM-powered ranking.".to_string(),
            },
            ScenarioTurn {
                user_message: "Now create the complete workflow using workflow_create with all the tasks wired together as a DAG, scheduled to run at 8 AM on weekdays. Include a final task that generates the briefing report.".to_string(),
                judge_expectation: "Agent should call workflow_create with a full DAG spec: ingestion tasks → aggregation → prioritization → briefing generation, with cron schedule '0 8 * * 1-5'. This is the key deliverable.".to_string(),
            },
        ],
        mechanical: MechanicalThresholds {
            min_files_created: 0,
            min_workflows_created: 1,
            min_memory_entities: 0,
            max_tool_errors: 2,
        },
        seed_fixture: None,
        seed_tag_promoter: false,
    }
}

#[path = "uat_fixture.rs"]
mod uat_fixture;

/// I-0040 end-to-end UAT: synthetic gmail + slack feed rows for two
/// workstreams, extractor runs during seed so the KB is warm, agent
/// then drives signal_search / signal_query / signal_timeline /
/// workstream_journal / workstream_dust / workstream_refine /
/// workstream_apply / workstream_rollback against real data.
fn signal_extraction_e2e_scenario() -> Scenario {
    Scenario {
        name: "signal-extraction-e2e".to_string(),
        objective: "Drive the I-0040 read + curation surface against two seeded workstreams. The seed loader pre-populates projections.db with synthetic gmail + slack rows for `work` and `dnd` workstreams, then runs the extractor synchronously so the agent sees a warm KB on turn 1.".to_string(),
        turns: vec![
            ScenarioTurn {
                user_message: "Switch to the `work` workstream, then use signal_search to find what we decided about Postgres. Quote the decision title and any key rationale.".to_string(),
                judge_expectation: "Agent should call workstream_switch (or workstream_show) then signal_search with a query like \"postgres\". Should surface the ledger/postgres decision extracted from the seeded gmail rows.".to_string(),
            },
            ScenarioTurn {
                user_message: "Use signal_query to list every Convention in this workstream — I want to see what process rules are codified.".to_string(),
                judge_expectation: "Agent should call signal_query with entity_type=\"convention\". Should return at least the on-call and code-review conventions extracted from the seeded rows.".to_string(),
            },
            ScenarioTurn {
                user_message: "First call workstream_switch to move into the `dnd` workstream — wait for the switch to confirm before doing anything else. Then call signal_timeline once to see the latest plot thread. Don't issue these as parallel tool calls; signal_timeline reads the active workstream's KB and will return the wrong data if it runs before the switch lands.".to_string(),
                judge_expectation: "Agent should call workstream_switch and signal_timeline as two SEQUENTIAL calls (not parallel). Should mention the Calidor / cult tracking arc as a recent plot thread.".to_string(),
            },
            ScenarioTurn {
                user_message: "We have a couple of old falcon-project entries in the `work` workstream that are stale. Switch back to `work` and run workstream_dust on the falcon cluster — preview the proposed summary before we commit anything.".to_string(),
                judge_expectation: "Agent should switch workstreams and call workstream_dust with tags=[\"falcon\"] (or similar). Returns dust proposals with a summary entity. Should report the proposal id(s) but NOT auto-apply.".to_string(),
            },
            ScenarioTurn {
                user_message: "List all pending steward proposals via workstream_refine so I can see what map / dust / doorwatch have suggested.".to_string(),
                judge_expectation: "Agent should call workstream_refine. Output should include the dust proposal from the previous turn (applied=false).".to_string(),
            },
            ScenarioTurn {
                user_message: "Apply the falcon dust proposal — pass the id you saw in refine to workstream_apply.".to_string(),
                judge_expectation: "Agent should call workstream_apply with the dust proposal's id. Status should be \"applied\". A new summary entity now exists in the work KB.".to_string(),
            },
            ScenarioTurn {
                user_message: "Confirm the apply worked: signal_search for \"falcon\" — you should see the new summary entity.".to_string(),
                judge_expectation: "signal_search should return the dust summary among the hits. Confirms apply mutated the KB as expected.".to_string(),
            },
            ScenarioTurn {
                user_message: "Actually, roll that apply back — I want to double-check the originals are still there. Use workstream_rollback with the same id.".to_string(),
                judge_expectation: "Agent calls workstream_rollback. Status: \"reverted\". A subsequent signal_search for \"falcon\" should show the originals but not the summary (the summary's SUMMARIZES edges are gone and the summary entity is removed from the KB).".to_string(),
            },
        ],
        mechanical: MechanicalThresholds {
            min_files_created: 0,
            min_workflows_created: 0,
            // Seed runs the extractor; even modestly stingy classification
            // should produce >= 6 entities across the two workstreams.
            min_memory_entities: 6,
            max_tool_errors: 3,
        },
        seed_fixture: Some("tests/fixtures/uat/signal-extraction-e2e.json".to_string()),
        // Dust scenario: don't pre-seed promotion proposals. A stray
        // tag-promoter row would confuse turn 5's workstream_refine
        // when the dust path itself stalls (UAT 23:31 regression).
        seed_tag_promoter: false,
    }
}

/// I-0040 T-0268: tag-promoter Extract→Suggest→Add cycle UAT.
/// Reuses the signal-extraction-e2e fixture, but exercises the
/// ontology growth path:
///   1. Inspect the active workstream's ontology.
///   2. Review pending steward proposals (tag-promoter should have
///      surfaced multiple promotion candidates after seed).
///   3. Apply one promotion.
///   4. Verify it now appears in the ontology with `added_via=promotion`.
///   5. Roll back.
///   6. Verify it's gone again.
fn tag_promoter_cycle_scenario() -> Scenario {
    Scenario {
        name: "tag-promoter-cycle".to_string(),
        objective: "Drive the I-0040 Extract→Suggest→Add cycle for tag promotion. The seed loader runs the tag-promoter subroutine after extraction so pending promotion proposals exist before turn 1.".to_string(),
        turns: vec![
            ScenarioTurn {
                user_message: "Switch to the `work` workstream, then use workstream_show to tell me what's currently in this workstream's tag ontology.".to_string(),
                judge_expectation: "Agent calls workstream_switch then workstream_show; reports the seeded ontology tags (falcon, ledger, postgres, on-call, code-review, rfc, team, infrastructure, migration, process).".to_string(),
            },
            ScenarioTurn {
                user_message: "Use workstream_refine to list any pending steward proposals — especially tag-promotion proposals. Summarize what each one would do if I applied it.".to_string(),
                judge_expectation: "Agent calls workstream_refine and reports at least one tag-promoter proposal with a tag name and a count. May explain each proposal would add the proposed tag to the ontology.".to_string(),
            },
            ScenarioTurn {
                user_message: "Pick the most useful-looking tag-promotion proposal. **First** call workstream_apply with the proposal id — wait for the response (status will be `applied`). **Only after** that call returns, call workstream_show to read the updated ontology. Do NOT issue workstream_apply and workstream_show as parallel tool calls in one response — they must be sequential or workstream_show will see the pre-apply ontology. Then report the updated tags_ontology list from workstream_show.".to_string(),
                judge_expectation: "Agent issues workstream_apply and workstream_show as SEQUENTIAL tool calls (apply finishes before show starts). The show response's `tags_ontology` array should contain the newly-promoted tag.".to_string(),
            },
            ScenarioTurn {
                user_message: "Roll it back with workstream_rollback. The `status` field in the response confirms whether it worked — don't double-check with workstream_show or workstream_tag. Just report the rollback status.".to_string(),
                judge_expectation: "Agent calls workstream_rollback exactly once (status=reverted) and reports the status. No additional verification tool calls.".to_string(),
            },
        ],
        mechanical: MechanicalThresholds {
            min_files_created: 0,
            min_workflows_created: 0,
            min_memory_entities: 4,
            max_tool_errors: 2,
        },
        seed_fixture: Some("tests/fixtures/uat/signal-extraction-e2e.json".to_string()),
        // This scenario IS the tag-promoter cycle — seed the proposals
        // so refine on turn 2 has something to find.
        seed_tag_promoter: true,
    }
}

fn all_scenarios() -> Vec<Scenario> {
    vec![
        github_monitor_scenario(),
        work_signal_pipeline_scenario(),
        signal_extraction_e2e_scenario(),
        tag_promoter_cycle_scenario(),
    ]
}

// ============================================================================
// Test Entry Point
// ============================================================================

#[tokio::test]
#[ignore] // Requires real LLM — run with: cargo test -p arawn-tests --test uat -- --ignored --nocapture
async fn uat_run() {
    // Config from env vars
    let model = std::env::var("UAT_MODEL").unwrap_or_else(|_| "gemma4:31b-cloud".to_string());
    let provider = std::env::var("UAT_PROVIDER").unwrap_or_else(|_| "https://ollama.com/v1".to_string());
    let api_key_env = std::env::var("UAT_API_KEY_ENV").unwrap_or_else(|_| "OLLAMA_API_KEY".to_string());
    let scenario_filter = std::env::var("UAT_SCENARIO").ok();

    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let base_dir = PathBuf::from(format!("/tmp/arawn-uat-{ts}"));

    println!("\n======================================================================");
    println!("  Arawn UAT — {model} via {provider}");
    println!("  Data dir: {}", base_dir.display());
    println!("======================================================================\n");

    // Build the server binary first
    println!("  Building arawn...");
    let build = Command::new("cargo")
        .args(["build", "-p", "arawn"])
        .output()
        .expect("cargo build");
    if !build.status.success() {
        eprintln!("  BUILD FAILED:\n{}", String::from_utf8_lossy(&build.stderr));
        panic!("cargo build failed");
    }

    let scenarios: Vec<Scenario> = match scenario_filter {
        Some(ref name) => all_scenarios().into_iter().filter(|s| s.name == *name).collect(),
        None => all_scenarios(),
    };

    let mut results = Vec::new();

    for scenario in &scenarios {
        let scenario_dir = base_dir.join(&scenario.name);
        println!("  [{}/{}] Scenario: {}", results.len() + 1, scenarios.len(), scenario.name);

        let mut harness = UatHarness::new(&scenario_dir, &model, &provider, &api_key_env);

        // If the scenario declares a seed fixture, load it BEFORE the
        // server boots so the agent sees a warm KB on turn 1. Fixture
        // path is relative to the arawn-tests manifest dir.
        if let Some(ref fixture_rel) = scenario.seed_fixture {
            let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(fixture_rel);
            println!("    Seeding from fixture: {}", fixture_path.display());
            let fx = uat_fixture::load(&fixture_path).expect("load fixture");
            let applied =
                uat_fixture::apply(&fx, &scenario_dir).expect("apply fixture");

            // Build a transient LLM client matching the server config
            // and drive the extractor synchronously across each
            // (workstream, feed_type). Skipping this would leave the KB
            // empty — extraction is part of the pipeline under test.
            let client = uat_fixture::build_seed_llm_client(&provider, &model, &api_key_env)
                .expect("build seed llm");
            let cap = Duration::from_secs(15 * 60);
            let processed = uat_fixture::drive_extraction(
                &applied,
                &scenario_dir,
                client,
                model.clone(),
                cap,
            )
            .await
            .expect("drive extraction");
            println!(
                "    Seed extraction complete: {} projection rows processed across {} workstreams",
                processed,
                applied.per_workstream.len()
            );

            // Drive tag-promoter only when the scenario opts in.
            // Running it on every seeded scenario polluted the dust
            // path: a stray tag-promoter proposal became the wrong
            // refine target whenever the dust call itself didn't
            // produce one (UAT 23:31).
            if scenario.seed_tag_promoter {
                let promoted = uat_fixture::drive_tag_promoter(&applied, &scenario_dir)
                    .await
                    .expect("drive tag-promoter");
                println!(
                    "    Seed tag-promoter: {} promotion proposals journaled",
                    promoted
                );
            }
        }

        // Start server
        harness.start_server().expect("start server");
        println!("    Waiting for server...");
        harness.wait_for_ready(Duration::from_secs(60)).await.expect("server ready");
        println!("    Server ready on port {}", harness.port);

        // Run scenario
        let result = harness.run_scenario(scenario, &model).await;

        // Write artifacts
        harness.write_artifacts(&result, scenario);

        // Print summary
        println!("    Turns: {} | Files: {} | Workflows: {} | Tool errors: {} | Mechanical: {}",
            result.turns.len(),
            result.mechanical.files_created,
            result.mechanical.workflows_created,
            result.mechanical.tool_errors,
            if result.mechanical.pass { "PASS" } else { "FAIL" },
        );
        for turn in &result.turns {
            let tools: Vec<&str> = turn.tool_calls.iter().map(|t| t.name.as_str()).collect();
            println!("      Turn {}: {} tool(s) [{}] — {:.0}s {}",
                turn.turn_number,
                turn.tool_calls.len(),
                tools.join(", "),
                turn.duration_ms as f64 / 1000.0,
                if turn.completed { "OK" } else { "INCOMPLETE" },
            );
            if let Some(msg) = &turn.error_message {
                println!("        → ERROR: {msg}");
            }
        }

        harness.stop();
        results.push(result);
    }

    // Summary
    println!("\n======================================================================");
    println!("  UAT SUMMARY — {model}");
    println!("----------------------------------------------------------------------");
    println!("  {:<30} {:>10} {:>8} {:>10} {:>8} {:>8}", "Scenario", "Mechanical", "Files", "Workflows", "Errors", "Time");
    println!("----------------------------------------------------------------------");
    for r in &results {
        println!("  {:<30} {:>10} {:>8} {:>10} {:>8} {:>7.0}s",
            r.scenario_name,
            if r.mechanical.pass { "PASS" } else { "FAIL" },
            r.mechanical.files_created,
            r.mechanical.workflows_created,
            r.mechanical.tool_errors,
            r.total_duration_ms as f64 / 1000.0,
        );
    }
    println!("======================================================================");
    println!("  Results: {}", base_dir.display());
    println!("  Judge:   angreal test uat-judge --results {}\n", base_dir.display());

    // Fail if any mechanical check failed
    let all_pass = results.iter().all(|r| r.mechanical.pass);
    assert!(all_pass, "One or more scenarios failed mechanical checks");
}

// ============================================================================
// Unit tests — run via `cargo test -p arawn-tests --test uat` (no --ignored).
// These test the harness internals without spinning up a real LLM.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ARAWN-T-0192 — workflow counter behavior.

    #[test]
    fn count_workflows_returns_zero_for_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(count_workflows_in(&tmp.path().join("does-not-exist")), 0);
    }

    #[test]
    fn count_workflows_returns_zero_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(count_workflows_in(tmp.path()), 0);
    }

    #[test]
    fn count_workflows_counts_subdirs_only() {
        let tmp = tempfile::tempdir().unwrap();
        // Two installed workflows
        std::fs::create_dir(tmp.path().join("alpha")).unwrap();
        std::fs::create_dir(tmp.path().join("beta")).unwrap();
        // Loose files at this level should not count (they're not what `workflow_create` produces)
        std::fs::write(tmp.path().join("README"), "ignored").unwrap();
        assert_eq!(count_workflows_in(tmp.path()), 2);
    }

    // ARAWN-T-0191 — error_message capture.

    #[test]
    fn apply_event_captures_error_message() {
        let event = json!({
            "event": "Error",
            "data": {
                "message": "LLM error: authentication error: HTTP 403: subscription required"
            }
        });
        let mut acc = TurnAccumulator::default();
        let stop = apply_event(&event, &mut acc);
        assert!(stop, "Error event should terminate the turn");
        assert!(acc.engine_error);
        assert_eq!(
            acc.error_message.as_deref(),
            Some("LLM error: authentication error: HTTP 403: subscription required"),
        );
        assert!(!acc.completed);
    }

    #[test]
    fn apply_event_error_with_missing_message_field_keeps_none() {
        let event = json!({"event": "Error", "data": {}});
        let mut acc = TurnAccumulator::default();
        assert!(apply_event(&event, &mut acc));
        assert!(acc.engine_error);
        assert_eq!(acc.error_message, None);
    }

    #[test]
    fn apply_event_complete_sets_final_text() {
        let event = json!({"event": "Complete", "data": {"final_text": "all done"}});
        let mut acc = TurnAccumulator::default();
        assert!(apply_event(&event, &mut acc));
        assert!(acc.completed);
        assert!(!acc.engine_error);
        assert_eq!(acc.assistant_text, "all done");
    }

    #[test]
    fn apply_event_streaming_text_appends() {
        let mut acc = TurnAccumulator::default();
        for chunk in &["hello ", "world"] {
            let event = json!({"event": "StreamingText", "data": {"text": *chunk}});
            assert!(!apply_event(&event, &mut acc));
        }
        assert_eq!(acc.assistant_text, "hello world");
        assert!(!acc.completed);
    }

    #[test]
    fn apply_event_ignores_rpc_ack() {
        let event = json!({"id": 101, "result": {"ok": true}});
        let mut acc = TurnAccumulator::default();
        assert!(!apply_event(&event, &mut acc));
        assert!(acc.assistant_text.is_empty());
        assert!(!acc.engine_error);
        assert!(!acc.completed);
    }

    #[test]
    fn apply_event_records_tool_calls_and_results() {
        let mut acc = TurnAccumulator::default();
        apply_event(
            &json!({
                "event": "ToolCallStart",
                "data": {"id": "t1", "name": "file_write", "input": {"path": "x.md"}}
            }),
            &mut acc,
        );
        apply_event(
            &json!({
                "event": "ToolCallResult",
                "data": {"id": "t1", "content": "ok", "is_error": false}
            }),
            &mut acc,
        );
        assert_eq!(acc.tool_calls.len(), 1);
        assert_eq!(acc.tool_calls[0].name, "file_write");
        assert_eq!(acc.tool_results.len(), 1);
        assert!(!acc.tool_results[0].is_error);
    }

    // Transcript serialization — verify the new field round-trips.

    #[test]
    fn turn_result_serializes_error_message_when_present() {
        let result = TurnResult {
            turn_number: 1,
            user_message: "hi".into(),
            assistant_text: String::new(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            engine_error: true,
            error_message: Some("HTTP 403: bad".into()),
            completed: false,
            duration_ms: 500,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""error_message":"HTTP 403: bad""#), "got: {json}");
    }

    #[test]
    fn turn_result_omits_error_message_when_none() {
        let result = TurnResult {
            turn_number: 1,
            user_message: "hi".into(),
            assistant_text: "fine".into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            engine_error: false,
            error_message: None,
            completed: true,
            duration_ms: 500,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("error_message"), "got: {json}");
    }
}

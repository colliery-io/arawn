//! End-to-end UAT: starts an isolated arawn server, drives multi-turn
//! conversations via WebSocket, collects artifacts, runs mechanical checks.
//!
//! Requires a real LLM (Ollama Cloud / Groq). Gated behind #[ignore].
//!
//! Run: cargo test -p arawn-tests --test uat -- --ignored --nocapture
//! Or via angreal: angreal test uat --model gemma4

use std::collections::HashMap;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTurn {
    pub user_message: String,
    pub judge_expectation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicalThresholds {
    pub min_files_created: usize,
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
    pub tool_errors: usize,
    pub pass: bool,
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

        // Mechanical checks
        let all_completed = turns.iter().all(|t| t.completed);
        let no_engine_errors = turns.iter().all(|t| !t.engine_error);
        let tool_use = turns.iter().any(|t| !t.tool_calls.is_empty());
        let tool_errors = turns.iter().flat_map(|t| &t.tool_results).filter(|r| r.is_error).count();

        let mech_pass = all_completed
            && no_engine_errors
            && workspace_files.len() >= scenario.mechanical.min_files_created;

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            model: model.to_string(),
            turns,
            mechanical: MechanicalCheckResult {
                all_turns_completed: all_completed,
                no_errors: no_engine_errors,
                tool_use_occurred: tool_use,
                files_created: workspace_files.len(),
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

        let mut assistant_text = String::new();
        let mut tool_calls = Vec::new();
        let mut tool_results = Vec::new();
        let mut engine_error = false;
        let mut completed = false;

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

            // Skip the RPC ack response
            if event.get("id").is_some() && event.get("result").is_some() {
                continue;
            }

            match event.get("event").and_then(|e| e.as_str()) {
                Some("StreamingText") => {
                    if let Some(t) = event["data"]["text"].as_str() {
                        assistant_text.push_str(t);
                    }
                }
                Some("ToolCallStart") => {
                    tool_calls.push(ToolCallRecord {
                        id: event["data"]["id"].as_str().unwrap_or("").to_string(),
                        name: event["data"]["name"].as_str().unwrap_or("").to_string(),
                        input: event["data"]["input"].clone(),
                    });
                }
                Some("ToolCallResult") => {
                    let is_err = event["data"]["is_error"].as_bool().unwrap_or(false);
                    tool_results.push(ToolResultRecord {
                        id: event["data"]["id"].as_str().unwrap_or("").to_string(),
                        content: event["data"]["content"].as_str().unwrap_or("").to_string(),
                        is_error: is_err,
                    });
                }
                Some("Complete") => {
                    if let Some(t) = event["data"]["final_text"].as_str() {
                        assistant_text = t.to_string();
                    }
                    completed = true;
                    break;
                }
                Some("Error") => {
                    engine_error = true;
                    break;
                }
                _ => {} // Flush, Usage, Warning, etc.
            }
        }

        TurnResult {
            turn_number,
            user_message: user_message.to_string(),
            assistant_text,
            tool_calls,
            tool_results,
            engine_error,
            completed,
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
            min_memory_entities: 0,
            max_tool_errors: 2,
        },
    }
}

fn work_signal_pipeline_scenario() -> Scenario {
    Scenario {
        name: "work-signal-pipeline".to_string(),
        objective: "Build a daily work signal processing pipeline that intakes meeting transcripts, Slack exports, and task updates, then extracts action items and produces a prioritized daily briefing.".to_string(),
        turns: vec![
            ScenarioTurn {
                user_message: "I need a daily work signal processing pipeline. Every morning it should intake signals from multiple sources — meeting transcripts, Slack channel exports, and Jira updates — then analyze, extract action items, and produce a prioritized daily briefing. Think through the architecture first.".to_string(),
                judge_expectation: "Agent should use think tool to design the pipeline architecture. Response should identify distinct processing stages (intake, extraction, aggregation, prioritization, output) and the data flow between them.".to_string(),
            },
            ScenarioTurn {
                user_message: "Let's start with the transcript processor. Write a module that takes a meeting transcript (plain text) and extracts: attendees, key decisions, action items with owners, and follow-up dates.".to_string(),
                judge_expectation: "Agent should create a file with a transcript processing module. Code should parse text to extract structured data: attendees, decisions, action items (with owners and dates).".to_string(),
            },
            ScenarioTurn {
                user_message: "Now write the signal aggregator that combines outputs from transcript processing, Slack digests, and task tracker updates into a unified daily signal feed.".to_string(),
                judge_expectation: "Agent should create an aggregator module that takes multiple input sources and merges them into a unified data structure. Should handle deduplication or cross-referencing between sources.".to_string(),
            },
            ScenarioTurn {
                user_message: "Add a prioritization engine that ranks signals by urgency (time-sensitive items first), impact (cross-team items higher), and staleness (older unresolved items bubble up).".to_string(),
                judge_expectation: "Agent should create prioritization logic with three dimensions: urgency, impact, and staleness. Should produce a scored/ranked list of work items.".to_string(),
            },
            ScenarioTurn {
                user_message: "Finally, generate a sample daily briefing from mock data so I can see the output format. Include at least 5 mock signals across the different sources.".to_string(),
                judge_expectation: "Agent should create a sample output demonstrating the full pipeline. Should show a structured briefing with prioritized items from at least 2-3 different sources, formatted for human reading.".to_string(),
            },
        ],
        mechanical: MechanicalThresholds {
            min_files_created: 3,
            min_memory_entities: 0,
            max_tool_errors: 2,
        },
    }
}

fn all_scenarios() -> Vec<Scenario> {
    vec![github_monitor_scenario(), work_signal_pipeline_scenario()]
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
        println!("    Turns: {} | Files: {} | Tool errors: {} | Mechanical: {}",
            result.turns.len(),
            result.mechanical.files_created,
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
        }

        harness.stop();
        results.push(result);
    }

    // Summary
    println!("\n======================================================================");
    println!("  UAT SUMMARY — {model}");
    println!("----------------------------------------------------------------------");
    println!("  {:<30} {:>10} {:>8} {:>8} {:>8}", "Scenario", "Mechanical", "Files", "Errors", "Time");
    println!("----------------------------------------------------------------------");
    for r in &results {
        println!("  {:<30} {:>10} {:>8} {:>8} {:>7.0}s",
            r.scenario_name,
            if r.mechanical.pass { "PASS" } else { "FAIL" },
            r.mechanical.files_created,
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

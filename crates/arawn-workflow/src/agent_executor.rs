//! Decision service — runs arawn's QueryEngine for workflow decision tasks.
//!
//! Workflow decision tasks call back to the arawn server via HTTP to get
//! agent-powered decisions. This module provides the service that handles
//! those requests, creating sessions and running the QueryEngine loop.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use tracing::{info, warn};

use arawn_core::Message;
use arawn_engine::{QueryEngine, QueryEngineConfig, ToolContext, ToolRegistry};
use arawn_llm::LlmClient;
use arawn_storage::Store;

/// Request from a workflow decision task.
#[derive(Debug, Deserialize)]
pub struct DecisionRequest {
    /// The prompt for the agent to reason about.
    pub prompt: String,
    /// Workstream context to use (defaults to "scratch").
    #[serde(default = "default_workstream")]
    pub workstream: String,
    /// Upstream pipeline data injected as context.
    #[serde(default)]
    pub upstream_data: Value,
}

fn default_workstream() -> String {
    "scratch".into()
}

/// Response returned to the workflow decision task.
#[derive(Debug, Serialize)]
pub struct DecisionResponse {
    /// The agent's response text.
    pub result: String,
    /// Session ID used for this decision (for debugging/auditing).
    pub session_id: String,
}

/// Service that handles decision task requests from workflow pipelines.
pub struct DecisionService {
    store: Arc<Mutex<Store>>,
    llm: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    engine_config: QueryEngineConfig,
}

impl DecisionService {
    pub fn new(
        store: Arc<Mutex<Store>>,
        llm: Arc<dyn LlmClient>,
        registry: Arc<ToolRegistry>,
        engine_config: QueryEngineConfig,
    ) -> Self {
        Self {
            store,
            llm,
            registry,
            engine_config,
        }
    }

    /// Execute a decision request — creates a session, runs the QueryEngine,
    /// and returns the agent's response.
    pub async fn execute(&self, req: DecisionRequest) -> Result<DecisionResponse, DecisionError> {
        // Resolve workstream
        let workstream = {
            let store = self.store.lock().unwrap();
            store
                .find_workstream_by_name(&req.workstream)
                .map_err(|e| DecisionError(format!("find workstream '{}': {e}", req.workstream)))?
                .ok_or_else(|| {
                    DecisionError(format!("workstream '{}' not found", req.workstream))
                })?
        };

        // Create a fresh session for this decision
        let mut session = arawn_core::Session::new(workstream.id);
        {
            let store = self.store.lock().unwrap();
            store
                .create_session(&session)
                .map_err(|e| DecisionError(format!("create session: {e}")))?;
        }

        // Inject upstream pipeline data as context
        if !req.upstream_data.is_null() {
            let preamble = format!(
                "You are executing a decision task within an automated workflow pipeline.\n\
                 The following upstream data is available from previous pipeline stages:\n\n\
                 ```json\n{}\n```\n\n\
                 Use this data to inform your decision.",
                serde_json::to_string_pretty(&req.upstream_data).unwrap_or_default()
            );
            session.add_message(Message::Summary {
                content: preamble,
                original_count: 0,
                estimated_tokens_saved: 0,
            });
        }

        // Add the prompt as user message
        session.add_message(Message::User {
            content: req.prompt.clone(),
        });

        // Build engine and run
        let session_id = session.id;
        let tool_ctx = ToolContext::new(&workstream, session_id);
        let mut engine =
            QueryEngine::new(Arc::clone(&self.llm), Arc::clone(&self.registry));

        let result = engine
            .run(&mut session, &tool_ctx)
            .await
            .map_err(|e| DecisionError(format!("engine run: {e}")))?;

        info!(
            workstream = %req.workstream,
            session = %session_id,
            "decision task completed"
        );

        Ok(DecisionResponse {
            result,
            session_id: session_id.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("decision error: {0}")]
pub struct DecisionError(pub String);

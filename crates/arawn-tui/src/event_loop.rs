use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CEvent, EventStream, MouseEventKind,
};
use ratatui::layout::Rect;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tracing::{debug, error, info, warn};

use crate::app::{App, ChatMessage, ChatRole};
use crate::event::map_key_event;
use crate::render::render;
use crate::ws_client::{
    EventUpdate, WsClient, WsEvent, engine_event_to_update, parse_engine_event, parse_system_notice,
};

/// Minimum interval between renders driven by streaming/event traffic.
/// 33ms ≈ 30fps. Caps the worst case where the engine emits dozens of
/// events per second; keeps the render path responsive without melting
/// it. State-change events (modal, error) bypass this — they call
/// `force_draw`, which renders immediately and resets the clock.
const MIN_FRAME_INTERVAL: Duration = Duration::from_millis(33);

/// Render if enough time has elapsed since the last draw. Otherwise mark
/// the app dirty so the next tick (or next force draw) flushes the change.
fn maybe_draw<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    if app.last_draw.elapsed() >= MIN_FRAME_INTERVAL {
        terminal.draw(|f| render(app, f))?;
        app.last_draw = Instant::now();
        app.dirty = false;
    } else {
        app.dirty = true;
    }
    Ok(())
}

/// Render now regardless of frame budget. Use for state-change events
/// the user must see immediately (errors, modal prompts, completion).
fn force_draw<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    terminal.draw(|f| render(app, f))?;
    app.last_draw = Instant::now();
    app.dirty = false;
    Ok(())
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Run the TUI connected to the given WebSocket server URL.
pub async fn run_tui(url: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to server
    info!(url, "connecting to Arawn server");
    let mut client = WsClient::connect(url).await?;
    info!("connected");

    // Load initial state
    debug!("loading initial workstreams");
    let workstreams = client.list_workstreams().await?;
    debug!(count = workstreams.len(), "workstreams loaded");
    let current_ws = workstreams.first().cloned();
    let sessions = if let Some(ref ws) = current_ws {
        debug!(ws_id = %ws.id, "loading sessions for workstream");
        client.list_sessions(Some(ws.id)).await?
    } else {
        debug!("no workstreams found, skipping session load");
        vec![]
    };
    debug!(count = sessions.len(), "sessions loaded");

    // Create session for this TUI instance
    let ws_id = current_ws.as_ref().map(|ws| ws.id);
    debug!("creating new session");
    let session = client.create_session(ws_id).await?;
    info!(session_id = %session.id, "session created");

    // Initialize app
    let mut app = App::new();
    app.workstreams = workstreams;
    app.current_workstream = current_ws;
    app.sessions = sessions;
    app.current_session = Some(session.clone());
    app.model_name = model_name.to_string();

    // Fetch available commands from server for autocomplete (skills, etc.)
    if let Ok(resp) = client.request_response("list_commands", serde_json::json!({})).await
        && let Some(commands) = resp.get("result").and_then(|r| r.as_array()) {
                let skills: Vec<(String, String)> = commands
                    .iter()
                    .filter(|c| c.get("kind").and_then(|k| k.as_str()) == Some("skill"))
                    .filter_map(|c| {
                        let name = c.get("name").and_then(|n| n.as_str())?.to_string();
                        let desc = c.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
                        Some((name, desc))
                    })
                    .collect();
                if !skills.is_empty() {
                    info!(count = skills.len(), "cached skill commands for autocomplete");
                    app.command_registry.register_skills(skills);
                }
            }

    // Fetch server capabilities and surface degraded-feature warnings.
    // Failure to retrieve capabilities is non-fatal — older servers won't have
    // this RPC; we just don't show the banner.
    if let Ok(caps) = client.get_capabilities().await {
        let embeddings_available = caps
            .get("embeddings_available")
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // assume available if older server omits the field
        if !embeddings_available {
            app.messages.push(crate::app::ChatMessage::new(
                crate::app::ChatRole::System,
                "⚠ Memory is running in keyword-only mode — semantic search is \
                 unavailable because the embedding model didn't load. \
                 Install the model file at \
                 ~/.arawn/models/all-MiniLM-L6-v2/model.onnx and restart \
                 the server. See docs/src/memory.md for details."
                    .to_string(),
            ));
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        original_hook(info);
    }));

    // Take the WS event receiver. Reader task is already running; main
    // loop selects on this channel instead of polling the read half
    // directly, so render time can't back-pressure the socket.
    let mut events = client
        .events_take()
        .ok_or("ws client events channel already taken")?;

    // Initial render
    force_draw(&mut terminal, &mut app)?;

    // Event loop
    let mut term_events = EventStream::new();
    let mut tick_interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            // Spinner tick — flushes any deferred render, advances spinner.
            _ = tick_interval.tick() => {
                if app.is_generating {
                    app.spinner_frame = (app.spinner_frame + 1) % 10;
                    // Always draw on tick while generating so the spinner
                    // animates and any throttled streaming updates flush.
                    force_draw(&mut terminal, &mut app)?;
                } else if app.dirty {
                    force_draw(&mut terminal, &mut app)?;
                }
            }
            // Terminal events (key presses)
            Some(Ok(event)) = term_events.next() => {
                if let CEvent::Key(key) = event
                    && let Some(action) = map_key_event(key, app.focus, app.is_generating, app.active_modal.is_some(), app.autocomplete.is_some())
                {
                    debug!(?action, focus = ?app.focus, generating = app.is_generating, "handling action");
                    app.handle_action(action.clone());

                    // Handle modal response — send back to server when modal closes
                    if app.active_modal.is_none()
                        && let Some((request_id, mut rx)) = app.pending_modal_response.take() {
                            // Err = not ready or cancelled — treat as None.
                            let selected_index = rx.try_recv().unwrap_or_default();
                            debug!(%request_id, ?selected_index, "modal close");

                            if request_id == "__history_branch__" {
                                // Local-only modal: parse the picked
                                // option's description (formatted by
                                // open_history_modal as "h=<i> c=<j>") to
                                // get both the original history index and
                                // the chat-only index. The chat index is
                                // what the server's truncate RPC takes.
                                if let Some(opt_idx) = selected_index
                                    && let Some(modal_active_options) = {
                                        // active_modal was already cleared
                                        // by ModalConfirm, so we can't read
                                        // it. Instead, recompute from
                                        // app.history (same logic as
                                        // open_history_modal).
                                        let mut chat_entries: Vec<(usize, usize)> = Vec::new();
                                        let mut chat_idx = 0usize;
                                        for (i, e) in app.history.iter().enumerate() {
                                            if e.is_chat {
                                                chat_entries.push((i, chat_idx));
                                                chat_idx += 1;
                                            }
                                        }
                                        // Reverse to match newest-first ordering.
                                        chat_entries.reverse();
                                        Some(chat_entries)
                                    }
                                    && let Some(&(history_idx, chat_idx)) =
                                        modal_active_options.get(opt_idx)
                                    && let Some(ref session) = app.current_session.clone() {
                                        match client
                                            .truncate_session_at_user_message(session.id, chat_idx)
                                            .await
                                        {
                                            Ok(detail) => {
                                                // Refresh local messages from the truncated state.
                                                // Use App::load_session_messages so tool_use /
                                                // tool_result / summary roles round-trip
                                                // correctly — the previous open-coded loop
                                                // dropped them.
                                                app.load_session_messages(&detail);
                                                // Load the picked prompt into input for editing.
                                                app.handle_action(crate::action::Action::HistoryRecallAt(history_idx));
                                                app.messages.push(ChatMessage::new(
                                                    crate::app::ChatRole::System,
                                                    format!("Branched: rewound to before prompt #{}. Edit and submit when ready.", chat_idx + 1),
                                                ));
                                                app.dirty = true;
                                            }
                                            Err(e) => {
                                                warn!(error = %e, "branch truncate failed");
                                                app.messages.push(ChatMessage::new(
                                                    crate::app::ChatRole::System,
                                                    format!("Branch failed: {e}"),
                                                ));
                                                app.dirty = true;
                                            }
                                        }
                                    }
                            } else {
                                let params = serde_json::json!({
                                    "request_id": request_id,
                                    "selected_index": selected_index,
                                });
                                if let Err(e) = client.send_request("user_input_response", params).await {
                                    warn!(%request_id, error = %e, "failed to send modal response");
                                }
                            }
                        }

                    // Handle cancel — fire-and-forget the cancel RPC so the
                    // server actually stops the model. Without this, only
                    // the local UI flips while the model keeps running and
                    // emits a duplicate Complete after the user thought
                    // they cancelled.
                    if std::mem::take(&mut app.pending_cancel)
                        && let Some(ref session) = app.current_session
                    {
                        let session_id = session.id;
                        debug!(session_id = %session_id, "sending cancel RPC");
                        if let Err(e) = client.cancel(session_id).await {
                            warn!(error = %e, "cancel RPC failed (model may still be running)");
                            // Don't surface to the user — local UI is
                            // already cancelled; this is best-effort.
                        }
                    }

                    // Handle submit — send message via WS
                    if let Some(content) = app.pending_submit.take()
                        && let Some(ref session) = app.current_session
                    {
                        debug!(session_id = %session.id, content_len = content.len(), "submitting message");
                        if let Err(e) = client.send_message(session.id, &content).await {
                            warn!(error = %e, "send_message failed");
                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {e}")));
                            app.is_generating = false;
                        }
                    }

                    // Handle slash command results that need WS interaction
                    if let Some(cmd_result) = app.pending_command.take() {
                        match cmd_result {
                            crate::command::CommandResult::QueryInventory(kind) => {
                                let params = serde_json::json!({"kind": kind});
                                if let Ok(resp) = client.request_response("query_inventory", params).await {
                                        if let Some(items) = resp.get("result").and_then(|r| r.as_array()) {
                                            let mut output = format!("**/{kind}** ({} items)\n\n| Name | Description |\n|------|-------------|\n", items.len());
                                            for item in items {
                                                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                                let desc = item.get("description").and_then(|d| d.as_str()).unwrap_or("");
                                                output.push_str(&format!("| {name} | {desc} |\n"));
                                            }
                                            app.messages.push(ChatMessage::new(ChatRole::System, output));
                                        } else {
                                            app.messages.push(ChatMessage::new(ChatRole::System, format!("No {kind} found.")));
                                        }
                                    }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::InvokeSkill { name, args } => {
                                // Send as a chat message that tells the LLM to invoke the skill
                                if let Some(ref session) = app.current_session {
                                    let content = format!("Invoke the skill '/{name}' with these arguments: {args}");
                                    app.messages.push(ChatMessage::new(ChatRole::User, format!("/{name} {args}")));
                                    app.is_generating = true;
                                    app.generation_started = Some(std::time::Instant::now());
                                    app.scroll_offset = 0;
                                    if let Err(e) = client.send_message(session.id, &content).await {
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {e}")));
                                        app.is_generating = false;
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::RememberFact(text) => {
                                // Route through LLM so it decides entity type, title, tags
                                if let Some(ref session) = app.current_session {
                                    let content = format!(
                                        "Use the memory_store tool to store this in the knowledge base. \
                                         Choose the appropriate entity_type (fact, decision, convention, \
                                         preference, person, or note), write a clear title, and add \
                                         relevant tags: {text}"
                                    );
                                    app.messages.push(ChatMessage::new(ChatRole::User, format!("/remember {text}")));
                                    app.is_generating = true;
                                    app.generation_started = Some(std::time::Instant::now());
                                    app.scroll_offset = 0;
                                    if let Err(e) = client.send_message(session.id, &content).await {
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {e}")));
                                        app.is_generating = false;
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::MemorySummary => {
                                if let Ok(resp) = client.request_response("get_memory_summary", serde_json::json!({})).await
                                    && let Some(result) = resp.get("result") {
                                            let mut output = String::from("**Knowledge Base**\n\n");
                                            for (label, key) in [("Global", "global"), ("Workstream", "workstream")] {
                                                if let Some(tier) = result.get(key) {
                                                    let total = tier.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
                                                    output.push_str(&format!("| {label} | {total} entities |\n|---|---|\n"));
                                                    if let Some(by_type) = tier.get("by_type").and_then(|b| b.as_array()) {
                                                        for entry in by_type {
                                                            let et = entry.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                                                            let count = entry.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
                                                            output.push_str(&format!("| {et} | {count} |\n"));
                                                        }
                                                    }
                                                    output.push('\n');
                                                }
                                            }
                                            app.messages.push(ChatMessage::new(ChatRole::System, output));
                                        }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::ForgetEntity(query) => {
                                // Route through LLM so it can search, confirm, and delete
                                if let Some(ref session) = app.current_session {
                                    let content = format!(
                                        "Use the memory_search tool to find entities matching \"{query}\", \
                                         then use memory_store or the appropriate approach to remove or \
                                         supersede them. Confirm what you're removing."
                                    );
                                    app.messages.push(ChatMessage::new(ChatRole::User, format!("/forget {query}")));
                                    app.is_generating = true;
                                    app.generation_started = Some(std::time::Instant::now());
                                    app.scroll_offset = 0;
                                    if let Err(e) = client.send_message(session.id, &content).await {
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {e}")));
                                        app.is_generating = false;
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::WorkstreamCreate(name) => {
                                let params = serde_json::json!({"name": name});
                                if let Ok(resp) = client.request_response("create_workstream", params).await {
                                        if resp.get("result").is_some() {
                                            // Refresh workstream list and switch to new one
                                            if let Ok(workstreams) = client.list_workstreams().await {
                                                app.workstreams = workstreams;
                                                if let Some(ws) = app.workstreams.iter().find(|w| w.name == name).cloned() {
                                                    app.current_workstream = Some(ws.clone());
                                                    if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                                                        app.sessions = sessions;
                                                    }
                                                    // Auto-create first session
                                                    if app.sessions.is_empty()
                                                        && let Ok(session) = client.create_session(Some(ws.id)).await {
                                                            app.current_session = Some(session.clone());
                                                            app.sessions.push(session);
                                                            app.messages.clear();
                                                            app.streaming_text.clear();
                                                        }
                                                    app.messages.push(ChatMessage::new(ChatRole::System, format!("Switched to workstream '{name}'")));
                                                }
                                            }
                                        } else if let Some(err) = resp.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {err}")));
                                        }
                                    }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::WorkstreamList => {
                                if let Ok(workstreams) = client.list_workstreams().await {
                                    let mut output = String::from("Workstreams:\n\n");
                                    for ws in &workstreams {
                                        let current = app.current_workstream.as_ref().map(|c| c.id) == Some(ws.id);
                                        let marker = if current { "▸ " } else { "  " };
                                        let sessions = client.list_sessions(Some(ws.id)).await.map(|s| s.len()).unwrap_or(0);
                                        output.push_str(&format!("{marker}{} ({} sessions)\n", ws.name, sessions));
                                    }
                                    app.messages.push(ChatMessage::new(ChatRole::System, output));
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::WorkstreamSwitch(name) => {
                                // Refresh workstream list and find by name
                                if let Ok(workstreams) = client.list_workstreams().await {
                                    app.workstreams = workstreams;
                                    if let Some(ws) = app.workstreams.iter().find(|w| w.name == name).cloned() {
                                        app.current_workstream = Some(ws.clone());
                                        if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                                            app.sessions = sessions;
                                        }
                                        if app.sessions.is_empty() {
                                            if let Ok(session) = client.create_session(Some(ws.id)).await {
                                                app.current_session = Some(session.clone());
                                                app.sessions.push(session);
                                                app.messages.clear();
                                                app.streaming_text.clear();
                                            }
                                        } else {
                                            let session = app.sessions[0].clone();
                                            app.current_session = Some(session.clone());
                                            if let Ok(detail) = client.load_session(session.id).await {
                                                app.load_session_messages(&detail);
                                            }
                                        }
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Switched to workstream '{name}'")));
                                    } else {
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Workstream '{name}' not found")));
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::SessionNew => {
                                let ws_id = app.current_workstream.as_ref().map(|ws| ws.id);
                                if let Ok(session) = client.create_session(ws_id).await {
                                    app.current_session = Some(session.clone());
                                    app.messages.clear();
                                    app.streaming_text.clear();
                                    if let Ok(sessions) = client.list_sessions(ws_id).await {
                                        app.sessions = sessions;
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::SessionList => {
                                let mut output = String::from("Sessions:\n\n");
                                for s in &app.sessions {
                                    let current = app.current_session.as_ref().map(|c| c.id) == Some(s.id);
                                    let marker = if current { "▸ " } else { "  " };
                                    let id_short = &s.id.to_string()[..8];
                                    let date = s.created_at.format("%Y-%m-%d %H:%M");
                                    output.push_str(&format!("{marker}{id_short}  {date}\n"));
                                }
                                app.messages.push(ChatMessage::new(ChatRole::System, output));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::PromoteSession(ws_name) => {
                                if let Some(ref session) = app.current_session {
                                    let params = serde_json::json!({
                                        "session_id": session.id.to_string(),
                                        "workstream_name": ws_name,
                                    });
                                    if let Ok(resp) = client.request_response("promote_session", params).await {
                                            if resp.get("result").and_then(|r| r.get("status")).and_then(|s| s.as_str()) == Some("promoted") {
                                                // Refresh state
                                                if let Ok(workstreams) = client.list_workstreams().await {
                                                    app.workstreams = workstreams;
                                                    if let Some(ws) = app.workstreams.iter().find(|w| w.name == ws_name).cloned() {
                                                        app.current_workstream = Some(ws.clone());
                                                        if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                                                            app.sessions = sessions;
                                                        }
                                                    }
                                                }
                                                app.messages.push(ChatMessage::new(ChatRole::System, format!("Session promoted to workstream '{ws_name}'")));
                                            } else if let Some(err) = resp.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                                app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {err}")));
                                            }
                                        }
                                } else {
                                    app.messages.push(ChatMessage::new(ChatRole::System, "No active session to promote".to_string()));
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::SetPermissionMode(mode) => {
                                match client.set_permission_mode(&mode).await {
                                    Ok(confirmed) => {
                                        app.permission_mode = confirmed.clone();
                                        let label = match confirmed.as_str() {
                                            "bypass" => "BYPASS (full autonomy)",
                                            "accept_edits" => "ACCEPT EDITS",
                                            "plan" => "PLAN (read-only)",
                                            _ => "DEFAULT",
                                        };
                                        app.messages.push(ChatMessage::new(
                                            ChatRole::System,
                                            format!("Permission mode set to {label}"),
                                        ));
                                    }
                                    Err(e) => {
                                        app.messages.push(ChatMessage::new(
                                            ChatRole::System,
                                            format!("Failed to set mode: {e}"),
                                        ));
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::WorkflowList => {
                                if let Ok(workflows) = client.list_workflows().await {
                                    if workflows.is_empty() {
                                        app.messages.push(ChatMessage::new(ChatRole::System, "No workflows installed.".to_string()));
                                    } else {
                                        let mut output = String::from("Installed workflows:\n\n");
                                        for wf in &workflows {
                                            let name = wf["name"].as_str().unwrap_or("?");
                                            let cron = wf["cron"].as_str().unwrap_or("manual");
                                            output.push_str(&format!("  {name}  ({cron})\n"));
                                        }
                                        app.messages.push(ChatMessage::new(ChatRole::System, output));
                                    }
                                } else {
                                    app.messages.push(ChatMessage::new(ChatRole::System, "Failed to list workflows.".to_string()));
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::WorkflowStatus(_name) => {
                                // Status requires the workflow runner which is server-side
                                // For now, show a message directing to the agent tool
                                app.messages.push(ChatMessage::new(
                                    ChatRole::System,
                                    "Use the workflow_status tool to check execution history.".to_string(),
                                ));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::PermissionsStatus => {
                                let body = match client.get_permissions_status().await {
                                    Ok(status) => format_permissions_status(&status),
                                    Err(e) => format!("Failed to fetch permissions status: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::IntegrationsList => {
                                let body = match client.list_integrations().await {
                                    Ok(items) => format_integrations_list(&items),
                                    Err(e) => format!("Failed to list integrations: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::IntegrationConnect(svc) => {
                                let body = match client.start_oauth_flow(&svc).await {
                                    Ok(started) => {
                                        let auth_url = started
                                            .get("auth_url")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        let opener_status = match try_open_url(auth_url) {
                                            OpenAttempt::Opened(cmd) => format!("Opened in browser via `{cmd}`."),
                                            OpenAttempt::NoOpener => "No browser opener detected on this platform.".to_string(),
                                            OpenAttempt::Failed(err) => format!("Browser open failed: {err}."),
                                        };
                                        app.oauth_in_flight =
                                            Some((svc.clone(), std::time::Instant::now()));
                                        format!(
                                            "Connecting **{svc}**…\n\n\
                                             {opener_status}\n\n\
                                             If the browser didn't open, paste this URL:\n\n  {auth_url}\n\n\
                                             _Waiting for browser authorization (5 min timeout). \
                                             You'll see a [integration] notice when it lands._"
                                        )
                                    }
                                    Err(e) => format!("/connect {svc} failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::IntegrationDisconnect(svc) => {
                                let body = match client.disconnect_integration(&svc).await {
                                    Ok(()) => format!("Disconnected **{svc}**. Stored credentials removed."),
                                    Err(e) => format!("/disconnect {svc} failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedDiscover(template) => {
                                let body = match template {
                                    Some(tpl) => match client.feed_discover(&tpl).await {
                                        Ok(dto) => format_feed_discover(&dto),
                                        Err(e) => format!("/watch list {tpl} failed: {e}"),
                                    },
                                    None => format_known_templates(),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedRegister(spec) => {
                                let payload = serde_json::json!({
                                    "template": spec.template,
                                    "feed_id": spec.feed_id,
                                    "params": spec.params,
                                    "cadence": spec.cadence,
                                });
                                let body = match client.feed_register(payload).await {
                                    Ok(dto) => format_feed_registered(&dto),
                                    Err(e) => format!("/watch failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedList => {
                                let body = match client.feed_list().await {
                                    Ok(list) => format_feed_list(&list),
                                    Err(e) => format!("/feeds failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedPause(id) => {
                                let body = match client.feed_pause(&id).await {
                                    Ok(dto) => format!(
                                        "Paused **{}**. Cron schedule deleted; \
                                         data dir at `{}` left intact.\n\n\
                                         _Run /feeds resume {} to bring it back._",
                                        dto.get("template").and_then(|v| v.as_str()).unwrap_or("?"),
                                        dto.get("data_dir").and_then(|v| v.as_str()).unwrap_or("?"),
                                        id,
                                    ),
                                    Err(e) => format!("/feeds pause {id} failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedResume(id) => {
                                let body = match client.feed_resume(&id).await {
                                    Ok(dto) => format!(
                                        "Resumed **{}**. Cron re-registered with cadence `{}`.",
                                        dto.get("template").and_then(|v| v.as_str()).unwrap_or("?"),
                                        dto.get("cadence").and_then(|v| v.as_str()).unwrap_or("?"),
                                    ),
                                    Err(e) => format!("/feeds resume {id} failed: {e}"),
                                };
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedRemove {
                                feed_id,
                                confirmed,
                            } => {
                                if !confirmed {
                                    // Slice 2 confirm path: print a
                                    // preview of what would be wiped
                                    // and ask the user to re-run with
                                    // --yes. Modal upgrade lands later.
                                    let body = match client.feed_list().await {
                                        Ok(list) => match list
                                            .iter()
                                            .find(|f| {
                                                f.get("id").and_then(|v| v.as_str())
                                                    == Some(feed_id.as_str())
                                            })
                                            .cloned()
                                        {
                                            Some(f) => {
                                                let template = f
                                                    .get("template")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("?");
                                                let dir = f
                                                    .get("data_dir")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("?");
                                                let size = f
                                                    .get("data_size_bytes")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0);
                                                format!(
                                                    "**Confirm decommission of `{feed_id}` ({template})?**\n\n\
                                                     This will:\n\
                                                     - Delete the cron schedule\n\
                                                     - Delete the DB row\n\
                                                     - Recursively wipe `{dir}` ({} on disk)\n\n\
                                                     _Cannot be undone._\n\n\
                                                     Re-run `/feeds rm {feed_id} --yes` to proceed.",
                                                    human_size(size)
                                                )
                                            }
                                            None => format!("No feed named '{feed_id}'."),
                                        },
                                        Err(e) => format!("/feeds rm preview failed: {e}"),
                                    };
                                    app.messages.push(ChatMessage::new(ChatRole::System, body));
                                    app.dirty = true;
                                } else {
                                    let body = match client.feed_remove(&feed_id).await {
                                        Ok(dto) => format!(
                                            "Decommissioned **{}** (`{}`). \
                                             Wiped {} from disk.",
                                            dto.get("template")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?"),
                                            feed_id,
                                            human_size(
                                                dto.get("bytes_wiped")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0),
                                            ),
                                        ),
                                        Err(e) => format!("/feeds rm {feed_id} failed: {e}"),
                                    };
                                    app.messages.push(ChatMessage::new(ChatRole::System, body));
                                    app.dirty = true;
                                }
                            }
                            _ => {} // Other command results handled in app.handle_action
                        }
                    }

                    // Handle sidebar select — load workstream sessions or switch session
                    if action == crate::action::Action::SidebarSelect {
                        match app.sidebar_section {
                            crate::app::SidebarSection::Workstreams => {
                                if let Some(ws) = app.workstreams.get(app.sidebar_ws_index).cloned() {
                                    app.current_workstream = Some(ws.clone());
                                    if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                                        app.sessions = sessions;
                                        app.sidebar_session_index = 0;
                                    }

                                    // Auto-create a session if the workstream has none,
                                    // or resume the most recent one.
                                    if app.sessions.is_empty() {
                                        if let Ok(session) = client.create_session(Some(ws.id)).await {
                                            app.current_session = Some(session.clone());
                                            app.sessions.push(session);
                                            app.messages.clear();
                                            app.streaming_text.clear();
                                        }
                                    } else {
                                        // Resume the first (most recent) session
                                        let session = app.sessions[0].clone();
                                        app.current_session = Some(session.clone());
                                        if let Ok(detail) = client.load_session(session.id).await {
                                            app.load_session_messages(&detail);
                                        }
                                    }

                                    app.sidebar_section = crate::app::SidebarSection::Sessions;
                                    app.dirty = true;
                                }
                            }
                            crate::app::SidebarSection::Sessions => {
                                if let Some(session) = app.sessions.get(app.sidebar_session_index).cloned() {
                                    app.current_session = Some(session.clone());
                                    if let Ok(detail) = client.load_session(session.id).await {
                                        app.load_session_messages(&detail);
                                    }
                                    app.focus = crate::app::Focus::Main;
                                    app.dirty = true;
                                }
                            }
                        }
                    }

                    // Handle new session
                    if action == crate::action::Action::NewSession {
                        let ws_id = app.current_workstream.as_ref().map(|ws| ws.id);
                        if let Ok(session) = client.create_session(ws_id).await {
                            app.current_session = Some(session.clone());
                            app.messages.clear();
                            app.streaming_text.clear();
                            if let Ok(sessions) = client.list_sessions(ws_id).await {
                                app.sessions = sessions;
                            }
                            app.focus = crate::app::Focus::Main;
                            app.dirty = true;
                        }
                    }

                    if app.should_quit {
                        break;
                    }

                    if app.dirty {
                        force_draw(&mut terminal, &mut app)?;
                    }
                }
                if let CEvent::Mouse(mouse) = event {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.scroll_offset = app.scroll_offset.saturating_add(3);
                            app.dirty = true;
                        }
                        MouseEventKind::ScrollDown => {
                            app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            app.dirty = true;
                        }
                        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            let col = mouse.column;
                            let row = mouse.row;

                            // Sidebar tab strip (opens sidebar)
                            if let Some(tab_rect) = app.layout.sidebar_tab
                                && rect_contains(tab_rect, col, row) {
                                    app.focus = crate::app::Focus::Sidebar;
                                    app.dirty = true;
                                }

                            // Sidebar panel
                            if let Some(sidebar_rect) = app.layout.sidebar
                                && rect_contains(sidebar_rect, col, row) {
                                    app.focus = crate::app::Focus::Sidebar;
                                    if let Some(ws_rect) = app.layout.sidebar_ws
                                        && rect_contains(ws_rect, col, row) {
                                            app.sidebar_section = crate::app::SidebarSection::Workstreams;
                                            let item_row = row.saturating_sub(ws_rect.y + 1) as usize;
                                            if item_row < app.workstreams.len() {
                                                app.sidebar_ws_index = item_row;
                                            }
                                        }
                                    if let Some(sess_rect) = app.layout.sidebar_sessions
                                        && rect_contains(sess_rect, col, row) {
                                            app.sidebar_section = crate::app::SidebarSection::Sessions;
                                            let item_row = row.saturating_sub(sess_rect.y + 1) as usize;
                                            if item_row < app.sessions.len() {
                                                app.sidebar_session_index = item_row;
                                            }
                                        }
                                    app.dirty = true;
                                }

                            // Input area — click to focus and place cursor
                            if rect_contains(app.layout.input, col, row) {
                                app.focus = crate::app::Focus::Main;
                                let offset = col.saturating_sub(app.layout.input.x + 1) as usize;
                                app.cursor_pos = offset.min(app.input_buffer.len());
                                app.dirty = true;
                            }

                            // Chat area clicks are intentionally not handled —
                            // use Option+drag for native terminal text selection.
                        }
                        _ => {}
                    }
                    if app.dirty {
                        force_draw(&mut terminal, &mut app)?;
                    }
                }
                if let CEvent::Resize(_, _) = event {
                    force_draw(&mut terminal, &mut app)?;
                }
            }

            // WebSocket events from the dedicated reader task. Streaming
            // tokens batch into the channel; we drain the queue, apply
            // everything, then render once. Render budget (MIN_FRAME_INTERVAL)
            // throttles streaming updates further; state changes bypass it
            // via `force_render`.
            Some(ev) = events.recv() => {
                let mut should_break = false;
                let mut force_render = false;
                let mut anything_applied = false;

                let apply_update = |update: EventUpdate, app: &mut App| -> bool {
                    // If the user cancelled this session's current turn,
                    // drop stream events from it — they're stale output
                    // from work the server hasn't finished aborting yet.
                    // Errors and Flush still pass through so the user
                    // sees real failures.
                    if app.cancelled_session.is_some() {
                        match &update {
                            EventUpdate::AppendStreamingText(_)
                            | EventUpdate::AddToolCall { .. }
                            | EventUpdate::AddToolResult { .. }
                            | EventUpdate::Complete(_)
                            | EventUpdate::Usage { .. }
                            | EventUpdate::Compaction(_) => return false,
                            _ => {}
                        }
                    }
                    match update {
                        EventUpdate::AppendStreamingText(text) => {
                            debug!(len = text.len(), "update: streaming text");
                            // Append into the live streaming buffer. Flushing into a
                            // permanent ChatMessage happens at turn boundaries
                            // (AddToolCall / Complete / Error), not per-chunk —
                            // otherwise every streaming token after the first
                            // gets its own message.
                            app.streaming_text.push_str(&text);
                            // Don't force a draw on every token — under fast
                            // streams, render time exceeds inter-token interval
                            // and the TUI falls behind. The 100ms spinner tick
                            // (active while is_generating) redraws periodically
                            // and picks up the accumulated streaming_text.
                            return false;
                        }
                        EventUpdate::AddToolCall { name, input, .. } => {
                            debug!(%name, "update: tool call start");
                            // Flush streaming text before tool call indicator
                            if !app.streaming_text.is_empty() {
                                let text = std::mem::take(&mut app.streaming_text);
                                app.messages.push(ChatMessage::new(ChatRole::Assistant, text));
                            }
                            app.active_tool = Some(name.clone());
                            let summary = crate::app::format_tool_input(&name, &input);
                            app.messages.push(ChatMessage::new(ChatRole::ToolCall { name: name.clone() }, summary));
                            return true; // Draw immediately
                        }
                        EventUpdate::AddToolResult { content, is_error, .. } => {
                            let name = app.messages.iter().rev()
                                .find_map(|m| match &m.role {
                                    ChatRole::ToolCall { name } => Some(name.clone()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| "tool".to_string());
                            debug!(%name, is_error, content_len = content.len(), "update: tool result");
                            app.active_tool = None;
                            app.messages.push(ChatMessage::new(ChatRole::ToolResult { name, is_error }, content));
                            return true; // Draw immediately
                        }
                        EventUpdate::Complete(final_text) => {
                            debug!(final_len = final_text.len(), messages = app.messages.len(), "update: complete");
                            // Flush any remaining streaming text
                            if !app.streaming_text.is_empty() {
                                let text = std::mem::take(&mut app.streaming_text);
                                app.messages.push(ChatMessage::new(ChatRole::Assistant, text));
                            }
                            // Add final text if non-empty and not already flushed
                            if !final_text.is_empty() {
                                app.messages.push(ChatMessage::new(ChatRole::Assistant, final_text));
                            }
                            app.is_generating = false;
                            app.active_tool = None;
                            app.scroll_offset = 0;
                            // Force draw — don't depend on a separate Flush event
                            return true;
                        }
                        EventUpdate::Error(message) => {
                            warn!(%message, "update: engine error");
                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {message}")));
                            app.is_generating = false;
                            app.active_tool = None;
                            app.streaming_text.clear();
                            // Force draw
                            return true;
                        }
                        EventUpdate::Warning(message) => {
                            warn!(%message, "update: engine warning");
                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Warning: {message}")));
                            return true;
                        }
                        EventUpdate::Compaction(count) => {
                            debug!(count, "update: compaction");
                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Context compacted ({count} messages summarized)")));
                        }
                        EventUpdate::Usage { input_tokens, output_tokens } => {
                            debug!(input_tokens, output_tokens, "update: usage");
                            app.token_usage = (input_tokens, output_tokens);
                        }
                        EventUpdate::UserInputRequest { request_id, title, subtitle, options } => {
                            debug!(%request_id, %title, option_count = options.len(), "update: user input request");
                            // Show modal — the request_id is stored so we can send back the response
                            let modal_options: Vec<crate::modal::ModalOption> = options
                                .iter()
                                .map(|o| {
                                    let mut mo = crate::modal::ModalOption::new(&o.label);
                                    if let Some(ref desc) = o.description {
                                        mo = mo.with_description(desc);
                                    }
                                    mo
                                })
                                .collect();
                            let (result_tx, result_rx) = tokio::sync::oneshot::channel();
                            let mut modal = crate::modal::ModalState::new(
                                title,
                                modal_options,
                                ratatui::style::Color::Yellow,
                                result_tx,
                            );
                            if let Some(sub) = subtitle {
                                modal = modal.with_subtitle(sub);
                            }
                            app.active_modal = Some(modal);

                            // Spawn a task to wait for the modal result and send it back via WS
                            let req_id = request_id.clone();
                            // We need to send the response back — store the rx for later
                            // The event loop will handle this after the modal closes
                            app.pending_modal_response = Some((req_id, result_rx));
                        }
                        EventUpdate::Flush => {
                            debug!("update: flush");
                            return true;
                        }
                    }
                    false
                };

                let handle_event = |ev: WsEvent, app: &mut App| -> (bool, bool, bool) {
                    // (applied, force, broke)
                    match ev {
                        WsEvent::Text(text) => {
                            if let Some(notice) = parse_system_notice(&text) {
                                apply_system_notice(&notice, app);
                                (true, true, false)
                            } else if let Some(event) = parse_engine_event(&text) {
                                let force = apply_update(engine_event_to_update(event), app);
                                (true, force, false)
                            } else {
                                (false, false, false)
                            }
                        }
                        WsEvent::Closed => {
                            warn!("server closed connection");
                            (false, false, true)
                        }
                        WsEvent::Error(e) => {
                            error!(error = %e, "WebSocket error");
                            (false, false, true)
                        }
                    }
                };

                let (a, f, b) = handle_event(ev, &mut app);
                anything_applied |= a;
                force_render |= f;
                should_break |= b;

                // Drain any further events that have already been queued by
                // the reader task while we were away. Bounded so a runaway
                // stream can't starve term-events / ticks indefinitely.
                let mut drained: u32 = 0;
                while !should_break && drained < 256 {
                    match events.try_recv() {
                        Ok(ev) => {
                            drained += 1;
                            let (a, f, b) = handle_event(ev, &mut app);
                            anything_applied |= a;
                            force_render |= f;
                            should_break |= b;
                        }
                        Err(_) => break,
                    }
                }
                if drained > 0 {
                    debug!(drained, force_render, "drained queued ws events");
                }

                if should_break { break; }
                if force_render {
                    force_draw(&mut terminal, &mut app)?;
                } else if anything_applied {
                    maybe_draw(&mut terminal, &mut app)?;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Render a `list_integrations` response as a markdown table the user can scan.
fn format_integrations_list(items: &[serde_json::Value]) -> String {
    use std::fmt::Write;
    if items.is_empty() {
        return "No integrations registered. (Build with integrations to enable Gmail / Calendar / Slack.)"
            .to_string();
    }
    let mut out = String::from("**Integrations**\n\n| Service | Connected |\n|---|---|\n");
    for item in items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let connected = item.get("connected").and_then(|v| v.as_bool()).unwrap_or(false);
        let mark = if connected { "✓" } else { "—" };
        let _ = writeln!(out, "| {name} | {mark} |");
    }
    out.push_str("\nRun `/connect <service>` to authorize, `/disconnect <service>` to drop credentials.");
    out
}

/// What `try_open_url` did. The TUI always prints the URL too, so even
/// `NoOpener` is a soft failure — the user can still copy/paste.
enum OpenAttempt {
    Opened(&'static str),
    NoOpener,
    Failed(String),
}

/// Best-effort browser open. Returns immediately — doesn't block on the
/// child process (which is the whole point of `open` / `xdg-open`).
fn try_open_url(url: &str) -> OpenAttempt {
    let opener: Option<&'static str> = if cfg!(target_os = "macos") {
        Some("open")
    } else if cfg!(target_os = "linux") {
        Some("xdg-open")
    } else if cfg!(target_os = "windows") {
        // `cmd /c start` needs an empty title arg before the URL.
        // We treat it as a special case below.
        Some("cmd")
    } else {
        None
    };
    let Some(cmd) = opener else { return OpenAttempt::NoOpener };

    let result = if cmd == "cmd" {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    } else {
        std::process::Command::new(cmd)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    };
    match result {
        Ok(_child) => OpenAttempt::Opened(cmd),
        Err(e) => OpenAttempt::Failed(e.to_string()),
    }
}

/// Push a server-side notice (plugin/config hot-reload outcome) into the
/// chat history as a system message. Failures get an "✗" prefix; successes
/// get an info marker. Both stay visible — fade-out is future work.
fn apply_system_notice(notice: &arawn_service::ServerNotice, app: &mut crate::app::App) {
    let marker = if notice.level == "error" { "✗" } else { "ℹ" };
    let body = format!("{marker} [{}] {}", notice.category, notice.message);
    app.messages
        .push(crate::app::ChatMessage::new(crate::app::ChatRole::System, body));

    // An integration notice (success or error) ends an in-flight OAuth
    // dance; clear the heartbeat so the user sees the resolution.
    if notice.category == "integration" {
        app.oauth_in_flight = None;
    }

    app.dirty = true;
}

/// Render `get_permissions_status` JSON as a human-readable system message.
fn format_permissions_status(status: &serde_json::Value) -> String {
    use std::fmt::Write;
    let mut out = String::from("**Permissions**\n\n");

    let mode = status.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
    let _ = writeln!(out, "Mode: `{mode}`");

    let render_list = |label: &str, key: &str, out: &mut String| {
        if let Some(arr) = status.get(key).and_then(|v| v.as_array())
            && !arr.is_empty()
        {
            let _ = writeln!(out, "\n{label}:");
            for item in arr {
                if let Some(s) = item.as_str() {
                    let _ = writeln!(out, "  - `{s}`");
                }
            }
        }
    };
    render_list("Deny rules", "deny_rules", &mut out);
    render_list("Allow rules", "allow_rules", &mut out);
    render_list("Ask rules", "ask_rules", &mut out);

    if let Some(decisions) = status.get("recent_decisions").and_then(|v| v.as_array()) {
        if decisions.is_empty() {
            let _ = writeln!(out, "\nNo decisions recorded yet this session.");
        } else {
            let _ = writeln!(out, "\nRecent decisions (newest first):");
            // Newest at the top — the audit buffer is push_back so the last
            // entry is most recent.
            for entry in decisions.iter().rev().take(20) {
                let ts = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
                let tool = entry.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?");
                let dec = entry.get("decision").and_then(|v| v.as_str()).unwrap_or("?");
                let reason = entry.get("reason").and_then(|v| v.as_str()).unwrap_or("?");
                let _ = writeln!(out, "  {ts}  {tool:<24}  {dec:<8}  {reason}");
            }
        }
    }
    out
}

/// Render a freshly-registered feed into a chat-ready system message.
fn format_feed_registered(dto: &serde_json::Value) -> String {
    let template = dto.get("template").and_then(|v| v.as_str()).unwrap_or("?");
    let id = dto.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let cadence = dto.get("cadence").and_then(|v| v.as_str()).unwrap_or("?");
    let dir = dto.get("data_dir").and_then(|v| v.as_str()).unwrap_or("?");
    format!(
        "Registered **{template}** as `{id}`.\n\n\
         - Cadence: `{cadence}`\n\
         - Data dir: `{dir}`\n\n\
         _Will fire on the next cron tick. Run /feeds to see status._"
    )
}

/// Render the `/feeds` listing as a markdown table-ish block. Compact
/// enough to fit in the chat pane without needing a modal — the modal
/// upgrade lands in slice 2 of T-0219.
fn format_feed_list(list: &[serde_json::Value]) -> String {
    if list.is_empty() {
        return "No feeds configured. Run /watch to register one.".into();
    }
    let mut s = String::from("**Configured feeds:**\n\n");
    for f in list {
        let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let template = f.get("template").and_then(|v| v.as_str()).unwrap_or("?");
        let cadence = f.get("cadence").and_then(|v| v.as_str()).unwrap_or("?");
        let enabled = f.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let last_run = f
            .get("last_run_at")
            .and_then(|v| v.as_str())
            .unwrap_or("(never)");
        let last_status = f
            .get("last_status")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let size = f
            .get("data_size_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let state = if enabled { "active" } else { "paused" };
        s.push_str(&format!(
            "- **{template}** `{id}` — `{cadence}` · {state} · last: {last_run} ({last_status}) · {} on disk\n",
            human_size(size)
        ));
    }
    s
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Render `feed_discover` results into a chat-pane block. Empty
/// `picker_supported=false` means the template's params are
/// free-form — nudge the user toward `/watch <tpl> <id> k=v` instead.
fn format_feed_discover(dto: &serde_json::Value) -> String {
    let template = dto
        .get("template")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let supported = dto
        .get("picker_supported")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rows = dto
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !supported {
        return format!(
            "**{template}** doesn't support discovery — its params \
             are free-form (sender pattern, label name, folder path, \
             etc.).\n\n\
             Use the typed form, e.g.:\n  \
             /watch {template} <feed_id> <key>=<value>"
        );
    }
    if rows.is_empty() {
        return format!(
            "No discoverable values for **{template}**. The integration \
             may not be connected, or the workspace has none of this kind."
        );
    }
    let mut out = format!("**Pick a value for `{template}`:**\n\n");
    for row in &rows {
        let label = row.get("label").and_then(|v| v.as_str()).unwrap_or("?");
        let hint = row
            .get("hint")
            .and_then(|v| v.as_str())
            .map(|h| format!("  _({h})_"))
            .unwrap_or_default();
        // Find the first key/value pair from params and render it as
        // a copy-pasteable token.
        let params = row.get("params").cloned().unwrap_or_default();
        let kv = params
            .as_object()
            .and_then(|m| m.iter().next())
            .map(|(k, v)| {
                let val = v
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string());
                format!("{k}={val}")
            })
            .unwrap_or_else(|| "?".into());
        out.push_str(&format!("- {label}{hint}\n  `{kv}`\n"));
    }
    out.push_str(&format!(
        "\n_To register: `/watch {template} <feed_id> <key>=<value>`._"
    ));
    out
}

/// Static help for `/watch list` with no template — points the user
/// at the canonical list and shows the discovery shortcut.
fn format_known_templates() -> String {
    "**Available feed templates:**\n\n\
     - slack/channel-archive · slack/dm-archive · slack/my-mentions\n\
     - calendar/upcoming-archive\n\
     - gmail/inbox-archive · gmail/sender-filter · gmail/label-archive\n\
     - drive/folder-sync · drive/recent\n\
     - confluence/space-archive\n\
     - jira/project-tracker · jira/assignee-tracker\n\n\
     Run `/watch list <template>` to pick a value when the template \
     supports discovery (slack/channel-archive, jira/project-tracker, \
     confluence/space-archive). Use the typed form `/watch <template> \
     <feed_id> key=value` for the rest."
        .into()
}

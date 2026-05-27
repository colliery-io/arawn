use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CEvent, EventStream, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
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

mod brief;
mod ceremony;
mod formats;
mod notices;
mod todo;
mod usage;
mod watch;

use brief::{refresh_brief_cache, render_brief_combined};
use ceremony::{
    current_iso_week, fetch_diary_body, fetch_priorities, fetch_tablet_id_and_status,
    handle_ceremony_overlay_key, refresh_active_ceremony_overlay, render_ceremony_retro,
    render_ceremony_today, render_ceremony_week,
};
use formats::{
    OpenAttempt, format_feed_discover, format_feed_list, format_feed_registered,
    format_integrations_list, format_known_templates, format_permissions_status, human_size,
    try_open_url,
};
use notices::apply_system_notice;
use todo::{fetch_open_todos, handle_todo_overlay_key};
use usage::render_usage;
use watch::{handle_watch_overlay_key, open_watch_modal};

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
    debug!("loading initial lenses");
    let lenses = client.list_lenses().await?;
    debug!(count = lenses.len(), "lenses loaded");
    let current_ws = lenses.first().cloned();
    let sessions = if let Some(ref ws) = current_ws {
        debug!(ws_id = %ws.id, "loading sessions for lens");
        client.list_sessions(Some(ws.id)).await?
    } else {
        debug!("no lenses found, skipping session load");
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
    app.lenses = lenses;
    app.current_lens = current_ws;
    app.sessions = sessions;
    app.current_session = Some(session.clone());
    app.model_name = model_name.to_string();

    // Fetch available commands from server for autocomplete (skills, etc.)
    if let Ok(resp) = client
        .request_response("list_commands", serde_json::json!({}))
        .await
        && let Some(commands) = resp.get("result").and_then(|r| r.as_array())
    {
        let skills: Vec<(String, String)> = commands
            .iter()
            .filter(|c| c.get("kind").and_then(|k| k.as_str()) == Some("skill"))
            .filter_map(|c| {
                let name = c.get("name").and_then(|n| n.as_str())?.to_string();
                let desc = c
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                Some((name, desc))
            })
            .collect();
        if !skills.is_empty() {
            info!(
                count = skills.len(),
                "cached skill commands for autocomplete"
            );
            app.command_registry.register_skills(skills);
        }
    }

    // I-0035 Phase 2 (T-0354) + Phase 3 T-B (T-0357) + Phase 4 T-B
    // (T-0360): pre-fetch today's brief so the empty-chat surface
    // and the dashboard right pane can both render without a
    // round-trip on first paint. Read-only against the ceremony
    // service; no LLM tokens spent. Falls back to None when no
    // tablets exist yet — that's the pre-onboarding state where
    // the T-0331 welcome wins. The helper is also called from the
    // briefing_ready handler so cron-fired refreshes use the same
    // path.
    refresh_brief_cache(&mut client, &mut app).await;

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
                } else if app.dirty || !app.toast_queue.is_empty() {
                    // Repaint when a toast is queued so it can age out
                    // at TTL even if nothing else is changing.
                    force_draw(&mut terminal, &mut app)?;
                }
            }
            // Terminal events (key presses)
            Some(Ok(event)) = term_events.next() => {
                // Ceremony overlays (priority modal / diary editor)
                // capture all input. Drive them directly with the raw
                // key event so multi-key bindings (space/d/a/ctrl-s)
                // work without inflating the global Action enum.
                if let CEvent::Key(key) = event
                    && app.ceremony_overlay.is_some()
                {
                    handle_ceremony_overlay_key(&mut client, &mut app, key).await;
                    if app.dirty {
                        force_draw(&mut terminal, &mut app)?;
                    }
                    continue;
                }
                if let CEvent::Key(key) = event
                    && app.todo_overlay.is_some()
                {
                    handle_todo_overlay_key(&mut client, &mut app, key).await;
                    if app.dirty {
                        force_draw(&mut terminal, &mut app)?;
                    }
                    continue;
                }
                if let CEvent::Key(key) = event
                    && app.watch_overlay.is_some()
                {
                    handle_watch_overlay_key(&mut client, &mut app, key).await;
                    if app.dirty {
                        force_draw(&mut terminal, &mut app)?;
                    }
                    continue;
                }
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
                                            for (label, key) in [("Global", "global"), ("Lens", "lens")] {
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
                            crate::command::CommandResult::LensCreate(name) => {
                                let params = serde_json::json!({"name": name});
                                if let Ok(resp) = client.request_response("create_lens", params).await {
                                        if resp.get("result").is_some() {
                                            // Refresh lens list and switch to new one
                                            if let Ok(lenses) = client.list_lenses().await {
                                                app.lenses = lenses;
                                                if let Some(ws) = app.lenses.iter().find(|w| w.name == name).cloned() {
                                                    app.current_lens = Some(ws.clone());
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
                                                    app.messages.push(ChatMessage::new(ChatRole::System, format!("Switched to lens '{name}'")));
                                                }
                                            }
                                        } else if let Some(err) = resp.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {err}")));
                                        }
                                    }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::LensList => {
                                if let Ok(lenses) = client.list_lenses().await {
                                    let mut output = String::from("Lenses:\n\n");
                                    for ws in &lenses {
                                        let current = app.current_lens.as_ref().map(|c| c.id) == Some(ws.id);
                                        let marker = if current { "▸ " } else { "  " };
                                        let sessions = client.list_sessions(Some(ws.id)).await.map(|s| s.len()).unwrap_or(0);
                                        output.push_str(&format!("{marker}{} ({} sessions)\n", ws.name, sessions));
                                    }
                                    app.messages.push(ChatMessage::new(ChatRole::System, output));
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::LensSwitch(name) => {
                                // Refresh lens list and find by name
                                if let Ok(lenses) = client.list_lenses().await {
                                    app.lenses = lenses;
                                    if let Some(ws) = app.lenses.iter().find(|w| w.name == name).cloned() {
                                        app.current_lens = Some(ws.clone());
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
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Switched to lens '{name}'")));
                                    } else {
                                        app.messages.push(ChatMessage::new(ChatRole::System, format!("Lens '{name}' not found")));
                                    }
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::SessionNew => {
                                let ws_id = app.current_lens.as_ref().map(|ws| ws.id);
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
                                        "lens_name": ws_name,
                                    });
                                    if let Ok(resp) = client.request_response("promote_session", params).await {
                                            if resp.get("result").and_then(|r| r.get("status")).and_then(|s| s.as_str()) == Some("promoted") {
                                                // Refresh state
                                                if let Ok(lenses) = client.list_lenses().await {
                                                    app.lenses = lenses;
                                                    if let Some(ws) = app.lenses.iter().find(|w| w.name == ws_name).cloned() {
                                                        app.current_lens = Some(ws.clone());
                                                        if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                                                            app.sessions = sessions;
                                                        }
                                                    }
                                                }
                                                app.messages.push(ChatMessage::new(ChatRole::System, format!("Session promoted to lens '{ws_name}'")));
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
                                            "full" => "FULL (full autonomy)",
                                            "edits" => "EDITS (auto-allow edits, ask for shell)",
                                            "plan" => "PLAN (read-only)",
                                            _ => "ASK (default)",
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
                            crate::command::CommandResult::FeedRun(id) => {
                                let body = match client.feed_run(&id).await {
                                    Ok(dto) => format!(
                                        "Ran **{}** (`{}`) on demand.\n\n\
                                         - Last status: `{}`\n\
                                         - Last run: {}\n\
                                         - Run count: {}",
                                        dto.get("template").and_then(|v| v.as_str()).unwrap_or("?"),
                                        id,
                                        dto.get("last_status").and_then(|v| v.as_str()).unwrap_or("?"),
                                        dto.get("last_run_at").and_then(|v| v.as_str()).unwrap_or("?"),
                                        dto.get("run_count").and_then(|v| v.as_u64()).unwrap_or(0),
                                    ),
                                    Err(e) => format!("/feeds run {id} failed: {e}"),
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
                            crate::command::CommandResult::CeremonyShowToday => {
                                let today = chrono::Utc::now()
                                    .date_naive()
                                    .format("%Y-%m-%d")
                                    .to_string();
                                let body = render_ceremony_today(&mut client, &today).await;
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::CeremonyShowWeek => {
                                let iso_week = current_iso_week();
                                let body = render_ceremony_week(&mut client, &iso_week).await;
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                // If the tablet is `open`, open the
                                // interactive priority modal on top of
                                // the chat surface. Otherwise stay
                                // read-only (renderer already pushed
                                // the markdown view).
                                if let Some((tablet_id, status)) =
                                    fetch_tablet_id_and_status(&mut client, "weekly", &iso_week).await
                                    && status == "open"
                                {
                                    let priorities = fetch_priorities(&mut client, &tablet_id).await;
                                    app.ceremony_overlay = Some(
                                        crate::ceremony_modal::CeremonyOverlay::Priority(
                                            crate::ceremony_modal::PriorityModalState::new(
                                                tablet_id,
                                                priorities,
                                            ),
                                        ),
                                    );
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::CeremonyShowRetro => {
                                let iso_week = current_iso_week();
                                let body = render_ceremony_retro(&mut client, &iso_week).await;
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                if let Some((tablet_id, _status)) =
                                    fetch_tablet_id_and_status(&mut client, "retro", &iso_week).await
                                {
                                    let body = fetch_diary_body(&mut client, &tablet_id).await;
                                    app.ceremony_overlay = Some(
                                        crate::ceremony_modal::CeremonyOverlay::Diary(
                                            crate::ceremony_modal::DiaryEditorState::new(
                                                tablet_id, body,
                                            ),
                                        ),
                                    );
                                }
                                app.dirty = true;
                            }
                            crate::command::CommandResult::BriefShow => {
                                let body = render_brief_combined(&mut client).await;
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::UsageShow { period } => {
                                let body = render_usage(&mut client, &period).await;
                                app.messages.push(ChatMessage::new(ChatRole::System, body));
                                app.dirty = true;
                            }
                            crate::command::CommandResult::TodoShow => {
                                let todos = fetch_open_todos(&mut client).await;
                                app.todo_overlay = Some(
                                    crate::todo_modal::TodoModalState::new(todos),
                                );
                                app.dirty = true;
                            }
                            crate::command::CommandResult::FeedWatchModal => {
                                open_watch_modal(&mut client, &mut app).await;
                                app.dirty = true;
                            }
                            _ => {} // Other command results handled in app.handle_action
                        }
                    }

                    // Handle sidebar select — load lens sessions and
                    // resume / create one. I-0035 Phase 3 T-A removed the
                    // Sessions sub-section; sessions are managed via
                    // `/session list` and `/session new` slash commands.
                    if action == crate::action::Action::SidebarSelect
                        && let Some(ws) = app.lenses.get(app.sidebar_ws_index).cloned()
                    {
                        app.current_lens = Some(ws.clone());
                        if let Ok(sessions) = client.list_sessions(Some(ws.id)).await {
                            app.sessions = sessions;
                            app.sidebar_session_index = 0;
                        }

                        // Auto-create a session if the lens has none,
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
                        app.focus = crate::app::Focus::Main;
                        app.dirty = true;
                    }

                    // Handle new session
                    if action == crate::action::Action::NewSession {
                        let ws_id = app.current_lens.as_ref().map(|ws| ws.id);
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

                            // Sidebar panel (lenses only post-T-0356).
                            if let Some(sidebar_rect) = app.layout.sidebar
                                && rect_contains(sidebar_rect, col, row)
                            {
                                app.focus = crate::app::Focus::Sidebar;
                                if let Some(ws_rect) = app.layout.sidebar_ws
                                    && rect_contains(ws_rect, col, row)
                                {
                                    app.sidebar_section =
                                        crate::app::SidebarSection::Lenses;
                                    let item_row =
                                        row.saturating_sub(ws_rect.y + 1) as usize;
                                    if item_row < app.lenses.len() {
                                        app.sidebar_ws_index = item_row;
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

                // Pending overlay refresh from a ceremony_event notice
                // (T-0308 slice 3). Runs RPCs after the event drain so
                // we don't hold the borrow-checker over an await inside
                // the closure-based event handler.
                if app.pending_todo_refresh {
                    app.pending_todo_refresh = false;
                    let fresh = fetch_open_todos(&mut client).await;
                    if let Some(s) = app.todo_overlay.as_mut() {
                        s.set_todos(fresh);
                    }
                }
                if app.pending_ceremony_refresh {
                    app.pending_ceremony_refresh = false;
                    refresh_active_ceremony_overlay(&mut client, &mut app).await;
                    force_render = true;
                }
                // I-0035 Phase 4 T-B: a briefing_ready notice arrived
                // — re-fetch the cached daily/weekly view + rebuild
                // the empty-chat brief markdown.
                if app.pending_brief_refresh {
                    app.pending_brief_refresh = false;
                    refresh_brief_cache(&mut client, &mut app).await;
                    force_render = true;
                }

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

// -----------------------------------------------------------------------------
// /todo overlay (I-0049 T-0314)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod ceremony_refresh_tests {
    use super::*;
    use crate::app::App;
    use crate::ceremony_modal::{CeremonyOverlay, PriorityModalState};

    fn notice_for(tablet_id: &str) -> arawn_service::ServerNotice {
        arawn_service::ServerNotice {
            level: "info".into(),
            category: "ceremony_event".into(),
            message: serde_json::json!({
                "event": "PriorityConfirmed",
                "data": {"priority_id": "p1", "tablet_id": tablet_id},
            })
            .to_string(),
            timestamp: "2026-05-16T00:00:00Z".into(),
        }
    }

    #[test]
    fn ceremony_event_for_active_tablet_flags_refresh() {
        let mut app = App::new();
        app.ceremony_overlay = Some(CeremonyOverlay::Priority(PriorityModalState::new(
            "t1".into(),
            vec![],
        )));
        apply_system_notice(&notice_for("t1"), &mut app);
        assert!(
            app.pending_ceremony_refresh,
            "matching tablet must flag a refresh"
        );
    }

    #[test]
    fn ceremony_event_for_other_tablet_is_ignored() {
        let mut app = App::new();
        app.ceremony_overlay = Some(CeremonyOverlay::Priority(PriorityModalState::new(
            "t1".into(),
            vec![],
        )));
        apply_system_notice(&notice_for("other"), &mut app);
        assert!(!app.pending_ceremony_refresh);
    }

    #[test]
    fn ceremony_event_with_no_overlay_is_ignored() {
        let mut app = App::new();
        apply_system_notice(&notice_for("t1"), &mut app);
        assert!(!app.pending_ceremony_refresh);
        // And does NOT spam chat — silent category.
        assert!(app.messages.is_empty());
    }

    #[test]
    fn non_ceremony_notices_still_render_into_chat() {
        let mut app = App::new();
        let n = arawn_service::ServerNotice {
            level: "info".into(),
            category: "plugin_reload".into(),
            message: "reloaded".into(),
            timestamp: "2026-05-16T00:00:00Z".into(),
        };
        apply_system_notice(&n, &mut app);
        assert_eq!(app.messages.len(), 1);
        assert!(!app.pending_ceremony_refresh);
    }

    // I-0035 Phase 4 T-B (T-0360) — briefing_ready handler.

    fn briefing_ready_notice() -> arawn_service::ServerNotice {
        arawn_service::ServerNotice {
            level: "info".into(),
            category: "briefing_ready".into(),
            message: "Brief updated — daily tablet for 2026-05-19".into(),
            timestamp: "2026-05-19T07:00:00Z".into(),
        }
    }

    #[test]
    fn briefing_ready_flags_refresh_and_posts_toast() {
        let mut app = App::new();
        apply_system_notice(&briefing_ready_notice(), &mut app);
        assert!(
            app.pending_brief_refresh,
            "briefing_ready must flag a brief-cache refresh"
        );
        assert_eq!(
            app.toast_queue.len(),
            1,
            "briefing_ready must enqueue a toast"
        );
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("Brief updated"));
        // Silent category — no chat noise.
        assert!(app.messages.is_empty());
    }

    #[test]
    fn briefing_ready_does_not_affect_ceremony_refresh() {
        let mut app = App::new();
        apply_system_notice(&briefing_ready_notice(), &mut app);
        // briefing_ready is a separate path from the legacy
        // ceremony_event channel — they don't interfere.
        assert!(!app.pending_ceremony_refresh);
        assert!(!app.pending_todo_refresh);
    }
}

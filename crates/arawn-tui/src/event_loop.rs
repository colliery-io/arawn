use std::io;

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CEvent, EventStream, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::{FutureExt, StreamExt};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

use ratatui::layout::Rect;

use crate::app::{App, ChatMessage, ChatRole};
use crate::event::map_key_event;
use crate::render::render;
use crate::ws_client::{EventUpdate, WsClient, engine_event_to_update, parse_engine_event};

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
    if let Ok(_id) = client.send_request("list_available_commands", serde_json::json!({})).await {
        if let Ok(resp) = client.read_response_raw().await {
            if let Some(commands) = resp.get("result").and_then(|r| r.as_array()) {
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

    // Initial render
    terminal.draw(|f| render(&mut app, f))?;

    // Event loop
    let mut term_events = EventStream::new();
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));

    loop {
        tokio::select! {
            // Spinner tick — only redraw when generating
            _ = tick_interval.tick() => {
                if app.is_generating {
                    app.spinner_frame = (app.spinner_frame + 1) % 10;
                    terminal.draw(|f| render(&mut app, f))?;
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
                    if app.active_modal.is_none() {
                        if let Some((request_id, mut rx)) = app.pending_modal_response.take() {
                            let selected_index = match rx.try_recv() {
                                Ok(idx) => idx,
                                Err(_) => None, // Not ready or cancelled
                            };
                            debug!(%request_id, ?selected_index, "sending modal response to server");
                            let params = serde_json::json!({
                                "request_id": request_id,
                                "selected_index": selected_index,
                            });
                            if let Err(e) = client.send_request("user_input_response", params).await {
                                warn!(%request_id, error = %e, "failed to send modal response");
                            }
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
                                if let Ok(_id) = client.send_request("query_inventory", params).await {
                                    if let Ok(resp) = client.read_response_raw().await {
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
                                if let Ok(_id) = client.send_request("memory_summary", serde_json::json!({})).await {
                                    if let Ok(resp) = client.read_response_raw().await {
                                        if let Some(result) = resp.get("result") {
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
                                    }
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
                                    app.sidebar_section = crate::app::SidebarSection::Sessions;
                                    app.dirty = true;
                                }
                            }
                            crate::app::SidebarSection::Sessions => {
                                if let Some(session) = app.sessions.get(app.sidebar_session_index).cloned() {
                                    app.current_session = Some(session.clone());
                                    // Load session messages via WS
                                    if client.send_request("load_session", serde_json::json!({"session_id": session.id.to_string()})).await.is_ok()
                                        && let Ok(resp) = client.read_response_raw().await
                                        && let Some(msgs) = resp.get("result").and_then(|r| r.get("messages")).and_then(|m| m.as_array())
                                    {
                                                app.messages.clear();
                                                for msg in msgs {
                                                    if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                                                        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                                                        let chat_msg = match role {
                                                            "user" => ChatMessage::new(ChatRole::User, content),
                                                            "assistant" => {
                                                                // Check for tool_uses — if present, emit tool call messages
                                                                if let Some(tool_uses) = msg.get("tool_uses").and_then(|t| t.as_array()) {
                                                                    for tu in tool_uses {
                                                                        let name = tu.get("name").and_then(|n| n.as_str()).unwrap_or("tool").to_string();
                                                                        let input = tu.get("input").cloned().unwrap_or(serde_json::Value::Null);
                                                                        let summary = crate::app::format_tool_input(&name, &input);
                                                                        app.messages.push(ChatMessage::new(
                                                                            ChatRole::ToolCall { name },
                                                                            summary,
                                                                        ));
                                                                    }
                                                                }
                                                                // Skip empty assistant messages (tool-use-only turns)
                                                                if content.is_empty() { continue; }
                                                                ChatMessage::new(ChatRole::Assistant, content)
                                                            }
                                                            "tool_result" => {
                                                                let is_error = msg.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                                                                // Find the tool name from the most recent tool call
                                                                let name = app.messages.iter().rev()
                                                                    .find_map(|m| match &m.role {
                                                                        ChatRole::ToolCall { name } => Some(name.clone()),
                                                                        _ => None,
                                                                    })
                                                                    .unwrap_or_else(|| "tool".to_string());
                                                                ChatMessage::new(
                                                                    ChatRole::ToolResult { name, is_error },
                                                                    content,
                                                                )
                                                            }
                                                            "summary" => ChatMessage::new(ChatRole::System, format!("[Summary] {content}")),
                                                            _ => continue,
                                                        };
                                                        app.messages.push(chat_msg);
                                                    }
                                                }
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
                        terminal.draw(|f| render(&mut app, f))?;
                        app.dirty = false;
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

                            // Hit-test: sidebar tab strip (opens sidebar)
                            if let Some(tab_rect) = app.layout.sidebar_tab
                                && rect_contains(tab_rect, col, row) {
                                    app.focus = crate::app::Focus::Sidebar;
                                    app.dirty = true;
                                }

                            // Hit-test against layout regions
                            if let Some(sidebar_rect) = app.layout.sidebar
                                && rect_contains(sidebar_rect, col, row) {
                                    app.focus = crate::app::Focus::Sidebar;
                                    // Check if click is in workstreams or sessions section
                                    if let Some(ws_rect) = app.layout.sidebar_ws
                                        && rect_contains(ws_rect, col, row) {
                                            app.sidebar_section = crate::app::SidebarSection::Workstreams;
                                            // Map row to index (skip border + title row)
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
                            if rect_contains(app.layout.chat, col, row) {
                                app.focus = crate::app::Focus::Main;
                                app.dirty = true;
                            }
                            if rect_contains(app.layout.input, col, row) {
                                app.focus = crate::app::Focus::Main;
                                // Place cursor at click position
                                let offset = col.saturating_sub(app.layout.input.x + 1) as usize;
                                app.cursor_pos = offset.min(app.input_buffer.len());
                                app.dirty = true;
                            }
                        }
                        _ => {}
                    }
                    if app.dirty {
                        terminal.draw(|f| render(&mut app, f))?;
                        app.dirty = false;
                    }
                }
                if let CEvent::Resize(_, _) = event {
                    terminal.draw(|f| render(&mut app, f))?;
                }
            }

            // WebSocket messages (streaming events)
            // Batch consecutive streaming text tokens before redrawing,
            // but draw immediately on state changes (tool call, result, complete).
            Some(msg) = client.read.next() => {
                let mut should_break = false;
                let mut needs_draw = false;

                // Process WS messages, accumulating state changes.
                // Only draw when we receive a Flush event from the server.
                let mut flush = false;

                let apply_update = |update: EventUpdate, app: &mut App| -> bool {
                    match update {
                        EventUpdate::AppendStreamingText(text) => {
                            debug!(len = text.len(), "update: streaming text");
                            // Flush any accumulated streaming text as an assistant message
                            // when we get new text (handles mid-loop narration from tool call turns)
                            if !app.streaming_text.is_empty() {
                                let prev = std::mem::take(&mut app.streaming_text);
                                app.messages.push(ChatMessage::new(ChatRole::Assistant, prev));
                            }
                            app.streaming_text.push_str(&text);
                            return true; // Draw immediately — narration should be visible
                        }
                        EventUpdate::AddToolCall { name, input, .. } => {
                            debug!(%name, "update: tool call start");
                            // Flush streaming text before tool call indicator
                            if !app.streaming_text.is_empty() {
                                let text = std::mem::take(&mut app.streaming_text);
                                app.messages.push(ChatMessage::new(ChatRole::Assistant, text));
                            }
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
                            app.scroll_offset = 0;
                            // Force draw — don't depend on a separate Flush event
                            return true;
                        }
                        EventUpdate::Error(message) => {
                            warn!(%message, "update: engine error");
                            app.messages.push(ChatMessage::new(ChatRole::System, format!("Error: {message}")));
                            app.is_generating = false;
                            app.streaming_text.clear();
                            // Force draw
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

                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Some(event) = parse_engine_event(&text) {
                            flush |= apply_update(engine_event_to_update(event), &mut app);
                        }
                    }
                    Ok(WsMessage::Close(frame)) => {
                        warn!(frame = ?frame, "server closed connection");
                        should_break = true;
                    }
                    Err(e) => {
                        error!(error = %e, "WebSocket error");
                        should_break = true;
                    }
                    _ => {}
                }

                // Drain any immediately queued messages up to the next Flush
                if !should_break && !flush {
                    let mut drained: u32 = 0;
                    loop {
                        match client.read.next().now_or_never() {
                            Some(Some(Ok(WsMessage::Text(text)))) => {
                                drained += 1;
                                if let Some(event) = parse_engine_event(&text) {
                                    flush |= apply_update(engine_event_to_update(event), &mut app);
                                    if flush { break; }
                                }
                            }
                            Some(Some(Ok(WsMessage::Close(frame)))) => {
                                warn!(frame = ?frame, "server closed during drain");
                                should_break = true;
                                break;
                            }
                            Some(Some(Err(e))) => {
                                error!(error = %e, "WebSocket error during drain");
                                should_break = true;
                                break;
                            }
                            _ => break, // No more queued messages
                        }
                    }
                    if drained > 0 {
                        debug!(drained, flush, "drained queued ws messages");
                    }
                }

                if should_break { break; }
                if flush {
                    debug!(messages = app.messages.len(), streaming_len = app.streaming_text.len(), "drawing frame");
                    terminal.draw(|f| render(&mut app, f))?;
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

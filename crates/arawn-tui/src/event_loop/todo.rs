use crate::app::App;

/// Fetch all open todos for the overlay. Filters out done + archived,
/// which is the default behaviour of `todos.list` with no filter.
pub(super) async fn fetch_open_todos(client: &mut crate::ws_client::WsClient) -> Vec<crate::todo_modal::TodoRow> {
    let params = serde_json::json!({"open_only": true});
    let payload = match client.request_response("todos.list", params).await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match payload.get("result").and_then(|r| r.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|row| {
            Some(crate::todo_modal::TodoRow {
                id: row.get("id")?.as_str()?.to_string(),
                body: row.get("body")?.as_str()?.to_string(),
                kind: row.get("kind")?.as_str()?.to_string(),
                workstream: row
                    .get("workstream")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                done_at: row
                    .get("done_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                due_at: row
                    .get("due_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect()
}

pub(super) async fn handle_todo_overlay_key(
    client: &mut crate::ws_client::WsClient,
    app: &mut App,
    key: crossterm::event::KeyEvent,
) {
    use crate::todo_modal::TodoOutcome;
    let Some(state) = app.todo_overlay.as_mut() else {
        return;
    };
    let outcome = state.handle_key(key);
    match outcome {
        TodoOutcome::None => {
            app.dirty = true;
        }
        TodoOutcome::Toggle { id, mark_done } => {
            let method = if mark_done { "todos.done" } else { "todos.undo" };
            let err = match client
                .request_response(method, serde_json::json!({"id": id}))
                .await
            {
                Ok(resp) => resp
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string()),
                Err(e) => Some(e.to_string()),
            };
            let fresh = fetch_open_todos(client).await;
            if let Some(s) = app.todo_overlay.as_mut() {
                s.set_todos(fresh);
                s.last_error = err;
            }
            app.dirty = true;
        }
        TodoOutcome::Archive { id } => {
            let err = match client
                .request_response("todos.archive", serde_json::json!({"id": id}))
                .await
            {
                Ok(resp) => resp
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string()),
                Err(e) => Some(e.to_string()),
            };
            let fresh = fetch_open_todos(client).await;
            if let Some(s) = app.todo_overlay.as_mut() {
                s.set_todos(fresh);
                s.last_error = err;
            }
            app.dirty = true;
        }
        TodoOutcome::Add { body } => {
            let err = match client
                .request_response(
                    "todos.create",
                    serde_json::json!({"body": body, "kind": "user"}),
                )
                .await
            {
                Ok(resp) => resp
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string()),
                Err(e) => Some(e.to_string()),
            };
            let fresh = fetch_open_todos(client).await;
            if let Some(s) = app.todo_overlay.as_mut() {
                s.set_todos(fresh);
                s.last_error = err;
            }
            app.dirty = true;
        }
        TodoOutcome::Close => {
            app.todo_overlay = None;
            app.dirty = true;
        }
    }
}


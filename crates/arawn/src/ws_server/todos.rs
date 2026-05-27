use serde_json::Value;

use crate::local_service::LocalService;

use super::Response;

/// Dispatch a `todos.*` RPC method. Acquires the store lock for the
/// call duration, constructs a `TodoService` wired to the service's
/// broadcast sender so mutations emit `TodoEvent`s, and routes the
/// method to the right entry point.
fn handle_todo_rpc(id: u64, method: &str, params: &Value, service: &LocalService) -> Response {
    let store = service.shared_store();
    let store_lock = match store.lock() {
        Ok(g) => g,
        Err(_) => {
            return Response::error(id, "internal_error", "store lock poisoned".into());
        }
    };
    let svc = arawn_storage::TodoService::new(store_lock.database())
        .with_events(service.todo_event_sender());

    match method {
        "todos.create" => match serde_json::from_value::<arawn_storage::NewTodo>(params.clone()) {
            Ok(req) => match svc.create(req) {
                Ok(t) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            },
            Err(e) => Response::error(id, "invalid_params", format!("todos.create: {e}")),
        },
        "todos.list" => {
            let filter = if params.is_null() {
                arawn_storage::ListFilter::default()
            } else {
                match serde_json::from_value::<arawn_storage::ListFilter>(params.clone()) {
                    Ok(f) => f,
                    Err(e) => {
                        return Response::error(id, "invalid_params", format!("todos.list: {e}"));
                    }
                }
            };
            match svc.list(filter) {
                Ok(rows) => Response::success(id, serde_json::to_value(&rows).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            }
        }
        "todos.get" => {
            let todo_id = match params.get("id").and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => return Response::error(id, "invalid_params", "id is required".into()),
            };
            match svc.get(&todo_id) {
                Ok(Some(t)) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Ok(None) => Response::success(id, Value::Null),
                Err(e) => Response::from_todo_error(id, &e),
            }
        }
        "todos.done" => match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => match svc.mark_done(s) {
                Ok(t) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            },
            _ => Response::error(id, "invalid_params", "id is required".into()),
        },
        "todos.undo" => match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => match svc.undo(s) {
                Ok(t) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            },
            _ => Response::error(id, "invalid_params", "id is required".into()),
        },
        "todos.patch" => {
            let todo_id = match params.get("id").and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => return Response::error(id, "invalid_params", "id is required".into()),
            };
            let patch_v = params.get("patch").cloned().unwrap_or(Value::Null);
            let patch = if patch_v.is_null() {
                arawn_storage::TodoPatch::default()
            } else {
                match serde_json::from_value::<arawn_storage::TodoPatch>(patch_v) {
                    Ok(p) => p,
                    Err(e) => {
                        return Response::error(id, "invalid_params", format!("todos.patch: {e}"));
                    }
                }
            };
            match svc.patch(&todo_id, patch) {
                Ok(t) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            }
        }
        "todos.archive" => match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => match svc.archive(s) {
                Ok(()) => Response::success(id, Value::Null),
                Err(e) => Response::from_todo_error(id, &e),
            },
            _ => Response::error(id, "invalid_params", "id is required".into()),
        },
        "todos.search" => {
            let q = params
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if q.is_empty() {
                return Response::error(id, "invalid_params", "query is required".into());
            }
            match svc.search(&q) {
                Ok(rows) => Response::success(id, serde_json::to_value(&rows).unwrap()),
                Err(e) => Response::from_todo_error(id, &e),
            }
        }
        other => Response::error(
            id,
            "method_not_found",
            format!("unknown todos method: {other}"),
        ),
    }
}

pub(super) fn dispatch(id: u64, method: &str, params: &Value, service: &LocalService) -> Response {
    handle_todo_rpc(id, method, params, service)
}

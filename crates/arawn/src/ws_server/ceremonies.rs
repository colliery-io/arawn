use std::sync::Arc;

use axum::extract::ws::Message as WsMessage;
use axum::extract::ws::WebSocket;
use futures::SinkExt;
use futures::stream::SplitSink;
use serde_json::Value;
use tracing::warn;

use crate::local_service::LocalService;

use super::Response;

/// Dispatch a `ceremonies.*` RPC method. Extracted from `handle_connection`
/// so the main loop stays readable. Sends the response (or an unavailable
/// error) on `sender` and returns; caller continues the main loop.
pub(super) async fn dispatch(
    id: u64,
    method: &str,
    params: &Value,
    service: &Arc<LocalService>,
    sender: &mut SplitSink<WebSocket, WsMessage>,
) {
    let cer = match service.ceremony_service() {
        Some(svc) => svc,
        None => {
            let resp = Response::error(
                id,
                "ceremony_unavailable",
                "ceremony engine not wired (workflow runner unavailable at boot)".into(),
            );
            let _ = sender
                .send(WsMessage::Text(
                    serde_json::to_string(&resp).unwrap().into(),
                ))
                .await;
            return;
        }
    };
    let resp: Response = match method {
        "ceremonies.get_retro_current" => {
            let iso_week = arawn_ceremonies::RetroCeremony::iso_week(chrono::Utc::now());
            match cer.get_by_period("retro", &iso_week) {
                Ok(Some(t)) => Response::success(id, serde_json::to_value(&t).unwrap()),
                Ok(None) => Response::success(id, Value::Null),
                Err(e) => Response::from_ceremony_error(id, &e),
            }
        }
        "ceremonies.get_by_period" => {
            let kind = params.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let period_key = params
                .get("period_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if kind.is_empty() || period_key.is_empty() {
                Response::error(
                    id,
                    "invalid_params",
                    "kind and period_key are required".into(),
                )
            } else {
                match cer.get_by_period(kind, period_key) {
                    Ok(Some(t)) => Response::success(id, serde_json::to_value(&t).unwrap()),
                    Ok(None) => Response::success(id, Value::Null),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.list_items" => {
            let tablet_id = params
                .get("tablet_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let section_key = params
                .get("section_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if tablet_id.is_empty() {
                Response::error(id, "invalid_params", "tablet_id is required".into())
            } else {
                match cer.list_items(&tablet_id, section_key.as_deref()) {
                    Ok(items) => Response::success(id, serde_json::to_value(&items).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.patch_item" => {
            let item_id = params
                .get("item_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let patch_val = params.get("patch").cloned().unwrap_or(Value::Null);
            let patch: arawn_ceremonies::ItemPatch =
                serde_json::from_value(patch_val).unwrap_or_default();
            if item_id.is_empty() {
                Response::error(id, "invalid_params", "item_id is required".into())
            } else {
                match cer.patch_item(&item_id, patch) {
                    Ok(dto) => Response::success(id, serde_json::to_value(&dto).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.add_item" => {
            match serde_json::from_value::<arawn_ceremonies::AddItemRequest>(params.clone()) {
                Ok(req) => match cer.add_item(req) {
                    Ok(dto) => Response::success(id, serde_json::to_value(&dto).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                },
                Err(e) => Response::error(id, "invalid_params", format!("add_item params: {e}")),
            }
        }
        "ceremonies.upsert_diary" => {
            let tablet_id = params
                .get("tablet_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let body = params
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if tablet_id.is_empty() {
                Response::error(id, "invalid_params", "tablet_id is required".into())
            } else {
                match cer.upsert_diary(&tablet_id, &body) {
                    Ok(()) => Response::success(id, serde_json::json!({"ok": true})),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.get_diary" => {
            let tablet_id = params
                .get("tablet_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if tablet_id.is_empty() {
                Response::error(id, "invalid_params", "tablet_id is required".into())
            } else {
                match cer.get_diary(&tablet_id) {
                    Ok(Some(body)) => Response::success(id, serde_json::json!({"body": body})),
                    Ok(None) => Response::success(id, Value::Null),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.run" => {
            let kind = params
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("retro")
                .to_string();
            match cer.run(&kind).await {
                Ok(arawn_ceremonies::DispatchOutcome::Generated { tablet_id }) => {
                    Response::success(
                        id,
                        serde_json::json!({
                            "status": "generated",
                            "tablet_id": tablet_id,
                        }),
                    )
                }
                Ok(arawn_ceremonies::DispatchOutcome::Skipped { reason }) => Response::success(
                    id,
                    serde_json::json!({
                        "status": "skipped",
                        "reason": reason,
                    }),
                ),
                Err(e) => Response::from_ceremony_error(id, &e),
            }
        }
        "ceremonies.list_notifications" => match cer.list_notifications() {
            Ok(n) => Response::success(id, serde_json::to_value(&n).unwrap()),
            Err(e) => Response::from_ceremony_error(id, &e),
        },
        "ceremonies.confirm_priority" => {
            let item_id = params
                .get("item_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if item_id.is_empty() {
                Response::error(id, "invalid_params", "item_id is required".into())
            } else {
                match cer.confirm_priority(&item_id) {
                    Ok(dto) => Response::success(id, serde_json::to_value(&dto).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.reject_priority" => {
            let item_id = params
                .get("item_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if item_id.is_empty() {
                Response::error(id, "invalid_params", "item_id is required".into())
            } else {
                match cer.reject_priority(&item_id) {
                    Ok(()) => Response::success(id, serde_json::json!({"ok": true})),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        "ceremonies.add_priority" => {
            match serde_json::from_value::<arawn_ceremonies::AddPriorityRequest>(params.clone()) {
                Ok(req) => match cer.add_priority(req) {
                    Ok(dto) => Response::success(id, serde_json::to_value(&dto).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                },
                Err(e) => {
                    Response::error(id, "invalid_params", format!("add_priority params: {e}"))
                }
            }
        }
        "ceremonies.list_priorities" => {
            let tablet_id = params
                .get("tablet_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if tablet_id.is_empty() {
                Response::error(id, "invalid_params", "tablet_id is required".into())
            } else {
                match cer.list_priorities(&tablet_id) {
                    Ok(list) => Response::success(id, serde_json::to_value(&list).unwrap()),
                    Err(e) => Response::from_ceremony_error(id, &e),
                }
            }
        }
        other => Response::error(
            id,
            "method_not_found",
            format!("unknown ceremonies method: {other}"),
        ),
    };
    if sender
        .send(WsMessage::Text(
            serde_json::to_string(&resp).unwrap().into(),
        ))
        .await
        .is_err()
    {
        warn!(id, "send failed, client gone");
    }
}

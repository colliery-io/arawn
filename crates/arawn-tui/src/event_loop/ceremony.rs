use crate::app::App;

/// ISO-week period key in the canonical `YYYY-WNN` form used by the
/// ceremony engine. Inlined here so the TUI doesn't depend on
/// arawn-ceremonies purely for this two-line helper.
pub(super) fn current_iso_week() -> String {
    use chrono::Datelike;
    let iso = chrono::Utc::now().iso_week();
    format!("{:04}-W{:02}", iso.year(), iso.week())
}

/// Fetch the daily tablet for `today`, then list its items, then
/// render. Single string is the system-message body.
pub(super) async fn render_ceremony_today(
    client: &mut crate::ws_client::WsClient,
    today: &str,
) -> String {
    let params = serde_json::json!({"kind": "daily", "period_key": today});
    let resp = match client
        .request_response("ceremonies.get_by_period", params)
        .await
    {
        Ok(v) => v,
        Err(e) => return format!("/today failed: {e}"),
    };
    let result = match resp.get("result") {
        Some(r) => r,
        None => {
            let err = resp
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return format!("/today failed: {err}");
        }
    };
    if result.is_null() {
        return format!(
            "No daily tablet for today ({today}) — run `/agent daily_run` or wait for the scheduled fire."
        );
    }
    let tablet: arawn_ceremonies::TabletDto = match serde_json::from_value(result.clone()) {
        Ok(t) => t,
        Err(e) => return format!("/today: malformed tablet response: {e}"),
    };
    let items = fetch_items(client, &tablet.id).await;
    let view = arawn_ceremonies::DailyView { tablet, items };
    arawn_ceremonies::render_daily(&view)
}

/// Fetch the weekly tablet for the current ISO week, then items, then
/// priorities, then render.
pub(super) async fn render_ceremony_week(
    client: &mut crate::ws_client::WsClient,
    iso_week: &str,
) -> String {
    let params = serde_json::json!({"kind": "weekly", "period_key": iso_week});
    let resp = match client
        .request_response("ceremonies.get_by_period", params)
        .await
    {
        Ok(v) => v,
        Err(e) => return format!("/week failed: {e}"),
    };
    let result = match resp.get("result") {
        Some(r) => r,
        None => {
            let err = resp
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return format!("/week failed: {err}");
        }
    };
    if result.is_null() {
        return format!(
            "No weekly tablet for {iso_week} — run `/agent weekly_run` or wait for Monday's scheduled fire."
        );
    }
    let tablet: arawn_ceremonies::TabletDto = match serde_json::from_value(result.clone()) {
        Ok(t) => t,
        Err(e) => return format!("/week: malformed tablet response: {e}"),
    };
    let items = fetch_items(client, &tablet.id).await;
    let priorities = fetch_priorities(client, &tablet.id).await;
    let view = arawn_ceremonies::WeeklyView {
        tablet,
        items,
        priorities,
    };
    arawn_ceremonies::render_weekly(&view)
}

/// Fetch the retro tablet for the current ISO week, then items, then
/// render. Diary fetch is a future RPC (T-0290 notes this) — pass
/// `None` for now so the renderer prints the placeholder.
pub(super) async fn render_ceremony_retro(
    client: &mut crate::ws_client::WsClient,
    iso_week: &str,
) -> String {
    let params = serde_json::json!({"kind": "retro", "period_key": iso_week});
    let resp = match client
        .request_response("ceremonies.get_by_period", params)
        .await
    {
        Ok(v) => v,
        Err(e) => return format!("/retro failed: {e}"),
    };
    let result = match resp.get("result") {
        Some(r) => r,
        None => {
            let err = resp
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return format!("/retro failed: {err}");
        }
    };
    if result.is_null() {
        return format!(
            "No retro tablet for {iso_week} — run `/agent retro_run` or wait for Friday's scheduled fire."
        );
    }
    let tablet: arawn_ceremonies::TabletDto = match serde_json::from_value(result.clone()) {
        Ok(t) => t,
        Err(e) => return format!("/retro: malformed tablet response: {e}"),
    };
    let items = fetch_items(client, &tablet.id).await;
    let view = arawn_ceremonies::RetroView {
        tablet,
        items,
        diary: None,
    };
    arawn_ceremonies::render_retro(&view)
}

/// Fetch the daily tablet + items for `today` and return a
/// `DailyView`. None when no daily tablet exists for that date.
/// Used by `/brief` and the empty-chat brief pre-fetch; shares the
/// same `ceremonies.get_by_period` path the `/today` renderer uses.
pub(super) async fn fetch_daily_view(
    client: &mut crate::ws_client::WsClient,
    today: &str,
) -> Option<arawn_ceremonies::DailyView> {
    let params = serde_json::json!({"kind": "daily", "period_key": today});
    let resp = client
        .request_response("ceremonies.get_by_period", params)
        .await
        .ok()?;
    let result = resp.get("result")?;
    if result.is_null() {
        return None;
    }
    let tablet: arawn_ceremonies::TabletDto = serde_json::from_value(result.clone()).ok()?;
    let items = fetch_items(client, &tablet.id).await;
    Some(arawn_ceremonies::DailyView { tablet, items })
}

/// Fetch the weekly tablet + items + priorities for `iso_week`.
/// Companion to `fetch_daily_view`.
pub(super) async fn fetch_weekly_view(
    client: &mut crate::ws_client::WsClient,
    iso_week: &str,
) -> Option<arawn_ceremonies::WeeklyView> {
    let params = serde_json::json!({"kind": "weekly", "period_key": iso_week});
    let resp = client
        .request_response("ceremonies.get_by_period", params)
        .await
        .ok()?;
    let result = resp.get("result")?;
    if result.is_null() {
        return None;
    }
    let tablet: arawn_ceremonies::TabletDto = serde_json::from_value(result.clone()).ok()?;
    let items = fetch_items(client, &tablet.id).await;
    let priorities = fetch_priorities(client, &tablet.id).await;
    Some(arawn_ceremonies::WeeklyView {
        tablet,
        items,
        priorities,
    })
}

/// Re-fetch the tablet for `(kind, period_key)` and return its id +
/// status. None if no tablet exists (the read-only renderer already
/// pushed the "no tablet for …" message).
pub(super) async fn fetch_tablet_id_and_status(
    client: &mut crate::ws_client::WsClient,
    kind: &str,
    period_key: &str,
) -> Option<(String, String)> {
    let params = serde_json::json!({"kind": kind, "period_key": period_key});
    let resp = client
        .request_response("ceremonies.get_by_period", params)
        .await
        .ok()?;
    let result = resp.get("result")?;
    if result.is_null() {
        return None;
    }
    let tablet: arawn_ceremonies::TabletDto = serde_json::from_value(result.clone()).ok()?;
    Some((tablet.id, tablet.status))
}

/// Pull any existing diary body from the retro tablet by listing its
/// Fetch the diary body for a retro tablet via the dedicated
/// `ceremonies.get_diary` RPC. Returns "" when no diary row exists.
pub(super) async fn fetch_diary_body(
    client: &mut crate::ws_client::WsClient,
    tablet_id: &str,
) -> String {
    let resp = match client
        .request_response(
            "ceremonies.get_diary",
            serde_json::json!({"tablet_id": tablet_id}),
        )
        .await
    {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    resp.get("result")
        .and_then(|r| r.get("body"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Drive the active ceremony overlay from a raw key event. Routes the
/// outcome to the corresponding RPC, then re-fetches the underlying
/// data so the overlay reflects the new state.
pub(super) async fn handle_ceremony_overlay_key(
    client: &mut crate::ws_client::WsClient,
    app: &mut App,
    key: crossterm::event::KeyEvent,
) {
    use crate::ceremony_modal::{CeremonyOverlay, DiaryOutcome, PriorityOutcome};

    app.dirty = true;
    let Some(overlay) = app.ceremony_overlay.as_mut() else {
        return;
    };
    match overlay {
        CeremonyOverlay::Priority(state) => {
            let outcome = state.handle_key(key);
            let tablet_id = state.tablet_id.clone();
            match outcome {
                PriorityOutcome::None => {}
                PriorityOutcome::Close => {
                    app.ceremony_overlay = None;
                }
                PriorityOutcome::Confirm { item_id } => {
                    let res = client
                        .request_response(
                            "ceremonies.confirm_priority",
                            serde_json::json!({"item_id": item_id}),
                        )
                        .await;
                    apply_priority_rpc_result(app, &tablet_id, client, res).await;
                }
                PriorityOutcome::Reject { item_id } => {
                    let res = client
                        .request_response(
                            "ceremonies.reject_priority",
                            serde_json::json!({"item_id": item_id}),
                        )
                        .await;
                    apply_priority_rpc_result(app, &tablet_id, client, res).await;
                }
                PriorityOutcome::Add { tablet_id, body } => {
                    let res = client
                        .request_response(
                            "ceremonies.add_priority",
                            serde_json::json!({
                                "tablet_id": tablet_id,
                                "body": body,
                                "rationale": "",
                            }),
                        )
                        .await;
                    apply_priority_rpc_result(app, &tablet_id, client, res).await;
                }
            }
        }
        CeremonyOverlay::Diary(state) => {
            let outcome = state.handle_key(key);
            match outcome {
                DiaryOutcome::None => {}
                DiaryOutcome::Close => {
                    app.ceremony_overlay = None;
                }
                DiaryOutcome::Save { tablet_id, body } => {
                    let res = client
                        .request_response(
                            "ceremonies.upsert_diary",
                            serde_json::json!({
                                "tablet_id": tablet_id,
                                "body": body,
                            }),
                        )
                        .await;
                    if let Some(CeremonyOverlay::Diary(d)) = app.ceremony_overlay.as_mut() {
                        match res {
                            Ok(v) if v.get("error").is_some() => {
                                d.last_error = Some(
                                    v.get("error")
                                        .and_then(|e| e.get("message"))
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("rpc error")
                                        .to_string(),
                                );
                            }
                            Ok(_) => {
                                d.editing = false;
                                d.last_error = None;
                            }
                            Err(e) => d.last_error = Some(e.to_string()),
                        }
                    }
                }
            }
        }
    }
}

/// After a priority RPC, refresh the modal's priorities list (or stash
/// the error). Shared by confirm/reject/add.
pub(super) async fn apply_priority_rpc_result(
    app: &mut App,
    tablet_id: &str,
    client: &mut crate::ws_client::WsClient,
    res: Result<serde_json::Value, Box<dyn std::error::Error>>,
) {
    use crate::ceremony_modal::CeremonyOverlay;
    let err = match res {
        Ok(v) => v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string()),
        Err(e) => Some(e.to_string()),
    };
    let fresh = fetch_priorities(client, tablet_id).await;
    if let Some(CeremonyOverlay::Priority(p)) = app.ceremony_overlay.as_mut() {
        p.set_priorities(fresh);
        p.last_error = err;
    }
}

/// Re-pull underlying data for whichever ceremony overlay is active so
/// it picks up server-side mutations (agent runs, other clients,
/// background sweeps). Called from the event loop after a
/// `ceremony_event` notice flags `pending_ceremony_refresh`.
pub(super) async fn refresh_active_ceremony_overlay(
    client: &mut crate::ws_client::WsClient,
    app: &mut App,
) {
    use crate::ceremony_modal::CeremonyOverlay;
    match app.ceremony_overlay.as_ref() {
        Some(CeremonyOverlay::Priority(p)) => {
            let tablet_id = p.tablet_id.clone();
            let fresh = fetch_priorities(client, &tablet_id).await;
            if let Some(CeremonyOverlay::Priority(p)) = app.ceremony_overlay.as_mut() {
                p.set_priorities(fresh);
            }
        }
        Some(CeremonyOverlay::Diary(d)) => {
            // Only refresh body when not editing (default to "edit
            // wins" — see T-0308 risk note: user's local buffer wins
            // over concurrent server updates).
            if !d.editing {
                let tablet_id = d.tablet_id.clone();
                let body = fetch_diary_body(client, &tablet_id).await;
                if let Some(CeremonyOverlay::Diary(d)) = app.ceremony_overlay.as_mut() {
                    d.body = body;
                    d.cursor = d.body.len();
                }
            }
        }
        None => {}
    }
}

pub(super) async fn fetch_items(
    client: &mut crate::ws_client::WsClient,
    tablet_id: &str,
) -> Vec<arawn_ceremonies::ItemDto> {
    let resp = match client
        .request_response(
            "ceremonies.list_items",
            serde_json::json!({"tablet_id": tablet_id}),
        )
        .await
    {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    resp.get("result")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default()
}

pub(super) async fn fetch_priorities(
    client: &mut crate::ws_client::WsClient,
    tablet_id: &str,
) -> Vec<arawn_ceremonies::PriorityDto> {
    let resp = match client
        .request_response(
            "ceremonies.list_priorities",
            serde_json::json!({"tablet_id": tablet_id}),
        )
        .await
    {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    resp.get("result")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default()
}

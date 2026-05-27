//! Event-loop glue for the `/watch` registration modal (ARAWN-I-0058 T-D).
//!
//! `open_watch_modal` fetches the template catalog and opens the overlay;
//! `handle_watch_overlay_key` routes the modal's [`WatchOutcome`] to the
//! `feed_schema` / `feed_register` RPCs. The modal itself (`watch_modal.rs`)
//! is pure state — all I/O lives here.

use arawn_service::{FeedDiscoverDto, FeedSchemaDto, FeedTemplateInfo};

use crate::app::{App, ChatMessage, ChatRole};
use crate::event_loop::formats::format_feed_registered;
use crate::watch_modal::{DiscoveryChoice, TemplateChoice, WatchModalState, WatchOutcome};
use crate::ws_client::WsClient;

/// Map a `feed_discover` response into pick-list choices for `field_key`: each
/// row's label/hint plus the value its `params[field_key]` resolves to. Rows
/// that don't carry that key (or aren't picker-supported) are dropped.
fn discovery_choices(value: &serde_json::Value, field_key: &str) -> Vec<DiscoveryChoice> {
    let dto: FeedDiscoverDto = match serde_json::from_value(value.clone()) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    if !dto.picker_supported {
        return Vec::new();
    }
    dto.rows
        .into_iter()
        .filter_map(|row| {
            let value = row
                .params
                .get(field_key)
                .and_then(|v| v.as_str())?
                .to_string();
            Some(DiscoveryChoice {
                label: row.label,
                hint: row.hint,
                value,
            })
        })
        .collect()
}

/// Fetch the template catalog and open the modal at stage 1. On RPC failure,
/// surfaces a system message and leaves the overlay closed.
pub(super) async fn open_watch_modal(client: &mut WsClient, app: &mut App) {
    match client.feed_templates().await {
        Ok(value) => {
            let infos: Vec<FeedTemplateInfo> = serde_json::from_value(value).unwrap_or_default();
            let choices = infos
                .into_iter()
                .map(|t| TemplateChoice {
                    name: t.name,
                    description: t.description,
                })
                .collect();
            app.watch_overlay = Some(WatchModalState::new(choices));
        }
        Err(e) => {
            app.messages.push(ChatMessage::new(
                ChatRole::System,
                format!("/watch failed to list templates: {e}"),
            ));
        }
    }
}

pub(super) async fn handle_watch_overlay_key(
    client: &mut WsClient,
    app: &mut App,
    key: crossterm::event::KeyEvent,
) {
    let Some(state) = app.watch_overlay.as_mut() else {
        return;
    };
    let outcome = state.handle_key(key);
    match outcome {
        WatchOutcome::None => {
            app.dirty = true;
        }
        WatchOutcome::Cancel => {
            app.watch_overlay = None;
            app.dirty = true;
        }
        WatchOutcome::TemplatePicked(template) => {
            // Fetch the schema and transition the form into stage 2. On
            // failure, drop a message and close (rare — the template just
            // came from the catalog).
            match client.feed_schema(&template).await {
                Ok(value) => match serde_json::from_value::<FeedSchemaDto>(value) {
                    Ok(dto) => {
                        if let Some(state) = app.watch_overlay.as_mut() {
                            state.enter_form(&dto.template, dto.params, &dto.default_cadence);
                        }
                    }
                    Err(e) => close_with_message(app, format!("/watch: bad schema payload: {e}")),
                },
                Err(e) => close_with_message(app, format!("/watch: couldn't load {template}: {e}")),
            }
            app.dirty = true;
        }
        WatchOutcome::Discover {
            template,
            field_key,
        } => {
            // Fetch the provider's choices and feed them into the field. The
            // modal opens the pick-list (or falls back to free text if empty).
            let choices = match client.feed_discover(&template).await {
                Ok(value) => discovery_choices(&value, &field_key),
                Err(_) => Vec::new(),
            };
            if let Some(state) = app.watch_overlay.as_mut() {
                state.set_field_choices(&field_key, choices);
            }
            app.dirty = true;
        }
        WatchOutcome::Submit {
            template,
            feed_id,
            params,
            cadence,
        } => {
            let payload = serde_json::json!({
                "template": template,
                "feed_id": feed_id,
                "params": params,
                "cadence": cadence,
            });
            match client.feed_register(payload).await {
                Ok(dto) => {
                    // Success: close the modal and report like the text path.
                    app.watch_overlay = None;
                    app.messages.push(ChatMessage::new(
                        ChatRole::System,
                        format_feed_registered(&dto),
                    ));
                }
                Err(e) => {
                    // Keep the form open with the server's error so the user
                    // can fix the offending field and retry.
                    if let Some(state) = app.watch_overlay.as_mut() {
                        state.last_error = Some(e.to_string());
                    }
                }
            }
            app.dirty = true;
        }
    }
}

fn close_with_message(app: &mut App, msg: String) {
    app.watch_overlay = None;
    app.messages.push(ChatMessage::new(ChatRole::System, msg));
}

#[cfg(test)]
mod tests {
    use super::discovery_choices;
    use serde_json::json;

    #[test]
    fn maps_discover_rows_to_choices_for_the_field() {
        let dto = json!({
            "template": "slack/channel-archive",
            "picker_supported": true,
            "rows": [
                {"label": "#design", "hint": "C1", "params": {"channel": "C1"}},
                {"label": "#eng", "hint": null, "params": {"channel": "C2"}},
                {"label": "no-key", "params": {"other": "x"}}
            ]
        });
        let out = discovery_choices(&dto, "channel");
        assert_eq!(out.len(), 2, "rows lacking the key are dropped");
        assert_eq!(out[0].label, "#design");
        assert_eq!(out[0].value, "C1");
        assert_eq!(out[0].hint.as_deref(), Some("C1"));
        assert_eq!(out[1].value, "C2");
    }

    #[test]
    fn non_picker_supported_yields_no_choices() {
        let dto = json!({"template": "x/y", "picker_supported": false, "rows": []});
        assert!(discovery_choices(&dto, "channel").is_empty());
    }
}

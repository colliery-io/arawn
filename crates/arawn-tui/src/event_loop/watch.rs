//! Event-loop glue for the `/watch` registration modal (ARAWN-I-0058 T-D).
//!
//! `open_watch_modal` fetches the template catalog and opens the overlay;
//! `handle_watch_overlay_key` routes the modal's [`WatchOutcome`] to the
//! `feed_schema` / `feed_register` RPCs. The modal itself (`watch_modal.rs`)
//! is pure state — all I/O lives here.

use arawn_service::{FeedSchemaDto, FeedTemplateInfo};

use crate::app::{App, ChatMessage, ChatRole};
use crate::event_loop::formats::format_feed_registered;
use crate::watch_modal::{TemplateChoice, WatchModalState, WatchOutcome};
use crate::ws_client::WsClient;

/// Fetch the template catalog and open the modal at stage 1. On RPC failure,
/// surfaces a system message and leaves the overlay closed.
pub(super) async fn open_watch_modal(client: &mut WsClient, app: &mut App) {
    match client.feed_templates().await {
        Ok(value) => {
            let infos: Vec<FeedTemplateInfo> =
                serde_json::from_value(value).unwrap_or_default();
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
                Err(e) => {
                    close_with_message(app, format!("/watch: couldn't load {template}: {e}"))
                }
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
                    app.messages
                        .push(ChatMessage::new(ChatRole::System, format_feed_registered(&dto)));
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

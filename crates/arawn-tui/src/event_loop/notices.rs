/// Push a server-side notice (plugin/config hot-reload outcome) into the
/// chat history as a system message. Failures get an "✗" prefix; successes
/// get an info marker. Both stay visible — fade-out is future work.
pub(super) fn apply_system_notice(notice: &arawn_service::ServerNotice, app: &mut crate::app::App) {
    // Ceremony events are silent — they only trigger an overlay
    // refresh (T-0308 slice 3). Pushing them into chat would spam
    // the user with `ItemUpdated` rows on every keypress.
    if notice.category == "ceremony_event" {
        ceremony_event_should_refresh(notice, app);
        app.dirty = true;
        return;
    }
    // Todo events (I-0049 T-0314) are silent like ceremony events —
    // they trigger a re-fetch of the open todo list when the modal
    // is up, no chat noise.
    if notice.category == "todo_event" {
        if app.todo_overlay.is_some() {
            app.pending_todo_refresh = true;
        }
        app.dirty = true;
        return;
    }
    // I-0035 Phase 4 T-B: briefing_ready notices trigger a brief
    // cache refresh + a toast — no chat message, the toast is the
    // user-visible affordance.
    if notice.category == "briefing_ready" {
        app.pending_brief_refresh = true;
        app.post_toast(notice.message.clone(), crate::toast::ToastLevel::Info);
        app.dirty = true;
        return;
    }

    let marker = if notice.level == "error" {
        "✗"
    } else {
        "ℹ"
    };
    let body = format!("{marker} [{}] {}", notice.category, notice.message);
    app.messages.push(crate::app::ChatMessage::new(
        crate::app::ChatRole::System,
        body,
    ));

    // An integration notice (success or error) ends an in-flight OAuth
    // dance; clear the heartbeat so the user sees the resolution.
    if notice.category == "integration" {
        app.oauth_in_flight = None;
    }

    app.dirty = true;
}

/// If a ceremony_event notice targets the tablet the user is currently
/// viewing in a ceremony overlay, flag the overlay for refresh. Pure
/// state mutation — the async RPC happens in the event loop after this.
pub(super) fn ceremony_event_should_refresh(
    notice: &arawn_service::ServerNotice,
    app: &mut crate::app::App,
) {
    // `message` carries the serialised CeremonyEvent JSON.
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&notice.message) else {
        return;
    };
    let event_tablet = payload
        .get("data")
        .and_then(|d| d.get("tablet_id"))
        .and_then(|v| v.as_str());
    let overlay_tablet = match app.ceremony_overlay.as_ref() {
        Some(crate::ceremony_modal::CeremonyOverlay::Priority(p)) => Some(p.tablet_id.as_str()),
        Some(crate::ceremony_modal::CeremonyOverlay::Diary(d)) => Some(d.tablet_id.as_str()),
        None => None,
    };
    if let (Some(et), Some(ot)) = (event_tablet, overlay_tablet)
        && et == ot
    {
        app.pending_ceremony_refresh = true;
    }
}

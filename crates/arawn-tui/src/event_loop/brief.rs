use super::ceremony::{current_iso_week, fetch_daily_view, fetch_weekly_view};
use crate::app::App;

/// I-0035 Phase 2 / 3 / 4: refresh the cached brief markdown +
/// parsed daily view on `App`. Used by the session-start preload
/// and by the briefing_ready ServerNotice handler. Idempotent;
/// when no tablets exist, leaves the caches as-is so the
/// pre-onboarding welcome continues to win.
pub(super) async fn refresh_brief_cache(client: &mut crate::ws_client::WsClient, app: &mut App) {
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let iso_week = current_iso_week();
    let daily = fetch_daily_view(client, &today).await;
    let weekly = fetch_weekly_view(client, &iso_week).await;
    if daily.is_none() && weekly.is_none() {
        return;
    }
    let view = arawn_ceremonies::BriefView {
        daily: daily.clone(),
        weekly,
    };
    app.brief_markdown = Some(arawn_ceremonies::render_brief(&view, chrono::Utc::now()));
    app.daily_view = daily;
}

/// I-0035 Phase 2 (T-0354): fetch today's daily tablet + this week's
/// weekly tablet and compose them into a single brief markdown
/// document. Returns the rendered string ready to push into a chat
/// message (or to cache for the empty-chat surface).
pub(super) async fn render_brief_combined(client: &mut crate::ws_client::WsClient) -> String {
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let iso_week = current_iso_week();
    let daily = fetch_daily_view(client, &today).await;
    let weekly = fetch_weekly_view(client, &iso_week).await;
    let view = arawn_ceremonies::BriefView { daily, weekly };
    arawn_ceremonies::render_brief(&view, chrono::Utc::now())
}

//! Web GUI: asset serving (GUI-F1), server-rendered shell + SSE push bridge
//! (GUI-F2). Initiative ARAWN-I-0070.
//!
//! Everything is served from the *same* axum router/listener as the WS-RPC
//! server, preserving the single-binary story.
//!
//! ## Hypermedia approach (GUI-F2)
//!
//! The page is server-rendered HTML ([`maud`], compile-checked) and live
//! updates arrive as **HTML fragments over SSE**: the `/events` endpoint
//! bridges the server-wide `ServerNotice` broadcast (the same stream the TUI
//! consumes — `briefing_ready`, feed/ceremony events, …) into SSE events, and
//! a tiny embedded client (`assets/app.js`, ~15 lines of vanilla `EventSource`)
//! swaps each fragment into the page by target selector.
//!
//! The chosen library was datastar, but it ships as a browser JS file (not a
//! cargo crate) and can't be vendored offline here. The server contract — SSE
//! of `{target, mode, html}` fragments — is library-agnostic, so datastar (or
//! htmx+SSE) drops in later by embedding its runtime and switching the client
//! attributes; the Rust side is unchanged.

use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::stream::{Stream, StreamExt};
use maud::{DOCTYPE, Markup, html};
use rust_embed::RustEmbed;
use tokio_stream::wrappers::BroadcastStream;

use arawn_ceremonies::{CeremonyService, ItemDto, PriorityDto, TabletDto};
use arawn_service::{ArawnService, ServerNotice, SystemStatus};
use arawn_storage::Todo;

use super::AppState;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct GuiAssets;

/// The review/triage surfaces the shell links to. Targets land in GUI-S1..S4;
/// until then the links resolve to placeholder routes.
const SURFACES: &[(&str, &str)] = &[
    ("/health", "Health"),
    ("/brief", "Brief"),
    ("/inbox", "Inbox"),
    ("/signals", "Signals"),
];

/// `GET /` — the server-rendered GUI shell.
pub(super) async fn shell() -> axum::response::Html<String> {
    axum::response::Html(shell_markup().into_string())
}

/// `GET /assets/{*path}` — serve a static asset from the embedded bundle.
pub(super) async fn gui_asset(Path(path): Path<String>) -> Response {
    serve_embedded(&format!("assets/{path}"))
}

/// `GET /events` — SSE stream of HTML fragments derived from the server-wide
/// `ServerNotice` broadcast. The browser applies each fragment in place; no
/// polling, no full reload.
pub(super) async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(notice_event_stream(state.service.subscribe_notices()))
        .keep_alive(KeepAlive::default())
}

/// Turn a `ServerNotice` broadcast receiver into the SSE event stream the
/// browser consumes: one `fragment` event per notice; lagged (slow-client)
/// gaps are dropped rather than erroring the stream. Extracted from `events`
/// so the broadcast→SSE forwarding is unit-testable without a full server.
fn notice_event_stream(
    rx: tokio::sync::broadcast::Receiver<ServerNotice>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(notice) => Some(Ok(Event::default()
                .event("fragment")
                .data(notice_fragment(&notice)))),
            Err(_) => None,
        }
    })
}

/// Shared page chrome: head, the surfaces nav (with the active surface
/// highlighted), the page `content`, and the SSE client script. Every GUI
/// surface renders through this so the shell stays consistent.
fn page(active: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Arawn" }
                link rel="stylesheet" href="/assets/app.css";
            }
            body {
                main class="shell" {
                    h1 { "Arawn" }
                    nav class="surfaces" aria-label="Review surfaces" {
                        @for (href, label) in SURFACES {
                            a class=(if *href == active { "surface active" } else { "surface" }) href=(href) { (label) }
                        }
                    }
                    (content)
                }
                script src="/assets/app.js" {}
            }
        }
    }
}

/// The home shell: tagline + the live regions the SSE client targets.
fn shell_markup() -> Markup {
    page(
        "/",
        html! {
            p class="tagline" { "Review & triage — served by the " code { "arawn" } " binary." }
            section id="brief-status" class="card" { "No brief update yet." }
            h2 { "Live notices" }
            ul id="notice-log" class="log" {}
        },
    )
}

/// `GET /health` — the observability dashboard, rendering the versioned
/// `status` RPC. Errors render an inline panel rather than a blank page.
pub(super) async fn health_page(State(state): State<AppState>) -> Response {
    match state.service.status().await {
        Ok(status) => axum::response::Html(health_markup(&status).into_string()).into_response(),
        Err(e) => axum::response::Html(
            page(
                "/health",
                html! { section class="panel err" { h3 { "Health unavailable" } p { (e.to_string()) } } },
            )
            .into_string(),
        )
        .into_response(),
    }
}

/// A small "available/unavailable" badge.
fn badge(ok: bool, up: &str, down: &str) -> Markup {
    html! { span class=(if ok { "badge ok" } else { "badge down" }) { (if ok { up } else { down }) } }
}

/// Class for a feed/ceremony status string: error-ish strings render red.
fn status_class(s: Option<&str>) -> &'static str {
    match s {
        Some(v) => {
            let v = v.to_lowercase();
            if v.contains("error") || v.contains("fail") || v.contains("reconnect") {
                "err"
            } else {
                "ok"
            }
        }
        None => "muted",
    }
}

/// Render the full health dashboard.
fn health_markup(s: &SystemStatus) -> Markup {
    let content = html! {
        h2 { "System health" }
        // Version-aware: a schema mismatch is surfaced but never blanks the page.
        @if s.version != arawn_service::SYSTEM_STATUS_VERSION {
            p class="banner" {
                "status schema v" (s.version)
                " — this client renders v" (arawn_service::SYSTEM_STATUS_VERSION)
                "; newer blocks may be partial."
            }
        }
        div class="panels" {
            // Feeds
            section class="panel" {
                h3 { "Feeds " (badge(s.feeds.available, "available", "unavailable")) }
                @if s.feeds.feeds.is_empty() {
                    p class="muted" { "No feeds registered." }
                } @else {
                    table {
                        thead { tr { th { "id" } th { "template" } th { "enabled" } th { "last run" } th { "status" } } }
                        tbody {
                            @for f in &s.feeds.feeds {
                                tr {
                                    td { (f.id) }
                                    td { (f.template) }
                                    td { (if f.enabled { "yes" } else { "no" }) }
                                    td { (f.last_run_at.as_deref().unwrap_or("—")) }
                                    td class=(status_class(f.last_status.as_deref())) { (f.last_status.as_deref().unwrap_or("—")) }
                                }
                            }
                        }
                    }
                }
            }
            // Ceremonies
            section class="panel" {
                h3 { "Ceremonies " (badge(s.ceremonies.available, "available", "unavailable")) }
                p class="muted" { "Pending notifications: " (s.ceremonies.pending_notifications.map(|n| n.to_string()).unwrap_or_else(|| "—".into())) }
                @if !s.ceremonies.recent_runs.is_empty() {
                    table {
                        thead { tr { th { "kind" } th { "period" } th { "outcome" } th { "ran at" } } }
                        tbody {
                            @for r in &s.ceremonies.recent_runs {
                                tr {
                                    td { (r.kind) }
                                    td { (r.period_key) }
                                    td class=(if r.outcome == "error" { "err" } else { "ok" }) {
                                        (r.outcome)
                                        @if let Some(e) = &r.error { " — " (e) }
                                    }
                                    td { (r.ran_at) }
                                }
                            }
                        }
                    }
                }
            }
            // Embedding
            section class="panel" {
                h3 { "Embedding " (badge(s.embedding.embedder_loaded, "loaded", "FTS-only")) }
                @if !s.embedding.embedder_loaded {
                    p class="muted" { "Embedder not loaded — memory search is degraded to full-text only." }
                }
                p class="muted" { "Pending: " (s.embedding.pending.map(|n| n.to_string()).unwrap_or_else(|| "—".into())) }
                @if let Some(errored) = s.embedding.errored {
                    @if errored > 0 {
                        p class="err" { "Errored backlog: " (errored) " row(s) parked after repeated embed failures." }
                    }
                }
            }
            // Extraction
            section class="panel" {
                h3 { "Extraction " (badge(s.extraction.available, "available", "unavailable")) }
                @if s.extraction.cursors.is_empty() {
                    p class="muted" { "No extraction cursors yet." }
                } @else {
                    table {
                        thead { tr { th { "lens" } th { "feed type" } th { "cursor" } } }
                        tbody {
                            @for c in &s.extraction.cursors {
                                tr { td { (c.lens) } td { (c.feed_type) } td { (c.cursor_ts.as_deref().unwrap_or("—")) } }
                            }
                        }
                    }
                }
            }
            // LLM
            section class="panel" {
                h3 {
                    "LLM"
                    @if let Some(reachable) = s.llm.engine_reachable {
                        " " (badge(reachable, "reachable", "unreachable"))
                    }
                }
                @if s.llm.clients.is_empty() {
                    p class="muted" { "No LLM clients configured." }
                } @else {
                    table {
                        thead { tr { th { "role" } th { "provider" } th { "model" } } }
                        tbody {
                            @for c in &s.llm.clients {
                                tr { td { (c.role) } td { (c.provider) } td { (c.model) } }
                            }
                        }
                    }
                }
            }
            // Steward
            section class="panel" {
                h3 { "Steward" }
                @if s.steward.recent_errors.is_empty() {
                    p class="muted" { "No recent steward errors." }
                } @else {
                    table {
                        thead { tr { th { "lens" } th { "subroutine" } th { "error" } th { "at" } } }
                        tbody {
                            @for e in &s.steward.recent_errors {
                                tr class="err" { td { (e.lens) } td { (e.subroutine) } td { (e.error) } td { (e.failed_at) } }
                            }
                        }
                    }
                }
            }
        }
    };
    page("/health", content)
}

/// Map a `ServerNotice` to the SSE fragment payload the client applies:
/// `{target, mode, html[, refresh]}`. `briefing_ready` replaces the
/// brief-status card *and* carries a `refresh` hint so an open `/brief` page
/// re-pulls its body in place (no full reload); everything else appends to the
/// live notice log.
fn notice_fragment(n: &ServerNotice) -> String {
    let briefing = n.category == "briefing_ready";
    let (target, mode) = if briefing {
        ("#brief-status", "replace")
    } else {
        ("#notice-log", "append")
    };
    let html = html! {
        span class=(format!("notice notice-{}", n.level)) {
            span class="cat" { (n.category) }
            " "
            (n.message)
        }
    }
    .into_string();
    let mut payload = serde_json::json!({ "target": target, "mode": mode, "html": html });
    if briefing {
        // "<url> <selector>": the client fetches url, then swaps that selector's
        // inner HTML into the same selector on the current page if present.
        payload["refresh"] = serde_json::json!("/brief #brief-body");
    }
    payload.to_string()
}

// ── GUI-S1: brief / ceremony-tablet surface ───────────────────────────────

/// One tablet plus its items/priorities/diary, ready to render.
struct BriefSection {
    tablet: TabletDto,
    items: Vec<ItemDto>,
    priorities: Vec<PriorityDto>,
    diary: Option<String>,
}

/// `GET /brief` — the daily/weekly brief and ceremony tablets. Refreshes in
/// place on a `briefing_ready` push (via the SSE `refresh` hint).
pub(super) async fn brief_page(State(state): State<AppState>) -> Response {
    let Some(cer) = state.service.ceremony_service() else {
        return axum::response::Html(
            page(
                "/brief",
                html! { section class="panel err" { h3 { "Brief unavailable" } p { "Ceremony engine not wired (workflow runner unavailable at boot)." } } },
            )
            .into_string(),
        )
        .into_response();
    };
    let sections = gather_brief(&cer);
    axum::response::Html(brief_markup(&sections).into_string()).into_response()
}

/// Collect the current daily (today, falling back to yesterday for timezone
/// boundaries) and weekly tablets. Missing tablets are simply omitted.
fn gather_brief(cer: &CeremonyService) -> Vec<BriefSection> {
    let now = chrono::Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let yesterday = (now - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let week = arawn_ceremonies::RetroCeremony::iso_week(now);

    let mut sections = Vec::new();
    let daily = cer
        .get_by_period("daily", &today)
        .ok()
        .flatten()
        .or_else(|| cer.get_by_period("daily", &yesterday).ok().flatten());
    if let Some(t) = daily {
        sections.push(build_section(cer, t));
    }
    if let Some(t) = cer.get_by_period("weekly", &week).ok().flatten() {
        sections.push(build_section(cer, t));
    }
    sections
}

fn build_section(cer: &CeremonyService, tablet: TabletDto) -> BriefSection {
    let items = cer.list_items(&tablet.id, None).unwrap_or_default();
    let priorities = cer.list_priorities(&tablet.id).unwrap_or_default();
    let diary = cer.get_diary(&tablet.id).ok().flatten();
    BriefSection {
        tablet,
        items,
        priorities,
        diary,
    }
}

/// Render the brief view. Pure (takes pre-fetched sections) so it's testable
/// without a ceremony service.
fn brief_markup(sections: &[BriefSection]) -> Markup {
    let content = html! {
        h2 { "Brief" }
        // Stable container the SSE `refresh` hint re-pulls in place.
        div id="brief-body" {
            @if sections.is_empty() {
                p class="muted" { "No brief yet. The daily and weekly ceremonies compose these; once one runs, it appears here." }
            } @else {
                @for s in sections { (render_section(s)) }
            }
        }
    };
    page("/brief", content)
}

fn render_section(s: &BriefSection) -> Markup {
    html! {
        section class="panel" {
            h3 {
                (cap(&s.tablet.kind)) " — " (s.tablet.period_key)
                @if s.tablet.recovered { " " span class="badge" { "recovered" } }
            }
            p class="muted" { "status: " (s.tablet.status) " · generated " (s.tablet.generated_at) }
            @if !s.priorities.is_empty() {
                h4 { "Priorities" }
                ul class="brief-list" {
                    @for p in &s.priorities {
                        li {
                            (item_text(&p.body))
                            @if p.confirmed_at.is_some() { " " span class="badge ok" { "confirmed" } }
                            @if p.done_at.is_some() { " " span class="badge" { "done" } }
                        }
                    }
                }
            }
            @for (section_key, items) in group_items(&s.items) {
                h4 { (section_label(&section_key)) }
                ul class="brief-list" {
                    @for it in items {
                        li class=(if it.done_at.is_some() { "done" } else { "" }) { (item_text(&it.body)) }
                    }
                }
            }
            @if let Some(d) = &s.diary {
                @if !d.is_empty() {
                    h4 { "Diary" }
                    p class="diary" { (d) }
                }
            }
        }
    }
}

/// Extract a human-readable line from an item/priority `body` JSON, trying the
/// common text-bearing keys before falling back to the compact JSON.
fn item_text(body: &serde_json::Value) -> String {
    for key in ["text", "summary", "headline", "title", "body", "content"] {
        if let Some(s) = body.get(key).and_then(|v| v.as_str())
            && !s.is_empty()
        {
            return s.to_string();
        }
    }
    if let Some(s) = body.as_str() {
        return s.to_string();
    }
    body.to_string()
}

/// Group items by `section_key` (ordered by section then ordinal), preserving
/// first-seen section order.
fn group_items(items: &[ItemDto]) -> Vec<(String, Vec<&ItemDto>)> {
    let mut sorted: Vec<&ItemDto> = items.iter().collect();
    sorted.sort_by(|a, b| {
        a.section_key
            .cmp(&b.section_key)
            .then(a.ordinal.cmp(&b.ordinal))
    });
    let mut out: Vec<(String, Vec<&ItemDto>)> = Vec::new();
    for it in sorted {
        match out.last_mut() {
            Some((k, v)) if *k == it.section_key => v.push(it),
            _ => out.push((it.section_key.clone(), vec![it])),
        }
    }
    out
}

/// Prettify a section key like `what_happened` → `What happened`.
fn section_label(key: &str) -> String {
    cap(&key.replace('_', " "))
}

/// Capitalize the first character of `s`.
fn cap(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// ── GUI-S2: action-item inbox ──────────────────────────────────────────────

/// `GET /inbox` — open action items as a scannable, keyboard-navigable list.
pub(super) async fn inbox_page(State(state): State<AppState>) -> Response {
    let todos = list_open_todos(&state);
    axum::response::Html(inbox_markup(&todos).into_string()).into_response()
}

/// `POST /inbox/{id}/{action}` — run a triage action (done / undo / snooze /
/// dismiss) and return the re-rendered row (empty body for dismiss, so the
/// client removes it). Mutates via the same `TodoService` the WS RPCs use, so
/// `TodoEvent`s still broadcast.
pub(super) async fn inbox_action(
    Path((id, action)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let store = state.service.shared_store();
    let Ok(guard) = store.lock() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "store lock poisoned").into_response();
    };
    let svc = arawn_storage::TodoService::new(guard.database())
        .with_events(state.service.todo_event_sender());
    match action.as_str() {
        "done" => to_row(svc.mark_done(&id)),
        "undo" => to_row(svc.undo(&id)),
        "snooze" => {
            // No dedicated snooze RPC — snooze == push the due date out a day.
            let patch = arawn_storage::TodoPatch {
                due_at: Some(chrono::Utc::now() + chrono::Duration::days(1)),
                ..Default::default()
            };
            to_row(svc.patch(&id, patch))
        }
        "dismiss" => match svc.archive(&id) {
            Ok(()) => (StatusCode::OK, "").into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        _ => (StatusCode::BAD_REQUEST, "unknown action").into_response(),
    }
}

fn to_row(res: Result<Todo, arawn_storage::StorageError>) -> Response {
    match res {
        Ok(t) => axum::response::Html(render_row(&t).into_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn list_open_todos(state: &AppState) -> Vec<Todo> {
    let store = state.service.shared_store();
    let Ok(guard) = store.lock() else {
        return Vec::new();
    };
    let svc = arawn_storage::TodoService::new(guard.database());
    let filter = arawn_storage::ListFilter {
        open_only: Some(true),
        ..Default::default()
    };
    svc.list(filter).unwrap_or_default()
}

/// Render the inbox page.
fn inbox_markup(todos: &[Todo]) -> Markup {
    let content = html! {
        h2 { "Inbox" }
        @if todos.is_empty() {
            p class="muted" { "Inbox zero — no open action items." }
        } @else {
            p class="muted" { (todos.len()) " open item(s). Keys: j/k move · x dismiss · e expand." }
            ul id="inbox-list" class="inbox" {
                @for t in todos {
                    li id=(format!("todo-{}", t.id)) class="todo-row" tabindex="0" { (render_row(t)) }
                }
            }
        }
    };
    page("/inbox", content)
}

/// Render the inner content of one inbox row (also returned by `inbox_action`
/// so the client can swap a single row in place).
fn render_row(t: &Todo) -> Markup {
    let done = t.done_at.is_some();
    html! {
        div class="todo-main" {
            span class=(if done { "todo-body done" } else { "todo-body" }) { (t.body) }
            @if let Some(lens) = &t.lens { span class="tag" { (lens) } }
            @if let Some(due) = &t.due_at { span class="tag due" { "due " (due.format("%Y-%m-%d").to_string()) } }
        }
        @if let Some(r) = &t.rationale {
            @if !r.is_empty() { p class="todo-rationale" { (r) } }
        }
        div class="todo-actions" {
            @if done {
                button type="button" data-action=(format!("/inbox/{}/undo", t.id)) { "Undo" }
            } @else {
                button type="button" data-action=(format!("/inbox/{}/done", t.id)) { "Done" }
                button type="button" data-action=(format!("/inbox/{}/snooze", t.id)) { "Snooze 1d" }
            }
            button type="button" class="danger" data-action=(format!("/inbox/{}/dismiss", t.id)) data-remove="1" { "Dismiss" }
        }
    }
}

/// Look up `path` in the embedded bundle and return it with a content-type,
/// or 404 if absent.
fn serve_embedded(path: &str) -> Response {
    match GuiAssets::get(path) {
        Some(file) => (
            [(header::CONTENT_TYPE, content_type_for(path))],
            file.data.into_owned(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// Minimal extension → MIME map covering the asset kinds a hypermedia GUI
/// ships. Unknown extensions fall back to a safe binary type.
fn content_type_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(category: &str) -> ServerNotice {
        ServerNotice {
            level: "info".into(),
            category: category.into(),
            message: "hello world".into(),
            timestamp: "2026-06-13T00:00:00Z".into(),
        }
    }

    #[test]
    fn assets_are_embedded() {
        assert!(GuiAssets::get("assets/app.css").is_some());
        assert!(GuiAssets::get("assets/app.js").is_some());
        assert!(GuiAssets::get("definitely-missing.xyz").is_none());
    }

    #[test]
    fn serve_css_is_200_with_type() {
        let resp = serve_embedded("assets/app.css");
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/css"), "content-type was {ct}");
    }

    #[test]
    fn serve_missing_is_404() {
        assert_eq!(
            serve_embedded("assets/nope.css").status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn content_type_map() {
        assert_eq!(content_type_for("a/b.css"), "text/css; charset=utf-8");
        assert_eq!(content_type_for("x.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type_for("noext"), "application/octet-stream");
    }

    #[test]
    fn shell_has_nav_and_live_regions() {
        let h = shell_markup().into_string();
        for (_, label) in SURFACES {
            assert!(h.contains(label), "shell nav missing {label}");
        }
        // Live regions the SSE client targets, plus the client script.
        assert!(h.contains("id=\"brief-status\""));
        assert!(h.contains("id=\"notice-log\""));
        assert!(h.contains("/assets/app.js"));
    }

    #[test]
    fn briefing_ready_replaces_brief_status() {
        let frag = notice_fragment(&notice("briefing_ready"));
        let v: serde_json::Value = serde_json::from_str(&frag).unwrap();
        assert_eq!(v["target"], "#brief-status");
        assert_eq!(v["mode"], "replace");
        assert!(v["html"].as_str().unwrap().contains("hello world"));
    }

    #[test]
    fn generic_notice_appends_to_log() {
        let frag = notice_fragment(&notice("feed_event"));
        let v: serde_json::Value = serde_json::from_str(&frag).unwrap();
        assert_eq!(v["target"], "#notice-log");
        assert_eq!(v["mode"], "append");
        assert!(v["html"].as_str().unwrap().contains("feed_event"));
    }

    #[tokio::test]
    async fn broadcast_notice_is_forwarded_as_sse_event() {
        // Proves the bridge wiring: a notice put on the broadcast channel
        // surfaces as one SSE event on the stream `events` serves. (Fragment
        // *content* is covered by the notice_fragment tests above; axum's
        // `Event` keeps its payload private.)
        let (tx, rx) = tokio::sync::broadcast::channel(8);
        let mut stream = std::pin::pin!(notice_event_stream(rx));
        tx.send(notice("briefing_ready")).unwrap();
        let item = stream.next().await;
        assert!(
            item.is_some(),
            "a broadcast notice should yield an SSE event"
        );
        assert!(item.unwrap().is_ok());
    }

    // ── GUI-S3: health dashboard ──────────────────────────────────────────

    fn sample_status() -> SystemStatus {
        use arawn_service::*;
        SystemStatus {
            version: SYSTEM_STATUS_VERSION,
            feeds: FeedsStatus {
                available: true,
                feeds: vec![FeedStatusRow {
                    id: "feed-1".into(),
                    template: "gmail/inbox".into(),
                    enabled: true,
                    last_run_at: Some("2026-06-13T00:00:00Z".into()),
                    last_status: Some("ok".into()),
                }],
            },
            ceremonies: CeremoniesStatus {
                available: true,
                pending_notifications: Some(2),
                recent_runs: vec![CeremonyRunStatus {
                    kind: "daily".into(),
                    period_key: "2026-06-13".into(),
                    outcome: "ok".into(),
                    error: None,
                    ran_at: "2026-06-13T06:00:00Z".into(),
                }],
            },
            embedding: EmbeddingStatus {
                embedder_loaded: true,
                pending: Some(0),
                errored: Some(0),
            },
            extraction: ExtractionStatus {
                available: true,
                cursors: vec![],
            },
            llm: LlmStatus {
                clients: vec![LlmClientStatus {
                    role: "engine".into(),
                    provider: "groq".into(),
                    model: "openai/gpt-oss-120b".into(),
                }],
                engine_reachable: None,
            },
            steward: StewardStatus {
                recent_errors: vec![],
            },
        }
    }

    #[test]
    fn health_renders_all_subsystem_panels() {
        let h = health_markup(&sample_status()).into_string();
        for panel in [
            "Feeds",
            "Ceremonies",
            "Embedding",
            "Extraction",
            "LLM",
            "Steward",
        ] {
            assert!(h.contains(panel), "health page missing {panel} panel");
        }
        assert!(h.contains("gmail/inbox"));
        assert!(h.contains("openai/gpt-oss-120b"));
        assert!(h.contains("surface active"));
    }

    #[test]
    fn health_marks_degraded_and_errored_states() {
        let mut s = sample_status();
        s.embedding.embedder_loaded = false;
        s.embedding.errored = Some(7);
        let h = health_markup(&s).into_string();
        assert!(h.contains("FTS-only") || h.contains("degraded"));
        assert!(
            h.contains("Errored backlog"),
            "errored backlog must surface"
        );
        assert!(h.contains('7'));
    }

    #[test]
    fn health_error_outcome_renders_red_class() {
        let mut s = sample_status();
        s.ceremonies.recent_runs[0].outcome = "error".into();
        s.ceremonies.recent_runs[0].error = Some("compose failed".into());
        let h = health_markup(&s).into_string();
        assert!(h.contains("compose failed"));
        assert!(h.contains("class=\"err\""));
    }

    #[test]
    fn health_version_mismatch_shows_banner_but_still_renders() {
        let mut s = sample_status();
        s.version = 999;
        let h = health_markup(&s).into_string();
        assert!(h.contains("status schema v999"), "version banner missing");
        assert!(h.contains("Feeds"));
    }

    #[test]
    fn status_class_flags_errors() {
        assert_eq!(status_class(Some("ok")), "ok");
        assert_eq!(status_class(Some("reconnect needed: auth")), "err");
        assert_eq!(status_class(Some("error: boom")), "err");
        assert_eq!(status_class(None), "muted");
    }

    // ── GUI-S1: brief surface ─────────────────────────────────────────────

    fn sample_section() -> BriefSection {
        BriefSection {
            tablet: TabletDto {
                id: "t-daily-1".into(),
                kind: "daily".into(),
                period_key: "2026-06-13".into(),
                generated_at: "2026-06-13T06:00:00Z".into(),
                status: "open".into(),
                lenses_scanned: serde_json::json!(["work"]),
                priorities_confirmed_at: None,
                recovered: false,
            },
            items: vec![
                ItemDto {
                    id: "i2".into(),
                    tablet_id: "t-daily-1".into(),
                    section_key: "what_happened".into(),
                    ordinal: 1,
                    kind: "composed".into(),
                    body: serde_json::json!({ "text": "Shipped the parser fix" }),
                    citation_id: Some("sig-1".into()),
                    done_at: None,
                    created_at: "2026-06-13T06:00:00Z".into(),
                },
                ItemDto {
                    id: "i1".into(),
                    tablet_id: "t-daily-1".into(),
                    section_key: "what_happened".into(),
                    ordinal: 0,
                    kind: "composed".into(),
                    body: serde_json::json!({ "text": "Reviewed three PRs" }),
                    citation_id: Some("sig-2".into()),
                    done_at: Some("2026-06-13T07:00:00Z".into()),
                    created_at: "2026-06-13T06:00:00Z".into(),
                },
            ],
            priorities: vec![PriorityDto {
                id: "p1".into(),
                tablet_id: "t-daily-1".into(),
                body: serde_json::json!({ "text": "Finish the GUI brief surface" }),
                rationale: "in flight".into(),
                citation_id: None,
                confirmed_at: Some("2026-06-13T06:30:00Z".into()),
                done_at: None,
                ordinal: 0,
                source: "confirmed".into(),
            }],
            diary: Some("Felt productive.".into()),
        }
    }

    #[test]
    fn brief_empty_shows_placeholder() {
        let h = brief_markup(&[]).into_string();
        assert!(h.contains("id=\"brief-body\""));
        assert!(h.contains("No brief yet"));
        assert!(h.contains("surface active")); // /brief nav highlighted
    }

    #[test]
    fn brief_renders_tablet_priorities_items_diary() {
        let h = brief_markup(&[sample_section()]).into_string();
        assert!(h.contains("Daily — 2026-06-13"), "header missing: {h}");
        assert!(h.contains("Finish the GUI brief surface")); // priority
        assert!(h.contains("What happened")); // section label prettified
        assert!(h.contains("Shipped the parser fix")); // item
        assert!(h.contains("Felt productive.")); // diary
        assert!(h.contains("badge ok")); // confirmed priority badge
    }

    #[test]
    fn brief_items_grouped_and_ordered() {
        let section = sample_section();
        let groups = group_items(&section.items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "what_happened");
        // Ordered by ordinal: i1 (0) before i2 (1).
        assert_eq!(groups[0].1[0].id, "i1");
        assert_eq!(groups[0].1[1].id, "i2");
    }

    #[test]
    fn item_text_prefers_text_keys_then_falls_back() {
        assert_eq!(item_text(&serde_json::json!({ "text": "hi" })), "hi");
        assert_eq!(item_text(&serde_json::json!({ "summary": "sum" })), "sum");
        assert_eq!(item_text(&serde_json::json!("raw string")), "raw string");
        // No known key → compact JSON fallback (non-empty).
        let fb = item_text(&serde_json::json!({ "x": 1 }));
        assert!(fb.contains("\"x\""));
    }

    #[test]
    fn section_label_prettifies() {
        assert_eq!(section_label("what_happened"), "What happened");
        assert_eq!(section_label("priorities"), "Priorities");
    }

    #[test]
    fn briefing_ready_fragment_carries_refresh_hint() {
        let v: serde_json::Value =
            serde_json::from_str(&notice_fragment(&notice("briefing_ready"))).unwrap();
        assert_eq!(v["refresh"], "/brief #brief-body");
        // Generic notices carry no refresh hint.
        let g: serde_json::Value =
            serde_json::from_str(&notice_fragment(&notice("feed_event"))).unwrap();
        assert!(g.get("refresh").is_none());
    }

    // ── GUI-S2: inbox ─────────────────────────────────────────────────────

    fn todo(id: &str, done: bool) -> Todo {
        Todo {
            id: id.into(),
            body: "Review PR #42".into(),
            rationale: Some("blocking the release".into()),
            kind: "task".into(),
            lens: Some("work".into()),
            created_at: chrono::Utc::now(),
            due_at: None,
            done_at: if done { Some(chrono::Utc::now()) } else { None },
            archived_at: None,
            attrs: serde_json::json!({}),
        }
    }

    #[test]
    fn inbox_empty_shows_inbox_zero() {
        let h = inbox_markup(&[]).into_string();
        assert!(h.contains("Inbox zero"));
        assert!(h.contains("surface active")); // /inbox nav highlighted
    }

    #[test]
    fn inbox_renders_rows_with_actions_and_keyboard_hint() {
        let h = inbox_markup(&[todo("t1", false)]).into_string();
        assert!(h.contains("Review PR #42"));
        assert!(h.contains("id=\"todo-t1\""));
        assert!(h.contains("/inbox/t1/done"));
        assert!(h.contains("/inbox/t1/snooze"));
        assert!(h.contains("/inbox/t1/dismiss"));
        assert!(h.contains("blocking the release")); // rationale present (CSS-hidden)
        assert!(h.contains("j/k move")); // keyboard hint
    }

    #[test]
    fn done_row_shows_undo_not_done() {
        let h = render_row(&todo("t9", true)).into_string();
        assert!(h.contains("/inbox/t9/undo"));
        assert!(!h.contains("/inbox/t9/done"));
        assert!(h.contains("todo-body done")); // struck-through styling hook
    }
}

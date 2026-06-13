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

use arawn_service::{ArawnService, ServerNotice, SystemStatus};

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
/// `{target, mode, html}`. `briefing_ready` replaces the brief-status card;
/// everything else appends to the live notice log.
fn notice_fragment(n: &ServerNotice) -> String {
    let (target, mode) = if n.category == "briefing_ready" {
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
    serde_json::json!({ "target": target, "mode": mode, "html": html }).to_string()
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
}

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

use arawn_service::ServerNotice;

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

/// Render the base layout shell with nav + the live regions the SSE client
/// targets.
fn shell_markup() -> Markup {
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
                    p class="tagline" { "Review & triage — served by the " code { "arawn" } " binary." }
                    nav class="surfaces" aria-label="Review surfaces" {
                        @for (href, label) in SURFACES {
                            a class="surface" href=(href) { (label) }
                        }
                    }
                    section id="brief-status" class="card" { "No brief update yet." }
                    h2 { "Live notices" }
                    ul id="notice-log" class="log" {}
                }
                script src="/assets/app.js" {}
            }
        }
    }
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
}

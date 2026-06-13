//! Web GUI asset serving (ARAWN-T-0491 / GUI-F1, initiative ARAWN-I-0070).
//!
//! Serves the build-time-embedded web GUI bundle from the *same* axum router
//! and listener as the WS-RPC server, so the single-binary / ARM64 story is
//! preserved (no separate static-file server, no runtime dependency).
//!
//! The bundle lives in `crates/arawn/web/dist/`. `rust-embed` reads it from
//! disk in debug builds (edit + reload an asset without recompiling) and bakes
//! it into the binary in release builds. Content-types are computed from the
//! file extension here rather than pulling an extra feature/crate.

use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct GuiAssets;

/// `GET /` — serve the GUI index page.
pub(super) async fn gui_index() -> Response {
    serve_embedded("index.html")
}

/// `GET /assets/{*path}` — serve a static asset from the embedded bundle.
pub(super) async fn gui_asset(Path(path): Path<String>) -> Response {
    serve_embedded(&format!("assets/{path}"))
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

    #[test]
    fn index_and_known_asset_are_embedded() {
        assert!(
            GuiAssets::get("index.html").is_some(),
            "index.html must be in the bundle"
        );
        assert!(
            GuiAssets::get("assets/app.css").is_some(),
            "assets/app.css must be in the bundle"
        );
        assert!(GuiAssets::get("definitely-missing.xyz").is_none());
    }

    #[test]
    fn serve_index_is_200_html() {
        let resp = serve_embedded("index.html");
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/html"), "content-type was {ct}");
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
}

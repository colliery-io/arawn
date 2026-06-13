//! WS `Origin` allowlisting + the CORS origin set for the web GUI
//! (ARAWN-T-0492 / GUI-G2, initiative ARAWN-I-0070).
//!
//! A browser page can open a WebSocket to *any* origin without CORS stopping
//! it — so without an `Origin` check, a malicious site the user happens to
//! visit could connect to their localhost `arawn` and drive the agent
//! (cross-site WebSocket hijacking). We validate `Origin` against an
//! allowlist. Crucially, **non-browser clients (the TUI via tungstenite) send
//! no `Origin`** and must keep working — so a *missing* Origin is allowed; only
//! a *present-and-disallowed* Origin is rejected.

use axum::http::{HeaderMap, header};

/// Build the effective origin allowlist: loopback defaults derived from the
/// bind `host:port`, plus any operator-configured extras. All lowercased.
/// v1 is plain HTTP (TLS is GUI-G5), so origins are `http://…`.
pub(super) fn build_allowlist(host: &str, port: u16, extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for h in ["localhost", "127.0.0.1", "[::1]"] {
        out.push(format!("http://{h}:{port}"));
    }
    // If bound to a concrete non-loopback host (a LAN IP/name), allow it too —
    // but not the wildcard binds, which name no single browsable origin.
    if !super::is_loopback_host(host) && !host.is_empty() && host != "0.0.0.0" && host != "::" {
        out.push(format!("http://{host}:{port}"));
    }
    out.extend(extra.iter().cloned());
    out.iter().map(|s| s.to_lowercase()).collect()
}

/// Outcome of checking a request's `Origin` header.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum OriginCheck {
    /// No `Origin` header — a non-browser client (e.g. the TUI). Allowed.
    Absent,
    /// `Origin` present and in the allowlist.
    Allowed,
    /// `Origin` present but not allowed — reject.
    Denied,
}

/// Check a request's `Origin` against the allowlist.
pub(super) fn check_origin(headers: &HeaderMap, allowlist: &[String]) -> OriginCheck {
    match headers.get(header::ORIGIN) {
        None => OriginCheck::Absent,
        Some(value) => {
            let origin = value.to_str().unwrap_or("").to_lowercase();
            // A real browser always sends a well-formed origin; an empty or
            // unreadable one present on the request is treated as disallowed.
            if !origin.is_empty() && allowlist.iter().any(|a| a == &origin) {
                OriginCheck::Allowed
            } else {
                OriginCheck::Denied
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_origin(origin: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::ORIGIN, origin.parse().unwrap());
        h
    }

    #[test]
    fn allowlist_includes_loopback_for_port() {
        let list = build_allowlist("127.0.0.1", 3100, &[]);
        assert!(list.contains(&"http://localhost:3100".to_string()));
        assert!(list.contains(&"http://127.0.0.1:3100".to_string()));
        assert!(list.contains(&"http://[::1]:3100".to_string()));
    }

    #[test]
    fn allowlist_adds_nonloopback_host_and_extras() {
        let list = build_allowlist("192.168.1.10", 3100, &["http://my.box:9000".into()]);
        assert!(list.contains(&"http://192.168.1.10:3100".to_string()));
        assert!(list.contains(&"http://my.box:9000".to_string()));
    }

    #[test]
    fn wildcard_bind_yields_no_wildcard_origin() {
        let list = build_allowlist("0.0.0.0", 3100, &[]);
        // Still has loopback, but no "http://0.0.0.0:3100".
        assert!(!list.iter().any(|o| o.contains("0.0.0.0")));
        assert!(list.contains(&"http://127.0.0.1:3100".to_string()));
    }

    #[test]
    fn absent_origin_is_allowed_for_non_browser_clients() {
        // The TUI sends no Origin — must not be rejected.
        assert_eq!(check_origin(&HeaderMap::new(), &[]), OriginCheck::Absent);
    }

    #[test]
    fn present_allowed_origin_passes() {
        let list = build_allowlist("127.0.0.1", 3100, &[]);
        assert_eq!(
            check_origin(&headers_with_origin("http://localhost:3100"), &list),
            OriginCheck::Allowed
        );
    }

    #[test]
    fn present_disallowed_origin_is_denied() {
        let list = build_allowlist("127.0.0.1", 3100, &[]);
        assert_eq!(
            check_origin(&headers_with_origin("http://evil.example.com"), &list),
            OriginCheck::Denied
        );
        // Right host, wrong port is also denied.
        assert_eq!(
            check_origin(&headers_with_origin("http://localhost:9999"), &list),
            OriginCheck::Denied
        );
    }

    #[test]
    fn origin_match_is_case_insensitive() {
        let list = build_allowlist("127.0.0.1", 3100, &[]);
        assert_eq!(
            check_origin(&headers_with_origin("HTTP://LocalHost:3100"), &list),
            OriginCheck::Allowed
        );
    }
}

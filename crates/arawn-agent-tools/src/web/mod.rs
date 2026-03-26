//! Web tools for fetching and searching the internet.
//!
//! Provides tools for web search and URL fetching.

mod fetch;
mod search;

#[cfg(test)]
mod tests;

use std::net::IpAddr;
use url::Url;

pub use fetch::{WebFetchConfig, WebFetchTool};
pub use search::{SearchProvider, SearchResult, WebSearchConfig, WebSearchTool};

/// Check if an IP address is private, loopback, link-local, or otherwise
/// internal. Used to prevent SSRF attacks where the agent could be tricked
/// into fetching internal resources or cloud metadata endpoints.
pub(crate) fn is_restricted_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()              // 127.0.0.0/8
                || v4.is_private()        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || v4.is_link_local()     // 169.254.0.0/16 (includes cloud metadata)
                || v4.is_broadcast()      // 255.255.255.255
                || v4.is_unspecified()    // 0.0.0.0
                || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64 // 100.64.0.0/10 (CGNAT / Tailscale)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()              // ::1
                || v6.is_unspecified()    // ::
                // fd00::/8 — unique local addresses
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // fe80::/10 — link-local
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Resolve a URL's hostname and check that none of its IP addresses are
/// restricted. Returns an error message if the URL targets a restricted IP.
pub(crate) async fn validate_url_not_ssrf(url: &Url) -> std::result::Result<(), String> {
    let host = match url.host_str() {
        Some(h) => h,
        None => return Err("URL has no host".to_string()),
    };

    // If the host is a raw IP literal, check it directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_restricted_ip(&ip) {
            return Err(format!(
                "Blocked: URL resolves to restricted IP address {}",
                ip
            ));
        }
        return Ok(());
    }

    // Otherwise resolve the hostname and check all returned addresses
    let port = url.port_or_known_default().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    match tokio::net::lookup_host(&addr).await {
        Ok(addrs) => {
            for sock_addr in addrs {
                if is_restricted_ip(&sock_addr.ip()) {
                    return Err(format!(
                        "Blocked: hostname '{}' resolves to restricted IP address {}",
                        host,
                        sock_addr.ip()
                    ));
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to resolve hostname '{}': {}", host, e)),
    }
}

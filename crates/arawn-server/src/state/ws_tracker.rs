//! WebSocket connection rate limiting per IP address.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::sync::RwLock;

/// Sliding window duration for WebSocket rate limiting.
const WS_RATE_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

/// Tracks WebSocket connection attempts per IP address.
#[derive(Debug, Clone)]
pub struct WsConnectionTracker {
    /// Connection timestamps per IP (sliding window).
    connections: Arc<RwLock<HashMap<IpAddr, Vec<Instant>>>>,
}

impl WsConnectionTracker {
    /// Create a new connection tracker.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a new connection from this IP should be allowed.
    ///
    /// Returns `Ok(())` if allowed, `Err(Response)` if rate limited.
    /// Also cleans up old entries.
    pub async fn check_rate(&self, ip: IpAddr, max_per_minute: u32) -> Result<(), Response> {
        let now = Instant::now();
        let window = WS_RATE_WINDOW;
        let cutoff = now - window;

        let mut connections = self.connections.write().await;

        // Get or create entry for this IP
        let timestamps = connections.entry(ip).or_insert_with(Vec::new);

        // Remove old timestamps outside the window
        timestamps.retain(|&t| t > cutoff);

        // Clean up IPs with no recent connections to prevent unbounded HashMap growth
        if timestamps.is_empty() {
            connections.remove(&ip);
            // Also periodically purge other stale entries
            connections.retain(|_, ts| !ts.is_empty());
        }

        // Re-get entry (may have been removed above)
        let timestamps = connections.entry(ip).or_insert_with(Vec::new);

        // Check rate
        if timestamps.len() >= max_per_minute as usize {
            tracing::warn!(
                ip = %ip,
                count = timestamps.len(),
                limit = max_per_minute,
                "WebSocket connection rate limit exceeded"
            );
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "WebSocket connection rate limit exceeded",
            )
                .into_response());
        }

        // Record this connection
        timestamps.push(now);

        Ok(())
    }

    /// Cleanup old entries from all IPs.
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let window = WS_RATE_WINDOW;
        let cutoff = now - window;

        let mut connections = self.connections.write().await;

        // Remove old timestamps and empty entries
        connections.retain(|_, timestamps| {
            timestamps.retain(|&t| t > cutoff);
            !timestamps.is_empty()
        });
    }
}

impl Default for WsConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

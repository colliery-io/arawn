//! Configuration traits for decoupled config passing between crates.
//!
//! These traits allow components to depend on configuration capabilities without
//! requiring direct knowledge of the full configuration structure. Each trait
//! represents a specific configuration capability.

use std::time::Duration;

/// Base trait for all configuration types.
///
/// Provides common functionality expected of all config types. Implementations
/// should be cheaply cloneable and thread-safe.
pub trait ConfigProvider: Clone + Send + Sync + 'static {}

/// Session management configuration.
///
/// Provides settings for session cache behavior including LRU eviction
/// and cleanup intervals.
pub trait HasSessionConfig: ConfigProvider {
    /// Maximum number of sessions to keep in cache before LRU eviction.
    fn max_sessions(&self) -> usize;

    /// Interval between cleanup runs for expired sessions.
    fn cleanup_interval(&self) -> Duration;

    /// Optional TTL for sessions (None = no expiry).
    fn session_ttl(&self) -> Option<Duration> {
        None
    }
}

/// Tool execution configuration.
///
/// Provides settings for tool execution limits and timeouts.
pub trait HasToolConfig: ConfigProvider {
    /// Timeout for shell command execution.
    fn shell_timeout(&self) -> Duration;

    /// Timeout for web/HTTP requests.
    fn web_timeout(&self) -> Duration;

    /// Maximum size of tool output in bytes before truncation.
    fn max_output_bytes(&self) -> usize;
}

/// Agent execution configuration.
///
/// Provides settings for agent behavior and limits.
pub trait HasAgentConfig: ConfigProvider {
    /// Maximum iterations for agent tool loops.
    fn max_iterations(&self) -> u32;

    /// Default timeout for agent operations.
    fn default_timeout(&self) -> Duration {
        Duration::from_secs(300) // 5 minutes
    }
}

/// Rate limiting configuration.
///
/// Provides settings for request rate limiting.
pub trait HasRateLimitConfig: ConfigProvider {
    /// Whether rate limiting is enabled.
    fn rate_limiting_enabled(&self) -> bool;

    /// Requests per minute per client.
    fn requests_per_minute(&self) -> u32;

    /// Burst allowance above steady rate.
    fn burst_size(&self) -> u32 {
        10
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Default implementations for common types
// ─────────────────────────────────────────────────────────────────────────────

/// Default configuration values used across the system.
///
/// These constants are the fallback values when no config file or explicit
/// override is provided. They can be overridden in `config.toml` under
/// their respective sections (e.g., `[server]`, `[agent.default]`, `[tools]`).
pub mod defaults {
    use std::time::Duration;

    // ── Session / Cache ──────────────────────────────────────────────
    /// Maximum number of sessions held in the in-memory cache.
    /// Override: `[session] max_sessions`
    pub const MAX_SESSIONS: usize = 10_000;
    /// Interval between session timeout sweeps (seconds).
    /// Override: `[session] cleanup_interval_secs`
    pub const CLEANUP_INTERVAL_SECS: u64 = 60;

    // ── Tool Execution ───────────────────────────────────────────────
    /// Shell/bash command timeout (seconds). Commands running longer are killed.
    /// Override: `[tools.shell] timeout_secs`
    pub const SHELL_TIMEOUT_SECS: u64 = 30;
    /// HTTP fetch timeout for web_fetch tool (seconds).
    /// Override: `[tools.web] timeout_secs`
    pub const WEB_TIMEOUT_SECS: u64 = 30;
    /// Global default max tool output size before truncation (bytes, 100KB).
    /// Per-tool overrides: shell=100KB, file_read=500KB, web_fetch=200KB, search=50KB.
    /// Override: `[tools.output] max_size_bytes`
    pub const MAX_OUTPUT_BYTES: usize = 102_400;

    // ── Agent Loop ───────────────────────────────────────────────────
    /// Maximum tool-call iterations per agent turn before the turn is truncated.
    /// The orchestrator (RLM) intentionally uses 1 to regain control after each tool call.
    /// Override: `[agent.default] max_iterations`
    pub const MAX_ITERATIONS: u32 = 25;

    // ── Rate Limiting ────────────────────────────────────────────────
    /// LLM API requests per minute (per-session token bucket).
    /// Override: `[server] api_rpm`
    pub const REQUESTS_PER_MINUTE: u32 = 120;
    /// Token bucket burst allowance above the per-minute rate.
    pub const BURST_SIZE: u32 = 10;

    // ── Server ───────────────────────────────────────────────────────
    /// Default HTTP server port.
    /// Override: `[server] port`
    pub const DEFAULT_PORT: u16 = 8080;
    /// Default bind address (localhost only).
    /// Override: `[server] bind`
    pub const DEFAULT_BIND: &str = "127.0.0.1";

    // ── Context Management ───────────────────────────────────────────
    /// Context usage warning threshold (percentage of max_context_tokens).
    /// When exceeded, the agent logs a warning and may request compaction.
    pub const CONTEXT_WARNING_PERCENT: u8 = 70;
    /// Context usage critical threshold (percentage of max_context_tokens).
    /// When exceeded, compaction is forced to avoid exceeding the model's window.
    pub const CONTEXT_CRITICAL_PERCENT: u8 = 90;

    pub fn cleanup_interval() -> Duration {
        Duration::from_secs(CLEANUP_INTERVAL_SECS)
    }

    pub fn shell_timeout() -> Duration {
        Duration::from_secs(SHELL_TIMEOUT_SECS)
    }

    pub fn web_timeout() -> Duration {
        Duration::from_secs(WEB_TIMEOUT_SECS)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Simple wrapper types for standalone config passing
// ─────────────────────────────────────────────────────────────────────────────

/// Standalone session configuration.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_types::config::{SessionConfigProvider, HasSessionConfig};
///
/// let config = SessionConfigProvider::default();
/// assert_eq!(config.max_sessions(), 10_000);
/// assert!(config.session_ttl().is_none());
/// ```
#[derive(Debug, Clone)]
pub struct SessionConfigProvider {
    pub max_sessions: usize,
    pub cleanup_interval: Duration,
    pub session_ttl: Option<Duration>,
}

impl Default for SessionConfigProvider {
    fn default() -> Self {
        Self {
            max_sessions: defaults::MAX_SESSIONS,
            cleanup_interval: defaults::cleanup_interval(),
            session_ttl: None,
        }
    }
}

impl ConfigProvider for SessionConfigProvider {}

impl HasSessionConfig for SessionConfigProvider {
    fn max_sessions(&self) -> usize {
        self.max_sessions
    }

    fn cleanup_interval(&self) -> Duration {
        self.cleanup_interval
    }

    fn session_ttl(&self) -> Option<Duration> {
        self.session_ttl
    }
}

/// Standalone tool configuration.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_types::config::{ToolConfigProvider, HasToolConfig};
/// use std::time::Duration;
///
/// let config = ToolConfigProvider {
///     shell_timeout: Duration::from_secs(60),
///     web_timeout: Duration::from_secs(15),
///     max_output_bytes: 50_000,
/// };
/// assert_eq!(config.shell_timeout(), Duration::from_secs(60));
/// ```
#[derive(Debug, Clone)]
pub struct ToolConfigProvider {
    pub shell_timeout: Duration,
    pub web_timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for ToolConfigProvider {
    fn default() -> Self {
        Self {
            shell_timeout: defaults::shell_timeout(),
            web_timeout: defaults::web_timeout(),
            max_output_bytes: defaults::MAX_OUTPUT_BYTES,
        }
    }
}

impl ConfigProvider for ToolConfigProvider {}

impl HasToolConfig for ToolConfigProvider {
    fn shell_timeout(&self) -> Duration {
        self.shell_timeout
    }

    fn web_timeout(&self) -> Duration {
        self.web_timeout
    }

    fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }
}

/// Standalone agent configuration.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_types::config::{AgentConfigProvider, HasAgentConfig};
///
/// let config = AgentConfigProvider::default();
/// assert_eq!(config.max_iterations(), 25);
/// assert_eq!(config.default_timeout().as_secs(), 300);
/// ```
#[derive(Debug, Clone)]
pub struct AgentConfigProvider {
    pub max_iterations: u32,
    pub default_timeout: Duration,
}

impl Default for AgentConfigProvider {
    fn default() -> Self {
        Self {
            max_iterations: defaults::MAX_ITERATIONS,
            default_timeout: Duration::from_secs(300),
        }
    }
}

impl ConfigProvider for AgentConfigProvider {}

impl HasAgentConfig for AgentConfigProvider {
    fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    fn default_timeout(&self) -> Duration {
        self.default_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfigProvider::default();
        assert_eq!(config.max_sessions(), defaults::MAX_SESSIONS);
        assert_eq!(config.cleanup_interval(), defaults::cleanup_interval());
        assert!(config.session_ttl().is_none());
    }

    #[test]
    fn test_tool_config_defaults() {
        let config = ToolConfigProvider::default();
        assert_eq!(config.shell_timeout(), defaults::shell_timeout());
        assert_eq!(config.web_timeout(), defaults::web_timeout());
        assert_eq!(config.max_output_bytes(), defaults::MAX_OUTPUT_BYTES);
    }

    #[test]
    fn test_agent_config_defaults() {
        let config = AgentConfigProvider::default();
        assert_eq!(config.max_iterations(), defaults::MAX_ITERATIONS);
    }

    #[test]
    fn test_custom_session_config() {
        let config = SessionConfigProvider {
            max_sessions: 5000,
            cleanup_interval: Duration::from_secs(120),
            session_ttl: Some(Duration::from_secs(3600)),
        };
        assert_eq!(config.max_sessions(), 5000);
        assert_eq!(config.cleanup_interval(), Duration::from_secs(120));
        assert_eq!(config.session_ttl(), Some(Duration::from_secs(3600)));
    }
}

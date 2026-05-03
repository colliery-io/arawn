use thiserror::Error;

/// Errors surfaced by the integration layer. Wraps `AuthError` from
/// `arawn-auth` so the OAuth + token-storage primitives feed in cleanly,
/// and adds integration-specific variants.
#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("integration '{0}' is not registered")]
    UnknownService(String),

    #[error("integration '{0}' is not connected — run /connect {0} first")]
    NotConnected(String),

    #[error("authentication: {0}")]
    Auth(#[from] arawn_auth::AuthError),

    #[error("credential I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("credential format: {0}")]
    Format(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("OAuth flow cancelled by user")]
    Cancelled,
}

impl IntegrationError {
    /// User-facing one-liner suitable for the engine error chain (T-0191).
    pub fn user_message(&self) -> String {
        match self {
            IntegrationError::UnknownService(s) => {
                format!("No integration named '{s}' is registered. Run /integrations to see what's available.")
            }
            IntegrationError::NotConnected(s) => {
                format!("Integration '{s}' is not connected. Run /connect {s} to set it up.")
            }
            IntegrationError::Auth(e) => format!("Authentication error: {e}"),
            IntegrationError::Io(e) => format!("Credential storage error: {e}"),
            IntegrationError::Format(msg) => format!("Credential format error: {msg}"),
            IntegrationError::Provider(msg) => format!("Provider error: {msg}"),
            IntegrationError::Cancelled => "OAuth flow cancelled.".to_string(),
        }
    }
}

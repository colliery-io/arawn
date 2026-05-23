use thiserror::Error;

/// Errors raised by the auth primitives.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The stored credential is no longer valid (refresh failed or revoked).
    #[error("authentication expired or invalid (re-run `arawn setup`)")]
    AuthExpired,

    /// Provider returned a non-success HTTP status.
    #[error("provider API error ({status}): {body}")]
    ApiError { status: u16, body: String },

    /// Network-level failure (DNS, TLS, connection refused, etc.).
    #[error("network error: {0}")]
    Network(String),

    /// Configuration or protocol shape is malformed (bad redirect URL,
    /// missing params, CSRF mismatch, key length, etc.).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Provider response or on-disk token couldn't be parsed / decrypted.
    #[error("failed to decode: {0}")]
    Decode(String),
}

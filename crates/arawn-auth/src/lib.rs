//! OAuth2 PKCE + encrypted token storage for arawn.
//!
//! Provides a provider-agnostic OAuth2 client (`OAuthClient`), a local
//! single-shot HTTP callback listener (`CallbackServer`), and an
//! encrypted-at-rest token store (`TokenStore`). Together these implement
//! the full "one-time setup" flow: open a browser, wait for consent,
//! exchange code for token, persist it safely.
//!
//! The crate is deliberately narrow — it owns the auth primitives and
//! nothing else. Higher-level integration/provider glue lives elsewhere.

pub mod error;
pub mod oauth2;
pub mod server;
pub mod token_store;

pub use error::AuthError;
pub use oauth2::{AuthRequest, OAuthClient, OAuthProviderConfig, Token};
pub use server::{CallbackResult, CallbackServer};
pub use token_store::TokenStore;

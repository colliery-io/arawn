//! The GraphQL documents arawn sends to Linear. One named operation per
//! const, so a change in Linear's schema shows up as a diff here
//! (NFR-002) and in the request bodies the tests check.

/// The key's user and workspace. `arawn setup linear` validates a key
/// with it before saving.
pub const VIEWER: &str = "query Viewer { viewer { id name email organization { name } } }";

//! Secrets management — storage and retrieval via age-encrypted store.
//!
//! Resolution for `api_key_ref`:
//! 1. Check secrets store (name lowercased)
//! 2. Check environment variable (as-is, then uppercased)

/// Result of API key resolution with provenance.
///
/// The `Debug` impl intentionally redacts `value` to prevent secret leakage
/// in log output. Use `.value` directly when the actual secret is needed.
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedSecret {
    /// The secret value.
    pub value: String,
    /// Where the secret was found.
    pub source: SecretSource,
}

impl std::fmt::Debug for ResolvedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedSecret")
            .field("value", &"[REDACTED]")
            .field("source", &self.source)
            .finish()
    }
}

/// Where a secret was resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretSource {
    /// Age-encrypted secret store.
    AgeStore,
    /// Environment variable.
    EnvVar(String),
}

impl std::fmt::Display for SecretSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretSource::AgeStore => write!(f, "secret store"),
            SecretSource::EnvVar(var) => write!(f, "env var {}", var),
        }
    }
}

/// Resolve an API key by reference name.
///
/// 1. Lowercase the name → check secrets store
/// 2. Check env var as-is, then uppercased
///
/// Returns `None` if not found anywhere.
pub fn resolve_api_key_ref(ref_name: &str) -> Option<ResolvedSecret> {
    // 1. Secrets store (lowercase lookup — store normalizes on write)
    if !cfg!(test) {
        if let Ok(store) = crate::AgeSecretStore::open_default() {
            if let Some(value) = store.get(ref_name) {
                if !value.is_empty() {
                    return Some(ResolvedSecret {
                        value,
                        source: SecretSource::AgeStore,
                    });
                }
            }
        }
    }

    // 2. Environment variable (as-is, then uppercased)
    for var_name in [ref_name.to_string(), ref_name.to_uppercase()] {
        if let Ok(value) = std::env::var(&var_name) {
            if !value.is_empty() {
                return Some(ResolvedSecret {
                    value,
                    source: SecretSource::EnvVar(var_name),
                });
            }
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Named secret CRUD (used by `arawn secrets` CLI)
// ─────────────────────────────────────────────────────────────────────────────

/// Store a named secret in the age-encrypted secret store.
/// Names are normalized to lowercase.
pub fn store_named_secret(name: &str, value: &str) -> std::result::Result<(), String> {
    let store = crate::AgeSecretStore::open_default()
        .map_err(|e| format!("opening secret store: {}", e))?;
    store
        .set(name, value)
        .map_err(|e| format!("storing secret: {}", e))
}

/// Delete a named secret from the age-encrypted secret store.
/// Names are normalized to lowercase.
pub fn delete_named_secret(name: &str) -> std::result::Result<(), String> {
    let store = crate::AgeSecretStore::open_default()
        .map_err(|e| format!("opening secret store: {}", e))?;
    store
        .delete(name)
        .map_err(|e| format!("deleting secret: {}", e))?;
    Ok(())
}

/// Retrieve a named secret from the age-encrypted store.
/// Names are normalized to lowercase.
pub fn get_named_secret(name: &str) -> std::result::Result<Option<String>, String> {
    let store = crate::AgeSecretStore::open_default()
        .map_err(|e| format!("opening secret store: {}", e))?;
    Ok(store.get(name))
}

/// List all secret names in the age store.
pub fn list_secrets() -> std::result::Result<Vec<String>, String> {
    let store = crate::AgeSecretStore::open_default()
        .map_err(|e| format!("opening secret store: {}", e))?;
    Ok(store.list())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_from_env_var() {
        std::env::set_var("TEST_ARAWN_SECRET_ABC", "test-value");
        let resolved = resolve_api_key_ref("TEST_ARAWN_SECRET_ABC");
        std::env::remove_var("TEST_ARAWN_SECRET_ABC");

        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.value, "test-value");
        assert_eq!(
            r.source,
            SecretSource::EnvVar("TEST_ARAWN_SECRET_ABC".to_string())
        );
    }

    #[test]
    fn test_resolve_uppercases_env_var() {
        std::env::set_var("GROQ_API_KEY", "gsk-test");
        let resolved = resolve_api_key_ref("groq_api_key");
        std::env::remove_var("GROQ_API_KEY");

        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.value, "gsk-test");
        assert_eq!(r.source, SecretSource::EnvVar("GROQ_API_KEY".to_string()));
    }

    #[test]
    fn test_resolve_none_when_nothing_available() {
        let resolved = resolve_api_key_ref("nonexistent_key_xyz_12345");
        assert!(resolved.is_none());
    }

    #[test]
    fn test_secret_source_display() {
        assert_eq!(SecretSource::AgeStore.to_string(), "secret store");
        assert_eq!(
            SecretSource::EnvVar("GROQ_API_KEY".to_string()).to_string(),
            "env var GROQ_API_KEY"
        );
    }

    #[test]
    fn test_resolved_secret_debug_redacts_value() {
        let secret = ResolvedSecret {
            value: "super-secret-api-key-12345".to_string(),
            source: SecretSource::AgeStore,
        };
        let debug = format!("{:?}", secret);
        assert!(
            !debug.contains("super-secret-api-key-12345"),
            "Debug output must not contain the secret value"
        );
        assert!(debug.contains("[REDACTED]"));
        assert!(debug.contains("AgeStore"));
    }
}

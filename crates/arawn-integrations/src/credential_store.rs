//! Encrypted-at-rest credential storage for non-OAuth secrets.
//!
//! OAuth tokens use [`arawn_auth::TokenStore`] directly — it's purpose-built
//! for `Token` structs and already does atomic writes, 0600 perms, and the
//! ChaCha20Poly1305 + per-data-dir master key dance.
//!
//! `CredentialStore<T>` is for everything else: webhook URLs, API keys,
//! anything serializable that's per-service and not an OAuth `Token`. It
//! shares the same master key (`<data_dir>/tokens/.master.key`) so a user
//! who's already used OAuth doesn't end up with two keyfiles, and a fresh
//! install bootstraps the same way regardless of which gets opened first.

use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::IntegrationError;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
/// Same filename TokenStore uses, same parent dir. Sharing the file means
/// both stores end up using the same master key regardless of init order.
const KEY_FILENAME: &str = ".master.key";
const KEY_PARENT: &str = "tokens";

/// Encrypted blob store, keyed by `<data_dir>/integrations/<service>/<entry>.bin`.
///
/// Generic over the payload type; payload must round-trip through serde_json.
pub struct CredentialStore<T: Serialize + DeserializeOwned> {
    integrations_dir: PathBuf,
    service: String,
    cipher: ChaCha20Poly1305,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> CredentialStore<T> {
    /// Open or initialize the store rooted at `<data_dir>/integrations/<service>/`.
    /// Creates the per-service directory and shares (or creates) the master
    /// key at `<data_dir>/tokens/.master.key`.
    pub fn open(data_dir: &Path, service: &str) -> Result<Self, IntegrationError> {
        let integrations_dir = data_dir.join("integrations").join(safe_segment(service));
        std::fs::create_dir_all(&integrations_dir)?;
        set_dir_mode(&integrations_dir)?;

        let key_dir = data_dir.join(KEY_PARENT);
        std::fs::create_dir_all(&key_dir)?;
        let key_path = key_dir.join(KEY_FILENAME);
        let key_bytes = if key_path.exists() {
            let bytes = std::fs::read(&key_path)?;
            if bytes.len() != KEY_LEN {
                return Err(IntegrationError::Format(format!(
                    "master key has wrong length ({} != {KEY_LEN})",
                    bytes.len()
                )));
            }
            bytes
        } else {
            let mut bytes = vec![0u8; KEY_LEN];
            OsRng.fill_bytes(&mut bytes);
            write_key(&key_path, &bytes)?;
            bytes
        };

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        Ok(Self {
            integrations_dir,
            service: service.to_string(),
            cipher,
            _phantom: PhantomData,
        })
    }

    /// Persist a serializable value under `entry`.
    pub fn save(&self, entry: &str, value: &T) -> Result<(), IntegrationError> {
        let plaintext = serde_json::to_vec(value)
            .map_err(|e| IntegrationError::Format(format!("serialize {entry}: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| IntegrationError::Format(format!("encrypt {entry}: {e}")))?;

        // On-disk: 12 bytes nonce || ciphertext (matches TokenStore's layout).
        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        let path = self.path_for(entry);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &blob)?;
        set_file_mode(&tmp, 0o600)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Load `entry`. `Ok(None)` if absent; `Err` only on real failures
    /// (decrypt failure, parse failure, IO error other than not-found).
    pub fn load(&self, entry: &str) -> Result<Option<T>, IntegrationError> {
        let path = self.path_for(entry);
        let blob = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        if blob.len() < NONCE_LEN + 16 {
            return Err(IntegrationError::Format(format!(
                "credential file '{entry}' too short ({})",
                blob.len()
            )));
        }
        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| IntegrationError::Format(format!(
                "credential '{entry}' decrypt failed (tampered?)"
            )))?;

        let value: T = serde_json::from_slice(&plaintext)
            .map_err(|e| IntegrationError::Format(format!("parse {entry}: {e}")))?;
        Ok(Some(value))
    }

    /// Remove `entry` if present. Idempotent.
    pub fn delete(&self, entry: &str) -> Result<(), IntegrationError> {
        let path = self.path_for(entry);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// True if this store has anything stored under `entry`.
    pub fn exists(&self, entry: &str) -> bool {
        self.path_for(entry).exists()
    }

    /// Service name this store is bound to.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Path to the per-service directory. Useful for tests and debug output.
    pub fn integrations_dir(&self) -> &Path {
        &self.integrations_dir
    }

    fn path_for(&self, entry: &str) -> PathBuf {
        self.integrations_dir.join(format!("{}.bin", safe_segment(entry)))
    }
}

/// Refuse path-separator characters in user-supplied service / entry names.
fn safe_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == '\0' {
                '_'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(unix)]
fn set_dir_mode(path: &Path) -> Result<(), IntegrationError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_mode(_path: &Path) -> Result<(), IntegrationError> {
    Ok(())
}

#[cfg(unix)]
fn set_file_mode(path: &Path, mode: u32) -> Result<(), IntegrationError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_file_mode(_path: &Path, _mode: u32) -> Result<(), IntegrationError> {
    Ok(())
}

fn write_key(path: &Path, bytes: &[u8]) -> Result<(), IntegrationError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    set_file_mode(&tmp, 0o600)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct WebhookCred {
        url: String,
        signing_secret: Option<String>,
    }

    #[test]
    fn round_trip_returns_what_was_saved() {
        let dir = tempdir().unwrap();
        let store: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "slack").unwrap();
        let cred = WebhookCred {
            url: "https://hooks.slack.example/T0000/B0000/secret".into(),
            signing_secret: Some("shh".into()),
        };
        store.save("default", &cred).unwrap();
        let loaded = store.load("default").unwrap().expect("present");
        assert_eq!(loaded, cred);
    }

    #[test]
    fn load_returns_none_when_absent() {
        let dir = tempdir().unwrap();
        let store: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "slack").unwrap();
        assert!(store.load("nothing").unwrap().is_none());
    }

    #[test]
    fn delete_is_idempotent() {
        let dir = tempdir().unwrap();
        let store: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "slack").unwrap();
        // Delete-before-save: not an error.
        store.delete("ghost").unwrap();
        let cred = WebhookCred {
            url: "https://x".into(),
            signing_secret: None,
        };
        store.save("real", &cred).unwrap();
        assert!(store.exists("real"));
        store.delete("real").unwrap();
        assert!(!store.exists("real"));
        // Delete-after-delete: also not an error.
        store.delete("real").unwrap();
    }

    #[test]
    fn second_store_on_same_data_dir_uses_same_key() {
        // Two stores opened at the same data_dir (different services) should
        // share the master key. Saving with one and reading "the same blob"
        // wouldn't normally work since they have different per-service dirs,
        // but we can verify they end up with the same key by writing on one
        // and reading on a freshly-opened second instance for the same service.
        let dir = tempdir().unwrap();
        {
            let s: CredentialStore<WebhookCred> =
                CredentialStore::open(dir.path(), "slack").unwrap();
            s.save("default", &WebhookCred { url: "u".into(), signing_secret: None })
                .unwrap();
        }
        // Re-open with a separate handle — must still decrypt.
        let s2: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "slack").unwrap();
        let loaded = s2.load("default").unwrap().expect("present");
        assert_eq!(loaded.url, "u");
    }

    #[test]
    fn path_segments_with_slashes_get_sanitized() {
        let dir = tempdir().unwrap();
        // Service name with a slash should not escape the integrations dir.
        let store: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "weird/service").unwrap();
        // Confirm directory landed under integrations/, not at parent level.
        let svc_dir = store.integrations_dir();
        assert!(svc_dir.starts_with(dir.path().join("integrations")));
        assert!(svc_dir.ends_with("weird_service"));
    }

    #[test]
    fn corrupted_blob_yields_format_error_not_panic() {
        let dir = tempdir().unwrap();
        let store: CredentialStore<WebhookCred> =
            CredentialStore::open(dir.path(), "slack").unwrap();
        // Write garbage that's the wrong shape for our crypto envelope.
        let path = store.path_for("default");
        std::fs::write(&path, b"too short").unwrap();
        match store.load("default") {
            Err(IntegrationError::Format(_)) => {} // expected
            other => panic!("expected Format error, got {other:?}"),
        }
    }
}

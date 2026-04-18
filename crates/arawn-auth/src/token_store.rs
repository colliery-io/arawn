//! Encrypted on-disk storage for OAuth tokens.
//!
//! Layout under `{data_dir}/tokens/`:
//!
//! ```text
//! .master.key          — 32-byte symmetric key, mode 0600
//! <provider>.json.enc  — chacha20poly1305 ciphertext of the JSON token
//! ```
//!
//! On first use the master key is generated with cryptographic randomness and
//! persisted with restrictive permissions. Subsequent opens load it.
//!
//! The agent must not be able to read this directory — see the Facility
//! System spec's security contract and the sensitive-paths deny list.

use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use rand::rngs::OsRng as RandOsRng;

use crate::error::AuthError;
use crate::oauth2::Token;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KEY_FILENAME: &str = ".master.key";

pub struct TokenStore {
    tokens_dir: PathBuf,
    cipher: ChaCha20Poly1305,
}

impl TokenStore {
    /// Open or initialise the token store under `{data_dir}/tokens/`.
    /// Creates the directory and master key on first use.
    pub fn open(data_dir: &Path) -> Result<Self, AuthError> {
        let tokens_dir = data_dir.join("tokens");
        std::fs::create_dir_all(&tokens_dir)
            .map_err(|e| AuthError::InvalidConfig(format!("create tokens dir: {e}")))?;
        Self::set_dir_mode(&tokens_dir)?;

        let key_path = tokens_dir.join(KEY_FILENAME);
        let key_bytes = if key_path.exists() {
            let bytes = std::fs::read(&key_path)
                .map_err(|e| AuthError::InvalidConfig(format!("read master key: {e}")))?;
            if bytes.len() != KEY_LEN {
                return Err(AuthError::InvalidConfig(format!(
                    "master key has wrong length ({} != {KEY_LEN})",
                    bytes.len()
                )));
            }
            bytes
        } else {
            let mut bytes = vec![0u8; KEY_LEN];
            RandOsRng.fill_bytes(&mut bytes);
            Self::write_key(&key_path, &bytes)?;
            bytes
        };

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        Ok(Self { tokens_dir, cipher })
    }

    /// Persist `token` for the named `provider`.
    pub fn save(&self, provider: &str, token: &Token) -> Result<(), AuthError> {
        let plaintext = serde_json::to_vec(token)
            .map_err(|e| AuthError::Decode(format!("serialize token: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| AuthError::InvalidConfig(format!("encrypt: {e}")))?;

        // On-disk: 12 bytes nonce || ciphertext
        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        let path = self.path_for(provider);
        // Atomic write via rename.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &blob)
            .map_err(|e| AuthError::InvalidConfig(format!("write tmp: {e}")))?;
        Self::set_file_mode(&tmp, 0o600)?;
        std::fs::rename(&tmp, &path)
            .map_err(|e| AuthError::InvalidConfig(format!("rename: {e}")))?;
        Ok(())
    }

    /// Load the token for `provider`, returning `Ok(None)` when absent.
    pub fn load(&self, provider: &str) -> Result<Option<Token>, AuthError> {
        let path = self.path_for(provider);
        let blob = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(AuthError::InvalidConfig(format!(
                    "read token file: {e}"
                )));
            }
        };

        if blob.len() < NONCE_LEN + 16 {
            return Err(AuthError::Decode(format!(
                "token file too short ({})",
                blob.len()
            )));
        }
        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| AuthError::Decode("token decrypt failed (tampered?)".into()))?;

        let token: Token = serde_json::from_slice(&plaintext)
            .map_err(|e| AuthError::Decode(format!("parse token JSON: {e}")))?;
        Ok(Some(token))
    }

    pub fn delete(&self, provider: &str) -> Result<(), AuthError> {
        let path = self.path_for(provider);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AuthError::InvalidConfig(format!(
                "delete token file: {e}"
            ))),
        }
    }

    pub fn tokens_dir(&self) -> &Path {
        &self.tokens_dir
    }

    fn path_for(&self, provider: &str) -> PathBuf {
        // Defensive: refuse path-separator characters in provider names.
        let safe: String = provider
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            .collect();
        self.tokens_dir.join(format!("{safe}.json.enc"))
    }

    fn write_key(path: &Path, bytes: &[u8]) -> Result<(), AuthError> {
        std::fs::write(path, bytes)
            .map_err(|e| AuthError::InvalidConfig(format!("write master key: {e}")))?;
        Self::set_file_mode(path, 0o600)?;
        Ok(())
    }

    #[cfg(unix)]
    fn set_file_mode(path: &Path, mode: u32) -> Result<(), AuthError> {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)
            .map_err(|e| AuthError::InvalidConfig(format!("chmod {mode:o}: {e}")))?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn set_file_mode(_path: &Path, _mode: u32) -> Result<(), AuthError> {
        // No-op on non-Unix platforms; ACLs would be the right answer but
        // arawn currently only targets Unix.
        Ok(())
    }

    #[cfg(unix)]
    fn set_dir_mode(path: &Path) -> Result<(), AuthError> {
        Self::set_file_mode(path, 0o700)
    }

    #[cfg(not(unix))]
    fn set_dir_mode(_path: &Path) -> Result<(), AuthError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn sample_token() -> Token {
        Token {
            access: "AT-12345".into(),
            refresh: Some("RT-67890".into()),
            expires_at: Some(Utc::now() + chrono::Duration::seconds(3600)),
            scope: Some("read write".into()),
            token_type: "Bearer".into(),
        }
    }

    #[test]
    fn save_then_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        store.save("google", &sample_token()).unwrap();

        let loaded = store.load("google").unwrap().expect("token should exist");
        assert_eq!(loaded.access, "AT-12345");
        assert_eq!(loaded.refresh.as_deref(), Some("RT-67890"));
    }

    #[test]
    fn load_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        assert!(store.load("never-saved").unwrap().is_none());
    }

    #[test]
    fn delete_then_load_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        store.save("slack", &sample_token()).unwrap();
        store.delete("slack").unwrap();
        assert!(store.load("slack").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        assert!(store.delete("nope").is_ok());
    }

    #[test]
    fn tampered_ciphertext_fails_decrypt() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        store.save("google", &sample_token()).unwrap();

        // Flip a single byte in the ciphertext (skip the 12-byte nonce).
        let path = store.path_for("google");
        let mut blob = std::fs::read(&path).unwrap();
        blob[NONCE_LEN + 1] ^= 0x01;
        std::fs::write(&path, &blob).unwrap();

        match store.load("google") {
            Err(AuthError::Decode(m)) => assert!(m.contains("decrypt")),
            other => panic!("expected Decode error, got {other:?}"),
        }
    }

    #[test]
    fn second_open_reuses_master_key() {
        let tmp = TempDir::new().unwrap();
        let s1 = TokenStore::open(tmp.path()).unwrap();
        s1.save("google", &sample_token()).unwrap();
        drop(s1);

        let s2 = TokenStore::open(tmp.path()).unwrap();
        let loaded = s2.load("google").unwrap().unwrap();
        assert_eq!(loaded.access, "AT-12345");
    }

    #[test]
    fn missing_master_key_after_save_fails_clearly() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        store.save("google", &sample_token()).unwrap();

        // Delete the master key — opening a new store will mint a new one,
        // and decrypt against it will fail.
        std::fs::remove_file(store.tokens_dir().join(KEY_FILENAME)).unwrap();

        let store2 = TokenStore::open(tmp.path()).unwrap();
        match store2.load("google") {
            Err(AuthError::Decode(_)) => {}
            other => panic!("expected Decode error after key rotation, got {other:?}"),
        }
    }

    #[test]
    fn provider_name_sanitization_rejects_path_chars() {
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        // Path-traversal attempt becomes a benign filename
        let p = store.path_for("../../etc/passwd");
        assert!(p.starts_with(store.tokens_dir()));
        assert_eq!(p.file_name().unwrap(), "etcpasswd.json.enc");
    }

    #[cfg(unix)]
    #[test]
    fn master_key_has_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let store = TokenStore::open(tmp.path()).unwrap();
        let key_path = store.tokens_dir().join(KEY_FILENAME);
        let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "master key should be 0600, was {mode:o}");
    }
}

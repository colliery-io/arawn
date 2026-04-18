//! Environment-variable filtering for child processes spawned by tools.
//!
//! The arawn parent process holds API keys and other secrets in its environment
//! (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GITHUB_TOKEN`, etc.). Without
//! filtering, every spawned shell command would inherit them — an agent can
//! then exfiltrate them with a single `env` invocation. This module produces
//! a sanitized environment containing only the variables required for common
//! development tooling (PATH, build caches, locale).

use std::collections::HashMap;

/// Exact env var names that are always safe to forward to children.
const SAFE_EXACT: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TERM",
    "LANG",
    "TMPDIR",
    "TMP",
    "TEMP",
    "PWD",
    "OLDPWD",
    // Build tool homes — needed by cargo/rustup/npm/pip/etc.
    "CARGO_HOME",
    "RUSTUP_HOME",
    "GOPATH",
    "GOROOT",
    "NPM_CONFIG_PREFIX",
    "PIP_CACHE_DIR",
    // Locale fallback (LC_* handled by prefix below)
    "LC_ALL",
];

/// Prefixes for env var names that are safe to forward.
const SAFE_PREFIXES: &[&str] = &[
    "LC_",  // locale
    "XDG_", // freedesktop dirs
];

/// Returns a filtered copy of the parent process environment, dropping any
/// variable that doesn't match the safe allowlist.
pub fn safe_env() -> HashMap<String, String> {
    std::env::vars().filter(|(k, _)| is_safe_env_name(k)).collect()
}

/// Returns true if `name` is on the safe allowlist.
pub fn is_safe_env_name(name: &str) -> bool {
    if SAFE_EXACT.contains(&name) {
        return true;
    }
    SAFE_PREFIXES.iter().any(|p| name.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_path_and_home() {
        assert!(is_safe_env_name("PATH"));
        assert!(is_safe_env_name("HOME"));
        assert!(is_safe_env_name("CARGO_HOME"));
    }

    #[test]
    fn allows_lc_and_xdg_prefixes() {
        assert!(is_safe_env_name("LC_ALL"));
        assert!(is_safe_env_name("LC_CTYPE"));
        assert!(is_safe_env_name("XDG_CONFIG_HOME"));
    }

    #[test]
    fn blocks_secrets() {
        assert!(!is_safe_env_name("ANTHROPIC_API_KEY"));
        assert!(!is_safe_env_name("OPENAI_API_KEY"));
        assert!(!is_safe_env_name("AWS_SECRET_ACCESS_KEY"));
        assert!(!is_safe_env_name("AWS_ACCESS_KEY_ID"));
        assert!(!is_safe_env_name("GITHUB_TOKEN"));
        assert!(!is_safe_env_name("GH_TOKEN"));
        assert!(!is_safe_env_name("MY_APP_SECRET"));
        // Anything ending in TOKEN/KEY/SECRET is blocked by virtue of not being on the allowlist
        assert!(!is_safe_env_name("ARBITRARY_TOKEN"));
    }

    #[test]
    fn safe_env_strips_test_secret() {
        // SAFETY: tests in this module run in a single process; setting then
        // unsetting an env var is fine for the duration of the test.
        unsafe {
            std::env::set_var("ARAWN_TEST_FAKE_KEY", "supersecret");
        }
        let env = safe_env();
        assert!(!env.contains_key("ARAWN_TEST_FAKE_KEY"));
        unsafe {
            std::env::remove_var("ARAWN_TEST_FAKE_KEY");
        }
    }
}

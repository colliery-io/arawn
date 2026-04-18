//! Shared sensitive-path deny list used across shell, glob, grep, and file tools.
//!
//! Centralizes the set of paths that should never be readable by agent tools,
//! regardless of which tool is doing the reading. The shell sandbox passes this
//! list to the OS sandbox; glob/grep/file tools call [`is_sensitive_path`] to
//! reject paths that resolve into any of these directories.

use std::path::{Path, PathBuf};

/// Build the list of sensitive paths that should be denied for reading.
///
/// Includes system auth files, SSH/GPG keys, cloud provider credentials,
/// container/package manager tokens, git credentials, database passwords,
/// shell history, and (on macOS) keychains/cookies.
pub fn sensitive_deny_read_paths() -> Vec<String> {
    let mut paths = vec![
        // System auth & security
        "/etc/shadow".to_string(),
        "/etc/gshadow".to_string(),
        "/etc/sudoers".to_string(),
        "/etc/sudoers.d".to_string(),
        "/etc/ssl/private".to_string(),
    ];

    if let Some(home) = dirs::home_dir() {
        let h = home.to_string_lossy();
        // SSH & GPG keys
        paths.push(format!("{h}/.ssh"));
        paths.push(format!("{h}/.gnupg"));
        // Cloud provider credentials
        paths.push(format!("{h}/.aws"));
        paths.push(format!("{h}/.azure"));
        paths.push(format!("{h}/.config/gcloud"));
        paths.push(format!("{h}/.kube"));
        // Container credentials
        paths.push(format!("{h}/.docker/config.json"));
        // Package manager tokens
        paths.push(format!("{h}/.npmrc"));
        // Git credentials
        paths.push(format!("{h}/.netrc"));
        paths.push(format!("{h}/.git-credentials"));
        // CLI tool credentials
        paths.push(format!("{h}/.config/gh"));
        paths.push(format!("{h}/.vault-token"));
        // Database passwords
        paths.push(format!("{h}/.pgpass"));
        paths.push(format!("{h}/.my.cnf"));
        // Shell history
        paths.push(format!("{h}/.bash_history"));
        paths.push(format!("{h}/.zsh_history"));
        // macOS specific
        #[cfg(target_os = "macos")]
        {
            paths.push(format!("{h}/Library/Keychains"));
            paths.push(format!("{h}/Library/Cookies"));
        }
    }

    paths
}

/// Returns true if `path` resolves into any sensitive directory.
///
/// Compares canonical forms when possible to defeat symlink and `..` traversal
/// tricks. Falls back to a lexical prefix check for non-existent paths.
pub fn is_sensitive_path(path: &Path) -> bool {
    let canonical_target = path.canonicalize().ok();

    for deny in sensitive_deny_read_paths() {
        let deny_path = PathBuf::from(&deny);
        let canonical_deny = deny_path.canonicalize().ok();

        match (canonical_target.as_ref(), canonical_deny.as_ref()) {
            (Some(target), Some(deny_canon)) => {
                if target == deny_canon || target.starts_with(deny_canon) {
                    return true;
                }
            }
            _ => {
                // Either target or deny doesn't exist on this system — fall back
                // to lexical prefix on the input path.
                if path == deny_path.as_path() || path.starts_with(&deny_path) {
                    return true;
                }
            }
        }
    }

    false
}

/// Returns true if `path` resolves into the OAuth token directory under
/// `data_dir/tokens/`. Used by glob/grep/file_* tools to deny token reads
/// even when the user has put the data dir on `allowed_paths`. Symlink-aware
/// when both the input and the deny target exist.
pub fn is_token_path(path: &Path, data_dir: &Path) -> bool {
    let tokens_dir = data_dir.join("tokens");
    let canonical_target = path.canonicalize().ok();
    let canonical_deny = tokens_dir.canonicalize().ok();

    match (canonical_target.as_ref(), canonical_deny.as_ref()) {
        (Some(t), Some(d)) => t == d || t.starts_with(d),
        _ => path == tokens_dir.as_path() || path.starts_with(&tokens_dir),
    }
}

/// Returns true if the file at `path` matches a known secret-file pattern.
///
/// Matches by basename so `./app/.env` and `../../.env` are both blocked, while
/// legitimate files like `env.rs`, `.env.example`, and `environment.ts` are not.
pub fn is_secret_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    is_secret_filename(name)
}

fn is_secret_filename(name: &str) -> bool {
    // Exact-match secret filenames
    const EXACT: &[&str] = &[
        ".env",
        ".envrc",
        "credentials.json",
        "credentials.yml",
        "credentials.yaml",
        "secrets.yml",
        "secrets.yaml",
        "secrets.json",
        "secrets.toml",
        "token.json",
        "tokens.json",
    ];
    if EXACT.iter().any(|s| name.eq_ignore_ascii_case(s)) {
        return true;
    }

    // Suffix-based extension matches
    const EXTENSIONS: &[&str] = &[".secret", ".key", ".pem", ".p12", ".pfx"];
    let lower = name.to_ascii_lowercase();
    if EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
        // ".key" by itself is a hidden file, allow if it's not exactly the suffix
        return true;
    }

    // .env.<something> — but allow .env.example / .env.sample / .env.template
    if let Some(rest) = lower.strip_prefix(".env.") {
        const ALLOWED_ENV_SUFFIXES: &[&str] = &["example", "sample", "template", "dist", "default"];
        if !ALLOWED_ENV_SUFFIXES.iter().any(|s| rest == *s) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_list_includes_ssh_and_aws() {
        let paths = sensitive_deny_read_paths();
        let joined = paths.join("\n");
        assert!(joined.contains(".ssh"));
        assert!(joined.contains(".aws"));
        assert!(joined.contains("/etc/shadow"));
    }

    #[test]
    fn ssh_dir_is_sensitive() {
        if let Some(home) = dirs::home_dir() {
            let ssh = home.join(".ssh");
            assert!(is_sensitive_path(&ssh));
            // Subpath also flagged
            assert!(is_sensitive_path(&ssh.join("id_rsa")));
        }
    }

    #[test]
    fn aws_dir_is_sensitive() {
        if let Some(home) = dirs::home_dir() {
            let aws = home.join(".aws");
            assert!(is_sensitive_path(&aws));
            assert!(is_sensitive_path(&aws.join("credentials")));
        }
    }

    #[test]
    fn ordinary_path_is_not_sensitive() {
        let dir = std::env::temp_dir();
        assert!(!is_sensitive_path(&dir));
    }

    #[test]
    fn etc_shadow_is_sensitive() {
        assert!(is_sensitive_path(Path::new("/etc/shadow")));
    }

    #[test]
    fn secret_file_basenames_blocked() {
        assert!(is_secret_file(Path::new(".env")));
        assert!(is_secret_file(Path::new("./app/.env")));
        assert!(is_secret_file(Path::new("../../.env")));
        assert!(is_secret_file(Path::new(".env.local")));
        assert!(is_secret_file(Path::new(".env.production")));
        assert!(is_secret_file(Path::new("credentials.json")));
        assert!(is_secret_file(Path::new("secrets.yml")));
        assert!(is_secret_file(Path::new("secrets.toml")));
        assert!(is_secret_file(Path::new("token.json")));
        assert!(is_secret_file(Path::new("tokens.json")));
        assert!(is_secret_file(Path::new("server.pem")));
        assert!(is_secret_file(Path::new("private.key")));
        assert!(is_secret_file(Path::new("cert.p12")));
        assert!(is_secret_file(Path::new("api.secret")));
    }

    #[test]
    fn token_path_detection() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path();
        let tokens = data_dir.join("tokens");
        std::fs::create_dir_all(&tokens).unwrap();

        // Direct match
        assert!(is_token_path(&tokens, data_dir));
        // Subpath
        assert!(is_token_path(&tokens.join("google.json.enc"), data_dir));
        // Nested subpath
        assert!(is_token_path(&tokens.join("subdir/file"), data_dir));
        // Sibling directories not token-pathed
        assert!(!is_token_path(&data_dir.join("workstreams"), data_dir));
        assert!(!is_token_path(&data_dir.join("tokens-other"), data_dir));
        // Outside data_dir entirely
        assert!(!is_token_path(Path::new("/tmp/somewhere/else"), data_dir));
    }

    #[test]
    fn token_path_defeats_dotdot_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path();
        let tokens = data_dir.join("tokens");
        std::fs::create_dir_all(&tokens).unwrap();
        std::fs::write(tokens.join("google.json.enc"), b"x").unwrap();

        // A relative-with-dots path that resolves into tokens/ via canonicalize.
        let workspace = data_dir.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let traversal = workspace.join("../tokens/google.json.enc");
        assert!(is_token_path(&traversal, data_dir));
    }

    #[test]
    fn legitimate_files_not_secret() {
        assert!(!is_secret_file(Path::new("env.rs")));
        assert!(!is_secret_file(Path::new("environment.ts")));
        assert!(!is_secret_file(Path::new(".env.example")));
        assert!(!is_secret_file(Path::new(".env.sample")));
        assert!(!is_secret_file(Path::new(".env.template")));
        assert!(!is_secret_file(Path::new("config.json")));
        assert!(!is_secret_file(Path::new("README.md")));
        assert!(!is_secret_file(Path::new("keychain.rs")));
    }
}

//! Command wrapping for macOS sandbox-exec.


use crate::config::SandboxRuntimeConfig;
use crate::error::SandboxError;
use crate::sandbox::macos::profile::{generate_log_tag, generate_profile};
use crate::utils::quote;

/// Wrap a command with sandbox-exec.
pub fn wrap_command(
    command: &str,
    config: &SandboxRuntimeConfig,
    http_proxy_port: Option<u16>,
    socks_proxy_port: Option<u16>,
    shell: Option<&str>,
    enable_log_monitor: bool,
) -> Result<(String, Option<String>), SandboxError> {
    let shell = shell.unwrap_or("/bin/bash");

    // Generate log tag for violation monitoring
    let log_tag = if enable_log_monitor {
        Some(generate_log_tag(command))
    } else {
        None
    };

    // Generate the Seatbelt profile
    let profile = generate_profile(config, http_proxy_port, socks_proxy_port, log_tag.as_deref());

    // Write profile to a unique temporary file
    let profile_path = write_profile_to_temp(&profile)?;

    // Build the wrapped command. It removes ITS OWN profile right after
    // `sandbox-exec` returns (preserving the real exit code) so the temp file
    // lives only for the brief window it's actually read. This is what makes
    // concurrent invocations safe — each cleans up after itself instead of
    // relying on a process-wide sweep that could delete another in-flight call's
    // profile. `cleanup_temp_profiles` remains only as an age-based safety net.
    let wrapped = format!(
        "sandbox-exec -f {} {} -c {}; __srt_ec=$?; rm -f {}; exit $__srt_ec",
        quote(&profile_path),
        shell,
        quote(command),
        quote(&profile_path),
    );

    Ok((wrapped, log_tag))
}

/// Monotonic per-process counter so every wrapped command gets its OWN
/// profile file. Without this, the filename was keyed only on the PID, so two
/// `wrap_command` calls running concurrently in the same process (e.g. parallel
/// tests, or two agent tool calls) wrote/cleaned-up the same
/// `srt-profile-<pid>.sb` and raced — one would overwrite or delete the file
/// out from under the other's `sandbox-exec -f`, producing intermittent
/// "No such file or directory" failures. (Rust-specific concurrency fix; the
/// upstream TS impl is single-flight per process.)
static PROFILE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Filename prefix shared by `write_profile_to_temp` and `cleanup_temp_profiles`.
fn profile_prefix() -> String {
    format!("srt-profile-{}-", std::process::id())
}

/// Write the profile to a unique temporary file.
fn write_profile_to_temp(profile: &str) -> Result<String, SandboxError> {
    use std::io::Write;
    use std::sync::atomic::Ordering;

    let temp_dir = std::env::temp_dir();
    let seq = PROFILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let filename = format!("{}{}.sb", profile_prefix(), seq);
    let path = temp_dir.join(filename);

    let mut file = std::fs::File::create(&path)?;
    file.write_all(profile.as_bytes())?;

    Ok(path.display().to_string())
}

/// Age, in seconds, below which a profile file is assumed possibly in-flight
/// and left alone by the safety-net sweep.
const STALE_PROFILE_SECS: u64 = 60;

/// Safety-net cleanup for this process's leaked profile files — only those a
/// crashed/killed `sandbox-exec` left behind. The happy path is self-cleaning
/// (see `wrap_command`), so this only removes files older than
/// `STALE_PROFILE_SECS`. The age check is what makes it safe to call
/// concurrently with other in-flight commands: a fresh profile (seconds old)
/// is never swept out from under a running `sandbox-exec`.
pub fn cleanup_temp_profiles() {
    let temp_dir = std::env::temp_dir();
    let prefix = profile_prefix();

    let Ok(entries) = std::fs::read_dir(&temp_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with(&prefix) && name.ends_with(".sb")) {
            continue;
        }
        // Only remove if comfortably older than any plausible in-flight call.
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .map(|age| age.as_secs() >= STALE_PROFILE_SECS)
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Generate proxy environment variables.
pub fn generate_proxy_env(
    http_proxy_port: u16,
    socks_proxy_port: u16,
) -> Vec<(String, String)> {
    let http_proxy = format!("http://localhost:{}", http_proxy_port);
    let socks_proxy = format!("socks5://localhost:{}", socks_proxy_port);

    vec![
        ("http_proxy".to_string(), http_proxy.clone()),
        ("HTTP_PROXY".to_string(), http_proxy.clone()),
        ("https_proxy".to_string(), http_proxy.clone()),
        ("HTTPS_PROXY".to_string(), http_proxy),
        ("ALL_PROXY".to_string(), socks_proxy.clone()),
        ("all_proxy".to_string(), socks_proxy),
        // For git SSH
        (
            "GIT_SSH_COMMAND".to_string(),
            format!(
                "ssh -o ProxyCommand='nc -X 5 -x localhost:{} %h %p'",
                socks_proxy_port
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_proxy_env() {
        let env = generate_proxy_env(3128, 1080);
        assert!(env.iter().any(|(k, v)| k == "http_proxy" && v.contains("3128")));
        assert!(env.iter().any(|(k, v)| k == "ALL_PROXY" && v.contains("1080")));
    }
}

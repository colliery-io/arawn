//! Bubblewrap command generation for Linux sandbox.

use std::path::Path;

use crate::config::SandboxRuntimeConfig;
use crate::error::SandboxError;
use crate::sandbox::linux::bridge::SocatBridge;
use crate::sandbox::linux::filesystem::generate_bind_mounts;
use crate::sandbox::linux::seccomp::{get_apply_seccomp_path, get_bpf_path};
use crate::utils::quote;

/// Check if bubblewrap is available.
pub fn check_bwrap() -> bool {
    std::process::Command::new("bwrap")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Generate the bubblewrap command for sandboxed execution.
pub fn generate_bwrap_command(
    command: &str,
    config: &SandboxRuntimeConfig,
    cwd: &Path,
    http_socket_path: Option<&str>,
    socks_socket_path: Option<&str>,
    http_proxy_port: u16,
    socks_proxy_port: u16,
    shell: Option<&str>,
) -> Result<(String, Vec<String>), SandboxError> {
    let shell = shell.unwrap_or("/bin/bash");

    // Generate filesystem mounts
    let (mounts, warnings) = generate_bind_mounts(
        &config.filesystem,
        cwd,
        config.ripgrep.as_ref(),
        config.mandatory_deny_search_depth,
    )?;

    // Build bwrap arguments
    let mut bwrap_args = vec![
        "bwrap".to_string(),
        "--unshare-net".to_string(), // Network isolation
        "--dev".to_string(),
        "/dev".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--tmpfs".to_string(),
        "/run".to_string(),
    ];

    // Start with read-only root filesystem
    bwrap_args.push("--ro-bind".to_string());
    bwrap_args.push("/".to_string());
    bwrap_args.push("/".to_string());

    // Add writable mounts
    for mount in &mounts {
        if !mount.readonly {
            bwrap_args.extend(mount.to_bwrap_args());
        }
    }

    // Add read-only (deny) mounts to override writable ones
    for mount in &mounts {
        if mount.readonly {
            bwrap_args.extend(mount.to_bwrap_args());
        }
    }

    // Set working directory
    bwrap_args.push("--chdir".to_string());
    bwrap_args.push(cwd.display().to_string());

    // Build the inner command with socat bridges and seccomp
    let inner_command = build_inner_command(
        command,
        config,
        http_socket_path,
        socks_socket_path,
        http_proxy_port,
        socks_proxy_port,
        shell,
    )?;

    // Add the command
    bwrap_args.push("--".to_string());
    bwrap_args.push(shell.to_string());
    bwrap_args.push("-c".to_string());
    bwrap_args.push(inner_command);

    // Join into a single command string
    let wrapped = bwrap_args
        .iter()
        .map(|s| quote(s))
        .collect::<Vec<_>>()
        .join(" ");

    Ok((wrapped, warnings))
}

/// Build the inner command to run inside bubblewrap.
/// This sets up socat bridges and applies seccomp before running the user command.
fn build_inner_command(
    command: &str,
    config: &SandboxRuntimeConfig,
    http_socket_path: Option<&str>,
    socks_socket_path: Option<&str>,
    http_proxy_port: u16,
    socks_proxy_port: u16,
    shell: &str,
) -> Result<String, SandboxError> {
    let mut parts = Vec::new();

    // The bridges run in the background for the life of the script. Their
    // output goes to /dev/null and the trap stops them when the script
    // exits; otherwise they hold the caller's stdout/stderr open and the
    // caller never sees EOF. The trap keeps the command's exit status.
    if http_socket_path.is_some() || socks_socket_path.is_some() {
        parts.push("trap 'kill $(jobs -p) 2>/dev/null' EXIT".to_string());
    }

    // Set up socat bridges for proxy access
    if let Some(http_sock) = http_socket_path {
        let bridge_cmd = SocatBridge::tcp_to_unix_command(http_proxy_port, http_sock);
        parts.push(format!("{} >/dev/null 2>&1 &", bridge_cmd));
    }

    if let Some(socks_sock) = socks_socket_path {
        let bridge_cmd = SocatBridge::tcp_to_unix_command(socks_proxy_port, socks_sock);
        parts.push(format!("{} >/dev/null 2>&1 &", bridge_cmd));
    }

    // Small delay to let socat bridges start
    if http_socket_path.is_some() || socks_socket_path.is_some() {
        parts.push("sleep 0.1".to_string());
    }

    // Apply seccomp filter and execute command
    if !config.network.allow_all_unix_sockets.unwrap_or(false) {
        // Try to use seccomp to block Unix socket creation
        if let (Ok(bpf_path), Ok(apply_path)) = (
            get_bpf_path(config.seccomp.as_ref()),
            get_apply_seccomp_path(config.seccomp.as_ref()),
        ) {
            // Export proxy environment variables before applying seccomp
            let env_vars = generate_proxy_env_string(http_proxy_port, socks_proxy_port);
            parts.push(env_vars);

            // Use apply-seccomp to apply the filter and exec the command
            parts.push(format!(
                "{} {} {} -c {}",
                apply_path.display(),
                bpf_path.display(),
                shell,
                quote(command)
            ));
        } else {
            // Seccomp not available, just run the command with warning
            tracing::warn!(
                "Seccomp not available - Unix socket creation will not be blocked"
            );
            parts.push(generate_proxy_env_string(http_proxy_port, socks_proxy_port));
            parts.push(format!("{} -c {}", shell, quote(command)));
        }
    } else {
        // Unix sockets allowed, just run the command
        parts.push(generate_proxy_env_string(http_proxy_port, socks_proxy_port));
        parts.push(format!("{} -c {}", shell, quote(command)));
    }

    Ok(join_script(&parts))
}

/// Join shell steps into one script. A step that ends in `&` already
/// ends its command, and bash rejects `& ;`, so it gets no `;`.
fn join_script(parts: &[String]) -> String {
    let mut script = String::new();
    for (i, part) in parts.iter().enumerate() {
        script.push_str(part);
        if i + 1 < parts.len() {
            script.push_str(if part.trim_end().ends_with('&') { " " } else { " ; " });
        }
    }
    script
}

/// Generate proxy environment variable exports.
fn generate_proxy_env_string(http_port: u16, socks_port: u16) -> String {
    format!(
        "export http_proxy='http://localhost:{}' https_proxy='http://localhost:{}' \
         HTTP_PROXY='http://localhost:{}' HTTPS_PROXY='http://localhost:{}' \
         ALL_PROXY='socks5://localhost:{}' all_proxy='socks5://localhost:{}'",
        http_port, http_port, http_port, http_port, socks_port, socks_port
    )
}

/// Generate proxy environment variables.
pub fn generate_proxy_env(http_port: u16, socks_port: u16) -> Vec<(String, String)> {
    let http_proxy = format!("http://localhost:{}", http_port);
    let socks_proxy = format!("socks5://localhost:{}", socks_port);

    vec![
        ("http_proxy".to_string(), http_proxy.clone()),
        ("HTTP_PROXY".to_string(), http_proxy.clone()),
        ("https_proxy".to_string(), http_proxy.clone()),
        ("HTTPS_PROXY".to_string(), http_proxy),
        ("ALL_PROXY".to_string(), socks_proxy.clone()),
        ("all_proxy".to_string(), socks_proxy),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_script_puts_no_semicolon_after_background_steps() {
        let parts = vec![
            "socat A &".to_string(),
            "socat B &".to_string(),
            "sleep 0.1".to_string(),
            "export x=1".to_string(),
            "/bin/bash -c 'echo hi'".to_string(),
        ];
        assert_eq!(
            join_script(&parts),
            "socat A & socat B & sleep 0.1 ; export x=1 ; /bin/bash -c 'echo hi'"
        );
    }

    #[test]
    fn test_join_script_runs_in_bash() {
        let parts = vec![
            "true &".to_string(),
            "export x=1".to_string(),
            "test \"$x\" = 1".to_string(),
        ];
        let status = std::process::Command::new("bash")
            .arg("-c")
            .arg(join_script(&parts))
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_background_steps_do_not_hold_output_open() {
        // Stand-ins for the socat bridges: they would run for 100 s.
        let parts = vec![
            "trap 'kill $(jobs -p) 2>/dev/null' EXIT".to_string(),
            "sleep 100 >/dev/null 2>&1 &".to_string(),
            "sleep 100 >/dev/null 2>&1 &".to_string(),
            "echo done".to_string(),
            "exit 3".to_string(),
        ];
        let started = std::time::Instant::now();
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(join_script(&parts))
            .output()
            .unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "done");
        assert_eq!(out.status.code(), Some(3));
    }

    #[test]
    fn test_generate_proxy_env_string() {
        let env = generate_proxy_env_string(3128, 1080);
        assert!(env.contains("http_proxy='http://localhost:3128'"));
        assert!(env.contains("ALL_PROXY='socks5://localhost:1080'"));
        // A step of the script: join_script adds the separators.
        assert!(!env.trim_end().ends_with(';'));
    }

    #[test]
    fn test_check_bwrap() {
        // This test will pass/fail based on system configuration
        let available = check_bwrap();
        println!("Bubblewrap available: {}", available);
    }
}

//! Logs command - view and tail operational logs.

use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use console::Style;

use super::Context;
use super::output;

/// Arguments for the logs command.
#[derive(Args, Debug)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m
  arawn logs                          Show recent local logs
  arawn logs -n 50                    Show last 50 lines
  arawn logs -f                       Follow log output (tail -f)
  arawn logs --file launchd-stdout    Read a specific log file
  arawn logs --remote                 Fetch logs from running server
  arawn logs --remote --list-files    List available server log files")]
pub struct LogsArgs {
    /// Number of lines to show (default: 25)
    #[arg(short = 'n', long, default_value = "25")]
    pub lines: usize,

    /// Follow log output continuously
    #[arg(short, long)]
    pub follow: bool,

    /// Log file to read (default: latest daily log)
    #[arg(long)]
    pub file: Option<String>,

    /// Fetch logs from the running server instead of reading local files
    #[arg(short, long)]
    pub remote: bool,

    /// List available log files (use with --remote)
    #[arg(long)]
    pub list_files: bool,
}

/// Run the logs command.
pub async fn run(args: LogsArgs, ctx: &Context) -> Result<()> {
    if args.remote || args.list_files {
        return run_remote(args, ctx).await;
    }

    let log_dir = arawn_config::xdg_config_dir()
        .map(|d| d.join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    if !log_dir.exists() {
        output::error(format!("Log directory not found: {}", log_dir.display()));
        output::hint("  Is the server configured? Try: arawn start");
        return Ok(());
    }

    let log_file = if let Some(ref name) = args.file {
        let path = log_dir.join(format!("{}.log", name));
        if !path.exists() {
            // Try exact name
            let exact = log_dir.join(name);
            if exact.exists() {
                exact
            } else {
                output::error(format!("Log file not found: {}", path.display()));
                println!();
                list_log_files(&log_dir)?;
                return Ok(());
            }
        } else {
            path
        }
    } else {
        // Find the latest log file
        find_latest_log(&log_dir)?
    };

    if args.follow {
        tail_follow(&log_file, args.lines).await
    } else {
        tail_lines(&log_file, args.lines)
    }
}

fn find_latest_log(log_dir: &std::path::Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();

    entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));

    entries
        .first()
        .map(|e| e.path())
        .ok_or_else(|| anyhow::anyhow!("No log files found in {}", log_dir.display()))
}

fn list_log_files(log_dir: &std::path::Path) -> Result<()> {
    output::hint("  Available log files:");
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "log") {
            let name = entry.file_name();
            println!(
                "    {}",
                Style::new().dim().apply_to(name.to_string_lossy())
            );
        }
    }
    Ok(())
}

fn tail_lines(path: &std::path::Path, n: usize) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;

    let start = lines.len().saturating_sub(n);
    for line in &lines[start..] {
        print_log_line(line);
    }

    Ok(())
}

async fn tail_follow(path: &std::path::Path, initial_lines: usize) -> Result<()> {
    // Show initial lines
    tail_lines(path, initial_lines)?;

    // Then follow
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::End(0))?;

    println!(
        "{}",
        Style::new()
            .dim()
            .apply_to("── following (Ctrl+C to stop) ──")
    );

    let mut buf = String::new();
    let mut reader = BufReader::new(file);

    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => {
                // No new data, wait a bit
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            Ok(_) => {
                let line = buf.trim_end();
                if !line.is_empty() {
                    print_log_line(line);
                }
            }
            Err(e) => {
                output::error(format!("Error reading log: {}", e));
                break;
            }
        }
    }

    Ok(())
}

async fn run_remote(args: LogsArgs, ctx: &Context) -> Result<()> {
    let client = crate::client::Client::new(&ctx.server_url)?;

    if args.list_files {
        match client.list_log_files().await {
            Ok(response) => {
                if ctx.json_output {
                    println!("{}", serde_json::to_string_pretty(&response.files)?);
                } else {
                    println!();
                    output::header("Server Log Files");
                    if response.files.is_empty() {
                        println!("  No log files found.");
                    } else {
                        for f in &response.files {
                            println!(
                                "  {} {}",
                                Style::new().cyan().apply_to(&f.name),
                                Style::new().dim().apply_to(format_size(f.size)),
                            );
                        }
                    }
                    println!();
                }
            }
            Err(e) => {
                super::print_cli_error(&e, &ctx.server_url, ctx.verbose);
            }
        }
        return Ok(());
    }

    match client
        .get_logs(Some(args.lines), args.file.as_deref())
        .await
    {
        Ok(response) => {
            if ctx.json_output {
                println!("{}", serde_json::to_string_pretty(&response)?);
                return Ok(());
            }

            println!(
                "{}",
                Style::new().dim().apply_to(format!(
                    "── {} ({} lines) ──",
                    response.file, response.count
                ))
            );
            for entry in &response.entries {
                print_log_line(&entry.line);
            }
        }
        Err(e) => {
            super::print_cli_error(&e, &ctx.server_url, ctx.verbose);
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn print_log_line(line: &str) {
    // Strip ANSI escape codes for cleaner output, then print
    let stripped = strip_ansi_escapes(line);
    println!("{}", stripped);
}

/// Simple ANSI escape code stripper.
fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if let Some(next) = chars.next()
                && next == '['
            {
                // CSI sequence — skip until we hit a letter
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // else: skip the single char after ESC
        } else {
            result.push(c);
        }
    }
    result
}

//! Arawn - Personal Research Agent for Edge Computing
//!
//! Main entry point for the Arawn CLI.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod client;
mod commands;

use commands::{
    agent, ask, auth, chat, config, logs, mcp, memory, notes, plugin, secrets, session, start,
    status, tui,
};

// ─────────────────────────────────────────────────────────────────────────────
// CLI Structure
// ─────────────────────────────────────────────────────────────────────────────

/// Arawn - Personal Research Agent for Edge Computing
#[derive(Parser)]
#[command(name = "arawn")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m
  arawn start                       Launch the server with default settings
  arawn ask \"What is Rust?\"         One-shot question
  arawn chat                        Interactive chat session
  arawn status                      Check if the server is running
  arawn config show                 Show resolved configuration")]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output as JSON (for scripting)
    #[arg(long, global = true)]
    pub json: bool,

    /// Server URL (overrides current context)
    #[arg(long, global = true, env = "ARAWN_SERVER_URL")]
    pub server: Option<String>,

    /// Use a specific context instead of the current one
    #[arg(long, global = true)]
    pub context: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the Arawn server
    Start(start::StartArgs),

    /// Show server status and resource usage
    Status(status::StatusArgs),

    /// Ask a one-shot question
    Ask(ask::AskArgs),

    /// Enter interactive chat mode (REPL)
    Chat(chat::ChatArgs),

    /// Memory operations
    Memory(memory::MemoryArgs),

    /// Note management
    Notes(notes::NotesArgs),

    /// Configuration management
    Config(config::ConfigArgs),

    /// Authentication management
    Auth(auth::AuthArgs),

    /// Plugin management
    Plugin(plugin::PluginArgs),

    /// Subagent management
    Agent(agent::AgentArgs),

    /// MCP server management
    Mcp(mcp::McpArgs),

    /// Manage encrypted secret store
    Secrets(secrets::SecretsArgs),

    /// View and manage chat sessions
    Session(session::SessionArgs),

    /// View operational logs
    Logs(logs::LogsArgs),

    /// Launch Terminal UI
    Tui(tui::TuiArgs),
}

// ─────────────────────────────────────────────────────────────────────────────
// Server URL Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the server URL from various sources.
///
/// Priority order:
/// 1. CLI `--server` flag (already checked by clap via `cli.server`)
/// 2. `--context` flag → lookup in client config
/// 3. Current context from client config
/// 4. ARAWN_SERVER_URL environment variable (already checked by clap)
/// 5. Default: http://localhost:8080
fn resolve_server_url(server_flag: Option<&str>, context_flag: Option<&str>) -> String {
    // 1. Explicit --server flag takes priority
    if let Some(url) = server_flag {
        return url.to_string();
    }

    // Try to load client config
    let config = arawn_config::load_client_config().ok();

    // 2. Explicit --context flag
    if let Some(ctx_name) = context_flag
        && let Some(config) = &config
    {
        if let Some(ctx) = config.get_context(ctx_name) {
            return ctx.server.clone();
        }
        // Context not found — fall through to default
        tracing::warn!("Context '{}' not found, using default", ctx_name);
    }

    // 3. Current context from config
    if let Some(config) = &config
        && let Some(url) = config.current_server_url()
    {
        return url;
    }

    // 4. Default
    "http://localhost:8080".to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        // Log through tracing so the error appears in log files with timestamps.
        // tracing may not be initialized if the error occurred during setup,
        // but tracing::error! is a no-op in that case — safe to call unconditionally.
        tracing::error!("{:#}", e);

        // Also print to stderr for immediate visibility (console/launchd-stderr.log).
        let red = console::Style::new().red();
        eprintln!("{} {:#}", red.apply_to("Error:"), e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Ask Subcommand ──────────────────────────────────────────────

    #[test]
    fn test_ask_with_prompt() {
        let cli = Cli::try_parse_from(["arawn", "ask", "What is Rust?"]).unwrap();
        match cli.command {
            Commands::Ask(args) => {
                assert_eq!(args.prompt, "What is Rust?");
                assert!(args.session.is_none());
                assert!(!args.no_memory);
            }
            _ => panic!("Expected Ask command"),
        }
    }

    #[test]
    fn test_ask_with_session() {
        let cli = Cli::try_parse_from(["arawn", "ask", "-s", "sess-123", "Follow up"]).unwrap();
        match cli.command {
            Commands::Ask(args) => {
                assert_eq!(args.prompt, "Follow up");
                assert_eq!(args.session.as_deref(), Some("sess-123"));
            }
            _ => panic!("Expected Ask command"),
        }
    }

    #[test]
    fn test_ask_with_no_memory() {
        let cli = Cli::try_parse_from(["arawn", "ask", "--no-memory", "Quick question"]).unwrap();
        match cli.command {
            Commands::Ask(args) => {
                assert!(args.no_memory);
                assert_eq!(args.prompt, "Quick question");
            }
            _ => panic!("Expected Ask command"),
        }
    }

    #[test]
    fn test_ask_missing_prompt() {
        let result = Cli::try_parse_from(["arawn", "ask"]);
        assert!(result.is_err());
    }

    // ── Memory Subcommand ───────────────────────────────────────────

    #[test]
    fn test_memory_search() {
        let cli = Cli::try_parse_from(["arawn", "memory", "search", "Rust ownership"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Search { query, limit } => {
                    assert_eq!(query, "Rust ownership");
                    assert_eq!(limit, 10); // default
                }
                _ => panic!("Expected Search"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_search_with_limit() {
        let cli = Cli::try_parse_from(["arawn", "memory", "search", "test", "-l", "5"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Search { query, limit } => {
                    assert_eq!(query, "test");
                    assert_eq!(limit, 5);
                }
                _ => panic!("Expected Search"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_recent() {
        let cli = Cli::try_parse_from(["arawn", "memory", "recent"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Recent { limit } => {
                    assert_eq!(limit, 10); // default
                }
                _ => panic!("Expected Recent"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_recent_with_limit() {
        let cli = Cli::try_parse_from(["arawn", "memory", "recent", "-l", "3"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Recent { limit } => {
                    assert_eq!(limit, 3);
                }
                _ => panic!("Expected Recent"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_stats() {
        let cli = Cli::try_parse_from(["arawn", "memory", "stats"]).unwrap();
        match cli.command {
            Commands::Memory(args) => {
                assert!(matches!(args.command, memory::MemoryCommand::Stats));
            }
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_reindex_defaults() {
        let cli = Cli::try_parse_from(["arawn", "memory", "reindex"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Reindex { dry_run, yes } => {
                    assert!(!dry_run);
                    assert!(!yes);
                }
                _ => panic!("Expected Reindex"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_reindex_dry_run() {
        let cli = Cli::try_parse_from(["arawn", "memory", "reindex", "--dry-run"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Reindex { dry_run, yes } => {
                    assert!(dry_run);
                    assert!(!yes);
                }
                _ => panic!("Expected Reindex"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_reindex_yes() {
        let cli = Cli::try_parse_from(["arawn", "memory", "reindex", "-y"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Reindex { dry_run, yes } => {
                    assert!(!dry_run);
                    assert!(yes);
                }
                _ => panic!("Expected Reindex"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_export_no_output() {
        let cli = Cli::try_parse_from(["arawn", "memory", "export"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Export { output } => {
                    assert!(output.is_none());
                }
                _ => panic!("Expected Export"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_export_with_output() {
        let cli = Cli::try_parse_from(["arawn", "memory", "export", "-o", "out.json"]).unwrap();
        match cli.command {
            Commands::Memory(args) => match args.command {
                memory::MemoryCommand::Export { output } => {
                    assert_eq!(output.as_deref(), Some("out.json"));
                }
                _ => panic!("Expected Export"),
            },
            _ => panic!("Expected Memory command"),
        }
    }

    #[test]
    fn test_memory_search_missing_query() {
        let result = Cli::try_parse_from(["arawn", "memory", "search"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_missing_subcommand() {
        let result = Cli::try_parse_from(["arawn", "memory"]);
        assert!(result.is_err());
    }

    // ── Notes Subcommand ────────────────────────────────────────────

    #[test]
    fn test_notes_add() {
        let cli = Cli::try_parse_from(["arawn", "notes", "add", "Remember to refactor"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::Add { content, tags } => {
                    assert_eq!(content, "Remember to refactor");
                    assert!(tags.is_empty());
                }
                _ => panic!("Expected Add"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_add_with_tags() {
        let cli = Cli::try_parse_from([
            "arawn", "notes", "add", "Fix auth", "-t", "todo", "-t", "backend",
        ])
        .unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::Add { content, tags } => {
                    assert_eq!(content, "Fix auth");
                    assert_eq!(tags, vec!["todo", "backend"]);
                }
                _ => panic!("Expected Add"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_list_default() {
        let cli = Cli::try_parse_from(["arawn", "notes", "list"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::List { limit } => {
                    assert_eq!(limit, 20); // default
                }
                _ => panic!("Expected List"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_list_with_limit() {
        let cli = Cli::try_parse_from(["arawn", "notes", "list", "-l", "5"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::List { limit } => {
                    assert_eq!(limit, 5);
                }
                _ => panic!("Expected List"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_search() {
        let cli = Cli::try_parse_from(["arawn", "notes", "search", "refactor"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::Search { query } => {
                    assert_eq!(query, "refactor");
                }
                _ => panic!("Expected Search"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_show() {
        let cli = Cli::try_parse_from(["arawn", "notes", "show", "abc123"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::Show { id } => {
                    assert_eq!(id, "abc123");
                }
                _ => panic!("Expected Show"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_delete() {
        let cli = Cli::try_parse_from(["arawn", "notes", "delete", "abc123"]).unwrap();
        match cli.command {
            Commands::Notes(args) => match args.command {
                notes::NotesCommand::Delete { id } => {
                    assert_eq!(id, "abc123");
                }
                _ => panic!("Expected Delete"),
            },
            _ => panic!("Expected Notes command"),
        }
    }

    #[test]
    fn test_notes_add_missing_content() {
        let result = Cli::try_parse_from(["arawn", "notes", "add"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_notes_show_missing_id() {
        let result = Cli::try_parse_from(["arawn", "notes", "show"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_notes_delete_missing_id() {
        let result = Cli::try_parse_from(["arawn", "notes", "delete"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_notes_search_missing_query() {
        let result = Cli::try_parse_from(["arawn", "notes", "search"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_notes_missing_subcommand() {
        let result = Cli::try_parse_from(["arawn", "notes"]);
        assert!(result.is_err());
    }

    // ── Auth Subcommand ─────────────────────────────────────────────

    #[test]
    fn test_auth_login() {
        let cli = Cli::try_parse_from(["arawn", "auth", "login"]).unwrap();
        match cli.command {
            Commands::Auth(args) => {
                assert!(matches!(args.command, auth::AuthCommand::Login));
            }
            _ => panic!("Expected Auth command"),
        }
    }

    #[test]
    fn test_auth_status() {
        let cli = Cli::try_parse_from(["arawn", "auth", "status"]).unwrap();
        match cli.command {
            Commands::Auth(args) => {
                assert!(matches!(args.command, auth::AuthCommand::Status));
            }
            _ => panic!("Expected Auth command"),
        }
    }

    #[test]
    fn test_auth_logout() {
        let cli = Cli::try_parse_from(["arawn", "auth", "logout"]).unwrap();
        match cli.command {
            Commands::Auth(args) => {
                assert!(matches!(args.command, auth::AuthCommand::Logout));
            }
            _ => panic!("Expected Auth command"),
        }
    }

    #[test]
    fn test_auth_token_default() {
        let cli = Cli::try_parse_from(["arawn", "auth", "token"]).unwrap();
        match cli.command {
            Commands::Auth(args) => match args.command {
                auth::AuthCommand::Token { generate } => {
                    assert!(!generate);
                }
                _ => panic!("Expected Token"),
            },
            _ => panic!("Expected Auth command"),
        }
    }

    #[test]
    fn test_auth_token_generate() {
        let cli = Cli::try_parse_from(["arawn", "auth", "token", "--generate"]).unwrap();
        match cli.command {
            Commands::Auth(args) => match args.command {
                auth::AuthCommand::Token { generate } => {
                    assert!(generate);
                }
                _ => panic!("Expected Token"),
            },
            _ => panic!("Expected Auth command"),
        }
    }

    #[test]
    fn test_auth_missing_subcommand() {
        let result = Cli::try_parse_from(["arawn", "auth"]);
        assert!(result.is_err());
    }

    // ── Session Subcommand ──────────────────────────────────────────

    #[test]
    fn test_session_list() {
        let cli = Cli::try_parse_from(["arawn", "session", "list"]).unwrap();
        match cli.command {
            Commands::Session(args) => {
                assert!(matches!(args.command, session::SessionCommands::List));
            }
            _ => panic!("Expected Session command"),
        }
    }

    #[test]
    fn test_session_show() {
        let cli = Cli::try_parse_from(["arawn", "session", "show", "sess-abc"]).unwrap();
        match cli.command {
            Commands::Session(args) => match args.command {
                session::SessionCommands::Show { id } => {
                    assert_eq!(id, "sess-abc");
                }
                _ => panic!("Expected Show"),
            },
            _ => panic!("Expected Session command"),
        }
    }

    #[test]
    fn test_session_show_missing_id() {
        let result = Cli::try_parse_from(["arawn", "session", "show"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_missing_subcommand() {
        let result = Cli::try_parse_from(["arawn", "session"]);
        assert!(result.is_err());
    }

    // ── Global Flags ────────────────────────────────────────────────

    #[test]
    fn test_global_verbose_flag() {
        let cli = Cli::try_parse_from(["arawn", "-v", "ask", "test"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_global_json_flag() {
        let cli = Cli::try_parse_from(["arawn", "--json", "ask", "test"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_global_server_flag() {
        let cli =
            Cli::try_parse_from(["arawn", "--server", "http://other:9090", "ask", "test"]).unwrap();
        assert_eq!(cli.server.as_deref(), Some("http://other:9090"));
    }

    #[test]
    fn test_global_context_flag() {
        let cli = Cli::try_parse_from(["arawn", "--context", "staging", "ask", "test"]).unwrap();
        assert_eq!(cli.context.as_deref(), Some("staging"));
    }

    #[test]
    fn test_no_command() {
        let result = Cli::try_parse_from(["arawn"]);
        assert!(result.is_err());
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Check if running TUI - need to skip console logging to avoid corrupting display
    let is_tui = matches!(cli.command, Commands::Tui(_));

    // Initialize tracing — console (human-readable) + rotating JSON file
    let filter = if cli.verbose {
        "arawn=debug,arawn_agent=debug,arawn_llm=debug,arawn_server=debug,arawn_oauth=debug,arawn_config=debug,info"
    } else {
        "arawn=info,arawn_agent=info,arawn_llm=info,arawn_server=info,arawn_oauth=info,warn"
    };

    let log_dir = arawn_config::xdg_config_dir()
        .map(|d| d.join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    let file_appender = tracing_appender::rolling::daily(&log_dir, "arawn.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::prelude::*;

    if is_tui {
        // TUI mode: tracing is set up by the TUI itself with a log buffer
        // Don't initialize here - the TUI will handle it
    } else {
        // Normal mode: console + file logging
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_filter(tracing_subscriber::EnvFilter::new(filter)),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_filter(tracing_subscriber::EnvFilter::new(
                        "arawn=trace,arawn_agent=trace,arawn_llm=trace,arawn_server=trace,arawn_oauth=trace,arawn_config=trace,info"
                    )),
            )
            .init();
    }

    // Get server URL: CLI flag > context > env var > default
    let server_url = resolve_server_url(cli.server.as_deref(), cli.context.as_deref());

    // Create context for commands
    let ctx = commands::Context {
        server_url,
        json_output: cli.json,
        verbose: cli.verbose,
    };

    // Dispatch to command handlers
    match cli.command {
        Commands::Start(args) => start::run(args, &ctx).await,
        Commands::Status(args) => status::run(args, &ctx).await,
        Commands::Ask(args) => ask::run(args, &ctx).await,
        Commands::Chat(args) => chat::run(args, &ctx).await,
        Commands::Memory(args) => memory::run(args, &ctx).await,
        Commands::Notes(args) => notes::run(args, &ctx).await,
        Commands::Config(args) => config::run(args, &ctx).await,
        Commands::Auth(args) => auth::run(args, &ctx).await,
        Commands::Plugin(args) => plugin::run(args, &ctx).await,
        Commands::Agent(args) => agent::run(args, &ctx).await,
        Commands::Mcp(args) => mcp::run(args, &ctx).await,
        Commands::Secrets(args) => secrets::run(args).await,
        Commands::Session(args) => session::run(args, &ctx).await,
        Commands::Logs(args) => logs::run(args, &ctx).await,
        Commands::Tui(args) => tui::run(args, &ctx).await,
    }
}

//! Session command - view and manage chat sessions.

use anyhow::Result;
use clap::{Args, Subcommand};
use console::Style;

use super::Context;
use super::output;
use crate::client::Client;

/// Arguments for the session command.
#[derive(Args, Debug)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m
  arawn session list                  List all sessions
  arawn session show <id>             Show session history with tool calls
  arawn session show <id> --json      Machine-readable output")]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommands,
}

#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// List all sessions
    List,
    /// Show session conversation history including tool calls
    Show {
        /// Session ID
        id: String,
    },
}

/// Run the session command.
pub async fn run(args: SessionArgs, ctx: &Context) -> Result<()> {
    match args.command {
        SessionCommands::List => list_sessions(ctx).await,
        SessionCommands::Show { id } => show_session(&id, ctx).await,
    }
}

async fn list_sessions(ctx: &Context) -> Result<()> {
    let client = Client::new(&ctx.server_url)?;

    match client.list_sessions().await {
        Ok(sessions) => {
            if ctx.json_output {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                println!();
                output::header("Sessions");

                if sessions.is_empty() {
                    println!("  No sessions found.");
                } else {
                    for s in &sessions {
                        println!(
                            "  {} {} ({} messages)",
                            Style::new().cyan().apply_to(&s.id),
                            Style::new().dim().apply_to(&s.created_at),
                            s.message_count,
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

    Ok(())
}

async fn show_session(session_id: &str, ctx: &Context) -> Result<()> {
    let client = Client::new(&ctx.server_url)?;

    match client.get_session_messages(session_id).await {
        Ok(response) => {
            if ctx.json_output {
                println!("{}", serde_json::to_string_pretty(&response.messages)?);
                return Ok(());
            }

            println!();
            output::header(&format!("Session {}", session_id));
            println!("  {} messages\n", response.count,);

            for msg in &response.messages {
                match msg.role.as_str() {
                    "user" => {
                        println!(
                            "{} {}",
                            Style::new().cyan().bold().apply_to("> "),
                            Style::new().white().apply_to(&msg.content),
                        );
                    }
                    "assistant" => {
                        for line in msg.content.lines() {
                            println!("  {}", Style::new().apply_to(line));
                        }
                    }
                    "tool_use" => {
                        if let Some(ref meta) = msg.metadata {
                            let name = meta.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                            let args = meta
                                .get("arguments")
                                .map(|v| {
                                    let s = v.to_string();
                                    output::truncate(&s, 80)
                                })
                                .unwrap_or_default();
                            println!(
                                "  {} {} {}",
                                Style::new().dim().apply_to("┄┄"),
                                Style::new().yellow().apply_to(name),
                                Style::new().dim().apply_to(args),
                            );
                        }
                    }
                    "tool_result" => {
                        if let Some(ref meta) = msg.metadata {
                            let success = meta
                                .get("success")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let marker = if success {
                                Style::new().green().apply_to("✓")
                            } else {
                                Style::new().red().apply_to("✗")
                            };
                            let preview = output::truncate(&msg.content, 120);
                            if !preview.is_empty() {
                                println!(
                                    "  {} {} {}",
                                    Style::new().dim().apply_to("  "),
                                    marker,
                                    Style::new().dim().apply_to(preview),
                                );
                            } else {
                                println!("  {} {}", Style::new().dim().apply_to("  "), marker,);
                            }
                        }
                    }
                    _ => {
                        println!("  [{}] {}", msg.role, output::truncate(&msg.content, 100));
                    }
                }
                println!();
            }
        }
        Err(e) => {
            super::print_cli_error(&e, &ctx.server_url, ctx.verbose);
        }
    }

    Ok(())
}

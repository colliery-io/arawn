//! Arawn Terminal UI — bare-bones chat interface over WebSocket.

pub mod app;
pub mod config;
pub mod events;
pub mod protocol;
pub mod render;
pub mod ws;

pub use config::TuiConfig;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

/// Run the TUI with the given configuration.
///
/// This is the main entry point called by the CLI crate.
pub async fn run_with_config(config: TuiConfig) -> Result<()> {
    // Connect to WebSocket
    let ws_url = config.ws_url();
    let (ws_tx, ws_rx, _status_rx) = ws::connect(ws_url);

    // Create app
    let mut app = app::App::new(&config, ws_tx);

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Start event handler
    let event_rx = events::spawn();

    // Run the app
    let result = app.run(&mut terminal, event_rx, ws_rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

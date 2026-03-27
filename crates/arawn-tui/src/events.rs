//! Terminal event handler (key events + tick).

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc;

/// Application events.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key press.
    Key(KeyEvent),
    /// A periodic tick (drives UI updates).
    Tick,
}

/// Spawn an event-reading loop that sends events via an unbounded channel.
///
/// Returns the receiver. The background task reads crossterm events with a
/// 100ms tick interval.
pub fn spawn() -> mpsc::UnboundedReceiver<Event> {
    let (tx, rx) = mpsc::unbounded_channel();
    let tick_rate = Duration::from_millis(100);

    tokio::spawn(async move {
        loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(evt) = event::read() {
                    match evt {
                        CrosstermEvent::Key(key) => {
                            if tx.send(Event::Key(key)).is_err() {
                                return;
                            }
                        }
                        _ => {} // Ignore mouse, resize, etc. for now
                    }
                }
            } else {
                // No event within tick_rate — send a tick
                if tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        }
    });

    rx
}

/// Create a channel pair for injecting events in tests.
pub fn test_channel() -> (mpsc::UnboundedSender<Event>, mpsc::UnboundedReceiver<Event>) {
    mpsc::unbounded_channel()
}

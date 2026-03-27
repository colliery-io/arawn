//! Terminal event handler (key events + tick).

use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

/// Application events.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key press.
    Key(KeyEvent),
    /// Pasted text.
    Paste(String),
    /// A periodic tick (drives UI updates).
    Tick,
}

/// Spawn an event-reading loop that sends events via an unbounded channel.
///
/// Uses crossterm's async `EventStream` instead of synchronous poll/read
/// to avoid blocking the tokio runtime.
pub fn spawn() -> mpsc::UnboundedReceiver<Event> {
    let (tx, rx) = mpsc::unbounded_channel();
    let tick_rate = Duration::from_millis(100);

    tokio::spawn(async move {
        let mut reader = EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);

        loop {
            tokio::select! {
                maybe_event = reader.next() => {
                    match maybe_event {
                        Some(Ok(CrosstermEvent::Key(key))) => {
                            if tx.send(Event::Key(key)).is_err() {
                                return;
                            }
                        }
                        Some(Ok(CrosstermEvent::Paste(text))) => {
                            if tx.send(Event::Paste(text)).is_err() {
                                return;
                            }
                        }
                        Some(Ok(_)) => {} // Ignore mouse, resize, focus, etc.
                        Some(Err(e)) => {
                            tracing::warn!("Error reading terminal event: {}", e);
                            return;
                        }
                        None => return, // Stream ended
                    }
                }
                _ = tick.tick() => {
                    if tx.send(Event::Tick).is_err() {
                        return;
                    }
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

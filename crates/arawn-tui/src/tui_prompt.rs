//! TUI implementation of the ModalPrompt trait.
//!
//! Sends modal requests through a channel to the TUI event loop,
//! which displays the InteractiveModal and sends the result back
//! via a oneshot channel.

use arawn_engine::permissions::{ModalPrompt, ModalRequest};
use async_trait::async_trait;
use ratatui::style::Color;
use tokio::sync::mpsc;

use crate::modal::{ModalOption, ModalState};

/// A request to show a modal in the TUI event loop.
pub struct TuiModalRequest {
    pub modal: ModalState,
}

/// TUI-based modal prompt. Sends modal requests to the event loop
/// and awaits the user's response via a oneshot channel.
pub struct TuiModalPrompt {
    tx: mpsc::Sender<TuiModalRequest>,
}

impl TuiModalPrompt {
    pub fn new(tx: mpsc::Sender<TuiModalRequest>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl ModalPrompt for TuiModalPrompt {
    async fn prompt(&self, request: ModalRequest) -> Option<usize> {
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // Convert engine ModalOptions to TUI ModalOptions
        let options: Vec<ModalOption> = request
            .options
            .iter()
            .map(|opt| {
                let mut mo = ModalOption::new(&opt.label);
                if let Some(ref desc) = opt.description {
                    mo = mo.with_description(desc);
                }
                mo
            })
            .collect();

        let mut modal = ModalState::new(
            &request.title,
            options,
            Color::Yellow,
            result_tx,
        );

        if let Some(subtitle) = request.subtitle {
            modal = modal.with_subtitle(subtitle);
        }

        if self.tx.send(TuiModalRequest { modal }).await.is_err() {
            return None; // Event loop gone
        }

        result_rx.await.unwrap_or(None)
    }
}

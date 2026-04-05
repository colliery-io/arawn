//! ModalPrompt implementation that sends prompts through an EngineEvent channel
//! and receives responses from a shared pending-responses map.
//!
//! Flow:
//! 1. Engine calls prompt() → sends EngineEvent::ModalPrompt via tx
//! 2. WS server forwards to client
//! 3. Client shows modal, user selects
//! 4. Client sends modal_response via WS
//! 5. WS server resolves the pending oneshot via the shared map
//! 6. prompt() returns the selected index

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use arawn_engine::permissions::{ModalOption, ModalPrompt, ModalRequest};
use arawn_service::{EngineEvent, ModalPromptOption};

/// Shared map of pending modal responses. The WS server inserts responses here.
pub type PendingModals = Arc<Mutex<HashMap<String, oneshot::Sender<Option<usize>>>>>;

/// Create a new empty pending modals map.
pub fn new_pending_modals() -> PendingModals {
    Arc::new(Mutex::new(HashMap::new()))
}

/// ModalPrompt that sends via an EngineEvent channel and waits for response.
pub struct ChannelModalPrompt {
    tx: mpsc::Sender<EngineEvent>,
    pending: PendingModals,
}

impl ChannelModalPrompt {
    pub fn new(tx: mpsc::Sender<EngineEvent>, pending: PendingModals) -> Self {
        Self { tx, pending }
    }
}

#[async_trait]
impl ModalPrompt for ChannelModalPrompt {
    async fn prompt(&self, request: ModalRequest) -> Option<usize> {
        let request_id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();

        // Register the pending response
        self.pending
            .lock()
            .unwrap()
            .insert(request_id.clone(), response_tx);

        // Convert engine ModalOptions to service ModalPromptOptions
        let options: Vec<ModalPromptOption> = request
            .options
            .iter()
            .map(|opt| ModalPromptOption {
                label: opt.label.clone(),
                description: opt.description.clone(),
            })
            .collect();

        // Send the event — engine is effectively paused until response arrives
        let event = EngineEvent::UserInputRequest {
            request_id: request_id.clone(),
            title: request.title,
            subtitle: request.subtitle,
            options,
        };

        if self.tx.send(event).await.is_err() {
            // Channel closed — clean up and deny
            self.pending.lock().unwrap().remove(&request_id);
            return None;
        }

        // Also send a Flush so the client renders the modal immediately
        let _ = self.tx.send(EngineEvent::Flush).await;

        // Wait for the response
        response_rx.await.unwrap_or(None)
    }
}

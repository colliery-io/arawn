//! `LocalService` inherent methods backing the `sessions.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use std::pin::Pin;

use arawn_core::{Message, Session};
use arawn_service::{
    ArawnService, EngineEvent,
    PromotionResult, ServiceError, SessionDetail, SessionInfo,
};
use arawn_storage::JsonlMessageStore;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;


use super::{LocalService, resolve_ws_dir_from_store};

impl LocalService {
    pub(super) async fn list_sessions_inner(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<Vec<SessionInfo>, ServiceError> {
        let store = self.store.lock().unwrap();
        let metas = match workstream_id {
            Some(ws_id) => store.list_sessions_for_workstream(ws_id),
            None => store.list_scratch_sessions(),
        }?;

        Ok(metas
            .into_iter()
            .map(|m| SessionInfo {
                id: m.id,
                workstream_id: m.workstream_id,
                created_at: m.created_at,
            })
            .collect())
    }

    pub(super) async fn create_session_inner(
        &self,
        workstream_id: Option<Uuid>,
    ) -> Result<SessionInfo, ServiceError> {
        let session = match workstream_id {
            Some(ws_id) => Session::new(ws_id),
            None => Session::scratch(),
        };

        {
            let store = self.store.lock().unwrap();
            store.create_session(&session)?;
        }

        info!(session_id = %session.id, "session created via service");

        // SessionStart hook — I-0056 T-C. Source = "startup" since this
        // is a fresh session create (vs "resume" on load_session). The
        // hook fires after the session is persisted but before the
        // response goes back to the caller.
        if let Some(ref runner) = self.hook_runner {
            let hook_input = arawn_engine::hooks::HookInput::SessionStart {
                session_id: session.id.to_string(),
                cwd: self.data_dir.display().to_string(),
                source: "startup".to_string(),
                metadata: std::collections::HashMap::new(),
            };
            let _ = runner.run(&hook_input).await;
        }

        Ok(SessionInfo {
            id: session.id,
            workstream_id: session.workstream_id(),
            created_at: session.created_at,
        })
    }

    pub(super) async fn load_session_inner(&self, id: Uuid) -> Result<SessionDetail, ServiceError> {
        // Get metadata from SQLite (sync, hold lock briefly)
        let (meta, ws_dir) = {
            let store = self.store.lock().unwrap();
            let meta = store
                .get_session_meta(id)?
                .ok_or_else(|| ServiceError::NotFound(format!("session {id}")))?;

            let ws_dir = resolve_ws_dir_from_store(&store, meta.workstream_id)?;
            (meta, ws_dir)
        };

        // Load messages from JSONL (async, no lock needed)
        let msg_store = JsonlMessageStore::new(&self.data_dir);
        let all_messages = msg_store.load(id, &ws_dir).await?;
        let messages = Session::load_compacted(all_messages);

        Ok(SessionDetail {
            id: meta.id,
            workstream_id: meta.workstream_id,
            created_at: meta.created_at,
            messages,
        })
    }

    pub(super) async fn truncate_session_at_user_message_inner(
        &self,
        id: Uuid,
        user_message_index: usize,
    ) -> Result<SessionDetail, ServiceError> {
        // Refuse if a generation is in flight on this session — truncating
        // mid-stream would corrupt persistent state.
        {
            let active = self.active_sessions.lock().unwrap();
            if active.contains(&id) {
                return Err(ServiceError::InvalidOperation(
                    "Session is currently processing a message; cancel first.".into(),
                ));
            }
        }

        let (_meta, ws_dir) = {
            let store = self.store.lock().unwrap();
            let meta = store
                .get_session_meta(id)?
                .ok_or_else(|| ServiceError::NotFound(format!("session {id}")))?;
            let ws_dir = resolve_ws_dir_from_store(&store, meta.workstream_id)?;
            (meta, ws_dir)
        };

        let msg_store = JsonlMessageStore::new(&self.data_dir);
        let all_messages = msg_store.load(id, &ws_dir).await?;

        // Walk to the Nth user message and stop at its index — that's
        // the count of messages we want to keep before it.
        let mut user_count = 0usize;
        let mut keep_count = all_messages.len(); // default no-op
        for (i, msg) in all_messages.iter().enumerate() {
            if matches!(msg, arawn_core::Message::User { .. }) {
                if user_count == user_message_index {
                    keep_count = i;
                    break;
                }
                user_count += 1;
            }
        }

        if keep_count < all_messages.len() {
            msg_store.truncate(id, &ws_dir, keep_count).await?;
        }

        // Re-load the truncated session for return.
        self.load_session(id).await
    }

    #[instrument(skip_all, fields(%session_id))]
    pub(super) async fn send_message_inner(
        &self,
        session_id: Uuid,
        content: String,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = EngineEvent> + Send>>, ServiceError> {
        // Prevent concurrent send_message calls to the same session
        {
            let mut active = self.active_sessions.lock().unwrap();
            if !active.insert(session_id) {
                return Err(ServiceError::InvalidOperation(
                    "Session is currently processing a message. Wait for the current request to complete.".into(),
                ));
            }
        }

        // Load session state
        let (meta, workstream, ws_dir, _) = self.load_session_state(session_id)?;

        // Load messages from JSONL
        let msg_store = JsonlMessageStore::new(&self.data_dir);
        let all_messages = msg_store.load(session_id, &ws_dir).await?;
        let messages = Session::load_compacted(all_messages);

        let mut session =
            Session::from_parts(meta.id, meta.workstream_id, meta.created_at, messages);

        // Add user message and persist
        let user_msg = Message::User {
            content: content.clone(),
        };
        session.add_message(user_msg.clone());

        let message_store = JsonlMessageStore::new(&self.data_dir);
        message_store.append(session_id, &ws_dir, &user_msg).await?;

        // Resolve workspace directory
        let is_scratch = workstream.name == "scratch";
        let workspace_dir = msg_store.sandbox_dir(&ws_dir, session_id, is_scratch);
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .map_err(arawn_storage::StorageError::from)?;

        // Build context and engine
        let ws_dir_owned = ws_dir.clone();
        let (ctx, prompt_context) =
            self.build_session_context(session_id, &workstream, &ws_dir, &workspace_dir, &content);

        let msgs_before = session.messages().len();
        let (tx, rx) = mpsc::channel::<EngineEvent>(64);

        let mut engine = self.build_engine(prompt_context, &tx);

        // Set up live progress channel
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::channel::<arawn_engine::ProgressEvent>(64);
        engine = engine.with_progress_sender(progress_tx);

        // Create cancellation token for this session
        let cancel_token = tokio_util::sync::CancellationToken::new();
        engine = engine.with_cancel_token(cancel_token.clone());
        self.cancel_tokens
            .lock()
            .unwrap()
            .insert(session_id, cancel_token);

        let data_dir = self.data_dir.clone();
        let store = self.store.clone();
        let active_sessions = self.active_sessions.clone();
        let cancel_tokens = self.cancel_tokens.clone();

        tokio::spawn(async move {
            let msg_store = JsonlMessageStore::new(&data_dir);

            // Forward progress events inline (same task as engine, no race condition).
            // We drain the progress channel after each await point in the engine loop
            // by spawning a forwarder that the engine feeds into.
            let event_tx_progress = tx.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(event) = progress_rx.recv().await {
                    match event {
                        arawn_engine::ProgressEvent::AssistantText { content } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::StreamingText { text: content })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                        arawn_engine::ProgressEvent::ToolCallStart { id, name, input } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::ToolCallStart { id, name, input })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                        arawn_engine::ProgressEvent::ToolCallResult {
                            id,
                            content,
                            is_error,
                        } => {
                            let _ = event_tx_progress
                                .send(EngineEvent::ToolCallResult {
                                    id,
                                    content,
                                    is_error,
                                })
                                .await;
                            let _ = event_tx_progress.send(EngineEvent::Flush).await;
                        }
                    }
                }
            });

            let engine_result = engine.run(&mut session, &ctx).await;

            // Drop the engine to release progress_tx — this closes the channel
            // so the forwarder can drain remaining events and exit.
            drop(engine);
            let _ = forwarder.await;

            match engine_result {
                Ok(final_text) => {
                    // Tool call events were already streamed live via progress_tx.
                    // Only emit compaction events here (not streamed live).
                    let new_msgs = &session.messages()[msgs_before..];
                    for msg in new_msgs {
                        if let Message::Summary { original_count, .. } = msg {
                            let _ = tx
                                .send(EngineEvent::CompactionOccurred {
                                    messages_summarized: *original_count,
                                })
                                .await;
                            let _ = tx.send(EngineEvent::Flush).await;
                        }
                    }

                    // Persist new messages
                    let mut persist_errors = Vec::new();
                    for msg in &session.messages()[msgs_before..] {
                        if let Err(e) = msg_store.append(session_id, &ws_dir_owned, msg).await {
                            error!(error = %e, "failed to persist message");
                            persist_errors.push(e.to_string());
                        }
                    }

                    // Persist stats
                    if let Ok(s) = store.lock()
                        && let Err(e) = s.update_session_stats(session_id, &session.stats)
                    {
                        warn!(error = %e, "failed to update session stats");
                    }

                    let _ = tx
                        .send(EngineEvent::Usage {
                            input_tokens: session.stats.input_tokens,
                            output_tokens: session.stats.output_tokens,
                        })
                        .await;

                    // Surface persistence failures as warnings before Complete
                    if !persist_errors.is_empty() {
                        let _ = tx.send(EngineEvent::Warning {
                            message: format!(
                                "Some messages could not be saved to disk ({} error{}). Your conversation may not survive a restart.",
                                persist_errors.len(),
                                if persist_errors.len() == 1 { "" } else { "s" }
                            ),
                        }).await;
                    }

                    let _ = tx.send(EngineEvent::Complete { final_text }).await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
                Err(e) => {
                    error!(%session_id, error = %e, "engine turn failed");
                    for msg in &session.messages()[msgs_before..] {
                        if let Err(pe) = msg_store.append(session_id, &ws_dir_owned, msg).await {
                            error!(error = %pe, "failed to persist message in error path");
                        }
                    }
                    if let Ok(s) = store.lock()
                        && let Err(se) = s.update_session_stats(session_id, &session.stats)
                    {
                        warn!(error = %se, "failed to update session stats in error path");
                    }
                    let _ = tx
                        .send(EngineEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                    let _ = tx.send(EngineEvent::Flush).await;
                }
            }

            // Release the session lock and cancel token so new messages can be sent
            active_sessions.lock().unwrap().remove(&session_id);
            cancel_tokens.lock().unwrap().remove(&session_id);
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    pub(super) async fn cancel_inner(&self, session_id: Uuid) -> Result<(), ServiceError> {
        let token = self.cancel_tokens.lock().unwrap().get(&session_id).cloned();
        match token {
            Some(token) => {
                info!(%session_id, "cancelling engine run");
                token.cancel();
                Ok(())
            }
            None => {
                debug!(%session_id, "cancel requested but no active engine run");
                Ok(())
            }
        }
    }

    pub(super) async fn promote_session_inner(
        &self,
        session_id: Uuid,
        workstream_name: &str,
    ) -> Result<PromotionResult, ServiceError> {
        let (ws_id, ws_name, ws_dir, scratch_workspace, target_workspace) = {
            let store = self.store.lock().unwrap();
            let ws = store
                .find_workstream_by_name(workstream_name)?
                .ok_or_else(|| ServiceError::NotFound(format!("workstream '{workstream_name}'")))?;

            let ws_dir = arawn_storage::workstream_dir_name(&ws.name, ws.id);
            let scratch_ws = store
                .sandbox_for("scratch", session_id, true)
                .join("workspace");
            let target_ws = store
                .sandbox_for(&ws_dir, session_id, false)
                .join("workspace");

            (ws.id, ws.name, ws_dir, scratch_ws, target_ws)
        };

        let msg_store = arawn_storage::JsonlMessageStore::new(&self.data_dir);
        msg_store
            .move_session(session_id, "scratch", &ws_dir)
            .await?;

        let sqlite_result = {
            let store = self.store.lock().unwrap();
            store.promote_session_metadata(session_id, ws_id)
        };
        if let Err(e) = sqlite_result {
            warn!(error = %e, "SQLite update failed during promotion, rolling back file move");
            let _ = msg_store.move_session(session_id, &ws_dir, "scratch").await;
            return Err(e.into());
        }

        if scratch_workspace.exists() {
            let _ =
                tokio::fs::create_dir_all(target_workspace.parent().unwrap_or(&target_workspace))
                    .await;
            if let Err(e) = tokio::fs::rename(&scratch_workspace, &target_workspace).await {
                warn!(error = %e, "workspace rename failed during promotion, files remain in scratch");
            }
        }

        Ok(PromotionResult {
            workstream_id: ws_id.to_string(),
            workstream_name: ws_name,
        })
    }

    pub(super) async fn resolve_user_input_inner(
        &self,
        request_id: &str,
        selected_index: Option<usize>,
    ) -> Result<(), ServiceError> {
        let mut pending = self.pending_modals.lock().unwrap();
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(selected_index);
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!(
                "no pending modal for {request_id}"
            )))
        }
    }

}

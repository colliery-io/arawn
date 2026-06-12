//! `LocalService` inherent methods backing the `health` + `status` RPCs
//! (ARAWN-I-0068 P2-1). The trait shell in `super::mod` delegates here.
//!
//! Protocol-first (ADR ARAWN-A-0005): these build the `HealthStatus` /
//! `SystemStatus` contracts that the TUI panel and the future web GUI both
//! render. The aggregation reads live `LocalService` state and never holds a
//! subsystem lock across an `.await`.

use std::sync::atomic::Ordering;

use crate::lock_ext::Recover;
use arawn_service::{
    CeremoniesStatus, CeremonyRunStatus, EmbeddingStatus, ExtractionCursor, ExtractionStatus,
    FeedStatusRow, FeedsStatus, HealthStatus, LlmClientStatus, LlmStatus, SYSTEM_STATUS_VERSION,
    ServiceError, StewardErrorStatus, StewardStatus, SystemStatus,
};

use super::LocalService;

impl LocalService {
    /// Cheap readiness probe. `ready` is the one-way startup flag; while it
    /// is false the gate reports a single human-readable blocking reason.
    pub(super) async fn health_inner(&self) -> Result<HealthStatus, ServiceError> {
        let ready = self.is_ready();
        let blocking = if ready {
            Vec::new()
        } else {
            vec![
                "server is still initializing (feeds/ceremonies/memory/storage wiring in progress)"
                    .to_string(),
            ]
        };
        Ok(HealthStatus { ready, blocking })
    }

    /// Aggregate the per-subsystem health dump. Each block degrades
    /// independently: a subsystem that's absent or errors reports
    /// `available: false` / `None` rather than failing the whole call.
    pub(super) async fn status_inner(&self) -> Result<SystemStatus, ServiceError> {
        Ok(SystemStatus {
            version: SYSTEM_STATUS_VERSION,
            feeds: self.feeds_status().await,
            ceremonies: self.ceremonies_status(),
            embedding: self.embedding_status(),
            extraction: self.extraction_status(),
            llm: self.llm_status(),
            steward: self.steward_status(),
        })
    }

    async fn feeds_status(&self) -> FeedsStatus {
        let available = self.feed_runtime.read().recover().is_some();
        if !available {
            return FeedsStatus {
                available: false,
                feeds: Vec::new(),
            };
        }
        // Reuse the same summary path `/feeds` renders.
        let feeds = match self.feed_list_inner().await {
            Ok(list) => list
                .into_iter()
                .map(|f| FeedStatusRow {
                    id: f.id,
                    template: f.template,
                    enabled: f.enabled,
                    last_run_at: f.last_run_at,
                    last_status: f.last_status,
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        FeedsStatus {
            available: true,
            feeds,
        }
    }

    fn ceremonies_status(&self) -> CeremoniesStatus {
        // Latest dispatch outcome per ceremony, from the persisted run
        // history (ARAWN-T-0477) — independent of whether the engine is
        // currently wired (the history outlives a single process).
        let recent_runs = {
            let store = self.store.lock().recover();
            arawn_storage::failure_history::latest_ceremony_runs(store.database().conn())
                .map(|rows| {
                    rows.into_iter()
                        .map(|r| CeremonyRunStatus {
                            kind: r.kind,
                            period_key: r.period_key,
                            outcome: r.outcome,
                            error: r.error,
                            ran_at: r.ran_at,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        match self.ceremony_service() {
            Some(svc) => CeremoniesStatus {
                available: true,
                pending_notifications: svc.list_notifications().ok().map(|n| n.len() as u64),
                recent_runs,
            },
            None => CeremoniesStatus {
                available: false,
                pending_notifications: None,
                recent_runs,
            },
        }
    }

    fn steward_status(&self) -> StewardStatus {
        // Recent steward subroutine failures from the persisted error log
        // (ARAWN-T-0477). The journal stays success-only; this is the error
        // side-channel.
        let recent_errors = {
            let store = self.store.lock().recover();
            arawn_storage::failure_history::recent_steward_errors(store.database().conn(), 20)
                .map(|rows| {
                    rows.into_iter()
                        .map(|r| StewardErrorStatus {
                            lens: r.lens_name,
                            subroutine: r.subroutine,
                            error: r.error,
                            failed_at: r.failed_at,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        StewardStatus { recent_errors }
    }

    fn embedding_status(&self) -> EmbeddingStatus {
        let embedder_loaded = self
            .memory_manager
            .as_ref()
            .map(|m| m.embedder().is_some())
            .unwrap_or(false);
        let pending = self
            .projections
            .read()
            .recover()
            .as_ref()
            .and_then(|p| p.pending_embedding_count().ok());
        EmbeddingStatus {
            embedder_loaded,
            pending,
        }
    }

    fn extraction_status(&self) -> ExtractionStatus {
        let available = self.extractor_available.load(Ordering::SeqCst);
        let cursors = {
            let store = self.store.lock().recover();
            let cursor_store = arawn_storage::ExtractorCursorStore::new(store.database());
            cursor_store
                .list_all()
                .map(|rows| {
                    rows.into_iter()
                        .map(|c| ExtractionCursor {
                            lens: c.lens_name,
                            feed_type: c.feed_type,
                            cursor_ts: c.last_source_ts.map(|t| t.to_rfc3339()),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        ExtractionStatus { available, cursors }
    }

    fn llm_status(&self) -> LlmStatus {
        let clients = self
            .llm_pool
            .entries()
            .map(|(name, cfg)| LlmClientStatus {
                role: name.clone(),
                provider: cfg.provider.clone(),
                model: cfg.model.clone(),
            })
            .collect();
        // v1: no live reachability probe — keep `status` cheap and
        // non-blocking. Misconfiguration is still visible via `clients`.
        LlmStatus {
            clients,
            engine_reachable: None,
        }
    }
}

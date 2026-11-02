//! `LocalService` inherent methods backing the `integrations.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use crate::lock_ext::Recover;
use std::sync::Arc;

use arawn_service::ServiceError;

use super::{LocalService, OAuthFlowCtx, default_feed_for_service};

impl LocalService {
    pub(super) async fn list_integrations_inner(
        &self,
    ) -> Result<Vec<arawn_service::IntegrationStatus>, ServiceError> {
        // Snapshot the registry to a Vec before awaiting on each integration's
        // is_connected() check — don't hold the RwLock across await points.
        let entries: Vec<(String, Arc<dyn arawn_integrations::Integration>)> = self
            .integration_registry
            .read()
            .recover()
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect();

        let mut out = Vec::with_capacity(entries.len());
        for (name, integration) in entries {
            let connected = integration.is_connected().await;
            out.push(arawn_service::IntegrationStatus { name, connected });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub(super) async fn start_oauth_flow_inner(
        &self,
        service: &str,
    ) -> Result<arawn_service::OAuthFlowStarted, ServiceError> {
        let integration = self
            .integration_registry
            .read()
            .recover()
            .get(service)
            .cloned()
            .ok_or_else(|| {
                ServiceError::InvalidOperation(format!("no integration registered for '{service}'"))
            })?;

        // Per ARAWN-A-0001: the connect flow runs asynchronously on the
        // server. The TUI gets the auth URL back from this RPC and should
        // open it; the rest of the flow (callback wait, code exchange,
        // credential persistence) happens in the spawned task and emits a
        // ServerNotice on completion. The OAuthFlowStarted reply is shaped
        // to satisfy "give me a URL to open" without blocking the caller.
        //
        // The actual auth URL comes from the integration during connect()
        // via the ConnectContext's publish_auth_url. Since the RPC needs to
        // return synchronously with the URL, we use a oneshot to bridge:
        // the spawned task publishes the URL to the oneshot the moment the
        // integration calls publish_auth_url, then continues the flow.
        let (url_tx, url_rx) = tokio::sync::oneshot::channel::<url::Url>();
        let notice_tx = self.notice_tx.clone();
        let service_name = service.to_string();
        let integration_for_task = Arc::clone(&integration);
        let feed_runtime_for_task = self.feed_runtime.clone();
        let reconcile_ctx = self.reconcile_ctx();

        tokio::spawn(async move {
            let ctx = OAuthFlowCtx {
                service: service_name.clone(),
                url_tx: tokio::sync::Mutex::new(Some(url_tx)),
                notice_tx: notice_tx.clone(),
            };
            let result = integration_for_task.connect(&ctx).await;
            let succeeded = result.is_ok();
            let notice = match &result {
                Ok(()) => arawn_service::ServerNotice {
                    level: "info".into(),
                    category: "integration".into(),
                    message: format!("{service_name} connected"),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
                Err(e) => arawn_service::ServerNotice {
                    level: "error".into(),
                    category: "integration".into(),
                    message: format!("{service_name} connection FAILED: {}", e.user_message()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            };
            let _ = notice_tx.send(notice);

            // Auto-create the personal default feed for this service,
            // if one is defined and the feeds runtime is available. It
            // is a singleton per TEMPLATE: skip when any feed already
            // uses the template, under whatever id (ARAWN-T-0510).
            // Feeds declared in arawn.toml that waited for this service
            // are created now (ARAWN-T-0506). Runs before the default-feed
            // auto-create, so a declared feed of the same template wins
            // and the singleton check below skips the default.
            if succeeded {
                // Announce only what this reconcile changed. Standing
                // problems are in the startup log and `arawn doctor`;
                // repeating them on every connect would blame the connect.
                let report = reconcile_ctx.reconcile().await;
                for line in &report.done {
                    let _ = notice_tx.send(arawn_service::ServerNotice {
                        level: "info".into(),
                        category: "feeds".into(),
                        message: format!("arawn.toml: {line}"),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }

            if succeeded {
                let runtime = feed_runtime_for_task.read().recover().clone();
                if let Some(runtime) = runtime
                    && let Some((template, feed_id)) = default_feed_for_service(&service_name)
                {
                    let existing: Vec<(String, String)> = match runtime.list_summaries().await {
                        Ok(s) => s.into_iter().map(|f| (f.id, f.template)).collect(),
                        Err(e) => {
                            let _ = notice_tx.send(arawn_service::ServerNotice {
                                level: "warn".into(),
                                category: "feeds".into(),
                                message: format!(
                                    "auto-create {template} for {service_name} skipped: \
                                     cannot list feeds: {e}"
                                ),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            });
                            return;
                        }
                    };
                    if !super::should_auto_create(&existing, template) {
                        return;
                    }
                    match runtime
                        .register_feed_dynamic(
                            template,
                            feed_id,
                            arawn_feeds::TemplateParams::default(),
                            None,
                        )
                        .await
                    {
                        Ok(_) => {
                            let _ = notice_tx.send(arawn_service::ServerNotice {
                                level: "info".into(),
                                category: "feeds".into(),
                                message: format!(
                                    "auto-registered {template} as `{feed_id}` after \
                                     /connect {service_name}"
                                ),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            });
                        }
                        // Every failure is surfaced, including a UNIQUE
                        // clash: the template check above already covers
                        // "this feed exists", so a clash here means a
                        // different feed holds the id.
                        Err(e) => {
                            let _ = notice_tx.send(arawn_service::ServerNotice {
                                level: "warn".into(),
                                category: "feeds".into(),
                                message: format!(
                                    "auto-create {template} for {service_name} failed: {e}"
                                ),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            });
                        }
                    }
                }
            }
        });

        // Wait briefly for the integration to publish its auth URL.
        // Most OAuth integrations call publish_auth_url within milliseconds;
        // a 5s ceiling protects the RPC from hanging if an integration is
        // poorly behaved.
        let url = match tokio::time::timeout(std::time::Duration::from_secs(5), url_rx).await {
            Ok(Ok(u)) => u,
            Ok(Err(_)) => {
                return Err(ServiceError::InvalidOperation(format!(
                    "integration '{service}' did not publish an auth URL"
                )));
            }
            Err(_) => {
                return Err(ServiceError::InvalidOperation(format!(
                    "integration '{service}' did not publish an auth URL within 5s"
                )));
            }
        };

        Ok(arawn_service::OAuthFlowStarted {
            service: service.to_string(),
            auth_url: url.to_string(),
        })
    }

    pub(super) async fn disconnect_integration_inner(
        &self,
        service: &str,
    ) -> Result<(), ServiceError> {
        let integration = self
            .integration_registry
            .read()
            .recover()
            .get(service)
            .cloned()
            .ok_or_else(|| {
                ServiceError::InvalidOperation(format!("no integration registered for '{service}'"))
            })?;

        integration
            .disconnect()
            .await
            .map_err(|e| ServiceError::InvalidOperation(e.user_message()))?;

        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "integration".into(),
            message: format!("{service} disconnected"),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(())
    }
}

//! `LocalService` inherent methods backing the `integrations.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use crate::lock_ext::Recover;
use std::sync::Arc;

use arawn_integrations::Integration as _;
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

    /// What `/connect <service>` connects. A Slack workspace is built
    /// from the Slack app, registered or not.
    fn connect_target(&self, service: &str) -> Result<ConnectTarget, ServiceError> {
        if let Some(alias) = service.strip_prefix("slack@") {
            let app = self.slack_app.read().recover().clone().ok_or_else(|| {
                ServiceError::InvalidOperation("Slack is not set up. Run: arawn setup slack".into())
            })?;
            let ws = app
                .workspace(alias)
                .map_err(|e| ServiceError::InvalidOperation(e.user_message()))?;
            return Ok(ConnectTarget::SlackWorkspace(Arc::new(ws)));
        }
        self.integration_registry
            .read()
            .recover()
            .get(service)
            .cloned()
            .map(ConnectTarget::Registered)
            .ok_or_else(|| {
                ServiceError::InvalidOperation(format!("no integration registered for '{service}'"))
            })
    }

    pub(super) async fn start_oauth_flow_inner(
        &self,
        service: &str,
    ) -> Result<arawn_service::OAuthFlowStarted, ServiceError> {
        let target = self.connect_target(service)?;

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
        let (url_tx, url_rx) = tokio::sync::oneshot::channel::<Result<url::Url, String>>();
        let notice_tx = self.notice_tx.clone();
        let service_name = service.to_string();
        let feed_runtime_for_task = self.feed_runtime.clone();
        let reconcile_ctx = self.reconcile_ctx();
        let integrations = Arc::clone(&self.integration_registry);
        let tools = Arc::clone(&self.registry);

        tokio::spawn(async move {
            let ctx = OAuthFlowCtx {
                service: service_name.clone(),
                url_tx: tokio::sync::Mutex::new(Some(url_tx)),
                notice_tx: notice_tx.clone(),
            };
            // Ok carries the name that is now connected.
            let result = match target {
                ConnectTarget::Registered(i) => {
                    i.connect(&ctx).await.map(|()| service_name.clone())
                }
                // A new workspace, or a reconnect that refreshes the token:
                // either way its tools are (re)registered, no restart.
                ConnectTarget::SlackWorkspace(ws) => ws.connect(&ctx).await.map(|()| {
                    let name = ws.name().to_string();
                    crate::slack_workspaces::register(&integrations, &tools, ws);
                    name
                }),
            };
            // Failed before any URL: give the caller the reason now
            // instead of a "no auth URL" timeout.
            if let Err(e) = &result
                && let Some(tx) = ctx.url_tx.lock().await.take()
            {
                let _ = tx.send(Err(e.user_message()));
            }
            let succeeded = result.is_ok();
            let connected_name = result.as_ref().cloned().unwrap_or_default();
            let notice = match &result {
                Ok(name) => arawn_service::ServerNotice {
                    level: "info".into(),
                    category: "integration".into(),
                    message: format!("{name} connected"),
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
                auto_create_default_feed(
                    &feed_runtime_for_task,
                    &notice_tx,
                    &connected_name,
                    &format!("/connect {service_name}"),
                )
                .await;
            }
        });

        // Wait briefly for the integration to publish its auth URL.
        // Most OAuth integrations call publish_auth_url within milliseconds;
        // a 5s ceiling protects the RPC from hanging if an integration is
        // poorly behaved.
        let url = match tokio::time::timeout(std::time::Duration::from_secs(5), url_rx).await {
            Ok(Ok(Ok(u))) => u,
            Ok(Ok(Err(reason))) => return Err(ServiceError::InvalidOperation(reason)),
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
        // A Slack workspace goes away with its tools; the others stay.
        if let Some(alias) = service.strip_prefix("slack@") {
            crate::slack_workspaces::deregister(&self.integration_registry, &self.registry, alias);
        }

        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "integration".into(),
            message: format!("{service} disconnected"),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(())
    }
}

/// See [`LocalService::connect_target`].
enum ConnectTarget {
    Registered(Arc<dyn arawn_integrations::Integration>),
    SlackWorkspace(Arc<arawn_integrations::slack::SlackIntegration>),
}

type FeedRuntimeCell = Arc<std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>>;

/// Auto-create the personal default feed for `service_name`, if one is
/// defined and the feeds runtime is available. It is a singleton per
/// TEMPLATE: skip when any feed already uses the template, under
/// whatever id (ARAWN-T-0510). `after` names the trigger in the notice.
pub(super) async fn auto_create_default_feed(
    feed_runtime: &FeedRuntimeCell,
    notice_tx: &tokio::sync::broadcast::Sender<arawn_service::ServerNotice>,
    service_name: &str,
    after: &str,
) {
    let runtime = feed_runtime.read().recover().clone();
    let (Some(runtime), Some((template, feed_id))) =
        (runtime, default_feed_for_service(service_name))
    else {
        return;
    };
    let notice = |level: &str, message: String| arawn_service::ServerNotice {
        level: level.into(),
        category: "feeds".into(),
        message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let existing: Vec<(String, String)> = match runtime.list_summaries().await {
        Ok(s) => s.into_iter().map(|f| (f.id, f.template)).collect(),
        Err(e) => {
            let _ = notice_tx.send(notice(
                "warn",
                format!(
                    "auto-create {template} for {service_name} skipped: cannot list feeds: {e}"
                ),
            ));
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
            let _ = notice_tx.send(notice(
                "info",
                format!("auto-registered {template} as `{feed_id}` after {after}"),
            ));
        }
        // Every failure is surfaced, including a UNIQUE clash: the
        // template check above already covers "this feed exists", so a
        // clash here means a different feed holds the id.
        Err(e) => {
            let _ = notice_tx.send(notice(
                "warn",
                format!("auto-create {template} for {service_name} failed: {e}"),
            ));
        }
    }
}

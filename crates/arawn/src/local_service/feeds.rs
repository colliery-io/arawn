//! `LocalService` inherent methods backing the `feeds.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.


use arawn_service::ServiceError;


use super::{LocalService, current_summary, feed_err, feed_summary_to_dto};

impl LocalService {
    pub(super) async fn feed_register_inner(
        &self,
        spec: arawn_service::FeedRegisterSpec,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        let params = arawn_feeds::TemplateParams::new(spec.params);
        let record = runtime
            .register_feed_dynamic(&spec.template, &spec.feed_id, params, spec.cadence)
            .await
            .map_err(feed_err)?;

        // Re-list to grab the freshly-written meta + dir size in one
        // place rather than synthesizing a half-populated DTO from the
        // bare record.
        let summaries = runtime.list_summaries().await.map_err(feed_err)?;
        let dto = summaries
            .into_iter()
            .find(|s| s.id == record.id)
            .map(feed_summary_to_dto)
            .ok_or_else(|| {
                ServiceError::Internal(format!(
                    "feed {} registered but not visible in list",
                    record.id
                ))
            })?;

        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "feeds".into(),
            message: format!("feed {} ({}) registered", dto.id, dto.template),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(dto)
    }

    pub(super) async fn feed_list_inner(&self) -> Result<Vec<arawn_service::FeedSummaryDto>, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        let summaries = runtime.list_summaries().await.map_err(feed_err)?;
        Ok(summaries.into_iter().map(feed_summary_to_dto).collect())
    }

    pub(super) async fn feed_pause_inner(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        runtime.pause_feed(feed_id).await.map_err(feed_err)?;
        let dto = current_summary(&runtime, feed_id).await?;
        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "feeds".into(),
            message: format!("feed {feed_id} paused"),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(dto)
    }

    pub(super) async fn feed_resume_inner(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        runtime.resume_feed(feed_id).await.map_err(feed_err)?;
        let dto = current_summary(&runtime, feed_id).await?;
        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "feeds".into(),
            message: format!("feed {feed_id} resumed"),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(dto)
    }

    pub(super) async fn feed_run_inner(&self, feed_id: &str) -> Result<arawn_service::FeedSummaryDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        runtime.run_feed_once(feed_id).await.map_err(feed_err)?;
        let dto = current_summary(&runtime, feed_id).await?;
        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "feeds".into(),
            message: format!(
                "feed {feed_id} run on demand — {} items, status {}",
                "?",
                dto.last_status.as_deref().unwrap_or("?"),
            ),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(dto)
    }

    pub(super) async fn feed_discover_inner(
        &self,
        template: &str,
    ) -> Result<arawn_service::FeedDiscoverDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        let rows = runtime
            .discover_template(template)
            .await
            .map_err(feed_err)?;
        Ok(match rows {
            Some(rows) => arawn_service::FeedDiscoverDto {
                template: template.into(),
                picker_supported: true,
                rows: rows
                    .into_iter()
                    .map(|r| arawn_service::FeedDiscoverRow {
                        label: r.label,
                        hint: r.hint,
                        params: r.params,
                    })
                    .collect(),
            },
            None => arawn_service::FeedDiscoverDto {
                template: template.into(),
                picker_supported: false,
                rows: vec![],
            },
        })
    }

    pub(super) async fn feed_remove_inner(
        &self,
        feed_id: &str,
    ) -> Result<arawn_service::FeedRemoveDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        let outcome = runtime.remove_feed(feed_id).await.map_err(feed_err)?;
        let dto = arawn_service::FeedRemoveDto {
            id: outcome.record.id.clone(),
            template: outcome.record.template.clone(),
            bytes_wiped: outcome.bytes_wiped,
        };
        let _ = self.notice_tx.send(arawn_service::ServerNotice {
            level: "info".into(),
            category: "feeds".into(),
            message: format!(
                "feed {} removed ({} bytes wiped)",
                outcome.record.id, outcome.bytes_wiped
            ),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        Ok(dto)
    }
}

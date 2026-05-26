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

    pub(super) async fn feed_schema_inner(
        &self,
        template: &str,
    ) -> Result<arawn_service::FeedSchemaDto, ServiceError> {
        let runtime = self.feed_runtime_or_err()?;
        let (schema, default_cadence) = runtime.template_schema(template).map_err(feed_err)?;
        Ok(arawn_service::FeedSchemaDto {
            template: template.into(),
            params: schema.into_iter().map(param_spec_to_dto).collect(),
            default_cadence,
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

/// Map a feeds-crate `ParamSpec` onto the service-layer DTO (the service
/// layer deliberately doesn't depend on arawn-feeds).
fn param_spec_to_dto(spec: arawn_feeds::ParamSpec) -> arawn_service::FeedParamSpecDto {
    use arawn_feeds::ParamKind;
    use arawn_service::FeedParamKindDto;
    let kind = match spec.kind {
        ParamKind::Text => FeedParamKindDto::Text,
        ParamKind::Int => FeedParamKindDto::Int,
        ParamKind::Bool => FeedParamKindDto::Bool,
        ParamKind::Path => FeedParamKindDto::Path,
        ParamKind::List => FeedParamKindDto::List,
        ParamKind::Since => FeedParamKindDto::Since,
        ParamKind::Enum(values) => FeedParamKindDto::Enum(values),
    };
    arawn_service::FeedParamSpecDto {
        key: spec.key,
        label: spec.label,
        kind,
        required: spec.required,
        default: spec.default,
        help: spec.help,
    }
}

#[cfg(test)]
mod tests {
    use super::param_spec_to_dto;
    use arawn_feeds::{ParamKind, ParamSpec, TemplateParams, default_registry};
    use arawn_service::FeedParamKindDto as K;

    #[test]
    fn param_spec_to_dto_maps_every_kind() {
        for (kind, expect) in [
            (ParamKind::Text, K::Text),
            (ParamKind::Int, K::Int),
            (ParamKind::Bool, K::Bool),
            (ParamKind::Path, K::Path),
            (ParamKind::List, K::List),
            (ParamKind::Since, K::Since),
        ] {
            let dto = param_spec_to_dto(ParamSpec::required("k", "L", kind, "h"));
            assert_eq!(dto.kind, expect);
        }
        let dto = param_spec_to_dto(ParamSpec::required(
            "m",
            "M",
            ParamKind::Enum(vec!["a".into(), "b".into()]),
            "h",
        ));
        assert_eq!(dto.kind, K::Enum(vec!["a".into(), "b".into()]));
    }

    /// What `feed_schema` returns for filesystem/folder — exercised via the
    /// same registry + defaults path `FeedRuntime::template_schema` uses,
    /// without standing up a full runtime.
    #[test]
    fn filesystem_schema_dto_shape() {
        let reg = default_registry();
        let tpl = reg.get("filesystem/folder").expect("registered");
        let params: Vec<_> = tpl
            .param_schema()
            .into_iter()
            .map(param_spec_to_dto)
            .collect();
        let keys: Vec<&str> = params.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["root", "recursive", "include", "exclude"]);
        assert!(params[0].required && matches!(params[0].kind, K::Path));

        let cadence = tpl
            .defaults(&TemplateParams::new(serde_json::json!({})))
            .cadence;
        assert_eq!(cadence, "*/15 * * * *");
    }
}

//! Ceremony engine wiring (I-0043 + I-0041), extracted from `main.rs`
//! as part of I-0054 T-E. The startup orchestrator calls
//! `wire_ceremony_engine` once after the workflow runner is up.
//!
//! Inputs are taken by reference; the function mutates `service` (sets
//! the ceremony service) and registers tools on the shared `registry`.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::{ArawnConfig, LocalService, LlmClientPool};

/// Wire the ceremony engine: build the connection, plugin registry, dispatcher,
/// service, runner, register per-plugin cron schedules, set the ceremony
/// service on `service`, and register ceremony agent tools on `registry`.
pub async fn wire_ceremony_engine(
    config: &ArawnConfig,
    workflow_runner_handle: Option<&Arc<arawn_workflow::WorkflowRunner>>,
    data_dir: &str,
    llm_pool: &Arc<LlmClientPool>,
    projections: Option<&Arc<arawn_projections::ProjectionStore>>,
    registry: &Arc<arawn_engine::ToolRegistry>,
    service: &mut LocalService,
) {
    // Ceremony engine (I-0043 + I-0041). The shared infra
    // (connection, plugin registry, dispatcher, service, runner)
    // is built when the cloacina workflow runner is available.
    // Per-plugin enablement is gated on `[ceremonies.<kind>]`
    // in arawn.toml: retro and daily each have their own
    // enabled-flag and override surface. Absent table or
    // missing fields → use the plugin's compiled-in defaults.
    let retro_cfg = config.ceremonies.get("retro");
    let retro_enabled = retro_cfg.is_none_or(crate::CeremonyConfig::is_enabled);
    let daily_cfg = config.ceremonies.get("daily");
    let daily_enabled = daily_cfg.is_none_or(crate::CeremonyConfig::is_enabled);
    let weekly_cfg = config.ceremonies.get("weekly");
    let weekly_enabled = weekly_cfg.is_none_or(crate::CeremonyConfig::is_enabled);

    if let Some(workflow_runner) = workflow_runner_handle
        && (retro_enabled || daily_enabled || weekly_enabled)
    {
        let cer_db_path = std::path::PathBuf::from(&data_dir).join("arawn.db");
        match rusqlite::Connection::open(&cer_db_path) {
            Ok(conn) => {
                let conn_handle = arawn_ceremonies::ConnHandle::new(conn);
                let plugin_reg = arawn_ceremonies::PluginRegistry::new();

                // Retro plugin construction (gated). Defer the
                // resolve_ceremony_tz fn definition below; inline
                // the resolution to avoid forward-ref churn.
                if retro_enabled {
                    let model_hint = retro_cfg
                        .and_then(|c| c.model.clone())
                        .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                    let (retro_client, retro_model) = llm_pool.resolve_hint(&model_hint);
                    let retro_tz_raw = retro_cfg.and_then(|c| c.timezone.as_deref());
                    let retro_tz: chrono_tz::Tz = {
                        use std::str::FromStr;
                        match retro_tz_raw {
                            None => chrono_tz::UTC,
                            Some(s) => {
                                let t = s.trim();
                                if t.is_empty() || t.eq_ignore_ascii_case("local") {
                                    chrono_tz::UTC
                                } else {
                                    chrono_tz::Tz::from_str(t).unwrap_or(chrono_tz::UTC)
                                }
                            }
                        }
                    };
                    // Cadence resolution (T-0367):
                    //   1. DB row in ceremony_config (live tool writes)
                    //   2. [ceremonies.retro] cadence in arawn.toml
                    //   3. Weekly default.
                    let toml_cadence = retro_cfg
                        .and_then(|c| c.cadence.as_deref())
                        .and_then(arawn_ceremonies::RetroCadence::parse);
                    let (cadence, mut anchor) =
                        arawn_ceremonies::RetroCeremony::load_persisted_cadence(
                            &conn_handle,
                            toml_cadence,
                        );
                    // Biweekly needs an anchor. If absent (first
                    // boot on this cadence), initialise to "this
                    // Monday" and persist so the cycle is stable.
                    if cadence == arawn_ceremonies::RetroCadence::Biweekly && anchor.is_none() {
                        use chrono::Datelike;
                        let today = chrono::Utc::now().date_naive();
                        let weekday_offset =
                            today.weekday().num_days_from_monday() as i64;
                        let this_monday =
                            today - chrono::Duration::days(weekday_offset);
                        anchor = Some(this_monday);
                        if let Err(e) = arawn_ceremonies::RetroCeremony::save_cadence(
                            &conn_handle,
                            cadence,
                            anchor,
                        ) {
                            warn!(error = %e, "failed to persist initial retro cadence anchor");
                        }
                    }
                    let retro = arawn_ceremonies::RetroCeremony::new(retro_client, retro_model)
                        .with_detectors(arawn_ceremonies::retro_v1_catalog())
                        .with_timezone(retro_tz)
                        .with_cadence(cadence, anchor);
                    if let Err(e) = plugin_reg.register(Arc::new(retro)) {
                        warn!(error = %e, "ceremony retro plugin registration failed");
                    }
                }

                // Daily + weekly both depend on the projection
                // store for calendar + attention sources. The
                // attention source is timezone-agnostic and can
                // be shared; the calendar source brackets day
                // windows in the configured ceremony timezone, so
                // daily and weekly each need their own instance
                // when their `[ceremonies.<kind>].timezone` strings
                // differ. We resolve the tz strings up-front and
                // build per-plugin calendar handles below.
                let daily_actually_enabled = daily_enabled && projections.is_some();
                let weekly_actually_enabled = weekly_enabled && projections.is_some();

                // Local helper: parse a CeremonyConfig.timezone
                // into a chrono_tz::Tz, defaulting to UTC for
                // "local"/missing values and warn-falling-back on
                // unknown IANA zones. Kept inline because it's
                // only used here.
                fn resolve_ceremony_tz(kind: &str, raw: Option<&str>) -> chrono_tz::Tz {
                    use std::str::FromStr;
                    match raw {
                        None => chrono_tz::UTC,
                        Some(s) => {
                            let trimmed = s.trim();
                            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("local") {
                                tracing::debug!(
                                    kind,
                                    "ceremony timezone '{trimmed}' → UTC fallback"
                                );
                                chrono_tz::UTC
                            } else {
                                chrono_tz::Tz::from_str(trimmed).unwrap_or_else(|_| {
                                    warn!(
                                        kind,
                                        raw = %trimmed,
                                        "unknown ceremony timezone — falling back to UTC"
                                    );
                                    chrono_tz::UTC
                                })
                            }
                        }
                    }
                }

                let daily_tz =
                    resolve_ceremony_tz("daily", daily_cfg.and_then(|c| c.timezone.as_deref()));
                let weekly_tz = resolve_ceremony_tz(
                    "weekly",
                    weekly_cfg.and_then(|c| c.timezone.as_deref()),
                );

                let attention_source: Option<Arc<dyn arawn_ceremonies::AttentionSource>> =
                    if (daily_actually_enabled || weekly_actually_enabled)
                        && let Some(projections) = projections.as_ref()
                    {
                        Some(Arc::new(arawn_engine::ProjectionsAttentionSource::new(
                            Arc::clone(projections),
                            service.shared_store(),
                        )))
                    } else {
                        None
                    };

                let daily_calendar: Option<Arc<dyn arawn_ceremonies::CalendarSource>> =
                    if daily_actually_enabled && let Some(projections) = projections.as_ref() {
                        Some(Arc::new(
                            arawn_engine::ProjectionsCalendarSource::new(Arc::clone(
                                projections,
                            ))
                            .with_tz(daily_tz),
                        ))
                    } else {
                        None
                    };
                let weekly_calendar: Option<Arc<dyn arawn_ceremonies::CalendarSource>> =
                    if weekly_actually_enabled && let Some(projections) = projections.as_ref() {
                        Some(Arc::new(
                            arawn_engine::ProjectionsCalendarSource::new(Arc::clone(
                                projections,
                            ))
                            .with_tz(weekly_tz),
                        ))
                    } else {
                        None
                    };

                // Daily plugin construction (gated). Requires the
                // projection store for calendar + attention
                // sources; degrades to "daily disabled" if it
                // isn't available.
                if daily_actually_enabled
                    && let (Some(calendar), Some(attention)) =
                        (daily_calendar.as_ref(), attention_source.as_ref())
                {
                    let model_hint = daily_cfg
                        .and_then(|c| c.model.clone())
                        .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                    let (daily_client, daily_model) = llm_pool.resolve_hint(&model_hint);
                    let daily = arawn_ceremonies::DailyCeremony::new(
                        daily_client,
                        daily_model,
                        Arc::clone(calendar),
                        Arc::clone(attention),
                    )
                    .with_timezone(daily_tz);
                    if let Err(e) = plugin_reg.register(Arc::new(daily)) {
                        warn!(error = %e, "ceremony daily plugin registration failed");
                    }
                } else if daily_enabled {
                    warn!(
                        "daily ceremony enabled in config but projection store unavailable — skipping"
                    );
                }

                // Weekly plugin construction (gated). Same
                // projection-store dependency as daily.
                if weekly_actually_enabled
                    && let (Some(calendar), Some(attention)) =
                        (weekly_calendar.as_ref(), attention_source.as_ref())
                {
                    let model_hint = weekly_cfg
                        .and_then(|c| c.model.clone())
                        .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                    let (weekly_client, weekly_model) = llm_pool.resolve_hint(&model_hint);
                    let weekly = arawn_ceremonies::WeeklyCeremony::new(
                        weekly_client,
                        weekly_model,
                        Arc::clone(calendar),
                        Arc::clone(attention),
                    )
                    .with_timezone(weekly_tz);
                    if let Err(e) = plugin_reg.register(Arc::new(weekly)) {
                        warn!(error = %e, "ceremony weekly plugin registration failed");
                    }
                } else if weekly_enabled {
                    warn!(
                        "weekly ceremony enabled in config but projection store unavailable — skipping"
                    );
                }

                let (event_tx, mut event_rx) = arawn_ceremonies::event_channel();
                // Forward ceremony events onto the existing notice
                // broadcast so the TUI's read loop can react (see
                // T-0308 slice 3). Drop-stale (lagged) errors are
                // silently ignored — the receiver re-subscribes on
                // the next event.
                {
                    let notice_tx_cer = service.notice_sender();
                    tokio::spawn(async move {
                        loop {
                            match event_rx.recv().await {
                                Ok(ev) => {
                                    // Side-by-side notices: the legacy
                                    // `ceremony_event` category carries the
                                    // raw JSON for existing TUI handlers
                                    // (priority modal refresh, etc.); the
                                    // I-0035 Phase 4 `briefing_ready`
                                    // category fires only on
                                    // `TabletGenerated` and triggers the
                                    // brief-cache refresh in the TUI.
                                    let message = serde_json::to_string(&ev)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    let now = chrono::Utc::now().to_rfc3339();
                                    let _ = notice_tx_cer.send(
                                        arawn_service::ServerNotice {
                                            level: "info".into(),
                                            category: "ceremony_event".into(),
                                            message: message.clone(),
                                            timestamp: now.clone(),
                                        },
                                    );
                                    if let arawn_ceremonies::CeremonyEvent::TabletGenerated {
                                        kind,
                                        period_key,
                                        ..
                                    } = &ev
                                    {
                                        let _ = notice_tx_cer.send(
                                            arawn_service::ServerNotice {
                                                level: "info".into(),
                                                category: "briefing_ready".into(),
                                                message: format!(
                                                    "Brief updated — {kind} tablet for {period_key}"
                                                ),
                                                timestamp: now,
                                            },
                                        );
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    continue;
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                let dispatcher = Arc::new(
                    arawn_ceremonies::EngineDispatcher::new(
                        conn_handle.clone(),
                        plugin_reg.clone(),
                    )
                    .with_events(event_tx.clone()),
                );
                // Back-fill missed daily/weekly ceremonies (T-0366).
                // Spawned as a background task so server-ready is
                // not gated on potentially-many LLM compose calls
                // (UAT regression: 14 days × 2 ceremonies blew
                // through the 60s ready timeout). The cron loop
                // attaches immediately below; if a freshly-fired
                // cron tick collides with a still-running back-fill
                // on the same period_key, dispatcher idempotency
                // makes whichever loses return Skipped.
                let backfill_lookback = config.backfill.ceremony_lookback_days;
                let backfill_registry = plugin_reg.clone();
                let backfill_dispatcher: Arc<dyn arawn_ceremonies::CeremonyDispatcher> =
                    Arc::clone(&dispatcher)
                        as Arc<dyn arawn_ceremonies::CeremonyDispatcher>;
                tokio::spawn(async move {
                    match arawn_ceremonies::backfill::run(
                        &backfill_registry,
                        backfill_dispatcher.as_ref(),
                        backfill_lookback,
                    )
                    .await
                    {
                        Ok(_report) => {}
                        Err(e) => {
                            warn!(error = %e, "ceremony back-fill failed");
                        }
                    }
                });

                let runner = arawn_ceremonies::CeremonyRunner::new(
                    plugin_reg,
                    workflow_runner.cloacina_runner(),
                    Arc::clone(&dispatcher) as Arc<dyn arawn_ceremonies::CeremonyDispatcher>,
                );
                let cer_service = Arc::new(
                    arawn_ceremonies::CeremonyService::new(
                        conn_handle.clone(),
                        Arc::clone(&dispatcher)
                            as Arc<dyn arawn_ceremonies::CeremonyDispatcher>,
                    )
                    .with_events(event_tx),
                );

                // Cron registration per enabled plugin. Each
                // `[ceremonies.<kind>]` schedule override applied
                // here; invalid expressions log a warn and fall
                // back to plugin default rather than abort.
                if retro_enabled {
                    let sched = retro_cfg.and_then(|c| {
                        c.schedule.as_ref().map(|expr| {
                            arawn_ceremonies::CronSchedule::new(
                                expr.clone(),
                                c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                            )
                        })
                    });
                    if let Err(e) = runner.register_one_with_schedule("retro", sched).await {
                        warn!(error = %e, "ceremony runner failed to register retro cron — manual runs still work");
                    }
                }
                if daily_actually_enabled {
                    let sched = daily_cfg.and_then(|c| {
                        c.schedule.as_ref().map(|expr| {
                            arawn_ceremonies::CronSchedule::new(
                                expr.clone(),
                                c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                            )
                        })
                    });
                    if let Err(e) = runner.register_one_with_schedule("daily", sched).await {
                        warn!(error = %e, "ceremony runner failed to register daily cron — manual runs still work");
                    }
                }
                if weekly_actually_enabled {
                    let sched = weekly_cfg.and_then(|c| {
                        c.schedule.as_ref().map(|expr| {
                            arawn_ceremonies::CronSchedule::new(
                                expr.clone(),
                                c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                            )
                        })
                    });
                    if let Err(e) = runner.register_one_with_schedule("weekly", sched).await {
                        warn!(error = %e, "ceremony runner failed to register weekly cron — manual runs still work");
                    }
                }

                // Sunday-night sweep — retro-only.
                if retro_enabled {
                    let sweep_handle = conn_handle.clone();
                    tokio::spawn(async move {
                        let mut interval =
                            tokio::time::interval(std::time::Duration::from_secs(3600));
                        interval
                            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        loop {
                            interval.tick().await;
                            match arawn_ceremonies::sweep_unreviewed_retros(&sweep_handle) {
                                Ok(0) => {}
                                Ok(n) => info!(transitioned = n, "retro sweep"),
                                Err(e) => warn!(error = %e, "retro sweep failed"),
                            }
                        }
                    });
                }

                service.set_ceremony_service(Arc::clone(&cer_service));

                // Retro agent tools (gated on retro_enabled).
                if retro_enabled {
                    registry.register(Box::new(arawn_engine::RetroRunTool::new(Arc::clone(
                        &cer_service,
                    ))));
                    registry.register(Box::new(arawn_engine::RetroCurrentTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::RetroListItemsTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::RetroSaveDiaryTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::RetroPatchItemTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::RetroSetCadenceTool::new(
                        Arc::clone(&cer_service),
                    )));
                }

                // Daily agent tools (gated on daily_actually_enabled).
                if daily_actually_enabled {
                    registry.register(Box::new(arawn_engine::DailyRunTool::new(Arc::clone(
                        &cer_service,
                    ))));
                    registry.register(Box::new(arawn_engine::DailyCurrentTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::DailyListItemsTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::DailyPatchItemTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::DailyAddTodoTool::new(
                        Arc::clone(&cer_service),
                    )));
                }

                // Weekly agent tools (gated on weekly_actually_enabled).
                if weekly_actually_enabled {
                    registry.register(Box::new(arawn_engine::WeeklyRunTool::new(Arc::clone(
                        &cer_service,
                    ))));
                    registry.register(Box::new(arawn_engine::WeeklyCurrentTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::WeeklyListItemsTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::WeeklyListPrioritiesTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::WeeklyConfirmPriorityTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::WeeklyRejectPriorityTool::new(
                        Arc::clone(&cer_service),
                    )));
                    registry.register(Box::new(arawn_engine::WeeklyAddPriorityTool::new(
                        Arc::clone(&cer_service),
                    )));
                }

                info!(
                    retro = retro_enabled,
                    daily = daily_actually_enabled,
                    weekly = weekly_actually_enabled,
                    "ceremony engine wired"
                );
            }
            Err(e) => warn!(error = %e, db = %cer_db_path.display(),
                "ceremony engine unavailable — could not open arawn.db"),
        }
    } else {
        debug!(
            "ceremony engine skipped — workflow runner not available or all ceremonies disabled"
        );
    }

}

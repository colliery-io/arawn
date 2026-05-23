//! T-0355 — Brief pipeline integration test.
//!
//! Exercises the full chain at the library layer:
//!
//!     scripted daily/weekly plugins
//!         → CeremonyService::run("daily")
//!         → CeremonyService::run("weekly")
//!         → service.get_by_period + list_items + list_priorities
//!         → BriefView { daily, weekly }
//!         → render_brief
//!
//! Goal: prove the renderer composes content the ceremony layer
//! actually produces. Companion to the unit tests on `render_brief`
//! (T-0349) and the TUI snapshot tests on the empty-chat surface
//! (T-0354), which test the deterministic surface with hand-built
//! fixtures rather than running the pipeline.
//!
//! No real LLM in scope — the scripted plugin pre-loads `NewItem`
//! values, sidestepping `compose()`'s LLM dependency.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

use arawn_ceremonies::service::{AddPriorityRequest, CeremonyService};
use arawn_ceremonies::{
    BriefView, ConnHandle, DailyView, DispatchOutcome, EngineDispatcher, PluginRegistry, WeeklyView,
    render_brief,
};
use arawn_ceremonies::plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, NewItem, UserItem,
};
use arawn_ceremonies::types::{GatheredFacts, ItemKind};

fn open_test_db() -> (TempDir, ConnHandle) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let _db = arawn_storage::Database::open(&db_path).expect("migrations");
    drop(_db);
    let conn = Connection::open(&db_path).expect("open conn");
    (tmp, ConnHandle::new(conn))
}

/// Minimal Ceremony impl: `compose()` returns whatever pre-loaded
/// items were handed to the plugin. No LLM, no DB scans during
/// gather. Used to drive `service.run(kind)` end-to-end without
/// any of the real plugins' compose-time complexity.
struct ScriptedPlugin {
    kind: &'static str,
    period: String,
    items: Mutex<Vec<NewItem>>,
}

#[async_trait]
impl Ceremony for ScriptedPlugin {
    fn kind(&self) -> &'static str {
        self.kind
    }
    fn period_key(&self, _now: DateTime<Utc>) -> String {
        self.period.clone()
    }
    fn period_window(
        &self,
        _period_key: &str,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), arawn_ceremonies::CeremonyError> {
        let now = Utc::now();
        Ok((now, now + Duration::days(1)))
    }
    fn default_schedule(&self) -> CronSchedule {
        CronSchedule::local("0 0 * * *")
    }
    async fn gather(
        &self,
        _ctx: &dyn CeremonyCtx,
    ) -> Result<GatheredFacts, arawn_ceremonies::CeremonyError> {
        Ok(GatheredFacts::new(json!({})))
    }
    async fn compose(
        &self,
        _ctx: &dyn CeremonyCtx,
        _facts: GatheredFacts,
    ) -> Result<Vec<NewItem>, arawn_ceremonies::CeremonyError> {
        Ok(std::mem::take(&mut *self.items.lock().unwrap()))
    }
}

const DAILY_PERIOD: &str = "2026-05-19";
const WEEKLY_PERIOD: &str = "2026-W21";

fn daily_tablet_id() -> String {
    format!("daily-{DAILY_PERIOD}")
}

fn weekly_tablet_id() -> String {
    format!("weekly-{WEEKLY_PERIOD}")
}

fn composed(tablet_id: &str, section: &str, ordinal: i32, text: &str, citation: &str) -> NewItem {
    NewItem::composed(ComposedItem {
        tablet_id: tablet_id.into(),
        section_key: section.into(),
        ordinal,
        kind: ItemKind::Freeform,
        body: json!({"text": text}),
        citation_id: citation.into(),
    })
}

fn user_item(tablet_id: &str, section: &str) -> NewItem {
    NewItem::user(UserItem {
        tablet_id: tablet_id.into(),
        section_key: section.into(),
        ordinal: 0,
        kind: ItemKind::Freeform,
        body: json!({"text": ""}),
    })
}

fn daily_seed_items() -> Vec<NewItem> {
    let tid = daily_tablet_id();
    vec![
        composed(&tid, "calendar", 0, "09:00 standup", "evt-standup"),
        composed(&tid, "todos", 0, "ship I-0035 brief", "todo-brief"),
        composed(&tid, "attention", 0, "RFC-0042 waiting on you", "rfc-0042"),
        composed(&tid, "alignment", 0, "Phase 2 lands this week", "prio-phase2"),
        // Diary placeholder — matches the daily plugin's user-item shape.
        user_item(&tid, "diary"),
    ]
}

fn weekly_seed_items() -> Vec<NewItem> {
    let tid = weekly_tablet_id();
    vec![
        composed(&tid, "calendar_shape", 0, "7 meetings", "cal-shape-w21"),
        composed(&tid, "deadlines", 0, "Tax form Friday", "deadline-tax"),
        composed(
            &tid,
            "from_last_retro",
            0,
            "Carried over: catch up on PR reviews",
            "retro-pr",
        ),
        user_item(&tid, "diary"),
    ]
}

fn build_service() -> (TempDir, CeremonyService) {
    let (tmp, conn) = open_test_db();
    let reg = PluginRegistry::new();
    reg.register(Arc::new(ScriptedPlugin {
        kind: "daily",
        period: DAILY_PERIOD.into(),
        items: Mutex::new(daily_seed_items()),
    }))
    .unwrap();
    reg.register(Arc::new(ScriptedPlugin {
        kind: "weekly",
        period: WEEKLY_PERIOD.into(),
        items: Mutex::new(weekly_seed_items()),
    }))
    .unwrap();
    let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), reg));
    let service = CeremonyService::new(conn, dispatcher);
    (tmp, service)
}

fn brief_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-19T15:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[tokio::test]
async fn brief_pipeline_renders_daily_and_weekly_content() {
    let (_tmp, service) = build_service();

    // Run both ceremonies. Both should generate tablets.
    let daily_outcome = service.run("daily").await.expect("daily dispatch");
    assert!(
        matches!(daily_outcome, DispatchOutcome::Generated { .. }),
        "expected daily Generated, got {daily_outcome:?}"
    );
    let weekly_outcome = service.run("weekly").await.expect("weekly dispatch");
    assert!(
        matches!(weekly_outcome, DispatchOutcome::Generated { .. }),
        "expected weekly Generated, got {weekly_outcome:?}"
    );

    // Add a priority to the weekly tablet so the Priorities section
    // has real content (priorities are a user-write surface; they
    // don't come from the plugin's compose path).
    let weekly_tablet = service
        .get_by_period("weekly", WEEKLY_PERIOD)
        .expect("weekly read")
        .expect("weekly tablet exists");
    service
        .add_priority(AddPriorityRequest {
            tablet_id: weekly_tablet.id.clone(),
            body: json!({"text": "Ship Phase 2 brief pipeline"}),
            rationale: "highest-leverage Phase 2 deliverable".into(),
        })
        .expect("add priority");

    // Read both tablets back and assemble the views.
    let daily_tablet = service
        .get_by_period("daily", DAILY_PERIOD)
        .expect("daily read")
        .expect("daily tablet exists");
    let daily_items = service
        .list_items(&daily_tablet.id, None)
        .expect("daily items");
    let weekly_items = service
        .list_items(&weekly_tablet.id, None)
        .expect("weekly items");
    let weekly_priorities = service
        .list_priorities(&weekly_tablet.id)
        .expect("weekly priorities");

    let view = BriefView {
        daily: Some(DailyView {
            tablet: daily_tablet,
            items: daily_items,
        }),
        weekly: Some(WeeklyView {
            tablet: weekly_tablet,
            items: weekly_items,
            priorities: weekly_priorities,
        }),
    };

    let md = render_brief(&view, brief_now());

    // Date header points at the daily period (preferred over weekly).
    assert!(md.contains(&format!("# Brief — {DAILY_PERIOD}")), "{md}");

    // Top-level structure preserved.
    assert!(md.contains("## Today\n"), "missing Today heading:\n{md}");
    assert!(
        md.contains("## This week\n"),
        "missing This week heading:\n{md}"
    );

    // Inner sections demoted to H3 (not H2) — proves the
    // demote_h2_to_h3 pipeline runs over real data.
    assert!(
        md.contains("### Today's calendar"),
        "expected demoted Today's calendar heading:\n{md}"
    );
    assert!(
        md.contains("### Priorities"),
        "expected demoted Priorities heading:\n{md}"
    );

    // Seeded content surfaces from both sides.
    assert!(md.contains("09:00 standup"), "missing daily calendar:\n{md}");
    assert!(
        md.contains("ship I-0035 brief"),
        "missing daily todo:\n{md}"
    );
    assert!(
        md.contains("RFC-0042 waiting on you"),
        "missing daily attention:\n{md}"
    );
    assert!(
        md.contains("Ship Phase 2 brief pipeline"),
        "missing weekly priority:\n{md}"
    );
    assert!(md.contains("7 meetings"), "missing calendar shape:\n{md}");
}

#[tokio::test]
async fn brief_pipeline_missing_weekly_renders_placeholder() {
    let (_tmp, service) = build_service();

    // Only run the daily ceremony; leave weekly unrun.
    service.run("daily").await.expect("daily dispatch");

    let daily_tablet = service
        .get_by_period("daily", DAILY_PERIOD)
        .expect("daily read")
        .expect("daily tablet exists");
    let daily_items = service
        .list_items(&daily_tablet.id, None)
        .expect("daily items");

    let view = BriefView {
        daily: Some(DailyView {
            tablet: daily_tablet,
            items: daily_items,
        }),
        weekly: None,
    };
    let md = render_brief(&view, brief_now());

    assert!(md.contains(&format!("# Brief — {DAILY_PERIOD}")), "{md}");
    assert!(md.contains("09:00 standup"));
    // Weekly placeholder fires when the weekly view is None.
    assert!(
        md.contains("_(no weekly tablet"),
        "expected weekly placeholder, got:\n{md}"
    );
    // No weekly content leaked through.
    assert!(
        !md.contains("Ship Phase 2 brief pipeline"),
        "weekly content present when weekly view is None:\n{md}"
    );
}

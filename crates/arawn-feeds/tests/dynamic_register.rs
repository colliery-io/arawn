//! Integration test for `FeedRuntime::register_feed_dynamic` —
//! the slice-1 plumbing for the `/watch` slash command.
//!
//! Exercises the full flow against a real `DefaultRunner` + sqlite:
//! validate → insert row → write meta.json → register cron. Then
//! confirms `list_summaries` reflects the new feed before any cron
//! firings happen (so the run_count is 0 and last_run_at is None).

use std::sync::Arc;

use arawn_feeds::{DataLayout, FeedClients, MetaStore, NoopClients, TemplateParams};
use cloacina::DefaultRunner;
use rusqlite::Connection;
use serde_json::json;
use tokio::sync::Mutex;

fn migrate(conn: &Connection) {
    // Mirror crates/arawn-storage/migrations/V2__feeds.sql so this
    // test can stand alone without dragging arawn-storage in.
    conn.execute_batch(
        "CREATE TABLE feeds (
            id TEXT PRIMARY KEY,
            template TEXT NOT NULL,
            params TEXT NOT NULL,
            cadence TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )
    .unwrap();
}

#[tokio::test]
async fn dynamic_register_full_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // Use an isolated cloacina runner — sqlite-backed in a tempdir so
    // cron registration round-trips through the real scheduler.
    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .expect("runner config");
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .expect("runner up"),
    );

    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let runtime = arawn_feeds::start(
        Arc::clone(&runner),
        Arc::clone(&conn),
        Arc::clone(&layout),
        Arc::clone(&registry),
        Arc::clone(&clients),
    )
    .await
    .expect("feeds start");

    // Register a stub/echo feed dynamically. stub/echo doesn't need
    // any provider clients, so the NoopClients bundle is fine.
    let record = runtime
        .register_feed_dynamic(
            "stub/echo",
            "demo",
            TemplateParams::new(json!({"message": "hi"})),
            None,
        )
        .await
        .expect("register dynamic");
    assert_eq!(record.id, "demo");
    assert!(record.enabled);

    // Row landed.
    {
        let c = conn.lock().await;
        let store = arawn_feeds::FeedStore::new(&c);
        let got = store.get("demo").unwrap().expect("row exists");
        assert_eq!(got.template, "stub/echo");
    }

    // meta.json exists with the template's initial cursor.
    let feed_dir = layout.feed_dir("stub/echo", "demo").unwrap();
    let meta = MetaStore::read(&feed_dir).unwrap().expect("meta written");
    assert_eq!(meta.template, "stub/echo");
    assert_eq!(meta.params.as_value()["message"], "hi");
    assert!(meta.last_run_at.is_none(), "no run before cron fires");

    // list_summaries reflects the feed before any firings.
    let summaries = runtime.list_summaries().await.unwrap();
    assert_eq!(summaries.len(), 1);
    let s = &summaries[0];
    assert_eq!(s.id, "demo");
    assert_eq!(s.template, "stub/echo");
    assert!(s.enabled);
    assert!(s.last_run_at.is_none());
    assert_eq!(s.run_count, 0);
}

#[tokio::test]
async fn pause_resume_round_trip_through_cloacina() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .unwrap();
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .unwrap(),
    );

    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let runtime = arawn_feeds::start(
        Arc::clone(&runner),
        Arc::clone(&conn),
        Arc::clone(&layout),
        Arc::clone(&registry),
        Arc::clone(&clients),
    )
    .await
    .unwrap();

    runtime
        .register_feed_dynamic(
            "stub/echo",
            "demo",
            TemplateParams::new(json!({"message": "hi"})),
            None,
        )
        .await
        .unwrap();
    let workflow_name = arawn_feeds::feed_workflow_name("demo");

    // Sanity: cron schedule exists pre-pause.
    let scheds = runner.list_cron_schedules(false, 100, 0).await.unwrap();
    assert!(scheds.iter().any(|s| s.workflow_name == workflow_name));

    // Pause: row goes enabled=0, cron schedule disappears.
    runtime.pause_feed("demo").await.unwrap();
    let summaries = runtime.list_summaries().await.unwrap();
    assert!(!summaries.iter().find(|s| s.id == "demo").unwrap().enabled);
    let scheds = runner.list_cron_schedules(false, 100, 0).await.unwrap();
    assert!(
        !scheds.iter().any(|s| s.workflow_name == workflow_name),
        "cron schedule deleted on pause"
    );

    // Resume: row flips back, cron schedule re-appears.
    runtime.resume_feed("demo").await.unwrap();
    let summaries = runtime.list_summaries().await.unwrap();
    assert!(summaries.iter().find(|s| s.id == "demo").unwrap().enabled);
    let scheds = runner.list_cron_schedules(false, 100, 0).await.unwrap();
    assert!(
        scheds.iter().any(|s| s.workflow_name == workflow_name),
        "cron schedule re-registered on resume"
    );
}

#[tokio::test]
async fn remove_wipes_cron_row_and_data_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .unwrap();
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .unwrap(),
    );

    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let runtime = arawn_feeds::start(
        Arc::clone(&runner),
        Arc::clone(&conn),
        Arc::clone(&layout),
        Arc::clone(&registry),
        Arc::clone(&clients),
    )
    .await
    .unwrap();

    runtime
        .register_feed_dynamic(
            "stub/echo",
            "demo",
            TemplateParams::new(json!({"message": "hi"})),
            None,
        )
        .await
        .unwrap();

    // Drop a marker file in the feed dir so we can prove the wipe.
    let feed_dir = layout.feed_dir("stub/echo", "demo").unwrap();
    std::fs::write(feed_dir.join("marker"), b"some bytes").unwrap();
    assert!(feed_dir.join("marker").exists());

    let outcome = runtime.remove_feed("demo").await.unwrap();
    assert_eq!(outcome.record.id, "demo");
    assert!(outcome.bytes_wiped > 0, "wiped count reflects the marker file");

    // Row gone.
    {
        let c = conn.lock().await;
        assert!(arawn_feeds::FeedStore::new(&c)
            .get("demo")
            .unwrap()
            .is_none());
    }
    // Dir gone.
    assert!(!feed_dir.exists());
    // Cron schedule gone.
    let workflow_name = arawn_feeds::feed_workflow_name("demo");
    let scheds = runner.list_cron_schedules(false, 100, 0).await.unwrap();
    assert!(!scheds.iter().any(|s| s.workflow_name == workflow_name));
}

#[tokio::test]
async fn pause_unknown_feed_returns_invalid_params() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .unwrap();
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .unwrap(),
    );
    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);
    let runtime = arawn_feeds::start(runner, conn, layout, registry, clients)
        .await
        .unwrap();

    let err = runtime.pause_feed("nope").await.unwrap_err();
    assert!(matches!(err, arawn_feeds::FeedError::InvalidParams(_)));
}

#[tokio::test]
async fn dynamic_register_is_idempotent_via_unique_constraint() {
    // Slice-4 backstop: calling register twice for the same feed_id
    // surfaces a Storage error containing UNIQUE — the auto-create
    // path on /connect leans on this to be a no-op when the user
    // re-runs /connect.
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();
    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .unwrap();
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .unwrap(),
    );
    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);
    let runtime = arawn_feeds::start(runner, conn, layout, registry, clients)
        .await
        .unwrap();

    runtime
        .register_feed_dynamic(
            "stub/echo",
            "demo",
            TemplateParams::default(),
            None,
        )
        .await
        .unwrap();

    let err = runtime
        .register_feed_dynamic(
            "stub/echo",
            "demo",
            TemplateParams::default(),
            None,
        )
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("UNIQUE") || msg.to_lowercase().contains("already"),
        "expected duplicate-key error, got: {msg}"
    );
}

#[tokio::test]
async fn dynamic_register_rolls_back_on_unknown_template() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    let runner_db = data_dir.join("cloacina.db");
    let runner_url = format!("sqlite://{}", runner_db.display());
    let cfg = cloacina::DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .unwrap();
    let runner = Arc::new(
        DefaultRunner::with_config(&runner_url, cfg)
            .await
            .unwrap(),
    );

    let conn = Connection::open(data_dir.join("feeds.db")).unwrap();
    migrate(&conn);
    let conn = Arc::new(Mutex::new(conn));
    let layout = Arc::new(DataLayout::new(&data_dir));
    let registry = Arc::new(arawn_feeds::default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let runtime = arawn_feeds::start(runner, conn.clone(), layout, registry, clients)
        .await
        .unwrap();

    let res = runtime
        .register_feed_dynamic(
            "no/such-template",
            "x",
            TemplateParams::default(),
            None,
        )
        .await;
    assert!(res.is_err(), "unknown template should fail");

    // Ensure no row leaked into the DB after the failure.
    let c = conn.lock().await;
    let store = arawn_feeds::FeedStore::new(&c);
    assert!(store.get("x").unwrap().is_none(), "row was rolled back");
}

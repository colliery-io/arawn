//! Cloacina-fire integration test.
//!
//! Stands up a real cloacina `DefaultRunner`, registers one stub feed
//! via `arawn_feeds::start`, then triggers a manual execution of the
//! per-feed workflow via cloacina's `execute` API (the same path
//! cloacina's cron scheduler would take when a cadence fires).
//!
//! This is the authoritative end-to-end test of the runtime. The
//! `tests/slack_channel_archive.rs` file tests `run_feed()` directly;
//! this test proves the full chain works through cloacina's
//! workflow registration + execution machinery.

use std::sync::Arc;

use cloacina::{Context, DefaultRunner, DefaultRunnerConfig, WorkflowExecutor};
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::sync::Mutex;

use arawn_feeds::{
    DataLayout, FeedClients, FeedStore, MetaStore, NoopClients, TemplateParams, default_registry,
    feed_workflow_name, new_record, start,
};

fn create_feeds_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE feeds (
            id          TEXT PRIMARY KEY,
            template    TEXT NOT NULL,
            params      TEXT NOT NULL,
            cadence     TEXT NOT NULL,
            enabled     INTEGER NOT NULL DEFAULT 1,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );",
    )
    .unwrap();
}

async fn build_runner(workflows_db: &std::path::Path) -> Arc<DefaultRunner> {
    let cfg = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .max_concurrent_tasks(2)
        .build()
        .expect("build runner config");

    let url = format!("sqlite://{}", workflows_db.display());
    let runner = DefaultRunner::with_config(&url, cfg)
        .await
        .expect("DefaultRunner init");
    Arc::new(runner)
}

#[tokio::test]
async fn cloacina_fires_feed_workflow_end_to_end() {
    let tmp = tempdir().unwrap();

    // 1. Real cloacina runner against a tempdir.
    let runner = build_runner(&tmp.path().join("workflows.db")).await;

    // 2. Open + initialize the feeds DB; insert one stub feed.
    let conn = Connection::open(tmp.path().join("arawn.db")).unwrap();
    create_feeds_schema(&conn);
    {
        let store = FeedStore::new(&conn);
        store
            .insert(&new_record(
                "fire-test",
                "stub/echo",
                TemplateParams::new(json!({ "message": "hello-cloacina" })),
                "*/15 * * * *",
            ))
            .unwrap();
    }
    let conn = Arc::new(Mutex::new(conn));

    // 3. Boot the feed runtime — registers the workflow + cron schedule
    //    against the live cloacina runner.
    let layout = Arc::new(DataLayout::new(tmp.path()));
    let registry = Arc::new(default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let _feed_runtime = start(
        runner.clone(),
        conn.clone(),
        layout.clone(),
        registry,
        clients,
    )
    .await
    .expect("feed runtime starts");

    // 4. Manually fire the workflow — this is the same code path cloacina
    //    invokes when a cron schedule's cadence elapses. Bypasses the
    //    15-minute wait without bypassing any of the runtime/wiring.
    let workflow_name = feed_workflow_name("fire-test");
    let result = runner
        .execute(&workflow_name, Context::new())
        .await
        .expect("workflow executes");

    // 5. Workflow completed successfully and the dispatch task ran.
    let status = format!("{:?}", result.status);
    assert!(
        status.contains("Completed") || status.contains("Success") || status.contains("OK"),
        "expected completion status, got: {status}"
    );

    // 6. Disk side-effects: meta.json shows one run, the stub template
    //    wrote a JSONL line.
    let feed_dir = layout.feed_dir("stub/echo", "fire-test").unwrap();
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.run_count, 1, "exactly one run recorded");
    assert_eq!(meta.last_status.as_deref(), Some("ok"));
    assert_eq!(meta.cursor["run_count"], 1);
    assert!(
        meta.last_run_at.is_some(),
        "last_run_at populated after run"
    );

    let log = std::fs::read_to_string(feed_dir.join("log.jsonl")).expect("log.jsonl exists");
    let line: Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
    assert_eq!(line["message"], "hello-cloacina");
    assert_eq!(line["run"], 1);
}

#[tokio::test]
async fn cloacina_fires_advance_cursor_across_two_executions() {
    let tmp = tempdir().unwrap();
    let runner = build_runner(&tmp.path().join("workflows.db")).await;

    let conn = Connection::open(tmp.path().join("arawn.db")).unwrap();
    create_feeds_schema(&conn);
    {
        let store = FeedStore::new(&conn);
        store
            .insert(&new_record(
                "iter",
                "stub/echo",
                TemplateParams::default(),
                "*/15 * * * *",
            ))
            .unwrap();
    }
    let conn = Arc::new(Mutex::new(conn));

    let layout = Arc::new(DataLayout::new(tmp.path()));
    let registry = Arc::new(default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let _rt = start(
        runner.clone(),
        conn.clone(),
        layout.clone(),
        registry,
        clients,
    )
    .await
    .unwrap();

    let name = feed_workflow_name("iter");
    runner.execute(&name, Context::new()).await.unwrap();
    runner.execute(&name, Context::new()).await.unwrap();
    runner.execute(&name, Context::new()).await.unwrap();

    let feed_dir = layout.feed_dir("stub/echo", "iter").unwrap();
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.run_count, 3);
    assert_eq!(meta.cursor["run_count"], 3);

    let lines: Vec<_> = std::fs::read_to_string(feed_dir.join("log.jsonl"))
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3, "one JSONL line per cloacina-driven run");
    assert_eq!(lines[0]["run"], 1);
    assert_eq!(lines[2]["run"], 3);
}

#[tokio::test]
async fn registering_a_feed_with_unknown_template_is_skipped_at_boot() {
    // start() should log and skip, not abort.
    let tmp = tempdir().unwrap();
    let runner = build_runner(&tmp.path().join("workflows.db")).await;

    let conn = Connection::open(tmp.path().join("arawn.db")).unwrap();
    create_feeds_schema(&conn);
    {
        let store = FeedStore::new(&conn);
        store
            .insert(&new_record(
                "ghost",
                "phantom/no-such-template",
                TemplateParams::default(),
                "*/15 * * * *",
            ))
            .unwrap();
        // Plus a real one so we can prove the start completes.
        store
            .insert(&new_record(
                "real",
                "stub/echo",
                TemplateParams::default(),
                "*/15 * * * *",
            ))
            .unwrap();
    }
    let conn = Arc::new(Mutex::new(conn));

    let layout = Arc::new(DataLayout::new(tmp.path()));
    let registry = Arc::new(default_registry());
    let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);

    let _rt = start(
        runner.clone(),
        conn.clone(),
        layout.clone(),
        registry,
        clients,
    )
    .await
    .expect("start completes despite the bad feed");

    // The real feed is callable (proves start finished registration).
    let name = feed_workflow_name("real");
    runner.execute(&name, Context::new()).await.unwrap();
    let feed_dir = layout.feed_dir("stub/echo", "real").unwrap();
    let meta = MetaStore::read(&feed_dir).unwrap().unwrap();
    assert_eq!(meta.run_count, 1);

    // The ghost feed never registered a workflow, so executing it errors.
    let ghost_name = feed_workflow_name("ghost");
    let err = runner.execute(&ghost_name, Context::new()).await;
    assert!(
        err.is_err(),
        "ghost workflow should not be registered with cloacina"
    );
}

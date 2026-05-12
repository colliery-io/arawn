//! Embed-pass behavior: walks NULL-embedding rows, calls a stub
//! embedder, writes vectors back, skips short bodies.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use arawn_projections::{
    embed::EMBEDDABLE_FEED_TYPES, gmail, run_embed_pass, Embedder, ProjectionStore,
};

struct StubEmbedder {
    calls: AtomicUsize,
    dim: usize,
}

impl StubEmbedder {
    fn new(dim: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            dim,
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Embedder for StubEmbedder {
    fn embed_batch<'a>(
        &'a self,
        texts: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, String>> + Send + 'a>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let dim = self.dim;
        let n = texts.len();
        Box::pin(async move {
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                out.push(vec![0.5_f32; dim]);
            }
            Ok(out)
        })
    }
}

fn fixture_message(id: &str, body: &str) -> gmail::GmailMessageProjection {
    gmail::GmailMessageProjection {
        id: gmail::projection_id("feed-1", id),
        feed_id: "feed-1".into(),
        source_id: id.into(),
        source_ts: chrono::Utc::now(),
        sender: Some("alice@example.com".into()),
        recipients: vec!["bob@example.com".into()],
        subject: format!("subject-{id}"),
        body_text: body.into(),
        thread_id: None,
        labels: vec![],
    }
}

#[tokio::test]
async fn embeds_rows_with_null_embedding() {
    let store = ProjectionStore::in_memory().unwrap();
    let msgs = vec![
        fixture_message("m1", "Long enough body to embed sensibly"),
        fixture_message("m2", "Another suitably long body for embedding"),
    ];
    store.write_batch(&msgs).unwrap();

    let embedder = StubEmbedder::new(384);
    let out = run_embed_pass(&store, &embedder, 32, 100).await.unwrap();
    assert_eq!(out.embedded, 2);
    assert_eq!(out.errors, 0);
    assert_eq!(out.skipped_empty, 0);
    assert_eq!(embedder.calls(), 1, "should batch both rows in one call");

    // Idempotent: no NULL rows left → next pass embeds nothing.
    let out2 = run_embed_pass(&store, &embedder, 32, 100).await.unwrap();
    assert_eq!(out2.embedded, 0);
}

#[tokio::test]
async fn skips_short_bodies_but_marks_them() {
    let store = ProjectionStore::in_memory().unwrap();
    let msgs = vec![
        fixture_message("short", "ok"),
        fixture_message("long", "A genuinely long body worth embedding"),
    ];
    store.write_batch(&msgs).unwrap();

    let embedder = StubEmbedder::new(384);
    let out = run_embed_pass(&store, &embedder, 32, 100).await.unwrap();
    assert_eq!(out.embedded, 1);
    assert_eq!(out.skipped_empty, 1);

    // Re-running should embed nothing — short row was sentinel-stamped,
    // long row already has a real vector.
    let out2 = run_embed_pass(&store, &embedder, 32, 100).await.unwrap();
    assert_eq!(out2.embedded, 0);
    assert_eq!(out2.skipped_empty, 0);
}

#[tokio::test]
async fn max_per_pass_caps_work() {
    let store = ProjectionStore::in_memory().unwrap();
    let msgs: Vec<_> = (0..10)
        .map(|i| {
            fixture_message(
                &format!("m{i}"),
                "Body text long enough to qualify for embedding",
            )
        })
        .collect();
    store.write_batch(&msgs).unwrap();

    let embedder = StubEmbedder::new(384);
    let out = run_embed_pass(&store, &embedder, 4, 4).await.unwrap();
    assert_eq!(out.embedded, 4);

    // Second pass picks up the remaining 6.
    let out2 = run_embed_pass(&store, &embedder, 4, 100).await.unwrap();
    assert_eq!(out2.embedded, 6);
}

#[tokio::test]
async fn known_feed_types_are_a_strict_subset_of_routed_types() {
    // Sanity: the embeddable list shouldn't drift from the dispatcher's
    // routed feed types (excluding jira_history which we intentionally
    // skip — see embed.rs).
    let expected: Vec<&str> = vec![
        "gmail_messages",
        "slack_messages",
        "slack_thread_messages",
        "drive_files",
        "jira_issues",
        "jira_comments",
        "confluence_pages",
        "calendar_events",
    ];
    let actual: Vec<&str> = EMBEDDABLE_FEED_TYPES.to_vec();
    assert_eq!(actual, expected);
}

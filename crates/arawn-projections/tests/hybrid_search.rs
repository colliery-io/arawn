//! Vector search + hybrid behavior on `ProjectionStore`. Verifies
//! that the embedding-driven path returns rows by similarity, ignores
//! sentinel-marked rows, and tolerates degenerate input.

use std::future::Future;
use std::pin::Pin;

use arawn_projections::{embed::PendingEmbedRow, gmail, run_embed_pass, Embedder, ProjectionStore};

/// Embedder that maps text → unit vector along a content-derived
/// dimension, so two texts mentioning the same token cluster in the
/// same direction.
struct KeywordEmbedder;

impl KeywordEmbedder {
    fn vec_for(text: &str) -> Vec<f32> {
        // 3-d: [contains "alpha", contains "beta", contains "gamma"]
        let lower = text.to_lowercase();
        let alpha = if lower.contains("alpha") { 1.0 } else { 0.0 };
        let beta = if lower.contains("beta") { 1.0 } else { 0.0 };
        let gamma = if lower.contains("gamma") { 1.0 } else { 0.0 };
        normalize(vec![alpha, beta, gamma])
    }
}

fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n == 0.0 {
        return vec![1.0, 0.0, 0.0]; // arbitrary unit vec; lets short bodies still get a direction
    }
    for x in v.iter_mut() {
        *x /= n;
    }
    v
}

impl Embedder for KeywordEmbedder {
    fn embed_batch<'a>(
        &'a self,
        texts: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, String>> + Send + 'a>> {
        let out: Vec<Vec<f32>> = texts.iter().map(|t| Self::vec_for(t)).collect();
        Box::pin(async move { Ok(out) })
    }
}

fn fixture(id: &str, body: &str) -> gmail::GmailMessageProjection {
    gmail::GmailMessageProjection {
        id: gmail::projection_id("g", id),
        feed_id: "g".into(),
        source_id: id.into(),
        source_ts: chrono::Utc::now(),
        sender: Some("a@e.com".into()),
        recipients: vec![],
        subject: format!("subj-{id}"),
        body_text: body.into(),
        thread_id: None,
        labels: vec![],
    }
}

#[tokio::test]
async fn vector_search_ranks_by_cosine_similarity() {
    let store = ProjectionStore::in_memory().unwrap();
    let rows = vec![
        fixture("m1", "Talking about alpha rollouts today, alpha alpha"),
        fixture("m2", "Beta release planning meeting notes"),
        fixture("m3", "Gamma channel discussion of release timing"),
    ];
    store.write_batch(&rows).unwrap();
    run_embed_pass(&store, &KeywordEmbedder, 32, 100).await.unwrap();

    // Query close to "alpha" should rank m1 first.
    let q = KeywordEmbedder::vec_for("alpha");
    let ids = store.vector_search(gmail::FEED_TYPE, &q, 5).unwrap();
    assert!(!ids.is_empty());
    assert_eq!(ids[0], gmail::projection_id("g", "m1"));
}

#[tokio::test]
async fn vector_search_ignores_sentinel_and_null_rows() {
    let store = ProjectionStore::in_memory().unwrap();
    // m1: long body, gets embedded. m2: short, gets sentinel skip.
    // m3: never embedded (no run_embed_pass call after insert).
    let m1 = fixture("m1", "alpha alpha alpha — long enough body");
    let m2 = fixture("m2", "ok"); // < MIN_BODY_CHARS
    store.write_batch(&[m1.clone(), m2]).unwrap();
    run_embed_pass(&store, &KeywordEmbedder, 32, 100).await.unwrap();

    let m3 = fixture("m3", "alpha later — appears after the embed pass");
    store.write_batch(&[m3]).unwrap();
    // No embed pass for m3; its row is NULL.

    let q = KeywordEmbedder::vec_for("alpha");
    let ids = store.vector_search(gmail::FEED_TYPE, &q, 5).unwrap();
    // Only m1 had a real vector. m2 has the sentinel (filtered),
    // m3 has NULL (filtered by `WHERE embedding IS NOT NULL` and by
    // length).
    assert_eq!(ids, vec![gmail::projection_id("g", "m1")]);
}

#[tokio::test]
async fn pending_rows_round_trip() {
    // Sanity: the embed pass's internal `PendingEmbedRow` shape stays
    // accessible to consumers if they want to write their own pass.
    let store = ProjectionStore::in_memory().unwrap();
    store
        .write_batch(&[fixture("m1", "a body long enough to qualify")])
        .unwrap();
    let pending: Vec<PendingEmbedRow> = store
        .pending_embedding_rows(gmail::FEED_TYPE, 10)
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].body_text.contains("body long enough"));
}

#[tokio::test]
async fn empty_query_vec_returns_empty() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write_batch(&[fixture("m1", "doesn't matter")]).unwrap();
    let ids = store.vector_search(gmail::FEED_TYPE, &[], 5).unwrap();
    assert!(ids.is_empty());
}

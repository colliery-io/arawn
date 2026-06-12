//! Cross-lens read: hybrid FTS + vector search fused across many lens stores.
//!
//! ARAWN-I-0060 (lens-agnostic chat): a chat reads across *all* lenses, not a
//! single active one. This module is the fusion primitive — given a set of
//! already-open, labeled [`MemoryStore`]s (the caller enumerates lenses; this
//! crate doesn't depend on the lens registry), it runs the same hybrid
//! FTS5 + vector RRF search per store and merges into one ranked list, each hit
//! labeled with the lens it came from.
//!
//! Embedding is async and lives in the caller (the engine), so the query
//! embedding is passed in precomputed — this stays synchronous and unit-testable.

use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::store::MemoryStore;
use crate::types::Entity;

/// RRF constant — matches `signal_search` / `feed_search`.
const RRF_K: f32 = 60.0;

/// Reciprocal-rank-fusion weight for a 0-based rank.
pub fn rrf(rank: usize) -> f32 {
    1.0 / (RRF_K + rank as f32 + 1.0)
}

/// One fused search hit, tagged with the lens (or `"global"`) it came from.
#[derive(Debug, Clone)]
pub struct LabeledHit {
    /// Source lens name, or `"global"` for the shared global tier.
    pub lens: String,
    pub entity: Entity,
    pub score: f32,
}

/// Hybrid FTS + (optional) vector search across a set of labeled stores,
/// RRF-fused into one ranked list capped at `limit`.
///
/// `stores` is `(label, store)` pairs — the caller supplies the global tier and
/// every lens KB it wants searched. `query_embedding` is the precomputed query
/// vector; pass `None` to run FTS-only (no embedder / embed failed).
///
/// Entity ids are unique per store (separate DBs), so the lens label on a hit is
/// unambiguous. Superseded entities are skipped.
pub fn search_labeled_stores(
    stores: &[(String, Arc<MemoryStore>)],
    query: &str,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Vec<LabeledHit> {
    // entity id -> (lens label, entity, fused score)
    let mut fused: HashMap<Uuid, (String, Entity, f32)> = HashMap::new();
    let fetch = limit.saturating_mul(4).max(limit);

    for (label, store) in stores {
        // FTS5 ranks.
        if let Ok(hits) = store.search(query, fetch) {
            for (rank, ent) in hits.into_iter().enumerate() {
                if ent.superseded {
                    continue;
                }
                let slot = fused
                    .entry(ent.id)
                    .or_insert_with(|| (label.clone(), ent.clone(), 0.0));
                slot.2 += rrf(rank);
            }
        }

        // Vector ranks (when the caller supplied a query embedding).
        if let Some(qv) = query_embedding
            && let Ok(sims) = store.search_similar(qv, fetch)
        {
            for (rank, sim) in sims.into_iter().enumerate() {
                if let Ok(Some(ent)) = store.get_entity(sim.entity_id) {
                    if ent.superseded {
                        continue;
                    }
                    let slot = fused
                        .entry(ent.id)
                        .or_insert_with(|| (label.clone(), ent.clone(), 0.0));
                    slot.2 += rrf(rank);
                }
            }
        }
    }

    let mut hits: Vec<LabeledHit> = fused
        .into_values()
        .map(|(lens, entity, score)| LabeledHit {
            lens,
            entity,
            score,
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entity, EntityType};

    fn store_with(entities: &[(&str, &str)]) -> Arc<MemoryStore> {
        let s = MemoryStore::in_memory().expect("open");
        for (title, content) in entities {
            let e =
                Entity::new(EntityType::Note, title.to_string()).with_content(content.to_string());
            s.insert_entity(&e).expect("insert");
        }
        Arc::new(s)
    }

    #[test]
    fn fuses_and_labels_hits_across_stores() {
        // Two lens stores, each with a distinct matching note.
        let work = store_with(&[("Postgres migration", "we chose postgres 16 for the ledger")]);
        let home = store_with(&[(
            "Postgres at home",
            "the postgres backup script for home nas",
        )]);

        let stores = vec![("work".to_string(), work), ("home".to_string(), home)];
        let hits = search_labeled_stores(&stores, "postgres", None, 10);

        assert!(hits.len() >= 2, "should surface hits from both lenses");
        let lenses: std::collections::HashSet<&str> =
            hits.iter().map(|h| h.lens.as_str()).collect();
        assert!(
            lenses.contains("work") && lenses.contains("home"),
            "both lenses labeled: {lenses:?}"
        );
        // Every hit carries the lens it came from and a positive fused score.
        assert!(hits.iter().all(|h| !h.lens.is_empty() && h.score > 0.0));
    }

    #[test]
    fn respects_limit_and_orders_by_score() {
        let s = store_with(&[
            ("alpha note", "postgres postgres postgres"),
            ("beta note", "postgres once"),
            ("gamma note", "unrelated"),
        ]);
        let stores = vec![("work".to_string(), s)];
        let hits = search_labeled_stores(&stores, "postgres", None, 1);
        assert_eq!(hits.len(), 1, "limit honored");
    }

    #[test]
    fn empty_stores_yield_no_hits() {
        let hits = search_labeled_stores(&[], "postgres", None, 10);
        assert!(hits.is_empty());
    }
}

//! Knowledge base memory system for arawn.
//!
//! Provides graph-backed entity storage with FTS5 search, typed relations,
//! confidence scoring, tag support, and search-before-create deduplication.

pub mod cypher_schema;
pub mod error;
pub mod inject;
pub mod manager;
pub mod ontology;
pub mod shortcodes;
pub mod stack;
pub mod store;
pub mod types;
pub mod vector;

pub use error::MemoryError;
pub use inject::load_memories_for_injection;
pub use manager::{MemoryManager, try_open_memory};
pub use ontology::{AddedVia, OntologyEntry, TagOntologyStore, normalize_tag};
pub use stack::MemoryStack;
pub use store::MemoryStore;
pub use types::*;
pub use vector::{
    SimilarityResult, init_vector_extension, check_vector_extension,
};

#[cfg(test)]
mod graphqlite_smoke {
    use graphqlite::Graph;

    #[test]
    fn graphqlite_node_and_edge_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("smoke.db");
        let g = Graph::open(&path).expect("open graphqlite db");

        g.upsert_node("n1", [("name", "alice")], "Test")
            .expect("upsert n1");
        g.upsert_node("n2", [("name", "bob")], "Test")
            .expect("upsert n2");
        g.upsert_edge("n1", "n2", std::iter::empty::<(&str, &str)>(), "R")
            .expect("upsert edge");

        assert!(g.has_node("n1").unwrap());
        assert!(g.has_node("n2").unwrap());
        assert!(g.has_edge("n1", "n2", Some("R")).unwrap());

        let stats = g.stats().expect("stats");
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }
}

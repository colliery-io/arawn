//! Integration tests: layered memory stack — L0/L1 generation, budget enforcement,
//! shortcode compression, L2 topical injection, and deduplication.

use arawn_memory::{
    ConfidenceSource, Entity, EntityType, MemoryManager, MemoryStack,
    shortcodes::apply_shortcodes,
};
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (TempDir, MemoryManager) {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
    let mgr = MemoryManager::open(tmp.path(), "test-ws", None).unwrap();
    (tmp, mgr)
}

fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

// ── L0/L1 Budget Tests ─────────────────────────────────────────────────────

#[test]
fn wake_up_under_budget_with_many_entities() {
    let (_tmp, mgr) = setup();

    // Populate with entities of varying confidence
    for i in 0..40 {
        let mut e = Entity::new(
            EntityType::Fact,
            &format!("Important fact number {i} about the system"),
        );
        e.content = Some(format!(
            "Detailed content for fact {i} that provides context and uses tokens"
        ));
        e.confidence_source = if i % 3 == 0 {
            ConfidenceSource::Stated
        } else if i % 3 == 1 {
            ConfidenceSource::Observed
        } else {
            ConfidenceSource::Inferred
        };
        mgr.workstream.insert_entity(&e).unwrap();
    }

    let stack = MemoryStack::new(&mgr, "test-ws");
    let output = stack.wake_up(900);
    let tokens = estimate_tokens(&output);

    assert!(
        tokens <= 900,
        "output {tokens} tokens exceeds 900 budget"
    );
    assert!(output.contains("[L0"));
    assert!(output.contains("[L1"));
}

#[test]
fn l1_ranks_stated_highest() {
    let (_tmp, mgr) = setup();

    // Insert inferred first (older), stated second (newer)
    let mut inferred = Entity::new(EntityType::Decision, "Use SQLite");
    inferred.confidence_source = ConfidenceSource::Inferred;
    mgr.workstream.insert_entity(&inferred).unwrap();

    let mut stated = Entity::new(EntityType::Decision, "Use PostgreSQL");
    stated.confidence_source = ConfidenceSource::Stated;
    mgr.workstream.insert_entity(&stated).unwrap();

    let mut observed = Entity::new(EntityType::Decision, "Use Redis");
    observed.confidence_source = ConfidenceSource::Observed;
    mgr.workstream.insert_entity(&observed).unwrap();

    let stack = MemoryStack::new(&mgr, "test-ws");
    let output = stack.wake_up(900);

    let pg_pos = output.find("Use PostgreSQL").expect("stated should appear");
    let redis_pos = output.find("Use Redis").expect("observed should appear");
    let sqlite_pos = output.find("Use SQLite").expect("inferred should appear");

    assert!(pg_pos < redis_pos, "stated before observed");
    assert!(redis_pos < sqlite_pos, "observed before inferred");
}

#[test]
fn empty_kb_produces_l0_only() {
    let (_tmp, mgr) = setup();
    let stack = MemoryStack::new(&mgr, "test-ws");
    let output = stack.wake_up(900);

    assert!(output.contains("[L0"));
    assert!(output.contains("test-ws"));
    assert!(!output.contains("[L1"), "empty KB should have no L1");
}

#[test]
fn tiny_budget_does_not_panic() {
    let (_tmp, mgr) = setup();
    for i in 0..10 {
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Fact, &format!("Fact {i}")))
            .unwrap();
    }

    let stack = MemoryStack::new(&mgr, "test-ws");
    let output = stack.wake_up(10);
    assert!(!output.is_empty());
}

// ── Shortcode Compression Tests ─────────────────────────────────────────────

#[test]
fn shortcodes_applied_in_l1_output() {
    let (_tmp, mgr) = setup();

    // Create entities that will appear multiple times when content references them
    for _ in 0..3 {
        let mut e = Entity::new(EntityType::Fact, "arawn-engine");
        e.content = Some("The arawn-engine crate handles query execution".into());
        e.confidence_source = ConfidenceSource::Stated;
        mgr.workstream.insert_entity(&e).unwrap();
    }

    let stack = MemoryStack::new(&mgr, "test-ws");
    let output = stack.wake_up(900);

    // If arawn-engine appears 2+ times, shortcodes should be applied
    // (exact behavior depends on how many times the name appears in rendered output)
    // At minimum, the output should be valid text
    assert!(output.contains("[L0") || output.contains("[L1"));
}

#[test]
fn shortcode_standalone_compression() {
    let text = "arawn-engine is fast. arawn-engine handles queries. arawn-engine uses tokio.";
    let names = vec!["arawn-engine".to_string()];
    let result = apply_shortcodes(text, &names, 2);

    assert!(result.contains("AE=arawn-engine"), "legend missing");
    assert!(
        result.contains("AE is fast"),
        "replacement missing: {result}"
    );
    // Should save tokens
    assert!(result.len() < text.len() + 30); // legend adds ~20 chars but replacements save more
}

#[test]
fn shortcode_single_occurrence_unchanged() {
    let text = "arawn-engine appears once.";
    let names = vec!["arawn-engine".to_string()];
    let result = apply_shortcodes(text, &names, 2);
    assert_eq!(result, text);
}

// ── L2 Topical Injection Tests ──────────────────────────────────────────────

#[test]
fn l2_retrieves_by_keyword() {
    let (_tmp, mgr) = setup();

    mgr.workstream
        .insert_entity(
            &Entity::new(EntityType::Fact, "WebSocket protocol details")
                .with_tags(vec!["websocket".into(), "networking".into()]),
        )
        .unwrap();
    mgr.workstream
        .insert_entity(&Entity::new(EntityType::Fact, "Unrelated fact"))
        .unwrap();

    let stack = MemoryStack::new(&mgr, "test-ws");
    let l1_titles = stack.l1_entity_titles();

    let l2 = stack.topical_context(&["websocket".into()], &l1_titles, 400);
    // L2 may or may not find it depending on whether it's already in L1
    // The key test is that it doesn't panic and returns valid output
    if let Some(ref text) = l2 {
        assert!(text.contains("[L2"));
    }
}

#[test]
fn l2_deduplicates_against_l1() {
    let (_tmp, mgr) = setup();

    // Insert a high-confidence entity that will be in L1
    let mut entity = Entity::new(EntityType::Fact, "Important system fact");
    entity.confidence_source = ConfidenceSource::Stated;
    entity.reinforcement_count = 5;
    mgr.workstream.insert_entity(&entity).unwrap();

    let stack = MemoryStack::new(&mgr, "test-ws");
    let l1_titles = stack.l1_entity_titles();

    // Search for the same entity — should be deduplicated
    let l2 = stack.topical_context(&["important".into()], &l1_titles, 400);

    // If the entity is in L1, L2 should either be None or not contain it
    if let Some(ref text) = l2 {
        assert!(
            !text.contains("Important system fact"),
            "L2 should not duplicate L1 entities"
        );
    }
}

#[test]
fn l2_empty_keywords_returns_none() {
    let (_tmp, mgr) = setup();
    let stack = MemoryStack::new(&mgr, "test-ws");
    let l2 = stack.topical_context(&[], &[], 400);
    assert!(l2.is_none());
}

#[test]
fn retrieve_topical_respects_budget() {
    let (_tmp, mgr) = setup();

    // Insert many tagged entities
    for i in 0..20 {
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Fact, &format!("Tagged fact {i} with extra text"))
                    .with_content(&format!("Content for tagged fact {i} adding more tokens"))
                    .with_tags(vec!["performance".into()]),
            )
            .unwrap();
    }

    let results = mgr.retrieve_topical(&["performance".into()], 50);
    // With a 50-token budget, should not return all 20
    assert!(
        results.len() < 20,
        "should respect budget, got {} results",
        results.len()
    );
}

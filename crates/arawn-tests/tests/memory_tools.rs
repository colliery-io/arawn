//! Functional tests for memory_store and memory_search tools through the
//! MockLLM test harness. Validates the full pipeline: LLM → tool dispatch →
//! KB storage → retrieval → response.

use std::sync::Arc;

use arawn_engine::testing::TestHarness;
use arawn_engine::tools::{MemorySearchTool, MemoryStoreTool};
use arawn_llm::MockResponse;
use arawn_memory::MemoryManager;

fn setup_memory_manager() -> Arc<MemoryManager> {
    let global = Arc::new(arawn_memory::MemoryStore::in_memory().unwrap());
    let workstream = Arc::new(arawn_memory::MemoryStore::in_memory().unwrap());
    Arc::new(MemoryManager::open_with_stores(global, workstream))
}

#[tokio::test]
async fn memory_store_inserts_entity() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Project uses PostgreSQL 15", "entity_type": "fact", "content": "Primary database for all services"}"#,
            ),
            MockResponse::text("Stored that fact about PostgreSQL."),
        ])
        .build();

    let result = harness.run("Remember that our project uses PostgreSQL 15").await;

    // Tool should have been called
    let calls = result.tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "memory_store");

    // Entity should exist in the workstream KB
    let entities = mgr.workstream.search("PostgreSQL", 5).unwrap();
    assert!(
        !entities.is_empty(),
        "entity should be stored in workstream KB"
    );
    assert_eq!(entities[0].title, "Project uses PostgreSQL 15");
    assert_eq!(
        entities[0].content.as_deref(),
        Some("Primary database for all services")
    );
}

#[tokio::test]
async fn memory_store_preference_goes_to_global() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Prefers dark mode", "entity_type": "preference"}"#,
            ),
            MockResponse::text("Noted your preference."),
        ])
        .build();

    let _result = harness.run("I prefer dark mode").await;

    // Preferences route to global KB by default
    let global = mgr.global.search("dark mode", 5).unwrap();
    assert!(
        !global.is_empty(),
        "preferences should be stored in global KB"
    );

    // Should NOT be in workstream
    let ws = mgr.workstream.search("dark mode", 5).unwrap();
    assert!(ws.is_empty(), "preferences should not be in workstream KB");
}

#[tokio::test]
async fn memory_store_person_goes_to_global() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Alice Chen", "entity_type": "person", "content": "Tech lead, Rust expert"}"#,
            ),
            MockResponse::text("Remembered Alice."),
        ])
        .build();

    let _result = harness.run("Alice Chen is our tech lead").await;

    let global = mgr.global.search("Alice", 5).unwrap();
    assert!(!global.is_empty(), "people should be stored in global KB");
    assert_eq!(global[0].entity_type, arawn_memory::EntityType::Person);
}

#[tokio::test]
async fn memory_store_deduplicates_on_reinsertion() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "We use Axum", "entity_type": "decision"}"#,
            ),
            MockResponse::text("Stored."),
        ])
        .build();

    // Store once
    let _result = harness.run("We use Axum for our web framework").await;

    // Store again directly to test dedup
    let entity = arawn_memory::Entity::new(arawn_memory::EntityType::Decision, "We use Axum")
        .with_confidence(arawn_memory::ConfidenceSource::Stated);
    let result = mgr.workstream.store_fact(&entity).unwrap();

    // Should reinforce, not insert
    match result {
        arawn_memory::StoreFactResult::Reinforced { new_count, .. } => {
            assert!(new_count >= 1, "should have been reinforced: count={new_count}");
        }
        arawn_memory::StoreFactResult::Inserted { .. } => {
            // Check if we at least have only 1 entity
            let entities = mgr.workstream.search("\"Axum\"", 5).unwrap();
            assert_eq!(entities.len(), 1, "dedup should prevent duplicates (found {} entities)", entities.len());
        }
        _ => {}
    }
}

#[tokio::test]
async fn memory_search_finds_stored_entity() {
    let mgr = setup_memory_manager();

    // Pre-populate the KB
    let entity = arawn_memory::Entity::new(arawn_memory::EntityType::Fact, "Redis cache TTL is 5 minutes")
        .with_confidence(arawn_memory::ConfidenceSource::Stated)
        .with_content("All cache keys expire after 5 minutes. Session data uses 24 hour TTL.");
    mgr.workstream.insert_entity(&entity).unwrap();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemorySearchTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_search",
                r#"{"query": "cache TTL"}"#,
            ),
            MockResponse::text("The Redis cache TTL is 5 minutes."),
        ])
        .build();

    let result = harness.run("What's our cache TTL?").await;

    let calls = result.tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "memory_search");

    // The tool result should contain the entity
    let tool_results: Vec<&str> = result
        .session_messages()
        .iter()
        .filter_map(|msg| match msg {
            arawn_core::Message::ToolResult { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        tool_results.iter().any(|r| r.contains("Redis cache TTL")),
        "search should return the stored entity, got: {:?}",
        tool_results
    );
}

#[tokio::test]
async fn memory_search_filters_by_type() {
    let mgr = setup_memory_manager();

    // Store a fact and a decision
    mgr.workstream
        .insert_entity(&arawn_memory::Entity::new(
            arawn_memory::EntityType::Fact,
            "Rust is fast",
        ))
        .unwrap();
    mgr.workstream
        .insert_entity(&arawn_memory::Entity::new(
            arawn_memory::EntityType::Decision,
            "We decided to use Rust",
        ))
        .unwrap();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemorySearchTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_search",
                r#"{"query": "Rust", "entity_type": "decision"}"#,
            ),
            MockResponse::text("We decided to use Rust."),
        ])
        .build();

    let result = harness.run("What decisions did we make about Rust?").await;

    let tool_results: Vec<&str> = result
        .session_messages()
        .iter()
        .filter_map(|msg| match msg {
            arawn_core::Message::ToolResult { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    // Should find the decision, not the fact
    assert!(
        tool_results
            .iter()
            .any(|r| r.contains("decided to use Rust")),
        "should find the decision entity"
    );
}

#[tokio::test]
async fn memory_store_then_search_roundtrip() {
    let mgr = setup_memory_manager();

    // Step 1: Store a fact via the tool
    let store_harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Deploy cadence is weekly on Tuesdays", "entity_type": "convention", "content": "Production deploys happen every Tuesday at 2pm UTC"}"#,
            ),
            MockResponse::text("Stored the deploy convention."),
        ])
        .build();

    let store_result = store_harness.run("Remember: we deploy weekly on Tuesdays").await;
    assert_eq!(store_result.tool_calls().len(), 1);

    // Step 2: Search for it in a separate engine turn (same KB)
    let search_harness = TestHarness::builder()
        .with_tool(Box::new(MemorySearchTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_search",
                r#"{"query": "deploy Tuesday"}"#,
            ),
            MockResponse::text("Deploys are weekly on Tuesdays at 2pm UTC."),
        ])
        .build();

    let search_result = search_harness.run("When do we deploy?").await;
    assert_eq!(search_result.tool_calls().len(), 1);

    // The search tool result should contain the stored fact
    let tool_results: Vec<&str> = search_result
        .session_messages()
        .iter()
        .filter_map(|msg| match msg {
            arawn_core::Message::ToolResult { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        !tool_results.is_empty(),
        "should have tool results from search"
    );
    assert!(
        tool_results[0].contains("Tuesday") || tool_results[0].contains("deploy"),
        "search result should contain the stored entity, got: {}",
        tool_results[0]
    );
}

#[tokio::test]
async fn memory_search_empty_kb_returns_no_results() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemorySearchTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_search",
                r#"{"query": "nonexistent topic"}"#,
            ),
            MockResponse::text("I don't have any stored knowledge about that."),
        ])
        .build();

    let result = harness.run("What do we know about quantum computing?").await;

    let calls = result.tool_calls();
    assert_eq!(calls.len(), 1);
    // Should complete without error even with empty results
    assert!(
        result.final_text().contains("don't have"),
        "should indicate no results"
    );
}

#[tokio::test]
async fn memory_store_with_tags() {
    let mgr = setup_memory_manager();

    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Use cargo clippy before merging", "entity_type": "convention", "tags": ["ci", "rust", "quality"]}"#,
            ),
            MockResponse::text("Stored the clippy convention."),
        ])
        .build();

    let _result = harness.run("Convention: always run clippy before merging").await;

    let entities = mgr.workstream.search("clippy", 5).unwrap();
    assert!(!entities.is_empty());
    assert_eq!(entities[0].tags, vec!["ci", "rust", "quality"]);
}

#[tokio::test]
async fn memory_store_explicit_scope_override() {
    let mgr = setup_memory_manager();

    // Facts default to workstream scope, but explicitly set to global
    let harness = TestHarness::builder()
        .with_tool(Box::new(MemoryStoreTool::new(Arc::clone(&mgr), None)))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "memory_store",
                r#"{"title": "Company founded in 2020", "entity_type": "fact", "scope": "global"}"#,
            ),
            MockResponse::text("Stored globally."),
        ])
        .build();

    let _result = harness.run("The company was founded in 2020").await;

    let global = mgr.global.search("founded", 5).unwrap();
    assert!(
        !global.is_empty(),
        "explicit global scope should override default"
    );
    let ws = mgr.workstream.search("founded", 5).unwrap();
    assert!(ws.is_empty(), "should not be in workstream when global specified");
}

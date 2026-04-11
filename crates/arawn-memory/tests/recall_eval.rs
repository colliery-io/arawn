//! Recall evaluation suite for the arawn-memory knowledge base.
//!
//! Measures retrieval quality across FTS search, MemoryStack L1/L2, and
//! topical retrieval. Reports recall@K, precision@K, and MRR metrics.

use std::collections::HashSet;
use std::sync::Arc;

use arawn_memory::*;

// ============================================================================
// Metrics
// ============================================================================

/// Recall@K: fraction of expected entities found in the top-K results.
fn recall_at_k(results: &[Entity], expected_titles: &[&str], k: usize) -> f64 {
    if expected_titles.is_empty() {
        return 1.0; // vacuously true
    }
    let top_k: HashSet<&str> = results.iter().take(k).map(|e| e.title.as_str()).collect();
    let hits = expected_titles
        .iter()
        .filter(|t| top_k.contains(**t))
        .count();
    hits as f64 / expected_titles.len() as f64
}

/// Precision@K: fraction of top-K results that are in the expected set.
fn precision_at_k(results: &[Entity], expected_titles: &[&str], k: usize) -> f64 {
    let top_k: Vec<&str> = results.iter().take(k).map(|e| e.title.as_str()).collect();
    if top_k.is_empty() {
        return 0.0;
    }
    let expected_set: HashSet<&str> = expected_titles.iter().copied().collect();
    let hits = top_k.iter().filter(|t| expected_set.contains(**t)).count();
    hits as f64 / top_k.len() as f64
}

/// Mean Reciprocal Rank: 1/rank of the first relevant result.
fn mrr(results: &[Entity], expected_titles: &[&str]) -> f64 {
    let expected_set: HashSet<&str> = expected_titles.iter().copied().collect();
    for (i, entity) in results.iter().enumerate() {
        if expected_set.contains(entity.title.as_str()) {
            return 1.0 / (i + 1) as f64;
        }
    }
    0.0
}

// ============================================================================
// Fixture KB
// ============================================================================

/// Build a populated MemoryStore with realistic entities for evaluation.
fn build_fixture_store() -> Arc<MemoryStore> {
    let store = Arc::new(MemoryStore::in_memory().unwrap());

    // === Facts (10) ===
    let facts = [
        ("Rust performance tuning", Some("Use cargo flamegraph for profiling. Avoid unnecessary allocations in hot loops. Prefer stack allocation over heap where possible.")),
        ("PostgreSQL connection pooling", Some("We use deadpool-diesel with max 20 connections per service. Connection timeout is 5 seconds.")),
        ("API rate limiting", Some("Rate limit is 100 requests per minute per API key. Burst of 20 allowed.")),
        ("Deployment pipeline uses GitHub Actions", Some("CI runs on push to main. Deploys to staging automatically, production requires manual approval.")),
        ("Session data stored in Redis", Some("Sessions expire after 24 hours. Redis cluster with 3 nodes.")),
        ("Frontend uses React 18 with TypeScript", Some("Strict TypeScript config. No any types allowed in production code.")),
        ("Database migrations use refinery", Some("Migrations are SQL files in migrations/ directory. Applied on server startup.")),
        ("Logging uses structured JSON format", Some("All services emit JSON logs to stdout. Collected by Datadog agent.")),
        ("Authentication via OAuth2 with PKCE", Some("Auth provider is Auth0. Refresh tokens valid for 30 days.")),
        ("Search index is Meilisearch", Some("Full-text search for user content. Reindexed every 5 minutes from PostgreSQL.")),
    ];

    for (title, content) in &facts {
        let mut entity = Entity::new(EntityType::Fact, *title)
            .with_confidence(ConfidenceSource::Stated);
        if let Some(c) = content {
            entity = entity.with_content(*c);
        }
        store.store_fact(&entity).unwrap();
    }

    // === Decisions (8) ===
    let decisions = [
        ("We decided to use gRPC for inter-service communication", Some("REST was too slow for real-time features. gRPC gives us streaming and better performance.")),
        ("We decided to monorepo all services", Some("Easier dependency management and atomic cross-service changes.")),
        ("We decided against GraphQL for v1", Some("Too much complexity for our team size. REST with OpenAPI is sufficient.")),
        ("Decided to use SQLite for local development", Some("Production uses PostgreSQL but SQLite simplifies local dev setup.")),
        ("Decision: no ORM, raw SQL with type-safe queries", Some("ORMs hide performance issues. We use sqlx for compile-time checked queries.")),
        ("We chose Axum over Actix for the web framework", Some("Better tokio integration, simpler middleware model, and tower compatibility.")),
        ("Decided to run background jobs via cloacina pipelines", Some("Replaces ad-hoc cron jobs with observable DAG workflows.")),
        ("Decision: feature flags via LaunchDarkly", Some("Gradual rollouts, A/B testing, kill switches for all new features.")),
    ];

    for (title, content) in &decisions {
        let mut entity = Entity::new(EntityType::Decision, *title)
            .with_confidence(ConfidenceSource::Stated);
        if let Some(c) = content {
            entity = entity.with_content(*c);
        }
        store.store_fact(&entity).unwrap();
    }

    // === Conventions (8) ===
    let conventions = [
        ("Always use snake_case for Rust identifiers", None),
        ("Never commit directly to main — always use PRs", None),
        ("Every PR requires at least one approval", None),
        ("Error types implement thiserror::Error", None),
        ("Tests go in inline #[cfg(test)] modules, not separate files", None),
        ("Use tracing crate for all logging, not println", None),
        ("Commit messages follow conventional commits format", Some("feat:, fix:, refactor:, docs:, test:, chore:")),
        ("All public functions must have doc comments", None),
    ];

    for (title, content) in &conventions {
        let mut entity = Entity::new(EntityType::Convention, *title)
            .with_confidence(ConfidenceSource::Stated);
        if let Some(c) = content {
            entity = entity.with_content(*c);
        }
        store.store_fact(&entity).unwrap();
    }

    // === Preferences (8) ===
    let preferences = [
        "Prefer explicit error handling over unwrap",
        "Prefer composition over inheritance",
        "Keep functions under 50 lines",
        "Prefer iterators over manual loops",
        "Dark mode for all editor themes",
        "Use 4-space indentation in Rust",
        "Prefer async over threading for I/O bound work",
        "Minimize dependencies — check crate quality before adding",
    ];

    for title in &preferences {
        let entity = Entity::new(EntityType::Preference, *title)
            .with_confidence(ConfidenceSource::Stated);
        store.store_fact(&entity).unwrap();
    }

    // === People (8) ===
    let people = [
        ("Alice Chen", Some("Tech lead. Rust expert. Owns the engine crate.")),
        ("Bob Martinez", Some("Backend engineer. Works on API and database layers.")),
        ("Carol Singh", Some("Frontend engineer. React specialist.")),
        ("Dave Kim", Some("DevOps. Manages CI/CD, infrastructure, and monitoring.")),
        ("Eve Thompson", Some("Product manager. Prioritizes features and roadmap.")),
        ("Frank Liu", Some("Security engineer. Reviews auth and permission code.")),
        ("Grace Park", Some("QA lead. Writes integration tests and regression suites.")),
        ("Hiro Tanaka", Some("Data engineer. Manages search index and analytics pipeline.")),
    ];

    for (title, content) in &people {
        let mut entity = Entity::new(EntityType::Person, *title)
            .with_confidence(ConfidenceSource::Stated);
        if let Some(c) = content {
            entity = entity.with_content(*c);
        }
        store.store_fact(&entity).unwrap();
    }

    // === Notes (8) ===
    let notes = [
        ("Memory system uses two-tier architecture", Some("Global KB for cross-workstream knowledge, workstream KB for project-specific facts.")),
        ("Compaction threshold is 85% of context window", None),
        ("Agent nesting limited to depth 3", None),
        ("WebSocket auth uses per-session tokens", None),
        ("MCP servers connect via stdio transport", None),
        ("Plugin hot-reload watches filesystem for changes", None),
        ("TUI uses ratatui with crossterm backend", Some("Terminal UI renders markdown, tables, and code blocks with syntax highlighting.")),
        ("Embedding model is 384-dimensional", Some("Uses ONNX runtime for local inference. No external API calls for embeddings.")),
    ];

    for (title, content) in &notes {
        let mut entity = Entity::new(EntityType::Note, *title)
            .with_confidence(ConfidenceSource::Observed);
        if let Some(c) = content {
            entity = entity.with_content(*c);
        }
        store.store_fact(&entity).unwrap();
    }

    // === Reinforced entities (boost ranking) ===
    // Re-store some entities to bump reinforcement_count
    for _ in 0..3 {
        let entity = Entity::new(EntityType::Fact, "Rust performance tuning")
            .with_confidence(ConfidenceSource::Stated);
        store.store_fact(&entity).unwrap(); // reinforces
    }
    for _ in 0..2 {
        let entity = Entity::new(EntityType::Convention, "Always use snake_case for Rust identifiers")
            .with_confidence(ConfidenceSource::Stated);
        store.store_fact(&entity).unwrap();
    }

    // === Superseded entity (should NEVER appear in results) ===
    let old_decision = Entity::new(EntityType::Decision, "We use REST for all APIs")
        .with_confidence(ConfidenceSource::Stated)
        .with_content("Old decision — superseded by gRPC decision");
    let old_result = store.store_fact(&old_decision).unwrap();
    // Supersede it with a new entity
    if let StoreFactResult::Inserted { entity_id } = old_result {
        let replacement = Entity::new(EntityType::Decision, "REST is deprecated in favor of gRPC")
            .with_confidence(ConfidenceSource::Stated);
        store.supersede_entity(entity_id, &replacement).ok();
    }

    store
}

/// Build a MemoryManager for stack tests using the fixture store.
fn build_fixture_manager() -> (Arc<MemoryStore>, MemoryManager) {
    let store = build_fixture_store();
    let mgr = MemoryManager::open_with_stores(
        Arc::clone(&store),
        Arc::new(MemoryStore::in_memory().unwrap()),
    );
    (store, mgr)
}

// ============================================================================
// Query Corpus
// ============================================================================

struct QueryCase {
    description: &'static str,
    query: &'static str,
    expected: Vec<&'static str>,
    category: QueryCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryCategory {
    ExactTitle,
    KeywordOverlap,
    ContentSearch,
    Paraphrase,
    Negative,
}

fn build_query_corpus() -> Vec<QueryCase> {
    vec![
        // --- Exact title matches ---
        QueryCase {
            description: "exact: Rust performance tuning",
            query: "Rust performance tuning",
            expected: vec!["Rust performance tuning"],
            category: QueryCategory::ExactTitle,
        },
        QueryCase {
            description: "exact: PostgreSQL connection pooling",
            query: "PostgreSQL connection pooling",
            expected: vec!["PostgreSQL connection pooling"],
            category: QueryCategory::ExactTitle,
        },
        QueryCase {
            description: "exact: Alice Chen",
            query: "Alice Chen",
            expected: vec!["Alice Chen"],
            category: QueryCategory::ExactTitle,
        },
        QueryCase {
            description: "exact: snake_case convention",
            query: "snake_case",
            expected: vec!["Always use snake_case for Rust identifiers"],
            category: QueryCategory::ExactTitle,
        },
        QueryCase {
            description: "exact: gRPC decision",
            query: "gRPC",
            expected: vec!["We decided to use gRPC for inter-service communication"],
            category: QueryCategory::ExactTitle,
        },

        // --- Keyword overlap ---
        QueryCase {
            description: "keyword: rust",
            query: "rust",
            expected: vec![
                "Rust performance tuning",
                "Always use snake_case for Rust identifiers",
            ],
            category: QueryCategory::KeywordOverlap,
        },
        QueryCase {
            description: "keyword: database",
            query: "database",
            expected: vec![
                "Database migrations use refinery",
            ],
            category: QueryCategory::KeywordOverlap,
        },
        QueryCase {
            description: "keyword: deployment",
            query: "deployment",
            expected: vec!["Deployment pipeline uses GitHub Actions"],
            category: QueryCategory::KeywordOverlap,
        },
        QueryCase {
            description: "keyword: testing",
            query: "testing",
            expected: vec!["Tests go in inline #[cfg(test)] modules, not separate files"],
            category: QueryCategory::KeywordOverlap,
        },
        QueryCase {
            description: "keyword: security",
            query: "security",
            expected: vec!["Frank Liu"],
            category: QueryCategory::KeywordOverlap,
        },

        // --- Content search (matches body, not title) ---
        QueryCase {
            description: "content: flamegraph",
            query: "flamegraph",
            expected: vec!["Rust performance tuning"],
            category: QueryCategory::ContentSearch,
        },
        QueryCase {
            description: "content: Datadog",
            query: "Datadog",
            expected: vec!["Logging uses structured JSON format"],
            category: QueryCategory::ContentSearch,
        },
        QueryCase {
            description: "content: Auth0",
            query: "Auth0",
            expected: vec!["Authentication via OAuth2 with PKCE"],
            category: QueryCategory::ContentSearch,
        },
        QueryCase {
            description: "content: tower compatibility",
            query: "tower",
            expected: vec!["We chose Axum over Actix for the web framework"],
            category: QueryCategory::ContentSearch,
        },
        QueryCase {
            description: "content: ratatui",
            query: "ratatui",
            expected: vec!["TUI uses ratatui with crossterm backend"],
            category: QueryCategory::ContentSearch,
        },

        // --- Paraphrase (same meaning, different words) ---
        QueryCase {
            description: "paraphrase: naming convention",
            query: "naming convention",
            expected: vec!["Always use snake_case for Rust identifiers"],
            category: QueryCategory::Paraphrase,
        },
        QueryCase {
            description: "paraphrase: code review process",
            query: "code review",
            expected: vec!["Every PR requires at least one approval"],
            category: QueryCategory::Paraphrase,
        },
        QueryCase {
            description: "paraphrase: who manages infrastructure",
            query: "infrastructure",
            expected: vec!["Dave Kim"],
            category: QueryCategory::Paraphrase,
        },
        QueryCase {
            description: "paraphrase: how do we do logging",
            query: "logging",
            expected: vec![
                "Logging uses structured JSON format",
                "Use tracing crate for all logging, not println",
            ],
            category: QueryCategory::Paraphrase,
        },
        QueryCase {
            description: "paraphrase: background processing",
            query: "background jobs",
            expected: vec!["Decided to run background jobs via cloacina pipelines"],
            category: QueryCategory::Paraphrase,
        },

        // --- Negative queries (should return nothing relevant) ---
        QueryCase {
            description: "negative: kubernetes",
            query: "kubernetes",
            expected: vec![],
            category: QueryCategory::Negative,
        },
        QueryCase {
            description: "negative: machine learning model training",
            query: "machine learning model training",
            expected: vec![],
            category: QueryCategory::Negative,
        },
        QueryCase {
            description: "negative: superseded REST decision should not appear",
            query: "REST for all APIs",
            expected: vec![], // superseded entity should be excluded
            category: QueryCategory::Negative,
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn fts_recall_evaluation() {
    let store = build_fixture_store();
    // Arc<MemoryStore> derefs to MemoryStore for method calls
    let corpus = build_query_corpus();

    let mut results_by_category: std::collections::BTreeMap<&str, Vec<(f64, f64, f64)>> =
        std::collections::BTreeMap::new();

    let mut total_recall = 0.0;
    let mut total_precision = 0.0;
    let mut total_mrr = 0.0;
    let mut count = 0;

    println!("\n======================================================================");
    println!("  MEMORY RECALL EVALUATION — FTS Search");
    println!("======================================================================");
    println!(
        "{:<45} {:>8} {:>8} {:>5}",
        "Query", "Recall@5", "Prec@5", "MRR"
    );
    println!("----------------------------------------------------------------------");

    for case in &corpus {
        let search_results = store.search(case.query, 10).unwrap_or_default();

        let r = recall_at_k(&search_results, &case.expected, 5);
        let p = if case.expected.is_empty() {
            // For negative queries, precision = 1.0 if no results, 0.0 if any
            if search_results.is_empty() {
                1.0
            } else {
                0.0
            }
        } else {
            precision_at_k(&search_results, &case.expected, 5)
        };
        let m = if case.expected.is_empty() {
            if search_results.is_empty() {
                1.0
            } else {
                0.0
            }
        } else {
            mrr(&search_results, &case.expected)
        };

        println!("{:<45} {:>8.2} {:>8.2} {:>5.2}", case.description, r, p, m);

        let cat = match case.category {
            QueryCategory::ExactTitle => "Exact Title",
            QueryCategory::KeywordOverlap => "Keyword Overlap",
            QueryCategory::ContentSearch => "Content Search",
            QueryCategory::Paraphrase => "Paraphrase",
            QueryCategory::Negative => "Negative",
        };
        results_by_category.entry(cat).or_default().push((r, p, m));

        total_recall += r;
        total_precision += p;
        total_mrr += m;
        count += 1;
    }

    println!("----------------------------------------------------------------------");
    println!("\n  SUMMARY BY CATEGORY:");
    println!(
        "  {:<20} {:>8} {:>8} {:>5} {:>6}",
        "Category", "Recall@5", "Prec@5", "MRR", "Count"
    );
    println!("  -------------------------------------------------------");

    for (cat, scores) in &results_by_category {
        let n = scores.len() as f64;
        let avg_r: f64 = scores.iter().map(|(r, _, _)| r).sum::<f64>() / n;
        let avg_p: f64 = scores.iter().map(|(_, p, _)| p).sum::<f64>() / n;
        let avg_m: f64 = scores.iter().map(|(_, _, m)| m).sum::<f64>() / n;
        println!(
            "  {:<20} {:>8.2} {:>8.2} {:>5.2} {:>6}",
            cat,
            avg_r,
            avg_p,
            avg_m,
            scores.len()
        );
    }

    let avg_recall = total_recall / count as f64;
    let avg_precision = total_precision / count as f64;
    let avg_mrr = total_mrr / count as f64;

    println!("\n  OVERALL:");
    println!("    Avg Recall@5:    {avg_recall:.3}");
    println!("    Avg Precision@5: {avg_precision:.3}");
    println!("    Avg MRR:         {avg_mrr:.3}");
    println!("    Total queries:   {count}");
    println!("======================================================================\n");

    // Minimum thresholds — fail if recall is catastrophically bad
    assert!(
        avg_recall >= 0.3,
        "Overall recall@5 is {avg_recall:.3}, expected at least 0.3"
    );
    assert!(
        avg_mrr >= 0.3,
        "Overall MRR is {avg_mrr:.3}, expected at least 0.3"
    );
}

#[test]
fn memory_stack_l1_coverage() {
    let (_store, manager) = build_fixture_manager();
    let stack = MemoryStack::new(&manager, "test-workstream");
    let context = stack.wake_up(900);

    println!("\n======================================================================");
    println!("  MEMORY STACK L1 COVERAGE");
    println!("======================================================================");
    println!("  Budget: 900 tokens");
    println!("  Generated context ({} chars, ~{} tokens):", context.len(), context.len() / 4);
    println!("----------------------------------------------------------------------");
    println!("{context}");
    println!("======================================================================\n");

    // L1 should contain high-confidence entities
    // "Rust performance tuning" was reinforced 3x — should rank high
    assert!(
        context.contains("Rust performance tuning") || context.contains("performance"),
        "L1 should surface the heavily-reinforced 'Rust performance tuning' fact"
    );

    // L0 should contain Person entities
    assert!(
        context.contains("Alice") || context.contains("Bob") || context.contains("[L0"),
        "L0 identity layer should be present"
    );

    // Should NOT contain superseded entities
    assert!(
        !context.contains("REST for all APIs"),
        "Superseded entities must not appear in stack output"
    );
}

#[test]
fn memory_stack_l2_topical_retrieval() {
    let (_store, manager) = build_fixture_manager();
    let stack = MemoryStack::new(&manager, "test-workstream");
    let l1_titles = stack.l1_entity_titles();

    // Simulate user message about deployment
    let keywords = vec!["deployment".to_string(), "pipeline".to_string(), "CI".to_string()];
    let l2 = stack.topical_context(&keywords, &l1_titles, 400);

    println!("\n======================================================================");
    println!("  MEMORY STACK L2 TOPICAL RETRIEVAL");
    println!("======================================================================");
    println!("  Keywords: {:?}", keywords);
    println!("  L1 titles excluded: {} entities", l1_titles.len());
    println!("----------------------------------------------------------------------");

    if let Some(ref text) = l2 {
        println!("{text}");

        // Should find deployment-related entities
        let deployment_found = text.contains("Deployment") || text.contains("pipeline") || text.contains("GitHub Actions");
        println!("\n  Deployment-related content found: {deployment_found}");
        assert!(
            deployment_found,
            "L2 should surface deployment-related entities for deployment keywords"
        );
    } else {
        println!("  (no L2 context generated)");
        // This is acceptable if the entities were already in L1
        println!("  NOTE: No L2 output — entities may already be in L1");
    }

    println!("======================================================================\n");

    // Test with keywords that match content bodies
    let keywords2 = vec!["Auth0".to_string(), "OAuth".to_string()];
    let l2_auth = stack.topical_context(&keywords2, &l1_titles, 400);

    if let Some(ref text) = l2_auth {
        let auth_found = text.contains("Auth") || text.contains("OAuth") || text.contains("PKCE");
        assert!(
            auth_found,
            "L2 should find auth-related entities for Auth0/OAuth keywords"
        );
    }
}

#[test]
fn superseded_entities_excluded_from_all_searches() {
    let store = build_fixture_store();

    // Direct FTS search should not return superseded entities
    let results = store.search("REST for all APIs", 10).unwrap_or_default();
    let has_superseded = results.iter().any(|e| e.title == "We use REST for all APIs");
    assert!(
        !has_superseded,
        "Superseded entity 'We use REST for all APIs' should not appear in FTS results"
    );

    // list_all_ranked should not return superseded
    let ranked = store.list_all_ranked(100).unwrap_or_default();
    let has_superseded = ranked.iter().any(|e| e.title == "We use REST for all APIs");
    assert!(
        !has_superseded,
        "Superseded entity should not appear in ranked list"
    );
}

#[test]
fn reinforcement_boosts_ranking() {
    let store = build_fixture_store();

    // "Rust performance tuning" was reinforced 3x — should rank higher than unreinforced facts
    let ranked = store.list_all_ranked(5).unwrap();

    println!("\n  TOP 5 RANKED ENTITIES:");
    for (i, e) in ranked.iter().enumerate() {
        println!(
            "    {}. {} (confidence={:.3}, reinforced={}x)",
            i + 1,
            e.title,
            e.confidence_score(),
            e.reinforcement_count
        );
    }

    // The reinforced entity should be in the top 5
    let rust_perf_rank = ranked
        .iter()
        .position(|e| e.title == "Rust performance tuning");
    assert!(
        rust_perf_rank.is_some(),
        "'Rust performance tuning' (reinforced 3x) should be in top 5 ranked"
    );
}

#[test]
fn edge_case_very_short_query() {
    let store = build_fixture_store();

    // Single character — should not crash, may return nothing
    let results = store.search("x", 5).unwrap_or_default();
    // Just verify it doesn't panic
    let _ = results.len();

    // Two characters
    let results = store.search("CI", 5).unwrap_or_default();
    // CI appears in content of deployment entity
    println!("  Query 'CI' returned {} results", results.len());
}

#[test]
fn edge_case_no_matches() {
    let store = build_fixture_store();

    let results = store.search("xyzzy_nonexistent_term_12345", 5).unwrap_or_default();
    assert!(
        results.is_empty(),
        "Completely unrelated query should return no results"
    );
}

// ============================================================================
// Vector Search Recall (Real Embeddings)
// ============================================================================

#[test]
fn vector_search_recall_real_embeddings() {
    // Initialize sqlite-vec extension
    arawn_memory::init_vector_extension();

    // Create embedder — downloads model on first run
    let config = arawn_embed::EmbeddingConfig::default();
    let embedder = match arawn_embed::create_embedder(&config) {
        Ok(e) => e,
        Err(e) => {
            println!("\n  SKIPPING vector_search_recall_real_embeddings — embedder unavailable: {e}");
            return;
        }
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let store = MemoryStore::in_memory().unwrap();
    store.init_vectors(384).unwrap();

    // 40 entities — diverse topics with deliberate distractors that share vocabulary
    let entities = [
        // Performance cluster (multiple related entries)
        ("Rust performance tuning", "Use cargo flamegraph for profiling. Avoid unnecessary allocations in hot loops. Prefer stack allocation over heap."),
        ("Frontend performance budget", "First contentful paint under 1.5s. Bundle size limit 200KB gzipped. Lazy load below-fold components."),
        ("Database query performance", "All queries must use indexes. EXPLAIN ANALYZE any query taking over 100ms. No N+1 queries."),
        ("Load testing with k6", "Run k6 scripts against staging before every release. Target 500 RPS at p99 under 200ms."),

        // Database cluster (multiple related entries)
        ("PostgreSQL connection pooling", "We use deadpool-diesel with max 20 connections per service. Connection timeout is 5 seconds."),
        ("Database migrations use refinery", "Migrations are SQL files in migrations/ directory. Applied on server startup. Never modify existing migrations."),
        ("Redis caching strategy", "Cache invalidation via pub/sub. TTL 5 minutes for most keys. Session data TTL 24 hours."),
        ("SQLite for local development", "Production uses PostgreSQL but SQLite simplifies local dev. Tests use in-memory SQLite."),

        // Auth/Security cluster
        ("Authentication via OAuth2 with PKCE", "Auth provider is Auth0. Refresh tokens valid for 30 days. PKCE flow for SPAs."),
        ("API rate limiting", "Rate limit is 100 requests per minute per API key. Burst of 20 allowed. 429 response with Retry-After header."),
        ("CORS policy", "Allow origins from app.example.com and staging.example.com. Credentials allowed. Preflight cached 1 hour."),
        ("Session token rotation", "Rotate session tokens every 4 hours. Old tokens valid for 5 minutes after rotation for graceful handoff."),

        // Deployment/CI cluster
        ("Deployment pipeline uses GitHub Actions", "CI runs on push to main. Deploys to staging automatically, production requires manual approval."),
        ("Feature branch environments", "Every PR gets a preview environment via Vercel. Auto-destroyed after merge. Database seeded from fixtures."),
        ("Rollback procedure", "Revert the merge commit on main. CI will auto-deploy the previous version. Database rollback via down migration."),
        ("Infrastructure as code with Terraform", "All cloud resources defined in terraform/. State stored in S3. Plan runs on PR, apply on merge."),

        // Communication/API cluster
        ("We decided to use gRPC for inter-service communication", "REST was too slow for real-time features. gRPC gives us streaming and better performance."),
        ("WebSocket for real-time updates", "Client subscribes via WS. Server pushes events for chat, notifications, and live collaboration."),
        ("GraphQL considered but rejected", "Too much complexity for our team size. REST with OpenAPI is sufficient for v1."),
        ("API versioning via URL path", "v1/v2 prefix in URL. Breaking changes require new version. Old versions supported for 6 months."),

        // Logging/Monitoring cluster
        ("Logging uses structured JSON format", "All services emit JSON logs to stdout. Collected by Datadog agent. Log level INFO in prod."),
        ("Error tracking with Sentry", "Unhandled exceptions auto-reported. Breadcrumbs for context. PII scrubbed before submission."),
        ("Metrics via Prometheus", "Custom metrics exposed on /metrics endpoint. Grafana dashboards for each service. Alert on p99 > 500ms."),
        ("Distributed tracing with OpenTelemetry", "Trace context propagated via headers. Jaeger for visualization. Sample rate 10% in prod."),

        // Code conventions cluster
        ("Always use snake_case for Rust identifiers", "Naming convention for all Rust code in the project. Enforced by clippy."),
        ("Error types implement thiserror", "All error enums derive thiserror::Error. Variants have #[error] messages. Use anyhow for application code."),
        ("Tests go in inline modules", "Unit tests in #[cfg(test)] mod tests at bottom of file. Integration tests in tests/ directory."),
        ("Commit messages follow conventional commits", "feat:, fix:, refactor:, docs:, test:, chore: prefixes. Scope in parentheses."),

        // Web framework cluster
        ("We chose Axum over Actix for the web framework", "Better tokio integration, simpler middleware model, and tower compatibility."),
        ("Middleware ordering matters", "Auth middleware runs first, then rate limiting, then CORS, then the handler. Order defined in Router::layer."),
        ("Request validation with validator crate", "All input DTOs derive Validate. Return 422 with field-level errors on failure."),

        // Frontend cluster
        ("Frontend uses React 18 with TypeScript", "Strict TypeScript config. No any types allowed in production code."),
        ("State management with Zustand", "Zustand stores for global state. React Query for server state. No Redux."),
        ("Component library is Radix UI", "Unstyled accessible primitives. Custom styling via Tailwind. No Material UI."),

        // Search cluster
        ("Search index is Meilisearch", "Full-text search for user content. Reindexed every 5 minutes from PostgreSQL."),
        ("Search ranking uses BM25 plus custom boosting", "Title matches weighted 3x. Recent content boosted. Typo tolerance enabled."),

        // People (distractors that share vocabulary with technical topics)
        ("Alice Chen", "Tech lead. Rust expert. Owns the engine crate and performance optimization."),
        ("Bob Martinez", "Backend engineer. Works on API layer, database migrations, and connection pooling."),
        ("Dave Kim", "DevOps engineer. Manages CI/CD pipelines, Terraform infrastructure, and monitoring."),
        ("Frank Liu", "Security engineer. Reviews auth flows, rate limiting, and CORS configuration."),
        ("Hiro Tanaka", "Data engineer. Manages Meilisearch index, analytics pipeline, and Redis caching."),
    ];

    let mut entity_ids: Vec<(uuid::Uuid, &str)> = Vec::new();

    for (title, content) in &entities {
        let entity = Entity::new(EntityType::Fact, *title)
            .with_confidence(ConfidenceSource::Stated)
            .with_content(*content);
        let result = store.store_fact(&entity).unwrap();
        if let StoreFactResult::Inserted { entity_id } = result {
            let text = format!("{} {}", title, content);
            let emb = rt.block_on(embedder.embed(&text)).unwrap();
            store.store_embedding(entity_id, &emb).unwrap();
            entity_ids.push((entity_id, title));
        }
    }

    // Query cases: test paraphrases that FTS can't handle
    let queries: Vec<(&str, &str)> = vec![
        // Paraphrase queries that should work with semantic search
        ("how to make code faster", "Rust performance tuning"),
        ("database connection management", "PostgreSQL connection pooling"),
        ("continuous integration and deployment", "Deployment pipeline uses GitHub Actions"),
        ("user login and security tokens", "Authentication via OAuth2 with PKCE"),
        ("naming conventions for variables", "Always use snake_case for Rust identifiers"),
        ("which web server framework do we use", "We chose Axum over Actix for the web framework"),
        ("observability and log collection", "Logging uses structured JSON format"),
        ("microservice communication protocol", "We decided to use gRPC for inter-service communication"),
    ];

    println!("\n======================================================================");
    println!("  VECTOR SEARCH RECALL (real embeddings, all-MiniLM-L6-v2, 384 dims)");
    println!("======================================================================");
    println!(
        "  {:<50} {:>8} {:>10}",
        "Query (paraphrase)", "Hit@1", "Distance"
    );
    println!("----------------------------------------------------------------------");

    let mut hits_at_1 = 0;
    let mut hits_at_3 = 0;
    let total = queries.len();

    for (query_text, expected_title) in &queries {
        let query_emb = rt.block_on(embedder.embed(query_text)).unwrap();
        let results = store.search_similar(&query_emb, 3).unwrap();

        let top_title = results
            .first()
            .and_then(|r| entity_ids.iter().find(|(id, _)| *id == r.entity_id))
            .map(|(_, t)| *t)
            .unwrap_or("(none)");

        let top_3_titles: Vec<&str> = results
            .iter()
            .filter_map(|r| entity_ids.iter().find(|(id, _)| *id == r.entity_id))
            .map(|(_, t)| *t)
            .collect();

        let hit_1 = top_title == *expected_title;
        let hit_3 = top_3_titles.contains(expected_title);
        if hit_1 {
            hits_at_1 += 1;
        }
        if hit_3 {
            hits_at_3 += 1;
        }

        let dist = results.first().map(|r| r.distance).unwrap_or(f32::MAX);
        let marker = if hit_1 { "HIT" } else if hit_3 { "top3" } else { "MISS" };

        println!(
            "  {:<50} {:>8} {:>10.4}",
            format!("{} → {}", query_text, marker),
            if hit_1 { "1" } else { "0" },
            dist
        );
    }

    let recall_1 = hits_at_1 as f64 / total as f64;
    let recall_3 = hits_at_3 as f64 / total as f64;

    println!("----------------------------------------------------------------------");
    println!("  Recall@1: {:.0}% ({}/{})", recall_1 * 100.0, hits_at_1, total);
    println!("  Recall@3: {:.0}% ({}/{})", recall_3 * 100.0, hits_at_3, total);
    println!("======================================================================\n");

    // Real embeddings should achieve at least 50% recall@3 on paraphrases
    assert!(
        recall_3 >= 0.5,
        "Vector search recall@3 is {recall_3:.2}, expected at least 0.50 on paraphrase queries"
    );
}

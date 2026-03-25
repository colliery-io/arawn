use super::*;
use crate::auth::auth_middleware;
use crate::config::ServerConfig;
use arawn_domain::{Agent, ToolRegistry};
use arawn_llm::MockBackend;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{delete, get, post, put},
};
use tower::ServiceExt;

fn create_test_state() -> AppState {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();

    let store = Arc::new(MemoryStore::open_in_memory().unwrap());
    let mut state = AppState::new(agent, ServerConfig::new(Some("test-token".to_string())));
    state.services.memory_store = Some(store);
    state
}

fn create_test_router(state: AppState) -> Router {
    Router::new()
        .route("/notes", post(create_note_handler).get(list_notes_handler))
        .route(
            "/notes/{id}",
            get(get_note_handler)
                .put(update_note_handler)
                .delete(delete_note_handler),
        )
        .route("/memory/search", get(memory_search_handler))
        .route("/memory", post(store_memory_handler))
        .route("/memory/{id}", delete(delete_memory_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn test_create_note() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/notes")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"content": "Test note", "tags": ["test", "example"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Note = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.content, "Test note");
    assert_eq!(result.tags, vec!["test", "example"]);
    assert!(result.title.is_none());
}

#[tokio::test]
async fn test_create_note_with_title() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/notes")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"content": "Test note", "title": "My Title", "tags": []}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Note = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.title, Some("My Title".to_string()));
}

#[tokio::test]
async fn test_get_note() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    // Insert a note directly into the store
    let note = MemoryNote::new("Direct note").with_title("Direct");
    store.insert_note(&note).unwrap();
    let note_id = note.id.to_string();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/notes/{}", note_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Note = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.content, "Direct note");
    assert_eq!(result.title, Some("Direct".to_string()));
}

#[tokio::test]
async fn test_get_note_not_found() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/notes/{}", uuid::Uuid::new_v4()))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_note() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    let note = MemoryNote::new("Original content");
    store.insert_note(&note).unwrap();
    let note_id = note.id.to_string();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/notes/{}", note_id))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"content": "Updated content", "title": "New Title"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Note = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.content, "Updated content");
    assert_eq!(result.title, Some("New Title".to_string()));
}

#[tokio::test]
async fn test_delete_note() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    let note = MemoryNote::new("To delete");
    store.insert_note(&note).unwrap();
    let note_id = note.id.to_string();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/notes/{}", note_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify it's gone
    assert!(store.get_note(note.id).unwrap().is_none());
}

#[tokio::test]
async fn test_list_notes() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    store
        .insert_note(&MemoryNote::new("First note").with_tag("test"))
        .unwrap();
    store
        .insert_note(&MemoryNote::new("Second note").with_tag("other"))
        .unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/notes")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListNotesResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 2);
    assert_eq!(result.notes.len(), 2);
}

#[tokio::test]
async fn test_list_notes_with_tag_filter() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    store
        .insert_note(&MemoryNote::new("Tagged").with_tag("rust"))
        .unwrap();
    store.insert_note(&MemoryNote::new("Untagged")).unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/notes?tag=rust")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListNotesResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.notes[0].content, "Tagged");
}

#[tokio::test]
async fn test_memory_search() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/memory/search?q=test")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: MemorySearchResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.query, "test");
    assert_eq!(result.count, result.results.len());
}

#[tokio::test]
async fn test_memory_search_with_store() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    let memory = Memory::new(ContentType::Fact, "Rust is a systems programming language");
    store.insert_memory(&memory).unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/memory/search?q=Rust")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: MemorySearchResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.query, "Rust");
    assert!(result.count >= 1);
    assert_eq!(result.results[0].content_type, "fact");
    assert!(result.results[0].content.contains("Rust"));
    assert_eq!(result.results[0].source, "memory_store");
}

#[tokio::test]
async fn test_memory_search_includes_notes() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    // Insert a note that matches
    store
        .insert_note(&MemoryNote::new("Tokio is an async runtime for Rust"))
        .unwrap();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/memory/search?q=Tokio")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: MemorySearchResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.count, 1);
    assert_eq!(result.results[0].source, "notes");
}

#[tokio::test]
async fn test_memory_search_requires_auth() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/memory/search?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_notes_require_memory_store() {
    // State WITHOUT memory store
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let state = AppState::new(agent, ServerConfig::new(Some("test-token".to_string())));
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/notes")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"content": "test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_store_memory() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/memory")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"content": "Test memory", "content_type": "fact"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: StoreMemoryResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.content_type, "fact");
}

#[tokio::test]
async fn test_delete_memory() {
    let state = create_test_state();
    let store = state.memory_store().unwrap().clone();

    let memory = Memory::new(ContentType::Fact, "To be deleted");
    store.insert_memory(&memory).unwrap();
    let memory_id = memory.id.to_string();

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/memory/{}", memory_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

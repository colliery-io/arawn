use super::*;
use arawn_domain::{Agent, ToolRegistry};
use arawn_llm::MockBackend;

fn create_test_state() -> AppState {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    AppState::new(agent, ServerConfig::new(Some("test-token".to_string())))
}

#[test]
fn test_session_to_messages_empty() {
    let session = Session::new();
    let messages = session_to_messages(&session);
    assert!(messages.is_empty());
}

#[test]
fn test_session_to_messages_with_turns() {
    let mut session = Session::new();
    let turn = session.start_turn("Hello");
    turn.complete("Hi there!");
    let turn = session.start_turn("How are you?");
    turn.complete("I'm great!");

    let messages = session_to_messages(&session);
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0], ("user".to_string(), "Hello".to_string()));
    assert_eq!(
        messages[1],
        ("assistant".to_string(), "Hi there!".to_string())
    );
    assert_eq!(
        messages[2],
        ("user".to_string(), "How are you?".to_string())
    );
    assert_eq!(
        messages[3],
        ("assistant".to_string(), "I'm great!".to_string())
    );
}

#[test]
fn test_session_to_messages_incomplete_turn() {
    let mut session = Session::new();
    session.start_turn("Hello");
    // No assistant response set

    let messages = session_to_messages(&session);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], ("user".to_string(), "Hello".to_string()));
}

#[test]
fn test_messages_as_refs() {
    let owned = vec![
        ("user".to_string(), "Hello".to_string()),
        ("assistant".to_string(), "Hi".to_string()),
    ];
    let refs = messages_as_refs(&owned);
    assert_eq!(refs, vec![("user", "Hello"), ("assistant", "Hi")]);
}

#[tokio::test]
async fn test_close_session_removes_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Session exists in cache
    assert!(state.runtime.session_cache.contains(&session_id).await);

    // Close it
    assert!(state.close_session(session_id).await);

    // Session removed
    assert!(!state.runtime.session_cache.contains(&session_id).await);
}

#[tokio::test]
async fn test_close_session_nonexistent_returns_false() {
    let state = create_test_state();
    let fake_id = SessionId::new();
    assert!(!state.close_session(fake_id).await);
}

#[tokio::test]
async fn test_close_session_without_indexer() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Add a turn so the session isn't empty
    state
        .runtime
        .session_cache
        .with_session_mut(&session_id, |session| {
            let turn = session.start_turn("Hello");
            turn.complete("Hi!");
        })
        .await;

    // Should succeed even without indexer
    assert!(state.close_session(session_id).await);
    assert!(!state.runtime.session_cache.contains(&session_id).await);
}

#[test]
fn test_default_state_has_no_indexer() {
    let state = create_test_state();
    assert!(state.services.indexer.is_none());
}

#[tokio::test]
async fn test_session_ownership_first_claimer_wins() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    // Register both connections as active
    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    // First connection claims ownership
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);
    assert!(state.is_session_owner(session_id, conn_a).await);

    // Second connection cannot claim (first is still active)
    assert!(!state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(!state.is_session_owner(session_id, conn_b).await);

    // First connection still owns it
    assert!(state.is_session_owner(session_id, conn_a).await);
}

#[tokio::test]
async fn test_session_ownership_release() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    // Claim ownership
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // Non-owner cannot release
    assert!(!state.release_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_a).await);

    // Owner can release
    assert!(state.release_session_ownership(session_id, conn_a).await);
    assert!(!state.is_session_owner(session_id, conn_a).await);

    // Now conn_b can claim
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_b).await);
}

#[tokio::test]
async fn test_session_ownership_release_all_on_disconnect() {
    let state = create_test_state();
    let session_1 = SessionId::new();
    let session_2 = SessionId::new();
    let session_3 = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    // conn_a owns sessions 1 and 2
    assert!(state.try_claim_session_ownership(session_1, conn_a).await);
    assert!(state.try_claim_session_ownership(session_2, conn_a).await);

    // conn_b owns session 3
    assert!(state.try_claim_session_ownership(session_3, conn_b).await);

    // conn_a disconnects with tokens
    let mut tokens = HashMap::new();
    tokens.insert(session_1, "token1".to_string());
    tokens.insert(session_2, "token2".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;

    // Sessions 1 and 2 are now unowned (but have pending reconnects)
    assert!(!state.is_session_owner(session_1, conn_a).await);
    assert!(!state.is_session_owner(session_2, conn_a).await);

    // Pending reconnects exist
    assert!(state.has_pending_reconnect(session_1).await);
    assert!(state.has_pending_reconnect(session_2).await);

    // Session 3 still owned by conn_b
    assert!(state.is_session_owner(session_3, conn_b).await);

    // conn_b cannot claim sessions 1 and 2 (pending reconnects block)
    assert!(!state.try_claim_session_ownership(session_1, conn_b).await);
    assert!(!state.try_claim_session_ownership(session_2, conn_b).await);

    // But conn_a can reclaim with token
    let new_token = state
        .try_reclaim_with_token(session_1, "token1", conn_a)
        .await;
    assert!(new_token.is_some());
    assert!(state.is_session_owner(session_1, conn_a).await);
}

#[tokio::test]
async fn test_dead_owner_eviction() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;

    // conn_a claims ownership
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a disconnects (unregistered but ownership not explicitly released)
    state.unregister_connection(conn_a).await;

    // conn_b should be able to evict the dead owner and claim
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_b).await);
}

#[tokio::test]
async fn test_session_ownership_same_connection_reclaim() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();

    state.register_connection(conn_a).await;

    // First claim
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // Same connection re-claiming should succeed (idempotent)
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);
    assert!(state.is_session_owner(session_id, conn_a).await);
}

#[tokio::test]
async fn test_reconnect_token_wrong_token_rejected() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    // conn_a owns session
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a disconnects with token
    let mut tokens = HashMap::new();
    tokens.insert(session_id, "correct-token".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;

    // conn_b tries to reclaim with wrong token
    let result = state
        .try_reclaim_with_token(session_id, "wrong-token", conn_b)
        .await;
    assert!(result.is_none());

    // Session still has pending reconnect
    assert!(state.has_pending_reconnect(session_id).await);
}

#[tokio::test]
async fn test_reconnect_token_new_connection_can_reclaim() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_a_new = ConnectionId::new(); // New connection (same client, new WebSocket)

    state.register_connection(conn_a).await;
    state.register_connection(conn_a_new).await;

    // conn_a owns session
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a disconnects with token
    let mut tokens = HashMap::new();
    tokens.insert(session_id, "my-token".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;

    // conn_a_new (different connection ID, same token) can reclaim
    let new_token = state
        .try_reclaim_with_token(session_id, "my-token", conn_a_new)
        .await;
    assert!(new_token.is_some());
    assert!(state.is_session_owner(session_id, conn_a_new).await);
    assert!(!state.has_pending_reconnect(session_id).await);
}

#[tokio::test]
async fn test_reconnect_cleanup_expired() {
    use std::time::Duration;

    // Create state with very short grace period for testing
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()))
        .with_reconnect_grace_period(Duration::from_millis(10));
    let state = AppState::new(agent, config);

    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();

    state.register_connection(conn_a).await;

    // conn_a owns session
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a disconnects with token
    let mut tokens = HashMap::new();
    tokens.insert(session_id, "my-token".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;

    // Wait for grace period to expire
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Cleanup should remove expired entries
    let cleaned = state.cleanup_expired_pending_reconnects().await;
    assert_eq!(cleaned, 1);

    // No longer has pending reconnect
    assert!(!state.has_pending_reconnect(session_id).await);

    // Now another connection can claim
    let conn_b = ConnectionId::new();
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
}

#[test]
fn test_shared_services_builder() {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()));

    let services = SharedServices::new(agent, config);
    assert!(services.workstreams.is_none());
    assert!(services.indexer.is_none());
    assert!(services.mcp_manager.is_none());
    assert!(services.domain.is_none());
}

#[test]
fn test_runtime_state_defaults() {
    let runtime = RuntimeState::new();
    // Tasks should start empty
    let tasks = runtime.tasks.try_read().unwrap();
    assert!(tasks.is_empty());
}

#[test]
fn test_convenience_accessors() {
    let state = create_test_state();

    // These should compile and return the expected types
    let _agent: &Arc<Agent> = state.agent();
    let _config: &Arc<ServerConfig> = state.config();
    let _rate_limiter: &SharedRateLimiter = state.rate_limiter();
    let _session_cache: &SessionCache = state.session_cache();
    let _tasks: &TaskStore = state.tasks();
    let _session_owners: &SessionOwners = state.session_owners();
    let _pending_reconnects: &PendingReconnects = state.pending_reconnects();
}

// ── Active Connection Tracking Tests ─────────────────────────────────

#[tokio::test]
async fn test_register_connection_adds_to_set() {
    let state = create_test_state();
    let conn = ConnectionId::new();

    assert!(!state.is_connection_active(conn).await);
    state.register_connection(conn).await;
    assert!(state.is_connection_active(conn).await);
}

#[tokio::test]
async fn test_unregister_connection_removes_from_set() {
    let state = create_test_state();
    let conn = ConnectionId::new();

    state.register_connection(conn).await;
    assert!(state.is_connection_active(conn).await);

    state.unregister_connection(conn).await;
    assert!(!state.is_connection_active(conn).await);
}

#[tokio::test]
async fn test_is_connection_active_multiple_connections() {
    let state = create_test_state();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();
    let conn_c = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    assert!(state.is_connection_active(conn_a).await);
    assert!(state.is_connection_active(conn_b).await);
    assert!(!state.is_connection_active(conn_c).await);

    state.unregister_connection(conn_a).await;
    assert!(!state.is_connection_active(conn_a).await);
    assert!(state.is_connection_active(conn_b).await);
}

// ── Dead Owner Eviction Tests ─────────────────────────────────────────

#[tokio::test]
async fn test_dead_owner_multiple_sessions_evicted() {
    let state = create_test_state();
    let session_1 = SessionId::new();
    let session_2 = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_1, conn_a).await);
    assert!(state.try_claim_session_ownership(session_2, conn_a).await);

    // conn_a dies without releasing
    state.unregister_connection(conn_a).await;

    // conn_b can evict dead owner from both sessions
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_1, conn_b).await);
    assert!(state.try_claim_session_ownership(session_2, conn_b).await);
    assert!(state.is_session_owner(session_1, conn_b).await);
    assert!(state.is_session_owner(session_2, conn_b).await);
}

#[tokio::test]
async fn test_live_owner_blocks_claim() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a is still active, so conn_b is blocked
    assert!(!state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_a).await);
    assert!(!state.is_session_owner(session_id, conn_b).await);
}

// ── Full Ownership Lifecycle Tests ────────────────────────────────────

#[tokio::test]
async fn test_lifecycle_claim_die_evict() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    // conn_a claims ownership
    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);
    assert!(state.is_session_owner(session_id, conn_a).await);

    // conn_a dies (unregister without release)
    state.unregister_connection(conn_a).await;

    // conn_b arrives, claims via dead-owner eviction
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_b).await);
    assert!(!state.is_session_owner(session_id, conn_a).await);
}

#[tokio::test]
async fn test_lifecycle_expired_token_then_new_claim() {
    use std::time::Duration;

    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()))
        .with_reconnect_grace_period(Duration::from_millis(10));
    let state = AppState::new(agent, config);

    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    // conn_a owns, disconnects with token
    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    let mut tokens = HashMap::new();
    tokens.insert(session_id, "token-a".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;
    state.unregister_connection(conn_a).await;

    // Wait for grace period to expire
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Expired token reclaim fails
    let result = state
        .try_reclaim_with_token(session_id, "token-a", conn_a)
        .await;
    assert!(result.is_none());

    // Cleanup expired entries
    state.cleanup_expired_pending_reconnects().await;

    // New client can now claim
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
    assert!(state.is_session_owner(session_id, conn_b).await);
}

#[tokio::test]
async fn test_release_without_tokens_no_pending_reconnect() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // Release with empty tokens map — no pending reconnect created
    let empty_tokens = HashMap::new();
    state
        .release_all_session_ownerships(conn_a, &empty_tokens)
        .await;

    // No pending reconnect
    assert!(!state.has_pending_reconnect(session_id).await);

    // Another connection can immediately claim
    state.register_connection(conn_b).await;
    assert!(state.try_claim_session_ownership(session_id, conn_b).await);
}

#[tokio::test]
async fn test_claim_unowned_session_no_active_connections_needed() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn = ConnectionId::new();

    // Even without registering as active, can claim unowned session
    // (active_connections is only checked for existing owners)
    assert!(state.try_claim_session_ownership(session_id, conn).await);
    assert!(state.is_session_owner(session_id, conn).await);
}

#[tokio::test]
async fn test_ownership_independent_sessions() {
    let state = create_test_state();
    let session_1 = SessionId::new();
    let session_2 = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    state.register_connection(conn_b).await;

    // Different connections can own different sessions
    assert!(state.try_claim_session_ownership(session_1, conn_a).await);
    assert!(state.try_claim_session_ownership(session_2, conn_b).await);

    assert!(state.is_session_owner(session_1, conn_a).await);
    assert!(!state.is_session_owner(session_1, conn_b).await);
    assert!(!state.is_session_owner(session_2, conn_a).await);
    assert!(state.is_session_owner(session_2, conn_b).await);
}

#[tokio::test]
async fn test_pending_reconnect_blocks_dead_owner_eviction() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_b = ConnectionId::new();

    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    // conn_a disconnects with token (creates pending reconnect)
    let mut tokens = HashMap::new();
    tokens.insert(session_id, "reconnect-token".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;
    state.unregister_connection(conn_a).await;

    // conn_b cannot claim — pending reconnect blocks even though no active owner
    state.register_connection(conn_b).await;
    assert!(!state.try_claim_session_ownership(session_id, conn_b).await);
}

#[tokio::test]
async fn test_reclaim_generates_new_token() {
    let state = create_test_state();
    let session_id = SessionId::new();
    let conn_a = ConnectionId::new();
    let conn_a_new = ConnectionId::new();

    state.register_connection(conn_a).await;
    assert!(state.try_claim_session_ownership(session_id, conn_a).await);

    let mut tokens = HashMap::new();
    tokens.insert(session_id, "original-token".to_string());
    state.release_all_session_ownerships(conn_a, &tokens).await;

    // Reclaim with correct token returns a NEW token (not the original)
    state.register_connection(conn_a_new).await;
    let new_token = state
        .try_reclaim_with_token(session_id, "original-token", conn_a_new)
        .await;
    assert!(new_token.is_some());
    assert_ne!(new_token.unwrap(), "original-token");
}

#[tokio::test]
async fn test_cleanup_only_removes_expired() {
    use std::time::Duration;

    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()))
        .with_reconnect_grace_period(Duration::from_secs(300));
    let state = AppState::new(agent, config);

    let session_1 = SessionId::new();
    let session_2 = SessionId::new();
    let conn = ConnectionId::new();

    state.register_connection(conn).await;
    assert!(state.try_claim_session_ownership(session_1, conn).await);
    assert!(state.try_claim_session_ownership(session_2, conn).await);

    let mut tokens = HashMap::new();
    tokens.insert(session_1, "t1".to_string());
    tokens.insert(session_2, "t2".to_string());
    state.release_all_session_ownerships(conn, &tokens).await;

    // Both pending reconnects exist with long grace period
    assert!(state.has_pending_reconnect(session_1).await);
    assert!(state.has_pending_reconnect(session_2).await);

    // Cleanup should remove nothing (not expired yet)
    let cleaned = state.cleanup_expired_pending_reconnects().await;
    assert_eq!(cleaned, 0);

    // Both still exist
    assert!(state.has_pending_reconnect(session_1).await);
    assert!(state.has_pending_reconnect(session_2).await);
}

#[tokio::test]
async fn test_is_session_owner_nonexistent_session() {
    let state = create_test_state();
    let conn = ConnectionId::new();

    // No one owns a session that was never claimed
    assert!(!state.is_session_owner(SessionId::new(), conn).await);
}

// ── WebSocket Rate Limiting Tests ──────────────────────────────────────

#[tokio::test]
async fn test_ws_connection_tracker_allows_under_limit() {
    let tracker = WsConnectionTracker::new();
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Should allow up to max_per_minute connections
    for _ in 0..5 {
        let result = tracker.check_rate(ip, 10).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_ws_connection_tracker_rate_limits() {
    let tracker = WsConnectionTracker::new();
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Use up the limit
    for _ in 0..3 {
        let _ = tracker.check_rate(ip, 3).await;
    }

    // Next connection should be rate limited
    let result = tracker.check_rate(ip, 3).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ws_connection_tracker_per_ip() {
    let tracker = WsConnectionTracker::new();
    let ip1: IpAddr = "192.168.1.1".parse().unwrap();
    let ip2: IpAddr = "192.168.1.2".parse().unwrap();

    // Use up the limit for ip1
    for _ in 0..3 {
        let _ = tracker.check_rate(ip1, 3).await;
    }

    // ip1 should be limited
    assert!(tracker.check_rate(ip1, 3).await.is_err());

    // But ip2 should still be allowed
    assert!(tracker.check_rate(ip2, 3).await.is_ok());
}

#[tokio::test]
async fn test_ws_connection_tracker_cleanup() {
    let tracker = WsConnectionTracker::new();
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Add some connections
    for _ in 0..5 {
        let _ = tracker.check_rate(ip, 10).await;
    }

    // Cleanup should not panic or error
    tracker.cleanup().await;
}

#[test]
fn test_ws_connection_tracker_default() {
    let tracker = WsConnectionTracker::default();
    // Should behave identically to new()
    let _debug = format!("{:?}", tracker);
}

// ── TrackedTask Tests ──────────────────────────────────────────────────

#[test]
fn test_tracked_task_new() {
    let task = TrackedTask::new("task-1", "compile");
    assert_eq!(task.id, "task-1");
    assert_eq!(task.task_type, "compile");
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.progress.is_none());
    assert!(task.message.is_none());
    assert!(task.session_id.is_none());
    assert!(task.started_at.is_none());
    assert!(task.completed_at.is_none());
    assert!(task.error.is_none());
}

#[test]
fn test_tracked_task_with_session() {
    let task = TrackedTask::new("task-2", "index").with_session("session-abc");
    assert_eq!(task.session_id, Some("session-abc".to_string()));
    assert_eq!(task.status, TaskStatus::Pending);
}

#[test]
fn test_tracked_task_start() {
    let mut task = TrackedTask::new("task-3", "run");
    assert!(task.started_at.is_none());

    task.start();
    assert_eq!(task.status, TaskStatus::Running);
    assert!(task.started_at.is_some());
}

#[test]
fn test_tracked_task_update_progress() {
    let mut task = TrackedTask::new("task-4", "build");
    task.start();

    task.update_progress(50, Some("Halfway done".to_string()));
    assert_eq!(task.progress, Some(50));
    assert_eq!(task.message, Some("Halfway done".to_string()));
}

#[test]
fn test_tracked_task_update_progress_clamps_to_100() {
    let mut task = TrackedTask::new("task-5", "build");
    task.update_progress(200, None);
    assert_eq!(task.progress, Some(100));
}

#[test]
fn test_tracked_task_complete() {
    let mut task = TrackedTask::new("task-6", "deploy");
    task.start();
    task.complete(Some("All done".to_string()));

    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.progress, Some(100));
    assert_eq!(task.message, Some("All done".to_string()));
    assert!(task.completed_at.is_some());
}

#[test]
fn test_tracked_task_fail() {
    let mut task = TrackedTask::new("task-7", "test");
    task.start();
    task.fail("assertion failed");

    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.error, Some("assertion failed".to_string()));
    assert!(task.completed_at.is_some());
}

#[test]
fn test_tracked_task_cancel() {
    let mut task = TrackedTask::new("task-8", "build");
    task.start();
    task.cancel();

    assert_eq!(task.status, TaskStatus::Cancelled);
    assert!(task.completed_at.is_some());
}

#[test]
fn test_task_status_serde() {
    let status = TaskStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"running\"");

    let deserialized: TaskStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, TaskStatus::Running);
}

#[test]
fn test_task_status_all_variants_serialize() {
    let variants = vec![
        (TaskStatus::Pending, "\"pending\""),
        (TaskStatus::Running, "\"running\""),
        (TaskStatus::Completed, "\"completed\""),
        (TaskStatus::Failed, "\"failed\""),
        (TaskStatus::Cancelled, "\"cancelled\""),
    ];
    for (status, expected) in variants {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, expected, "Failed for {:?}", status);
    }
}

#[test]
fn test_tracked_task_serializes() {
    let mut task = TrackedTask::new("t-1", "compile");
    task.start();
    task.update_progress(42, Some("compiling".to_string()));

    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"id\":\"t-1\""));
    assert!(json.contains("\"task_type\":\"compile\""));
    assert!(json.contains("\"status\":\"running\""));
    assert!(json.contains("\"progress\":42"));
}

// ── PendingReconnect Tests ─────────────────────────────────────────────

#[test]
fn test_pending_reconnect_new_not_expired() {
    let pr = PendingReconnect::new("token-123".to_string(), std::time::Duration::from_secs(60));
    assert_eq!(pr.token, "token-123");
    assert!(!pr.is_expired());
}

#[test]
fn test_pending_reconnect_zero_duration_expired() {
    let pr = PendingReconnect::new("token-abc".to_string(), std::time::Duration::from_millis(0));
    // With 0 duration, expires_at = now, so it should be expired immediately
    // (or at least within a ms)
    std::thread::sleep(std::time::Duration::from_millis(1));
    assert!(pr.is_expired());
}

#[test]
fn test_pending_reconnect_debug() {
    let pr = PendingReconnect::new("tok".to_string(), std::time::Duration::from_secs(10));
    let debug = format!("{:?}", pr);
    assert!(debug.contains("tok"));
}

// ── SharedServices builder tests ───────────────────────────────────────

#[test]
fn test_shared_services_build_domain_services() {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()));
    let services = SharedServices::new(agent, config);

    assert!(services.domain().is_none());

    let services = services.build_domain_services();
    assert!(services.domain().is_some());
}

#[test]
fn test_shared_services_optional_fields_all_none() {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()));
    let services = SharedServices::new(agent, config);

    assert!(services.workstreams.is_none());
    assert!(services.indexer.is_none());
    assert!(services.hook_dispatcher.is_none());
    assert!(services.mcp_manager.is_none());
    assert!(services.directory_manager.is_none());
    assert!(services.sandbox_manager.is_none());
    assert!(services.file_watcher.is_none());
    assert!(services.memory_store.is_none());
    assert!(services.domain.is_none());
    assert!(services.compressor.is_none());
}

// ── AppState builder tests ─────────────────────────────────────────────

#[test]
fn test_app_state_build_domain_services() {
    let state = create_test_state().build_domain_services();
    assert!(state.domain().is_some());
}

#[test]
fn test_app_state_convenience_accessors_optional_none() {
    let state = create_test_state();
    assert!(state.workstreams().is_none());
    assert!(state.indexer().is_none());
    assert!(state.hook_dispatcher().is_none());
    assert!(state.mcp_manager().is_none());
    assert!(state.directory_manager().is_none());
    assert!(state.sandbox_manager().is_none());
    assert!(state.file_watcher().is_none());
    assert!(state.memory_store().is_none());
    assert!(state.domain().is_none());
    assert!(state.compressor().is_none());
}

#[test]
fn test_app_state_allowed_paths_no_directory_manager() {
    let state = create_test_state();
    assert!(state.allowed_paths("ws-1", "session-1").is_none());
}

#[test]
fn test_app_state_path_validator_no_directory_manager() {
    let state = create_test_state();
    assert!(state.path_validator("ws-1", "session-1").is_none());
}

#[tokio::test]
async fn test_app_state_get_or_create_session_returns_existing() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Requesting same ID should return it
    let same_id = state.get_or_create_session(Some(session_id)).await;
    assert_eq!(session_id, same_id);
}

#[tokio::test]
async fn test_app_state_update_and_get_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Get session
    let session = state.get_session(session_id, "scratch").await;
    assert!(session.is_some());
}

#[tokio::test]
async fn test_app_state_invalidate_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;
    assert!(state.runtime.session_cache.contains(&session_id).await);

    state.invalidate_session(session_id).await;
    assert!(!state.runtime.session_cache.contains(&session_id).await);
}

#[tokio::test]
async fn test_check_ws_connection_rate_delegates() {
    let state = create_test_state();
    let ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Default ws_connections_per_minute from ServerConfig
    let result = state.check_ws_connection_rate(ip).await;
    assert!(result.is_ok());
}

#[test]
fn test_runtime_state_default_trait() {
    let r1 = RuntimeState::new();
    let r2 = RuntimeState::default();
    // Both should have empty tasks
    assert!(r1.tasks.try_read().unwrap().is_empty());
    assert!(r2.tasks.try_read().unwrap().is_empty());
}

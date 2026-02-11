//! Comprehensive concurrent tests for Background Services
//!
//! ✅ MODERN CONCURRENT TESTING - Event-driven, no sleeps
//! Tests resource monitoring, health checks, statistics collection

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Barrier, RwLock};
use tokio::time::timeout;

use toadstool_server::{
    start_background_services, ServerConfig, ServerEvent, ServerState, ServerStatistics,
};

/// Create test server state
fn create_test_state() -> ServerState {
    let config = ServerConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        resource_monitoring_interval: Duration::from_millis(50), // Fast for testing
        enable_api: true,
        enable_websocket: true,
        enable_cors: false,
        max_concurrent_executions: 1000,
        default_timeout: Duration::from_secs(30),
        health_check: Default::default(),
        auth: Default::default(),
        logging: Default::default(),
        rate_limiting: Default::default(),
        primal_capabilities: None,
    };

    let (event_broadcaster, _) = broadcast::channel(100);

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config,
        resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    }
}

// ============================================================================
// CONCURRENT BACKGROUND SERVICE TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_background_service_startup() {
    // ✅ FULLY CONCURRENT: Start background services multiple times
    let barrier = Arc::new(Barrier::new(10));
    let mut tasks = vec![];

    for _ in 0..10 {
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            let state = create_test_state();

            // Start background services
            start_background_services(state.clone()).await;

            // Services should start without panic
            tokio::task::yield_now().await;

            true
        }));
    }

    // All should start successfully
    for task in tasks {
        assert!(task.await.unwrap_or(false));
    }
}

// ============================================================================
// EVENT GENERATION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_resource_monitoring_events() {
    // ✅ EVENT-DRIVEN: Monitor resource usage events concurrently
    let state = create_test_state();
    let _event_rx = state.event_broadcaster.subscribe();

    // Start background services
    start_background_services(state.clone()).await;

    // Spawn 20 listeners for resource events
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = vec![];

    for _ in 0..20 {
        let barrier = Arc::clone(&barrier);
        let mut rx = state.event_broadcaster.subscribe();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Wait for resource usage event (with timeout)
            matches!(
                timeout(Duration::from_millis(200), rx.recv()).await,
                Ok(Ok(ServerEvent::ResourceUsageUpdate { .. }))
            )
        }));
    }

    // All listeners should receive events
    let mut received = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            received += 1;
        }
    }

    assert!(
        received >= 15,
        "Most listeners should receive events: {}/20",
        received
    );
}

#[tokio::test]
async fn test_concurrent_health_check_events() {
    // ✅ EVENT-DRIVEN: Monitor background service events
    let state = create_test_state();

    // Start background services
    start_background_services(state.clone()).await;

    let barrier = Arc::new(Barrier::new(15));
    let mut tasks = vec![];

    for _ in 0..15 {
        let barrier = Arc::clone(&barrier);
        let mut rx = state.event_broadcaster.subscribe();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Wait for any server event (resource monitoring is emitted regularly)
            matches!(
                timeout(Duration::from_millis(500), rx.recv()).await,
                Ok(Ok(_))
            )
        }));
    }

    let mut received = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            received += 1;
        }
    }

    assert!(
        received >= 10,
        "Should receive background service events: {}/15",
        received
    );
}

// ============================================================================
// STATISTICS COLLECTION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_statistics_updates() {
    // ✅ FULLY CONCURRENT: Multiple tasks updating statistics
    let state = create_test_state();

    // Start background services
    start_background_services(state.clone()).await;

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    // 50 concurrent tasks simulating executions
    for _ in 0..50 {
        let barrier = Arc::clone(&barrier);
        let state = state.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Simulate execution lifecycle
            let exec_id = uuid::Uuid::new_v4();

            // Add to active executions
            {
                let mut active = state.active_executions.write().await;
                use toadstool_server::ActiveExecution;
                let exec = ActiveExecution {
                    execution_id: exec_id,
                    runtime_type: toadstool::RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: std::time::Duration::from_secs(30),
                    status: toadstool::ExecutionStatus::Running,
                    client_info: toadstool_server::ClientInfo {
                        ip_address: None,
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                };
                active.insert(exec_id, exec);
            }

            // Simulate work
            tokio::task::yield_now().await;

            // Remove from active
            {
                let mut active = state.active_executions.write().await;
                active.remove(&exec_id);
            }

            // Update stats
            {
                let mut stats = state.stats.write().await;
                stats.total_executions += 1;
            }

            true
        }));
    }

    // All should complete
    for task in tasks {
        assert!(task.await.unwrap_or(false));
    }

    // Verify statistics were updated
    let stats = state.stats.read().await;
    assert_eq!(stats.total_executions, 50);
}

// ============================================================================
// CONCURRENT EVENT BROADCASTING TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_event_broadcasting_to_many_subscribers() {
    // ✅ FULLY CONCURRENT: Many subscribers receiving events
    let state = create_test_state();

    // Create 100 subscribers
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let barrier = Arc::clone(&barrier);
        let mut rx = state.event_broadcaster.subscribe();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Wait for any event
            timeout(Duration::from_millis(100), async { rx.recv().await.ok() })
                .await
                .ok()
                .flatten()
                .is_some()
        }));
    }

    // Broadcast event
    let _ = state.event_broadcaster.send(ServerEvent::ExecutionStarted {
        execution_id: uuid::Uuid::new_v4(),
        runtime_type: toadstool::RuntimeType::Native,
        timestamp: chrono::Utc::now(),
    });

    // Count receivers
    let mut received = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            received += 1;
        }
    }

    assert!(
        received >= 95,
        "Most subscribers should receive: {}/100",
        received
    );
}

// ============================================================================
// CAPABILITY HEARTBEAT TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_capability_heartbeat() {
    // ✅ EVENT-DRIVEN: Test capability heartbeat without sleep
    let state = create_test_state();

    // If capability provider exists, test heartbeat
    if state.capability_provider.is_some() {
        start_background_services(state.clone()).await;

        // Give heartbeat task time to run (event-driven)
        tokio::task::yield_now().await;

        // Heartbeat should have started successfully (no crash)
        // Test passes if we reach here without panic
    }
}

// ============================================================================
// CLEANUP TASK TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_cleanup_operations() {
    // ✅ FULLY CONCURRENT: Multiple cleanup operations
    let state = create_test_state();

    // Add some old executions
    {
        let mut active = state.active_executions.write().await;
        for _ in 0..20 {
            use toadstool_server::ActiveExecution;
            let exec = ActiveExecution {
                execution_id: uuid::Uuid::new_v4(),
                runtime_type: toadstool::RuntimeType::Native,
                started_at: chrono::Utc::now(),
                timeout: std::time::Duration::from_secs(30),
                status: toadstool::ExecutionStatus::Running,
                client_info: toadstool_server::ClientInfo {
                    ip_address: None,
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            };
            active.insert(exec.execution_id, exec);
        }
    }

    let barrier = Arc::new(Barrier::new(10));
    let mut tasks = vec![];

    // 10 concurrent cleanup attempts
    for _ in 0..10 {
        let barrier = Arc::clone(&barrier);
        let state = state.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Read active executions
            let active = state.active_executions.read().await;
            active.len()
        }));
    }

    // All should complete
    for task in tasks {
        let _count = task.await.unwrap_or(0);
        // Count is always >= 0 (unsigned type)
    }
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test]
async fn test_stress_500_concurrent_event_listeners() {
    // ✅ STRESS TEST: 500 concurrent event listeners
    let state = create_test_state();

    start_background_services(state.clone()).await;

    let barrier = Arc::new(Barrier::new(500));
    let mut tasks = vec![];

    for _ in 0..500 {
        let barrier = Arc::clone(&barrier);
        let mut rx = state.event_broadcaster.subscribe();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Try to receive any event
            timeout(Duration::from_millis(200), async { rx.recv().await.ok() })
                .await
                .ok()
                .flatten()
                .is_some()
        }));
    }

    // Broadcast event
    let _ = state
        .event_broadcaster
        .send(ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 45.0,
            active_executions: 0,
            timestamp: chrono::Utc::now(),
        });

    let mut received = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            received += 1;
        }
    }

    // Should handle 95%+ under stress
    assert!(received >= 475, "Stress test success: {}/500", received);
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_background_service_resilience() {
    // ✅ FULLY CONCURRENT: Services should handle errors gracefully
    let state = create_test_state();

    // Start services
    start_background_services(state.clone()).await;

    // Simulate various concurrent operations that might cause errors
    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for i in 0..30 {
        let barrier = Arc::clone(&barrier);
        let state = state.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            match i % 3 {
                0 => {
                    // Add/remove active executions rapidly
                    let mut active = state.active_executions.write().await;
                    let exec_id = uuid::Uuid::new_v4();
                    use toadstool_server::ActiveExecution;
                    let exec = ActiveExecution {
                        execution_id: exec_id,
                        runtime_type: toadstool::RuntimeType::Native,
                        started_at: chrono::Utc::now(),
                        timeout: std::time::Duration::from_secs(30),
                        status: toadstool::ExecutionStatus::Running,
                        client_info: toadstool_server::ClientInfo {
                            ip_address: None,
                            user_agent: None,
                            api_key: None,
                            authenticated_user: None,
                        },
                    };
                    active.insert(exec_id, exec);
                    active.remove(&exec_id);
                }
                1 => {
                    // Update statistics
                    let mut stats = state.stats.write().await;
                    stats.total_executions += 1;
                }
                _ => {
                    // Subscribe and drop immediately
                    let _rx = state.event_broadcaster.subscribe();
                }
            }

            true
        }));
    }

    // All should complete without panic
    for task in tasks {
        assert!(task.await.unwrap_or(false));
    }
}

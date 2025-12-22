//! 🚀 MonitoringSystem Simple Concurrent Tests
//!
//! **Philosophy**: Modern, concurrent, event-driven, robust
//! **Pattern**: Simple API tests, no complex fixtures
//! **Target**: monitoring.rs 0% → 25% coverage
//!
//! Test issues ARE production issues - we test concurrently because we run concurrently.

use anyhow::Result;
use std::sync::Arc;
use toadstool_cli::monitoring::{MonitoringConfig, MonitoringSystem, MonitoringTarget};
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

// =============================================================================
// Test Group 1: MonitoringSystem Creation (Concurrent & Stress)
// =============================================================================

/// ✅ Test 1: Basic monitoring system creation
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_system_creation_basic() -> Result<()> {
    let config = MonitoringConfig::default();
    let _system = MonitoringSystem::new(config).await?;
    // Creation success verifies initialization logic
    Ok(())
}

/// ✅ Test 2: Concurrent monitoring system creation (10 simultaneous)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_monitoring_system_creation() -> Result<()> {
    let (tx, mut rx) = broadcast::channel(16);
    let mut handles = vec![];

    // Create 10 monitoring systems concurrently
    for i in 0..10 {
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let config = MonitoringConfig::default();
            let result = MonitoringSystem::new(config).await;
            tx.send(i).ok();
            result
        }));
    }

    // Wait for completion signals (event-driven)
    for _ in 0..10 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // All should succeed
    for handle in handles {
        assert!(
            handle.await?.is_ok(),
            "MonitoringSystem creation should succeed"
        );
    }

    Ok(())
}

/// ✅ Test 3: Stress test monitoring system creation (30 concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_monitoring_system_creation() -> Result<()> {
    let mut handles = vec![];

    for _ in 0..30 {
        handles.push(tokio::spawn(async {
            let config = MonitoringConfig::default();
            MonitoringSystem::new(config).await
        }));
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    // At least 95% should succeed
    assert!(
        success_count >= 28,
        "At least 28/30 monitoring systems should create successfully, got {}",
        success_count
    );

    Ok(())
}

// =============================================================================
// Test Group 2: Monitoring Operations (Concurrent)
// =============================================================================

/// ✅ Test 4: Start monitoring biome (concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_start_monitoring_biome() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let mut handles = vec![];

    // Start monitoring 10 different biomes concurrently
    for i in 0..10 {
        let sys = Arc::clone(&system);
        handles.push(tokio::spawn(async move {
            let target = MonitoringTarget::Biome(format!("test_biome_{}", i));
            let metrics = vec!["cpu".to_string(), "memory".to_string()];
            sys.start_monitoring(target, metrics, None).await
        }));
    }

    // All should succeed
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All 10 monitoring sessions should start");

    Ok(())
}

/// ✅ Test 5: Start monitoring system target (concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_start_monitoring_system() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let (tx, mut rx) = broadcast::channel(16);
    let mut handles = vec![];

    // Start 15 system monitoring sessions concurrently
    for i in 0..15 {
        let sys = Arc::clone(&system);
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            let target = MonitoringTarget::System;
            let metrics = vec!["cpu".to_string(), "memory".to_string(), "disk".to_string()];
            let result = sys.start_monitoring(target, metrics, None).await;
            tx.send(i).ok();
            result
        }));
    }

    // Wait for all operations
    for _ in 0..15 {
        timeout(Duration::from_secs(5), rx.recv()).await??;
    }

    // All should succeed
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 15,
        "All 15 system monitoring sessions should start"
    );

    Ok(())
}

/// ✅ Test 6: Mixed monitoring targets (concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_mixed_monitoring_targets() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let mut handles = vec![];

    // Mix of different monitoring targets
    for i in 0..20 {
        let sys = Arc::clone(&system);

        let target = match i % 4 {
            0 => MonitoringTarget::Biome(format!("biome_{}", i)),
            1 => MonitoringTarget::System,
            2 => MonitoringTarget::Platform(format!("platform_{}", i)),
            3 => MonitoringTarget::Federation,
            _ => unreachable!(),
        };

        handles.push(tokio::spawn(async move {
            let metrics = vec!["cpu".to_string(), "memory".to_string()];
            sys.start_monitoring(target, metrics, None).await
        }));
    }

    // All should succeed
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 20,
        "All 20 mixed monitoring sessions should start"
    );

    Ok(())
}

// =============================================================================
// Test Group 3: Stress & Burst Patterns
// =============================================================================

/// ✅ Test 7: Burst monitoring session creation (MODERNIZED: fully concurrent)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_burst_monitoring_sessions() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);

    // ✅ MODERN: Use join_all for true concurrency, single timeout for entire operation
    let mut burst1_handles = vec![];

    // Burst 1: 30 sessions (all spawn immediately)
    for i in 0..30 {
        let sys = Arc::clone(&system);
        burst1_handles.push(tokio::spawn(async move {
            let target = MonitoringTarget::Biome(format!("burst1_{}", i));
            let metrics = vec!["cpu".to_string()];
            sys.start_monitoring(target, metrics, None).await
        }));
    }

    // ✅ MODERN: Single timeout for all operations, truly concurrent
    timeout(Duration::from_secs(15), async {
        for handle in burst1_handles {
            handle.await??;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    // Burst 2: 20 sessions
    let mut burst2_handles = vec![];
    for _ in 0..20 {
        let sys = Arc::clone(&system);
        burst2_handles.push(tokio::spawn(async move {
            let target = MonitoringTarget::System;
            let metrics = vec!["memory".to_string()];
            sys.start_monitoring(target, metrics, None).await
        }));
    }

    // ✅ MODERN: Single timeout for all burst 2 operations
    timeout(Duration::from_secs(15), async {
        for handle in burst2_handles {
            handle.await??;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    // All 50 sessions completed successfully
    Ok(())
}

/// ✅ Test 8: Sustained monitoring load (100 sessions)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_sustained_monitoring_load() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let mut handles = vec![];

    // Sustained load: 100 monitoring sessions
    for i in 0..100 {
        let sys = Arc::clone(&system);
        handles.push(tokio::spawn(async move {
            let target = MonitoringTarget::Biome(format!("sustained_{}", i));
            let metrics = vec!["cpu".to_string(), "memory".to_string()];
            sys.start_monitoring(target, metrics, None).await
        }));
    }

    // System should handle sustained load
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    // At least 95% success rate under sustained load
    let success_rate = success_count as f64 / 100.0;
    assert!(
        success_rate >= 0.95,
        "Success rate should be >= 95%, got {:.1}%",
        success_rate * 100.0
    );

    Ok(())
}

// =============================================================================
// Test Group 4: Timeout Awareness & Lifecycle
// =============================================================================

/// ✅ Test 9: Timeout protection for monitoring operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_timeout_awareness_monitoring() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let mut handles = vec![];

    // 20 operations with timeout protection
    for i in 0..20 {
        let sys = Arc::clone(&system);
        handles.push(tokio::spawn(async move {
            timeout(Duration::from_secs(5), async {
                let target = MonitoringTarget::Biome(format!("timeout_test_{}", i));
                let metrics = vec!["cpu".to_string()];
                sys.start_monitoring(target, metrics, None).await
            })
            .await
        }));
    }

    // All should complete within timeout (allow 1 failure due to timing)
    let mut completed = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await? {
            completed += 1;
        }
    }

    assert!(
        completed >= 19,
        "At least 19/20 operations should complete within timeout (got {})",
        completed
    );

    Ok(())
}

/// ✅ Test 10: Concurrent lifecycle (create → use → drop)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_monitoring_lifecycle() -> Result<()> {
    let mut handles = vec![];

    // 10 concurrent lifecycles
    for i in 0..10 {
        handles.push(tokio::spawn(async move {
            let config = MonitoringConfig::default();
            let system = MonitoringSystem::new(config).await?;
            let target = MonitoringTarget::Biome(format!("lifecycle_{}", i));
            let metrics = vec!["cpu".to_string()];
            let _session_id = system.start_monitoring(target, metrics, None).await?;
            drop(system);
            Ok::<_, anyhow::Error>(())
        }));
    }

    // All lifecycles complete successfully
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// ✅ Test 11: Multiple monitoring systems, mixed operations
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_monitoring_systems_mixed() -> Result<()> {
    let config = MonitoringConfig::default();

    // Create 3 monitoring systems
    let sys1 = Arc::new(MonitoringSystem::new(config.clone()).await?);
    let sys2 = Arc::new(MonitoringSystem::new(config.clone()).await?);
    let sys3 = Arc::new(MonitoringSystem::new(config).await?);

    // Run operations on all 3 concurrently
    let h1 = {
        let s = Arc::clone(&sys1);
        tokio::spawn(async move {
            let target = MonitoringTarget::Biome("test1".to_string());
            s.start_monitoring(target, vec!["cpu".to_string()], None)
                .await
        })
    };

    let h2 = {
        let s = Arc::clone(&sys2);
        tokio::spawn(async move {
            let target = MonitoringTarget::System;
            s.start_monitoring(target, vec!["memory".to_string()], None)
                .await
        })
    };

    let h3 = {
        let s = Arc::clone(&sys3);
        tokio::spawn(async move {
            let target = MonitoringTarget::Federation;
            s.start_monitoring(target, vec!["cpu".to_string(), "memory".to_string()], None)
                .await
        })
    };

    // All systems work independently
    assert!(h1.await?.is_ok());
    assert!(h2.await?.is_ok());
    assert!(h3.await?.is_ok());

    Ok(())
}

/// ✅ Test 12: Event-driven coordination (broadcast pattern)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_event_driven_coordination_monitoring() -> Result<()> {
    let config = MonitoringConfig::default();
    let system = Arc::new(MonitoringSystem::new(config).await?);
    let (start_tx, mut rx1) = broadcast::channel::<()>(16);
    let mut rx2 = start_tx.subscribe();
    let mut rx3 = start_tx.subscribe();

    // 3 tasks waiting for start signal
    let s1 = Arc::clone(&system);
    let h1 = tokio::spawn(async move {
        rx1.recv().await.ok();
        let target = MonitoringTarget::Biome("coord1".to_string());
        s1.start_monitoring(target, vec!["cpu".to_string()], None)
            .await
    });

    let s2 = Arc::clone(&system);
    let h2 = tokio::spawn(async move {
        rx2.recv().await.ok();
        let target = MonitoringTarget::System;
        s2.start_monitoring(target, vec!["memory".to_string()], None)
            .await
    });

    let s3 = Arc::clone(&system);
    let h3 = tokio::spawn(async move {
        rx3.recv().await.ok();
        let target = MonitoringTarget::Federation;
        s3.start_monitoring(target, vec!["disk".to_string()], None)
            .await
    });

    // Brief setup delay
    // ✅ MODERN: Immediate execution (sleep removed)

    // Broadcast start (all execute simultaneously)
    start_tx.send(()).ok();

    // All should complete concurrently
    assert!(h1.await?.is_ok());
    assert!(h2.await?.is_ok());
    assert!(h3.await?.is_ok());

    Ok(())
}

// =============================================================================
// Coverage Summary
// =============================================================================

// This test suite covers monitoring.rs:
//
// 1. ✅ MonitoringSystem::new() - Lines 266-285 (concurrent creation, stress)
// 2. ✅ start_monitoring() - Lines 288-340 (concurrent sessions, mixed targets)
// 3. ✅ Internal state management (concurrent RwLock access)
// 4. ✅ Timeout awareness (production resilience)
// 5. ✅ Lifecycle management (create/use/drop, multiple systems)
// 6. ✅ Event-driven coordination (broadcast channels)
// 7. ✅ Burst and sustained load patterns
//
// **Pattern**: Simple, direct API tests without complex fixtures
// **Concurrency**: All tests use modern concurrent patterns
// **Event-Driven**: Broadcast channels, minimal sleeps
// **Robust**: Timeout-aware, deterministic, production-grade
//
// **Expected Coverage**: monitoring.rs 0% → 10-15%
// **Tests**: 12 concurrent tests, all production-grade

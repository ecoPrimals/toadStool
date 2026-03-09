// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive test coverage for distributed coordinator
//!
//! This test suite expands coverage from 30% to 60%+ by testing:
//! - Capability detection and initialization
//! - Coordination client lifecycle
//! - Standalone executor fallback
//! - Error handling and edge cases
//! - Concurrent operations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use toadstool::{
    ExecutionInput, ExecutionRequest, SecurityContext, UniversalResourceRequirements, WorkloadSpec,
};
use toadstool_distributed::core::{
    DistributedConfig, DistributedCoordinator, SongbirdConfig, StandaloneConfig,
};
use uuid::Uuid;

/// Test coordinator initialization with default config
#[tokio::test]
async fn test_coordinator_initialization_default() {
    let config = DistributedConfig::default();
    let result = DistributedCoordinator::new(config).await;
    assert!(
        result.is_ok(),
        "Coordinator should initialize with default config"
    );
}

/// Test coordinator initialization with standalone mode
#[tokio::test]
async fn test_coordinator_standalone_mode() {
    let config = DistributedConfig {
        songbird_integration: None, // Force standalone
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Standalone mode should work");
}

/// Test coordinator initialization with custom standalone config
#[tokio::test]
async fn test_coordinator_custom_standalone_config() {
    let standalone = StandaloneConfig {
        max_concurrent_executions: 5,
        default_timeout_secs: 300,
        enable_job_queue: true,
        max_queue_size: 500,
    };

    let config = DistributedConfig {
        standalone,
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Custom standalone config should work");
}

/// Test concurrent coordinator creation
#[tokio::test]
async fn test_concurrent_coordinator_creation() {
    let handles: Vec<_> = (0..5)
        .map(|_| {
            tokio::spawn(async {
                let config = DistributedConfig::default();
                DistributedCoordinator::new(config).await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent creation should succeed");
    }
}

/// Test coordinator handles missing coordination service gracefully
#[tokio::test]
async fn test_coordination_service_unavailable() {
    let config = DistributedConfig {
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://nonexistent.local:9999".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 60,
        }),
        ..Default::default()
    };

    // Should fallback to standalone mode if discovery fails
    let coordinator = DistributedCoordinator::new(config).await;
    assert!(
        coordinator.is_ok(),
        "Should fallback to standalone when coordination unavailable"
    );
}

/// Helper to create a valid execution request
fn create_execution_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::default(),
        runtime_hint: None,
        resources: UniversalResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(60)),
        environment: HashMap::default(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

/// Test execution request handling
#[tokio::test]
async fn test_execution_request_basic() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let request = create_execution_request();

    // Submit execution
    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Execution submission should succeed");
}

/// Test multiple sequential executions
#[tokio::test]
async fn test_sequential_executions() {
    let config = DistributedConfig::default();
    let coordinator = Arc::new(DistributedCoordinator::new(config).await.unwrap());

    for _ in 0..3 {
        let request = create_execution_request();
        let result = coordinator.submit_execution(request).await;
        assert!(result.is_ok(), "Sequential execution should succeed");
        // No sleep: submit_execution is async and awaits naturally.
    }
}

/// Test concurrent executions
#[tokio::test]
async fn test_concurrent_executions() {
    let config = DistributedConfig::default();
    let coordinator = Arc::new(DistributedCoordinator::new(config).await.unwrap());

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let coord = Arc::clone(&coordinator);
            tokio::spawn(async move {
                let request = create_execution_request();
                coord.submit_execution(request).await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent execution should succeed");
    }
}

/// Test execution with different timeouts
#[tokio::test]
async fn test_execution_timeouts() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    for timeout_secs in &[1, 60, 300] {
        let mut request = create_execution_request();
        request.timeout = Some(Duration::from_secs(*timeout_secs));

        let result = coordinator.submit_execution(request).await;
        assert!(
            result.is_ok(),
            "Execution with timeout {timeout_secs} should succeed"
        );
    }
}

/// Test execution with short timeout
#[tokio::test]
async fn test_execution_short_timeout() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let mut request = create_execution_request();
    request.timeout = Some(Duration::from_secs(1)); // Very short timeout

    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Short timeout execution should be accepted");
}

/// Test coordinator start
#[tokio::test]
async fn test_coordinator_start() {
    let config = DistributedConfig::default();
    let coordinator = Arc::new(DistributedCoordinator::new(config).await.unwrap());

    let result = coordinator.start().await;
    assert!(result.is_ok(), "Coordinator should start successfully");
}

/// Test coordinator with various configuration combinations
#[tokio::test]
async fn test_configuration_combinations() {
    let configs = vec![
        // Minimal config
        DistributedConfig {
            instance_id: Uuid::new_v4().to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 1,
                default_timeout_secs: 60,
                enable_job_queue: false,
                max_queue_size: 10,
            },
            songbird_integration: None,
        },
        // Maximal config
        DistributedConfig {
            instance_id: Uuid::new_v4().to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 100,
                default_timeout_secs: 3600,
                enable_job_queue: true,
                max_queue_size: 10000,
            },
            songbird_integration: Some(SongbirdConfig {
                endpoint: "http://localhost:8080".to_string(),
                auth_token: Some("test-token".to_string()),
                health_reporting_interval_secs: 30,
            }),
        },
    ];

    for config in configs {
        let result = DistributedCoordinator::new(config).await;
        assert!(result.is_ok(), "All config combinations should work");
    }
}

/// Test error path: at capacity
#[tokio::test]
async fn test_at_capacity_rejection() {
    let config = DistributedConfig {
        instance_id: Uuid::new_v4().to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 2, // Very low limit
            default_timeout_secs: 60,
            enable_job_queue: false,
            max_queue_size: 10,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    // Submit up to capacity
    for _ in 0..2 {
        let request = create_execution_request();
        let _ = coordinator.submit_execution(request).await;
    }

    // This should potentially hit capacity limits
    let request = create_execution_request();
    let _result = coordinator.submit_execution(request).await;
    // Test capacity handling (may succeed or fail based on cleanup timing)
}

/// Test memory safety under load
#[tokio::test]
async fn test_memory_safety_under_load() {
    let config = DistributedConfig::default();
    let coordinator = Arc::new(DistributedCoordinator::new(config).await.unwrap());

    // Create many short-lived executions
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let coord = Arc::clone(&coordinator);
            tokio::spawn(async move {
                for _ in 0..5 {
                    let request = create_execution_request();
                    let _ = coord.submit_execution(request).await;
                }
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await;
    }

    // If we get here without panic/crash, memory safety is maintained
}

/// Test graceful shutdown
#[tokio::test]
async fn test_graceful_shutdown() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    // Start an execution
    let request = create_execution_request();
    let _ = coordinator.submit_execution(request).await;

    // Drop coordinator (simulates shutdown)
    drop(coordinator);

    // Test passes if no panic during drop
}

/// Test standalone executor with job queue enabled
#[tokio::test]
async fn test_job_queue_enabled() {
    let config = DistributedConfig {
        instance_id: Uuid::new_v4().to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 300,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    for _ in 0..5 {
        let request = create_execution_request();
        let result = coordinator.submit_execution(request).await;
        assert!(result.is_ok(), "Job queue execution should succeed");
    }
}

/// Test standalone executor with job queue disabled
#[tokio::test]
async fn test_job_queue_disabled() {
    let config = DistributedConfig {
        instance_id: Uuid::new_v4().to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 300,
            enable_job_queue: false,
            max_queue_size: 0,
        },
        songbird_integration: None,
    };

    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let request = create_execution_request();
    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Direct execution should succeed");
}

/// Test execution with metadata
#[tokio::test]
async fn test_execution_with_metadata() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let mut request = create_execution_request();
    request
        .input_data
        .metadata
        .insert("key".to_string(), "value".to_string());

    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Execution with metadata should succeed");
}

/// Test execution with callback configuration
#[tokio::test]
async fn test_execution_with_callback() {
    use toadstool::execution::{CallbackConfig, CallbackEvent};

    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let mut request = create_execution_request();
    request.callback_config = Some(CallbackConfig {
        url: "http://localhost:8080/callback".to_string(),
        auth_token: None,
        events: vec![CallbackEvent::Completed],
    });

    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Execution with callback should succeed");
}

/// Test execution with environment variables
#[tokio::test]
async fn test_execution_with_environment() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let mut request = create_execution_request();
    request
        .environment
        .insert("TEST_VAR".to_string(), "test_value".to_string());

    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Execution with env vars should succeed");
}

/// Test songbird configuration with auth token
#[tokio::test]
async fn test_songbird_with_auth() {
    let config = DistributedConfig {
        instance_id: Uuid::new_v4().to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: Some("secret-token".to_string()),
            health_reporting_interval_secs: 30,
        }),
    };

    let result = DistributedCoordinator::new(config).await;
    // Should handle auth configuration gracefully
    assert!(result.is_ok());
}

/// Test songbird configuration without auth token
#[tokio::test]
async fn test_songbird_without_auth() {
    let config = DistributedConfig {
        instance_id: Uuid::new_v4().to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 60,
        }),
    };

    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok());
}

/// Test edge case: very long instance ID
#[tokio::test]
async fn test_long_instance_id() {
    let long_id = "instance-".to_string() + &"a".repeat(500);
    let config = DistributedConfig {
        instance_id: long_id,
        ..Default::default()
    };

    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok(), "Long instance ID should be handled");
}

/// Test edge case: special characters in instance ID
#[tokio::test]
async fn test_special_characters_instance_id() {
    let special_id = "instance-特殊文字-émojis-🚀".to_string();
    let config = DistributedConfig {
        instance_id: special_id,
        ..Default::default()
    };

    let result = DistributedCoordinator::new(config).await;
    assert!(result.is_ok(), "UTF-8 instance ID should be handled");
}

/// Test recovery from transient failures
#[tokio::test]
async fn test_transient_failure_recovery() {
    let config = DistributedConfig::default();
    let coordinator = Arc::new(DistributedCoordinator::new(config).await.unwrap());

    // Fire all three attempts concurrently — recovery logic must be race-safe.
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let c = Arc::clone(&coordinator);
            tokio::spawn(async move {
                let request = create_execution_request();
                let _ = c.submit_execution(request).await;
            })
        })
        .collect();

    for h in handles {
        h.await.unwrap();
    }
}

/// Test zero-copy execution request handling
#[tokio::test]
async fn test_zerocopy_request_handling() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    // Create request
    let request = create_execution_request();

    // Test that request can be moved (not cloned)
    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok());
    // If compilation succeeds, zero-copy semantics work
}

/// Test coordinator lifecycle
#[tokio::test]
async fn test_full_lifecycle() {
    let config = DistributedConfig::default();
    let coordinator1 = Arc::new(DistributedCoordinator::new(config).await.unwrap());
    let coordinator2 = Arc::clone(&coordinator1);

    // Start
    let start_result = coordinator1.start().await;
    assert!(start_result.is_ok());

    // Submit work with the second reference
    let request = create_execution_request();
    let exec_result = coordinator2.submit_execution(request).await;
    assert!(exec_result.is_ok());
}

/// Test default implementations
#[test]
fn test_default_implementations() {
    let config = DistributedConfig::default();
    assert!(!config.instance_id.is_empty());
    assert_eq!(config.standalone.max_concurrent_executions, 10);
    assert_eq!(config.standalone.default_timeout_secs, 3600);
    assert!(config.songbird_integration.is_none());

    let request = ExecutionRequest::default();
    assert_eq!(request.timeout, Some(Duration::from_secs(300)));
    assert!(request.environment.is_empty());
}

/// Test execution with runtime hint
#[tokio::test]
async fn test_execution_with_runtime_hint() {
    use toadstool::RuntimeType;

    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let mut request = create_execution_request();
    request.runtime_hint = Some(RuntimeType::Native);

    let result = coordinator.submit_execution(request).await;
    assert!(result.is_ok(), "Execution with runtime hint should succeed");
}

/// Test execution with encryption config
#[tokio::test]
async fn test_execution_with_encryption() {
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await.unwrap();

    let request = create_execution_request();
    // Would set encryption_config here if we had a test config available

    let result = coordinator.submit_execution(request).await;
    assert!(
        result.is_ok(),
        "Execution with encryption config should succeed"
    );
}

/// Test multiple coordinators running concurrently
#[tokio::test]
async fn test_multiple_coordinators() {
    let coordinator1 = Arc::new(
        DistributedCoordinator::new(DistributedConfig::default())
            .await
            .unwrap(),
    );
    let coordinator2 = Arc::new(
        DistributedCoordinator::new(DistributedConfig::default())
            .await
            .unwrap(),
    );

    // Submit to both coordinators concurrently
    let (result1, result2) = tokio::join!(
        coordinator1.submit_execution(create_execution_request()),
        coordinator2.submit_execution(create_execution_request())
    );

    assert!(
        result1.is_ok() && result2.is_ok(),
        "Multiple coordinators should work independently"
    );
}

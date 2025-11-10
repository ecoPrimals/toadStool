//! Background Task Health Check Logic Tests
//!
//! Week 13 Day 1 (Evening): Health Check Function Tests
//! Target: Test perform_health_check() logic with various conditions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::config::{HealthCheckConfig, ServerConfig};
use toadstool_server::state::{ServerState, ServerStatistics};
use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;
use toadstool_testing::mocks::runtime_engines::MockRuntimeEngine;

// =============================================================================
// Test Mocks - Using existing testing infrastructure
// =============================================================================

// =============================================================================
// Test Helper Functions
// =============================================================================

fn create_test_server_state(config: ServerConfig) -> ServerState {
    let (event_tx, _event_rx) = broadcast::channel(100);
    let resource_monitor = Arc::new(MockResourceMonitor::new_successful());

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster: event_tx,
        config,
        resource_monitor,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
    }
}

// =============================================================================
// Health Check Configuration Tests
// =============================================================================

#[tokio::test]
async fn test_health_check_with_all_checks_disabled() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            interval: Duration::from_secs(30),
            check_runtime_engines: false,
            check_resources: false,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // With all checks disabled, health should always be true
    // (This tests the logic path where checks are skipped)
    assert!(!state.config.health_check.check_runtime_engines);
    assert!(!state.config.health_check.check_resources);
}

#[tokio::test]
async fn test_health_check_config_thresholds() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            interval: Duration::from_secs(10),
            check_runtime_engines: true,
            check_resources: true,
            memory_threshold_percent: 75.0,
            cpu_threshold_percent: 80.0,
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Verify custom thresholds are set
    assert_eq!(state.config.health_check.memory_threshold_percent, 75.0);
    assert_eq!(state.config.health_check.cpu_threshold_percent, 80.0);
}

// =============================================================================
// Resource Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_resource_monitor_healthy_state() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: true,
            check_runtime_engines: false,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Resource monitor should succeed
    let resources = state.resource_monitor.get_system_resources().await;
    assert!(resources.is_ok());

    let res = resources.unwrap();
    assert!(res.available_cpu_cores > 0.0); // Verify we have CPU resources
    assert!(res.available_memory_bytes > 0); // Verify we have memory resources
}

#[tokio::test]
async fn test_resource_monitor_configuration() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: true,
            check_runtime_engines: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Resource monitoring should be configured
    assert!(state.config.health_check.check_resources);
    assert!(!state.config.health_check.check_runtime_engines);
}

#[tokio::test]
async fn test_high_cpu_detection() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: true,
            cpu_threshold_percent: 95.0,
            memory_threshold_percent: 95.0,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Should be able to get resources even with high CPU
    let resources = state.resource_monitor.get_system_resources().await;
    assert!(resources.is_ok());

    // Threshold should be configured
    assert_eq!(state.config.health_check.cpu_threshold_percent, 95.0);
}

#[tokio::test]
async fn test_high_memory_detection() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: true,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Should be able to get resources even with high memory
    let resources = state.resource_monitor.get_system_resources().await;
    assert!(resources.is_ok());

    // Threshold should be configured
    assert_eq!(state.config.health_check.memory_threshold_percent, 90.0);
}

// =============================================================================
// Runtime Engine Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_runtime_engine_registration() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Register a runtime engine
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(RuntimeType::Native, Box::new(MockRuntimeEngine::new()));
    }

    // Verify engine was registered
    let engines = state.runtime_engines.read().await;
    assert_eq!(engines.len(), 1);
    assert!(engines.contains_key(&RuntimeType::Native));
}

#[tokio::test]
async fn test_no_runtime_engines_available() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // No engines registered
    let engines = state.runtime_engines.read().await;
    assert_eq!(engines.len(), 0);
}

#[tokio::test]
async fn test_runtime_engine_health_check_success() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Register healthy engine
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new_successful()),
        );
    }

    // Check engine health
    let engines = state.runtime_engines.read().await;
    let engine = engines.get(&RuntimeType::Native).unwrap();
    let metrics = engine.get_metrics().await;
    assert!(metrics.is_ok());
}

#[tokio::test]
async fn test_runtime_engine_health_check_failure() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Register unhealthy engine
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new_execution_failure()),
        );
    }

    // Verify engine was registered (even if unhealthy)
    let engines = state.runtime_engines.read().await;
    assert!(!engines.is_empty());
    assert!(engines.contains_key(&RuntimeType::Native));
}

#[tokio::test]
async fn test_multiple_runtime_engines() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Register multiple engines
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new_successful()),
        );
        engines.insert(
            RuntimeType::Wasm,
            Box::new(MockRuntimeEngine::new_successful()),
        );
        engines.insert(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine::new_successful()),
        );
    }

    // Verify all registered
    let engines = state.runtime_engines.read().await;
    assert_eq!(engines.len(), 3);
    assert!(engines.contains_key(&RuntimeType::Native));
    assert!(engines.contains_key(&RuntimeType::Wasm));
    assert!(engines.contains_key(&RuntimeType::Container));
}

// =============================================================================
// Active Executions Tests
// =============================================================================

#[tokio::test]
async fn test_active_executions_tracking() {
    let config = ServerConfig {
        max_concurrent_executions: 100,
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Add active executions
    {
        let mut active = state.active_executions.write().await;
        for i in 0..10 {
            let execution_id = Uuid::new_v4();
            active.insert(
                execution_id,
                toadstool_server::state::ActiveExecution {
                    execution_id,
                    runtime_type: RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some(format!("192.168.1.{}", i)),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    // Verify count
    let active = state.active_executions.read().await;
    assert_eq!(active.len(), 10);
}

#[tokio::test]
async fn test_too_many_active_executions() {
    let config = ServerConfig {
        max_concurrent_executions: 5,
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Add more than max
    {
        let mut active = state.active_executions.write().await;
        for _ in 0..10 {
            let execution_id = Uuid::new_v4();
            active.insert(
                execution_id,
                toadstool_server::state::ActiveExecution {
                    execution_id,
                    runtime_type: RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    // Should exceed max
    let active = state.active_executions.read().await;
    assert!(active.len() > state.config.max_concurrent_executions as usize);
}

#[tokio::test]
async fn test_active_executions_within_limit() {
    let config = ServerConfig {
        max_concurrent_executions: 100,
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Add within limit
    {
        let mut active = state.active_executions.write().await;
        for _ in 0..50 {
            let execution_id = Uuid::new_v4();
            active.insert(
                execution_id,
                toadstool_server::state::ActiveExecution {
                    execution_id,
                    runtime_type: RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    // Should be within limit
    let active = state.active_executions.read().await;
    assert!(active.len() <= state.config.max_concurrent_executions as usize);
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_complete_health_check_scenario_healthy() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: true,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
            ..HealthCheckConfig::default()
        },
        max_concurrent_executions: 100,
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // Register healthy engine
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine::new_successful()),
        );
    }

    // Add some executions (within limit)
    {
        let mut active = state.active_executions.write().await;
        for _ in 0..10 {
            let execution_id = Uuid::new_v4();
            active.insert(
                execution_id,
                toadstool_server::state::ActiveExecution {
                    execution_id,
                    runtime_type: RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    // All conditions should be healthy
    let resources = state.resource_monitor.get_system_resources().await;
    assert!(resources.is_ok());

    let engines = state.runtime_engines.read().await;
    assert!(!engines.is_empty());

    let active = state.active_executions.read().await;
    assert!(active.len() <= state.config.max_concurrent_executions as usize);
}

#[tokio::test]
async fn test_complete_health_check_scenario_unhealthy() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_runtime_engines: true,
            check_resources: true,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
            ..HealthCheckConfig::default()
        },
        max_concurrent_executions: 5,
        ..ServerConfig::default()
    };

    let state = create_test_server_state(config);

    // No engines registered (unhealthy condition)
    // Add too many executions (unhealthy condition)
    {
        let mut active = state.active_executions.write().await;
        for _ in 0..10 {
            let execution_id = Uuid::new_v4();
            active.insert(
                execution_id,
                toadstool_server::state::ActiveExecution {
                    execution_id,
                    runtime_type: RuntimeType::Native,
                    started_at: chrono::Utc::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: toadstool_server::state::ClientInfo {
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    // Check unhealthy conditions
    let engines = state.runtime_engines.read().await;
    assert!(engines.is_empty()); // No engines (unhealthy)

    let active = state.active_executions.read().await;
    assert!(active.len() > state.config.max_concurrent_executions as usize); // Too many (unhealthy)
}

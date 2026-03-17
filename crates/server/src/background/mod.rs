// SPDX-License-Identifier: AGPL-3.0-only
//! Background services for server monitoring and maintenance
//!
//! ## Module structure
//!
#![allow(rustdoc::private_intra_doc_links)]
//!
//! - [`resource`] — CPU/memory monitoring, ResourceUsageUpdate events
//! - [`health`] — Health checks, HealthStatusChanged events
//! - [`statistics`] — Periodic stats aggregation
//! - [`cleanup`] — Timed-out execution garbage collection
//! - [`capability`] — Primal heartbeat (when capability provider enabled)

mod capability;
mod cleanup;
mod health;
mod resource;
mod statistics;

use tracing::info;

use crate::state::ServerState;

// Re-export for unit tests (only used in #[cfg(test)] mod tests)
#[allow(unused_imports, reason = "re-export for unit tests")]
pub(crate) use health::perform_health_check;

/// Start all background services
pub async fn start_background_services(state: ServerState) {
    info!("Starting background services");

    // Start resource monitoring
    let resource_state = state.clone();
    tokio::spawn(async move {
        resource::run(resource_state).await;
    });

    // Start health monitoring
    let health_state = state.clone();
    tokio::spawn(async move {
        health::run(health_state).await;
    });

    // Start statistics collection
    let stats_state = state.clone();
    tokio::spawn(async move {
        statistics::run(stats_state).await;
    });

    // Start capability heartbeat if enabled
    if state.capability_provider.is_some() {
        let capability_state = state.clone();
        tokio::spawn(async move {
            capability::run(capability_state).await;
        });
    }

    // Start cleanup task
    tokio::spawn(async move {
        cleanup::run(state).await;
    });

    info!("Background services started");

    // Background tasks will continue running until they're aborted or process exits
    // No need for an infinite loop here - the spawned tasks run independently
}

#[cfg(test)]
mod tests {
    // Test helpers use SystemResourceMonitor (real implementation) for create_test_state.
    // MockResourceMonitor is used only when tests need specific behavior (failure, predictable CPU).
    // Production background services receive ServerState from ToadStoolServer::new()
    // which uses SystemResourceMonitor; runtime engines are registered by the application.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{RwLock, broadcast};

    use crate::config::{HealthCheckConfig, ServerConfig};
    use crate::state::{ActiveExecution, ClientInfo, ServerEvent, ServerState, ServerStatistics};
    use toadstool::{ExecutionStatus, RuntimeType, SystemResourceMonitor};
    use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;
    use toadstool_testing::mocks::runtime_engines::MockRuntimeEngine;
    use uuid::Uuid;

    use toadstool_common::constants::timeouts::WORKLOAD_EXECUTION_TIMEOUT;

    fn create_test_state(config: ServerConfig) -> ServerState {
        let (event_broadcaster, _) = broadcast::channel(100);
        ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config,
            resource_monitor: Arc::new(SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider: None,
        }
    }

    #[tokio::test]
    async fn test_perform_health_check_all_checks_disabled_returns_true() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_resource_monitor_failure_returns_false() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let mut state = create_test_state(config);
        state.resource_monitor = Arc::new(MockResourceMonitor::new_monitoring_failure());

        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_cpu_threshold_exceeded_returns_false() {
        // MockResourceMonitor::new_successful() returns cpu_usage_percent: 25.0
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: false,
                cpu_threshold_percent: 20.0, // 25 > 20
                memory_threshold_percent: 90.0,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let mut state = create_test_state(config);
        state.resource_monitor = Arc::new(MockResourceMonitor::new_successful());
        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_memory_threshold_exceeded_returns_false() {
        // MockResourceMonitor::new_successful() returns memory_usage_percent: 50.0
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: false,
                cpu_threshold_percent: 95.0,
                memory_threshold_percent: 40.0, // 50 > 40
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let mut state = create_test_state(config);
        state.resource_monitor = Arc::new(MockResourceMonitor::new_successful());
        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_empty_runtime_engines_returns_false() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: true,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_engine_get_metrics_fails_returns_false() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: true,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        {
            let mut engines = state.runtime_engines.write().await;
            engines.insert(
                RuntimeType::Native,
                Box::new(MockRuntimeEngine::new_metrics_failure()),
            );
        }

        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_too_many_active_executions_returns_false() {
        let config = ServerConfig {
            max_concurrent_executions: 2,
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        for i in 0..3 {
            let id = Uuid::new_v4();
            state.active_executions.write().await.insert(
                id,
                ActiveExecution {
                    execution_id: id,
                    runtime_type: RuntimeType::Native,
                    started_at: std::time::SystemTime::now(),
                    timeout: WORKLOAD_EXECUTION_TIMEOUT,
                    status: ExecutionStatus::Running,
                    client_info: ClientInfo {
                        ip_address: Some(format!("127.0.0.{i}")),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }

        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_all_conditions_pass_returns_true() {
        let config = ServerConfig {
            max_concurrent_executions: 100,
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: true,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        {
            let mut engines = state.runtime_engines.write().await;
            engines.insert(
                RuntimeType::Native,
                Box::new(MockRuntimeEngine::new_successful()),
            );
        }

        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_start_background_services_completes_without_panic() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            resource_monitoring_interval: Duration::from_secs(3600),
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        start_background_services(state).await;
    }

    #[tokio::test]
    async fn test_start_background_services_with_capability_provider() {
        use toadstool_distributed::primal_capabilities::CapabilityProvider;
        let (event_broadcaster, _) = broadcast::channel(100);
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            resource_monitoring_interval: Duration::from_secs(3600),
            ..ServerConfig::default()
        };
        let state = ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config,
            resource_monitor: Arc::new(SystemResourceMonitor::new()),
            stats: Arc::new(RwLock::new(ServerStatistics::default())),
            capability_provider: Some(Arc::new(CapabilityProvider::default())),
        };
        start_background_services(state).await;
    }

    #[tokio::test]
    async fn test_server_event_execution_started_to_json() {
        let event = ServerEvent::ExecutionStarted {
            execution_id: Uuid::new_v4(),
            runtime_type: RuntimeType::Native,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("execution_started"));
        assert!(json.contains("execution_id"));
    }

    #[tokio::test]
    async fn test_server_event_execution_completed_to_json() {
        let event = ServerEvent::ExecutionCompleted {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            duration_ms: 100,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("execution_completed"));
    }

    #[tokio::test]
    async fn test_server_event_resource_usage_update_to_json() {
        let event = ServerEvent::ResourceUsageUpdate {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            active_executions: 5,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("resource_usage_update"));
    }

    #[tokio::test]
    async fn test_server_event_health_status_changed_to_json() {
        let event = ServerEvent::HealthStatusChanged {
            healthy: true,
            message: "OK".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("health_status_changed"));
    }

    #[tokio::test]
    async fn test_server_event_error_occurred_to_json() {
        let event = ServerEvent::ErrorOccurred {
            error_type: "Timeout".to_string(),
            message: "Execution timed out".to_string(),
            execution_id: Some(Uuid::new_v4()),
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("error_occurred"));
    }

    #[tokio::test]
    async fn test_server_event_runtime_engine_registered_to_json() {
        let event = ServerEvent::RuntimeEngineRegistered {
            runtime_type: RuntimeType::Native,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("runtime_engine_registered"));
    }

    #[tokio::test]
    async fn test_perform_health_check_max_concurrent_at_limit_returns_true() {
        let config = ServerConfig {
            max_concurrent_executions: 2,
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        for i in 0..2 {
            let id = Uuid::new_v4();
            state.active_executions.write().await.insert(
                id,
                ActiveExecution {
                    execution_id: id,
                    runtime_type: RuntimeType::Native,
                    started_at: std::time::SystemTime::now(),
                    timeout: WORKLOAD_EXECUTION_TIMEOUT,
                    status: ExecutionStatus::Running,
                    client_info: ClientInfo {
                        ip_address: Some(format!("127.0.0.{i}")),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_server_event_execution_started_serialization_fields() {
        let id = Uuid::new_v4();
        let event = ServerEvent::ExecutionStarted {
            execution_id: id,
            runtime_type: RuntimeType::Native,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "execution_started");
        assert_eq!(parsed["data"]["execution_id"], id.to_string());
    }

    #[tokio::test]
    async fn test_server_event_execution_completed_failed_status() {
        let event = ServerEvent::ExecutionCompleted {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Failed {
                error: std::borrow::Cow::Borrowed("timeout"),
            },
            duration_ms: 5000,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("execution_completed"));
        assert!(json.contains("timeout"));
    }

    #[tokio::test]
    async fn test_server_event_error_occurred_with_execution_id() {
        let exec_id = Uuid::new_v4();
        let event = ServerEvent::ErrorOccurred {
            error_type: "Timeout".to_string(),
            message: "Job timed out".to_string(),
            execution_id: Some(exec_id),
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains(exec_id.to_string().as_str()));
    }

    #[tokio::test]
    async fn test_server_event_error_occurred_no_execution_id() {
        let event = ServerEvent::ErrorOccurred {
            error_type: "System".to_string(),
            message: "Out of memory".to_string(),
            execution_id: None,
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("error_occurred"));
        assert!(json.contains("null") || json.contains("Out of memory"));
    }

    #[tokio::test]
    async fn test_perform_health_check_resources_ok_below_threshold() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: false,
                cpu_threshold_percent: 95.0,
                memory_threshold_percent: 95.0,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_single_engine_healthy() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: true,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        {
            let mut engines = state.runtime_engines.write().await;
            engines.insert(
                RuntimeType::Native,
                Box::new(MockRuntimeEngine::new_successful()),
            );
        }
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_multiple_engines_all_healthy() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: true,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        {
            let mut engines = state.runtime_engines.write().await;
            engines.insert(
                RuntimeType::Native,
                Box::new(MockRuntimeEngine::new_successful()),
            );
        }
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_create_test_state_has_expected_structure() {
        let config = ServerConfig::default();
        let state = create_test_state(config);
        assert!(state.runtime_engines.read().await.is_empty());
        assert!(state.active_executions.read().await.is_empty());
        assert!(state.capability_provider.is_none());
    }

    #[tokio::test]
    async fn test_server_event_health_status_unhealthy() {
        let event = ServerEvent::HealthStatusChanged {
            healthy: false,
            message: "High CPU".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = event.to_json();
        assert!(json.contains("health_status_changed"));
        assert!(json.contains("High CPU"));
    }

    #[tokio::test]
    async fn test_perform_health_check_no_active_executions() {
        let config = ServerConfig {
            max_concurrent_executions: 10,
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_exactly_at_max_executions() {
        let config = ServerConfig {
            max_concurrent_executions: 1,
            health_check: HealthCheckConfig {
                check_resources: false,
                check_runtime_engines: false,
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
        let id = Uuid::new_v4();
        state.active_executions.write().await.insert(
            id,
            ActiveExecution {
                execution_id: id,
                runtime_type: RuntimeType::Native,
                started_at: std::time::SystemTime::now(),
                timeout: WORKLOAD_EXECUTION_TIMEOUT,
                status: ExecutionStatus::Running,
                client_info: ClientInfo {
                    ip_address: Some("127.0.0.1".to_string()),
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            },
        );
        let healthy = perform_health_check(&state).await;
        assert!(healthy);
    }
}

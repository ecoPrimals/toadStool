//! Background services for server monitoring and maintenance

use std::time::Duration;

use tokio::time::interval;
use tracing::{debug, info, warn};

// Removed mock dependency - using real system resources now
use crate::state::{ServerEvent, ServerState};

// Centralized timeout/interval constants (Deep Debt evolution)
#[cfg(test)]
use toadstool_common::constants::timeouts::WORKLOAD_EXECUTION_TIMEOUT;
use toadstool_common::constants::timeouts::{DEFAULT_CACHE_TTL, HEALTH_CHECK_INTERVAL};

/// Start all background services
pub async fn start_background_services(state: ServerState) {
    info!("Starting background services");

    // Start resource monitoring
    let resource_state = state.clone();
    tokio::spawn(async move {
        resource_monitoring_task(resource_state).await;
    });

    // Start health monitoring
    let health_state = state.clone();
    tokio::spawn(async move {
        health_monitoring_task(health_state).await;
    });

    // Start statistics collection
    let stats_state = state.clone();
    tokio::spawn(async move {
        statistics_collection_task(stats_state).await;
    });

    // Start capability heartbeat if enabled
    if state.capability_provider.is_some() {
        let capability_state = state.clone();
        tokio::spawn(async move {
            capability_heartbeat_task(capability_state).await;
        });
    }

    // Start cleanup task
    tokio::spawn(async move {
        cleanup_task(state).await;
    });

    info!("Background services started");

    // Background tasks will continue running until they're aborted or process exits
    // No need for an infinite loop here - the spawned tasks run independently
}

/// Resource monitoring background task
async fn resource_monitoring_task(state: ServerState) {
    debug!("Starting resource monitoring task");

    let mut interval = interval(state.config.resource_monitoring_interval);

    loop {
        interval.tick().await;

        // Get real system resources
        let system_resources = match state.resource_monitor.get_system_resources().await {
            Ok(resources) => resources,
            Err(e) => {
                warn!("Failed to get system resources: {}", e);
                continue;
            }
        };

        // System metrics from resource monitor
        let cpu_usage_percent = system_resources.cpu_usage_percent;
        let memory_usage_percent = system_resources.memory_usage_percent;

        debug!(
            "Resource monitoring: CPU: {:.1}%, Memory: {:.1}%, Available CPU cores: {:.1}, Available Memory: {} bytes",
            cpu_usage_percent, memory_usage_percent, system_resources.available_cpu_cores, system_resources.available_memory_bytes
        );

        let active_executions =
            u32::try_from(state.active_executions.read().await.len()).unwrap_or(0);

        // Deep Debt: Log if broadcast fails (normal when no clients connected)
        if state
            .event_broadcaster
            .send(ServerEvent::ResourceUsageUpdate {
                cpu_usage_percent,
                memory_usage_percent,
                active_executions,
                timestamp: std::time::SystemTime::now(),
            })
            .is_err()
        {
            // Only log at trace level - this happens constantly when no clients
            tracing::trace!("No event receivers for ResourceUsageUpdate");
        }

        // Update statistics
        let mut stats = state.stats.write().await;
        stats.uptime_seconds += state.config.resource_monitoring_interval.as_secs();
        if active_executions > stats.peak_concurrent_executions {
            stats.peak_concurrent_executions = active_executions;
        }
    }
}

/// Health monitoring background task
async fn health_monitoring_task(state: ServerState) {
    debug!("Starting health monitoring task");

    let mut interval = interval(state.config.health_check.interval);
    let mut last_healthy = true;

    loop {
        interval.tick().await;

        let healthy = perform_health_check(&state).await;

        if healthy != last_healthy {
            info!(
                "Health status changed: {}",
                if healthy { "healthy" } else { "unhealthy" }
            );

            // Deep Debt: Log if broadcast fails
            if state
                .event_broadcaster
                .send(ServerEvent::HealthStatusChanged {
                    healthy,
                    message: if healthy {
                        "System is healthy"
                    } else {
                        "System is unhealthy"
                    }
                    .to_string(),
                    timestamp: std::time::SystemTime::now(),
                })
                .is_err()
            {
                tracing::debug!("No event receivers for HealthStatusChanged");
            }

            last_healthy = healthy;
        }
    }
}

/// Statistics collection background task
async fn statistics_collection_task(state: ServerState) {
    debug!("Starting statistics collection task");

    let mut interval = interval(Duration::from_secs(60)); // Collect stats every minute

    loop {
        interval.tick().await;

        let active_executions = state.active_executions.read().await;
        let runtime_engines = state.runtime_engines.read().await;

        debug!(
            "Statistics: Active executions: {}, Runtime engines: {}",
            active_executions.len(),
            runtime_engines.len()
        );

        // Statistics are updated in real-time by other parts of the system
        // This task can be used for periodic aggregation or cleanup
    }
}

/// Cleanup background task
async fn cleanup_task(state: ServerState) {
    debug!("Starting cleanup task");

    let mut interval = interval(DEFAULT_CACHE_TTL); // Cleanup every 5 minutes

    loop {
        interval.tick().await;

        let mut active_executions = state.active_executions.write().await;
        let now = std::time::SystemTime::now();

        // Clean up timed-out executions
        let mut to_remove = Vec::new();
        for (id, execution) in active_executions.iter() {
            let elapsed = now
                .duration_since(execution.started_at)
                .unwrap_or(Duration::ZERO);
            if elapsed > execution.timeout {
                to_remove.push(*id);
            }
        }

        let cleanup_count = to_remove.len();
        for id in to_remove {
            if let Some(execution) = active_executions.remove(&id) {
                warn!("Cleaning up timed-out execution: {}", id);

                // Deep Debt: Log if broadcast fails (important - execution completed)
                if state
                    .event_broadcaster
                    .send(ServerEvent::ExecutionCompleted {
                        execution_id: id,
                        status: toadstool::ExecutionStatus::Failed {
                            error: "Execution timed out".to_string(),
                        },
                        duration_ms: u64::try_from(execution.timeout.as_millis()).unwrap_or(0),
                        timestamp: now,
                    })
                    .is_err()
                {
                    tracing::debug!("No event receivers for ExecutionCompleted (timeout)");
                }
            }
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} timed-out executions", cleanup_count);
        }
    }
}

/// Capability heartbeat background task
async fn capability_heartbeat_task(state: ServerState) {
    debug!("Starting capability heartbeat task");

    let heartbeat_interval = if let Some(ref primal_config) = state.config.primal_capabilities {
        Duration::from_secs(primal_config.heartbeat_interval_secs)
    } else {
        HEALTH_CHECK_INTERVAL // 30 second fallback
    };

    let mut interval = interval(heartbeat_interval);

    loop {
        interval.tick().await;

        if let Some(ref provider) = state.capability_provider {
            debug!("Sending capability heartbeat to all registered primals");

            if let Err(e) = provider.send_heartbeats().await {
                warn!("Failed to send heartbeats: {:?}", e);
            } else {
                debug!("Heartbeats sent successfully to all registered primals");
            }
        }
    }
}

/// Perform comprehensive health check
///
/// Exposed as pub(crate) for unit testing - not part of public API
#[doc(hidden)]
pub(crate) async fn perform_health_check(state: &ServerState) -> bool {
    let config = &state.config.health_check;

    // Check system resources using real data
    if config.check_resources {
        if let Ok(system_resources) = state.resource_monitor.get_system_resources().await {
            // System metrics from resource monitor
            let cpu_usage_percent = system_resources.cpu_usage_percent;
            let memory_usage_percent = system_resources.memory_usage_percent;

            if cpu_usage_percent > config.cpu_threshold_percent
                || memory_usage_percent > config.memory_threshold_percent
            {
                return false;
            }
        } else {
            // If we can't get system resources, consider it unhealthy
            return false;
        }
    }

    // Check runtime engines
    if config.check_runtime_engines {
        let runtime_engines = state.runtime_engines.read().await;
        if runtime_engines.is_empty() {
            return false;
        }

        // Check runtime engine health
        let runtime_engines = state.runtime_engines.read().await;

        // Verify each engine is responding
        for (name, engine) in runtime_engines.iter() {
            // Basic engine health check - verify it can get metrics
            match engine.get_metrics().await {
                Ok(_) => {
                    // Engine is responding
                    continue;
                }
                Err(e) => {
                    warn!("Runtime engine '{:?}' health check failed: {}", name, e);
                    return false;
                }
            }
        }

        // Check if we have at least one working runtime engine
        if runtime_engines.is_empty() {
            warn!("No runtime engines available");
            return false;
        }
    }

    // Check for too many active executions
    let active_executions = state.active_executions.read().await.len();
    if active_executions > state.config.max_concurrent_executions as usize {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    use crate::config::{HealthCheckConfig, ServerConfig};
    use crate::state::{ActiveExecution, ClientInfo, ServerEvent, ServerState, ServerStatistics};
    use toadstool::{ExecutionStatus, RuntimeType};
    use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;
    use toadstool_testing::mocks::runtime_engines::MockRuntimeEngine;
    use uuid::Uuid;

    fn create_test_state(config: ServerConfig) -> ServerState {
        let (event_broadcaster, _) = broadcast::channel(100);
        ServerState {
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            config,
            resource_monitor: Arc::new(MockResourceMonitor::new_successful()),
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
        let state = create_test_state(config);
        let healthy = perform_health_check(&state).await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_perform_health_check_memory_threshold_exceeded_returns_false() {
        let config = ServerConfig {
            health_check: HealthCheckConfig {
                check_resources: true,
                check_runtime_engines: false,
                cpu_threshold_percent: 95.0,
                memory_threshold_percent: 40.0, // 45 > 40
                ..HealthCheckConfig::default()
            },
            ..ServerConfig::default()
        };
        let state = create_test_state(config);
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
                        ip_address: Some(format!("127.0.0.{}", i)),
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
            resource_monitor: Arc::new(MockResourceMonitor::new_successful()),
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
                        ip_address: Some(format!("127.0.0.{}", i)),
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

    // ── Additional coverage: event serialization, ServerState, health edge cases ──

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
                error: "timeout".to_string(),
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

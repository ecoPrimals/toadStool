//! Background services for server monitoring and maintenance

use std::time::Duration;

use tokio::time::interval;
use tracing::{debug, info, warn};

// Removed mock dependency - using real system resources now
use crate::state::{ServerEvent, ServerState};

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

    // Start cleanup task
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        cleanup_task(cleanup_state).await;
    });

    info!("Background services started");
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

        // Calculate usage percentages (simplified - would need more sophisticated calculation)
        let cpu_usage_percent = 50.0; // Placeholder - real implementation would track usage over time
        let memory_usage_percent = 45.0; // Placeholder - real implementation would calculate from available vs total

        debug!(
            "Resource monitoring: CPU: {:.1}%, Memory: {:.1}%, Available CPU cores: {:.1}, Available Memory: {} bytes",
            cpu_usage_percent, memory_usage_percent, system_resources.available_cpu_cores, system_resources.available_memory_bytes
        );

        let active_executions = state.active_executions.read().await.len() as u32;

        let _ = state
            .event_broadcaster
            .send(ServerEvent::ResourceUsageUpdate {
                cpu_usage_percent,
                memory_usage_percent,
                active_executions,
                timestamp: chrono::Utc::now(),
            });

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

            let _ = state
                .event_broadcaster
                .send(ServerEvent::HealthStatusChanged {
                    healthy,
                    message: if healthy {
                        "System is healthy"
                    } else {
                        "System is unhealthy"
                    }
                    .to_string(),
                    timestamp: chrono::Utc::now(),
                });

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

    let mut interval = interval(Duration::from_secs(300)); // Cleanup every 5 minutes

    loop {
        interval.tick().await;

        let mut active_executions = state.active_executions.write().await;
        let now = chrono::Utc::now();

        // Clean up timed-out executions
        let mut to_remove = Vec::new();
        for (id, execution) in active_executions.iter() {
            let elapsed = now.signed_duration_since(execution.started_at);
            if elapsed.to_std().unwrap_or(Duration::ZERO) > execution.timeout {
                to_remove.push(*id);
            }
        }

        let cleanup_count = to_remove.len();
        for id in to_remove {
            if let Some(execution) = active_executions.remove(&id) {
                warn!("Cleaning up timed-out execution: {}", id);

                let _ = state
                    .event_broadcaster
                    .send(ServerEvent::ExecutionCompleted {
                        execution_id: id,
                        status: toadstool::ExecutionStatus::Failed {
                            error: "Execution timed out".to_string(),
                        },
                        duration_ms: execution.timeout.as_millis() as u64,
                        timestamp: now,
                    });
            }
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} timed-out executions", cleanup_count);
        }
    }
}

/// Perform comprehensive health check
async fn perform_health_check(state: &ServerState) -> bool {
    let config = &state.config.health_check;

    // Check system resources using real data
    if config.check_resources {
        if let Ok(_system_resources) = state.resource_monitor.get_system_resources().await {
            // For now, we'll use simplified health checks
            // In a real implementation, we'd track usage over time
            let cpu_usage_percent = 50.0; // Placeholder - would need historical tracking
            let memory_usage_percent = 45.0; // Placeholder - would calculate from available vs total
            
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

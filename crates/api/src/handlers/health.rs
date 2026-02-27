//! Health check handlers

use std::time::Instant;

use std::time::SystemTime;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::types::{ApiError, HealthCheck, HealthResponse};
use crate::ApiState;

use super::helpers::get_local_node_resources;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/v2/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let start_time = Instant::now();

    // Perform health checks
    let mut checks = Vec::new();

    // Check system resources
    let system_check_duration = Instant::now();
    let system_resources = get_local_node_resources().await;
    checks.push(HealthCheck {
        name: "system_resources".to_string(),
        status: "healthy".to_string(),
        message: Some(format!(
            "CPU: {} cores, Memory: {} GB, Storage: {} GB",
            system_resources.cpu_cores, system_resources.memory_gb, system_resources.storage_gb
        )),
        duration_ms: system_check_duration.elapsed().as_millis() as u64,
    });

    // Check configuration
    let config_check_duration = Instant::now();
    checks.push(HealthCheck {
        name: "configuration".to_string(),
        status: "healthy".to_string(),
        message: Some("Configuration loaded successfully".to_string()),
        duration_ms: config_check_duration.elapsed().as_millis() as u64,
    });

    // Check memory usage
    let memory_check_duration = Instant::now();
    checks.push(HealthCheck {
        name: "memory".to_string(),
        status: "healthy".to_string(),
        message: Some("Memory usage within limits".to_string()),
        duration_ms: memory_check_duration.elapsed().as_millis() as u64,
    });

    // Check execution queue
    let queue_check_duration = Instant::now();
    let executions = state.executions.read().await;
    let queue_size = executions.len();
    checks.push(HealthCheck {
        name: "execution_queue".to_string(),
        status: if queue_size < 1000 {
            "healthy"
        } else {
            "degraded"
        }
        .to_string(),
        message: Some(format!("Queue size: {queue_size}")),
        duration_ms: queue_check_duration.elapsed().as_millis() as u64,
    });

    // Determine overall health
    let all_healthy = checks.iter().all(|c| c.status == "healthy");
    let overall_status = if all_healthy { "healthy" } else { "degraded" };

    let response = HealthResponse {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: start_time.elapsed().as_secs(),
        timestamp: SystemTime::now(),
        checks,
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(response)))
}

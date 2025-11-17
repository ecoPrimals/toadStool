//! Metrics retrieval handlers
//!
//! This module contains handlers for retrieving execution and API metrics

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::constants::{
    METRIC_CPU_USAGE, METRIC_DISK_USAGE, METRIC_EXECUTION_DURATION, METRIC_EXECUTION_STATUS,
    METRIC_MEMORY_USAGE, METRIC_NETWORK_RX, METRIC_NETWORK_TX,
};
use crate::types::{ApiError, ExecutionMetrics, MetricPoint, TimeRange};
use crate::ApiState;

/// Get execution metrics
#[utoipa::path(
    get,
    path = "/api/v2/executions/{execution_id}/metrics",
    params(
        ("execution_id" = Uuid, Path, description = "Execution ID"),
        ("start" = Option<String>, Query, description = "Start time (ISO 8601)"),
        ("end" = Option<String>, Query, description = "End time (ISO 8601)")
    ),
    responses(
        (status = 200, description = "Execution metrics", body = ExecutionMetrics),
        (status = 404, description = "Execution not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn get_execution_metrics(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting metrics for execution {}", execution_id);

    // Check if execution exists
    let executions = state.executions.read().await;
    if !executions.contains_key(&execution_id) {
        return Err(ApiError::new(
            "EXECUTION_NOT_FOUND",
            &format!("Execution {execution_id} not found"),
        ));
    }
    drop(executions);

    // Parse time range from params
    let now = Utc::now();
    let start = params
        .get("start")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| now - chrono::Duration::hours(1));
    let end = params
        .get("end")
        .and_then(|s| s.parse().ok())
        .unwrap_or(now);

    // Generate sample metrics
    let metrics_data = vec![
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_EXECUTION_DURATION.to_string(),
            value: 1250.0,
            unit: "milliseconds".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_CPU_USAGE.to_string(),
            value: 45.5,
            unit: "percent".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_MEMORY_USAGE.to_string(),
            value: 524.288,
            unit: "MB".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_DISK_USAGE.to_string(),
            value: 1024.0,
            unit: "MB".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_NETWORK_RX.to_string(),
            value: 1.0,
            unit: "MB".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_NETWORK_TX.to_string(),
            value: 2.0,
            unit: "MB".to_string(),
        },
        MetricPoint {
            timestamp: now,
            metric_name: METRIC_EXECUTION_STATUS.to_string(),
            value: 1.0,
            unit: "status".to_string(),
        },
    ];

    let metrics = ExecutionMetrics {
        execution_id,
        metrics: metrics_data,
        time_range: TimeRange { start, end },
    };

    Ok(Json(metrics))
}

/// Get API metrics
#[utoipa::path(
    get,
    path = "/api/v2/metrics",
    responses(
        (status = 200, description = "API metrics", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "monitoring"
)]
pub async fn get_api_metrics(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let metrics = state.metrics.read().await;
    Ok(Json(serde_json::json!({
        "total_requests": metrics.total_requests,
        "successful_requests": metrics.successful_requests,
        "failed_requests": metrics.failed_requests,
        "average_response_time_ms": metrics.average_response_time_ms,
        "active_executions": state.executions.read().await.len(),
    })))
}

//! Execution-related API handlers
//!
//! This module contains all handlers related to workload execution management:
//! - Submitting new executions
//! - Checking execution status
//! - Listing executions with filtering
//! - Cancelling executions

use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::constants::DEFAULT_NODE_ID;
use crate::types::{
    ApiError, ApiEvent, ExecutionFilter, ExecutionInfo, ExecutionRequest, ExecutionResponse,
    ExecutionStatus, MonitoringEndpoints, PaginatedResponse, PaginationInfo, ResourceAllocation,
};
use crate::ApiState;

use super::helpers::get_base_url;

/// Submit a new execution request
#[utoipa::path(
    post,
    path = "/api/v2/executions",
    request_body = ExecutionRequest,
    responses(
        (status = 201, description = "Execution submitted successfully", body = ExecutionResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn submit_execution(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<ExecutionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Validate request
    if let Err(validation_errors) = request.validate() {
        warn!(
            "Validation failed for request {}: {:?}",
            request_id, validation_errors
        );
        return Err(ApiError::validation_error(&validation_errors).with_request_id(request_id));
    }

    info!(
        "Received execution request {} for workload type: {:?}",
        request_id, request.workload
    );

    let execution_id = Uuid::new_v4();
    let now = Utc::now();

    // Create execution info
    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Submitted,
        runtime_type: request.runtime_type.clone(),
        submitted_at: now,
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: Some(0.0),
        error_message: None,
        resource_usage: None,
        metadata: request.metadata.clone(),
    };

    // Store execution info
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    // Create resource allocation
    let resource_allocation = ResourceAllocation {
        node_id: DEFAULT_NODE_ID.to_string(),
        cpu_cores: request
            .resources
            .as_ref()
            .and_then(|r| r.cpu_cores)
            .unwrap_or(1.0),
        memory_mb: request
            .resources
            .as_ref()
            .and_then(|r| r.memory_mb)
            .unwrap_or(512),
        storage_mb: request
            .resources
            .as_ref()
            .and_then(|r| r.storage_mb)
            .unwrap_or(1024),
        gpu_count: request
            .resources
            .as_ref()
            .and_then(|r| r.gpu_count)
            .unwrap_or(0),
    };

    // Create monitoring endpoints
    let base_url = get_base_url(&headers);
    let monitoring_endpoints = MonitoringEndpoints {
        status_url: format!("{base_url}/api/v2/executions/{execution_id}"),
        logs_url: format!("{base_url}/api/v2/executions/{execution_id}/logs"),
        metrics_url: format!("{base_url}/api/v2/executions/{execution_id}/metrics"),
        websocket_url: format!(
            "{}/api/v2/executions/{}/ws",
            base_url.replace("http", "ws"),
            execution_id
        ),
    };

    // Create response
    let response = ExecutionResponse {
        execution_id,
        status: ExecutionStatus::Submitted,
        submitted_at: now,
        estimated_completion: None,
        queue_position: Some(1),
        resource_allocation: Some(resource_allocation),
        monitoring_endpoints,
    };

    // Broadcast event
    let event = ApiEvent::ExecutionStarted {
        execution_id,
        runtime_type: request.runtime_type,
        timestamp: now,
    };
    let _ = state.event_broadcaster.send(event);

    // Update metrics
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        let elapsed = start_time.elapsed().as_millis() as f64;
        metrics.average_response_time_ms = f64::midpoint(metrics.average_response_time_ms, elapsed);
    }

    info!("Execution {} submitted successfully", execution_id);
    Ok((StatusCode::CREATED, Json(response)))
}

/// Get execution status
#[utoipa::path(
    get,
    path = "/api/v2/executions/{execution_id}",
    params(
        ("execution_id" = Uuid, Path, description = "Execution ID")
    ),
    responses(
        (status = 200, description = "Execution found", body = ExecutionInfo),
        (status = 404, description = "Execution not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn get_execution_status(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting status for execution {}", execution_id);

    let executions = state.executions.read().await;
    if let Some(info) = executions.get(&execution_id) {
        debug!(
            "Found execution {} with status {:?}",
            execution_id, info.status
        );
        Ok(Json(info.clone()))
    } else {
        warn!("Execution {} not found", execution_id);
        Err(ApiError::new(
            "EXECUTION_NOT_FOUND",
            &format!("Execution {execution_id} not found"),
        ))
    }
}

/// List executions with filtering and pagination
#[utoipa::path(
    get,
    path = "/api/v2/executions",
    params(ExecutionFilter),
    responses(
        (status = 200, description = "Executions found", body = PaginatedResponse<ExecutionInfo>),
        (status = 400, description = "Invalid filter parameters", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn list_executions(
    State(state): State<ApiState>,
    Query(filter): Query<ExecutionFilter>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate filter parameters
    if let Err(validation_errors) = filter.validate() {
        return Err(ApiError::validation_error(&validation_errors));
    }

    let page = filter.page.unwrap_or(1);
    let per_page = filter.per_page.unwrap_or(10);

    debug!("Listing executions with filter: {:?}", filter);

    let executions = state.executions.read().await;
    let mut filtered_executions: Vec<_> = executions
        .values()
        .filter(|exec| {
            // Apply filters
            if let Some(status) = &filter.status {
                if exec.status != *status {
                    return false;
                }
            }
            if let Some(runtime_type) = &filter.runtime_type {
                if exec.runtime_type != *runtime_type {
                    return false;
                }
            }
            if let Some(after) = &filter.submitted_after {
                if exec.submitted_at < *after {
                    return false;
                }
            }
            if let Some(before) = &filter.submitted_before {
                if exec.submitted_at > *before {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by submission time (newest first)
    filtered_executions.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));

    let total_items = filtered_executions.len() as u64;
    let total_pages = total_items.div_ceil(u64::from(per_page));
    let start_index = ((page - 1) * per_page) as usize;
    let end_index = std::cmp::min(start_index + per_page as usize, filtered_executions.len());

    let page_data = filtered_executions[start_index..end_index]
        .iter()
        .map(|exec| (*exec).clone())
        .collect();

    let pagination = PaginationInfo {
        page,
        per_page,
        total_pages: total_pages as u32,
        total_items,
        has_next: page < total_pages as u32,
        has_prev: page > 1,
    };

    let response = PaginatedResponse {
        data: page_data,
        pagination,
    };

    Ok(Json(response))
}

/// Cancel an execution
#[utoipa::path(
    delete,
    path = "/api/v2/executions/{execution_id}",
    params(
        ("execution_id" = Uuid, Path, description = "Execution ID")
    ),
    responses(
        (status = 200, description = "Execution cancelled", body = ExecutionInfo),
        (status = 404, description = "Execution not found", body = ApiError),
        (status = 409, description = "Execution cannot be cancelled", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn cancel_execution(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Cancelling execution {}", execution_id);

    let mut executions = state.executions.write().await;
    if let Some(info) = executions.get_mut(&execution_id) {
        // Check if execution can be cancelled
        match info.status {
            ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled => {
                return Err(ApiError::new(
                    "EXECUTION_NOT_CANCELLABLE",
                    &format!(
                        "Execution {} is in state {:?} and cannot be cancelled",
                        execution_id, info.status
                    ),
                ));
            }
            _ => {}
        }

        info.status = ExecutionStatus::Cancelled;
        info.completed_at = Some(Utc::now());

        // Calculate duration if started
        if let Some(started_at) = info.started_at {
            info.duration_ms = Some((Utc::now() - started_at).num_milliseconds() as u64);
        }

        // Broadcast event
        let event = ApiEvent::ExecutionCompleted {
            execution_id,
            status: ExecutionStatus::Cancelled,
            duration_ms: info.duration_ms.unwrap_or(0),
            timestamp: Utc::now(),
        };
        let _ = state.event_broadcaster.send(event);

        info!("Execution {} cancelled successfully", execution_id);
        Ok(Json(info.clone()))
    } else {
        warn!("Execution {} not found for cancellation", execution_id);
        Err(ApiError::new(
            "EXECUTION_NOT_FOUND",
            &format!("Execution {execution_id} not found"),
        ))
    }
}

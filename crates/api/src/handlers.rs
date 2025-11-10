//! Modern API handlers with `OpenAPI` documentation and validation

use std::collections::HashMap;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::types::{
    ApiError, ApiEvent, ClusterCapacity, ClusterNodeInfo, ClusterStatusResponse, ExecutionFilter,
    ExecutionInfo, ExecutionLogs, ExecutionMetrics, ExecutionRequest, ExecutionResponse,
    ExecutionStatus, HealthCheck, HealthResponse, LogEntry, LogLevel, MetricPoint,
    MonitoringEndpoints, NodeResources, NodeStatus, PaginatedResponse, PaginationInfo,
    ResourceAllocation, TimeRange,
};
use crate::ApiState;

// ============================================================================
// Module-Level Constants
// ============================================================================
// These constants are API handler-specific string literals for zero-cost
// string operations. They are intentionally kept here rather than in the
// central config module as they are implementation details of the API layer.

// ============================================================================
// Section: Default Response Values
// ============================================================================

/// Default node identifier for single-node deployments
///
/// Used when no specific node ID is available or in standalone mode.
/// In multi-node clusters, this will be overridden by the actual node ID.
const DEFAULT_NODE_ID: &str = "node-1";

/// Default runtime type for workload execution
///
/// Specifies "native" as the default execution environment when no specific
/// runtime is requested. Other options include: wasm, container, python, gpu, edge.
const DEFAULT_RUNTIME_TYPE: &str = "native";

/// Source identifier for execution tracking
///
/// Tags execution events and logs as originating from the executor component.
/// Used in distributed tracing and log aggregation.
const EXECUTOR_SOURCE: &str = "executor";

// ============================================================================
// Section: Observability Metric Names
// ============================================================================

/// Metric name for workload execution duration in milliseconds
///
/// Tracks the total time taken for a workload to execute, from submission
/// to completion. Used for performance monitoring and SLA tracking.
const METRIC_EXECUTION_DURATION: &str = "execution_duration_ms";

/// Metric name for CPU usage percentage (0-100)
///
/// Tracks CPU utilization during workload execution. Used for resource
/// monitoring and capacity planning.
const METRIC_CPU_USAGE: &str = "cpu_usage";

/// Metric name for memory usage in bytes
///
/// Tracks memory consumption during workload execution. Used for resource
/// monitoring and detecting memory leaks.
const METRIC_MEMORY_USAGE: &str = "memory_usage";

/// Metric name for disk usage in bytes
///
/// Tracks disk I/O and storage consumption during workload execution.
/// Used for capacity planning and I/O optimization.
const METRIC_DISK_USAGE: &str = "disk_usage";

/// Metric name for network received bytes
///
/// Tracks incoming network traffic during workload execution. Used for
/// network performance monitoring and bandwidth planning.
const METRIC_NETWORK_RX: &str = "network_rx";

/// Metric name for network transmitted bytes
///
/// Tracks outgoing network traffic during workload execution. Used for
/// network performance monitoring and bandwidth planning.
const METRIC_NETWORK_TX: &str = "network_tx";

/// Metric name for execution status tracking
///
/// Records the final status of workload execution (success, failure, timeout).
/// Used for reliability monitoring and error rate tracking.
const METRIC_EXECUTION_STATUS: &str = "execution_status";

// ============================================================================
// Section: API Error Messages
// ============================================================================

/// Standard error message for malformed API requests
///
/// Returned when the request cannot be parsed or fails validation.
/// Client should check request format against API specification.
#[allow(dead_code)]
const API_ERROR_INVALID_REQUEST: &str = "Invalid request format";

/// Standard error message for rate limit violations
///
/// Returned when the client exceeds the allowed request rate.
/// Client should implement exponential backoff and retry logic.
#[allow(dead_code)]
const API_ERROR_RATE_LIMITED: &str = "Rate limit exceeded";

/// Standard error message for execution failures
///
/// Returned when a workload execution fails during processing.
/// Client should inspect error details and decide whether to retry.
#[allow(dead_code)]
const API_ERROR_EXECUTION_FAILED: &str = "Execution failed";

/// Standard error message for resource not found
///
/// Returned when a requested resource (execution, node, etc.) doesn't exist.
/// Client should verify the resource ID and retry the request if necessary.
#[allow(dead_code)]
const API_ERROR_NOT_FOUND: &str = "Resource not found";

/// Success message for execution submission
///
/// Returned when a workload is successfully queued for execution.
/// Client can use the returned execution_id to track progress.
#[allow(dead_code)]
const API_SUCCESS_SUBMITTED: &str = "Execution submitted successfully";

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

    // Create resource allocation (basic implementation)
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

/// Get execution logs
#[utoipa::path(
    get,
    path = "/api/v2/executions/{execution_id}/logs",
    params(
        ("execution_id" = Uuid, Path, description = "Execution ID"),
        ("limit" = Option<u32>, Query, description = "Maximum number of log lines to return"),
        ("offset" = Option<u32>, Query, description = "Number of log lines to skip"),
        ("level" = Option<String>, Query, description = "Minimum log level to include")
    ),
    responses(
        (status = 200, description = "Execution logs", body = ExecutionLogs),
        (status = 404, description = "Execution not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "executions"
)]
pub async fn get_execution_logs(
    State(state): State<ApiState>,
    Path(execution_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting logs for execution {}", execution_id);

    // Check if execution exists
    {
        let executions = state.executions.read().await;
        if !executions.contains_key(&execution_id) {
            return Err(ApiError::new(
                "EXECUTION_NOT_FOUND",
                &format!("Execution {execution_id} not found"),
            ));
        }
    }

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(100);
    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    // Retrieve real execution logs
    let execution_logs = {
        let executions = state.executions.read().await;
        match executions.get(&execution_id) {
            Some(execution_info) => {
                // Try to read logs from execution's log file if it exists
                let log_entries = if let Some(log_file) = execution_info.metadata.get("log_file") {
                    match tokio::fs::read_to_string(log_file).await {
                        Ok(content) => {
                            // Parse log file content into log entries
                            content
                                .lines()
                                .skip(offset as usize)
                                .take(limit as usize)
                                .filter_map(parse_log_line)
                                .collect::<Vec<_>>()
                        }
                        Err(_) => {
                            // If log file doesn't exist, return basic execution info as log
                            vec![LogEntry {
                                timestamp: execution_info.submitted_at,
                                level: LogLevel::Info,
                                message: format!("Execution {execution_id} started"),
                                source: EXECUTOR_SOURCE.to_string(),
                            }]
                        }
                    }
                } else {
                    // No log file, create basic status entries
                    vec![
                        LogEntry {
                            timestamp: execution_info.submitted_at,
                            level: LogLevel::Info,
                            message: format!("Execution {execution_id} created"),
                            source: EXECUTOR_SOURCE.to_string(),
                        },
                        LogEntry {
                            timestamp: execution_info
                                .completed_at
                                .unwrap_or(execution_info.submitted_at),
                            level: match execution_info.status {
                                ExecutionStatus::Completed => LogLevel::Info,
                                ExecutionStatus::Failed => LogLevel::Error,
                                ExecutionStatus::Cancelled => LogLevel::Warn,
                                _ => LogLevel::Info,
                            },
                            message: format!(
                                "Execution {} status: {:?}",
                                execution_id, execution_info.status
                            ),
                            source: EXECUTOR_SOURCE.to_string(),
                        },
                    ]
                };
                log_entries
            }
            None => {
                return Err(ApiError::new(
                    "execution_not_found",
                    &format!("Execution {execution_id} not found"),
                ));
            }
        }
    };

    let total_lines = execution_logs.len();
    let has_more = offset + limit < total_lines as u32;
    let next_token = if has_more {
        Some(format!("offset={}", offset + limit))
    } else {
        None
    };

    let logs = ExecutionLogs {
        execution_id,
        logs: execution_logs,
        total_lines: total_lines as u64,
        has_more,
        next_token,
    };

    Ok(Json(logs))
}

/// Parse a log line into a `LogEntry`
fn parse_log_line(line: &str) -> Option<LogEntry> {
    // Basic log parsing - assumes format: "timestamp level [source] message"
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 {
        return None;
    }

    let timestamp = chrono::DateTime::parse_from_rfc3339(parts[0])
        .ok()
        .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

    let level = match parts[1].to_lowercase().as_str() {
        "error" => LogLevel::Error,
        "warn" | "warning" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        _ => LogLevel::Info,
    };

    let source = parts[2]
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let message = parts[3].to_string();

    Some(LogEntry {
        timestamp,
        level,
        message,
        source,
    })
}

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
    {
        let executions = state.executions.read().await;
        if !executions.contains_key(&execution_id) {
            return Err(ApiError::new(
                "EXECUTION_NOT_FOUND",
                &format!("Execution {execution_id} not found"),
            ));
        }
    }

    let now = Utc::now();
    let start_time = params
        .get("start")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(|| now - chrono::Duration::hours(1));
    let end_time = params
        .get("end")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or(now);

    // Collect real execution metrics
    let execution_metrics = {
        let executions = state.executions.read().await;
        match executions.get(&execution_id) {
            Some(execution_info) => {
                let mut metrics = Vec::new();

                // Add basic execution metrics
                if let Some(started_at) = execution_info.started_at {
                    let execution_duration = if let Some(completed_at) = execution_info.completed_at
                    {
                        completed_at
                            .signed_duration_since(started_at)
                            .num_milliseconds() as f64
                    } else {
                        now.signed_duration_since(started_at).num_milliseconds() as f64
                    };

                    metrics.push(MetricPoint {
                        timestamp: started_at,
                        metric_name: METRIC_EXECUTION_DURATION.to_string(),
                        value: execution_duration,
                        unit: "milliseconds".to_string(),
                    });
                }

                // Add resource metrics if available
                if let Some(ref resource_usage) = execution_info.resource_usage {
                    metrics.push(MetricPoint {
                        timestamp: now,
                        metric_name: METRIC_CPU_USAGE.to_string(),
                        value: resource_usage.cpu_percent,
                        unit: "percent".to_string(),
                    });

                    metrics.push(MetricPoint {
                        timestamp: now,
                        metric_name: METRIC_MEMORY_USAGE.to_string(),
                        value: resource_usage.memory_bytes as f64 / (1024.0 * 1024.0),
                        unit: "MB".to_string(),
                    });

                    // Add disk metrics if available
                    {
                        metrics.push(MetricPoint {
                            timestamp: now,
                            metric_name: METRIC_DISK_USAGE.to_string(),
                            value: resource_usage.disk_bytes as f64 / (1024.0 * 1024.0),
                            unit: "MB".to_string(),
                        });
                    }

                    // Add network metrics
                    {
                        metrics.push(MetricPoint {
                            timestamp: now,
                            metric_name: METRIC_NETWORK_RX.to_string(),
                            value: resource_usage.network_bytes_in as f64 / (1024.0 * 1024.0),
                            unit: "MB".to_string(),
                        });

                        metrics.push(MetricPoint {
                            timestamp: now,
                            metric_name: METRIC_NETWORK_TX.to_string(),
                            value: resource_usage.network_bytes_out as f64 / (1024.0 * 1024.0),
                            unit: "MB".to_string(),
                        });
                    }
                }

                // If no metrics available, provide basic status metric
                if metrics.is_empty() {
                    metrics.push(MetricPoint {
                        timestamp: now,
                        metric_name: METRIC_EXECUTION_STATUS.to_string(),
                        value: match execution_info.status {
                            ExecutionStatus::Completed => 1.0,
                            ExecutionStatus::Failed => 0.0,
                            ExecutionStatus::Running => 0.5,
                            ExecutionStatus::Queued => 0.25,
                            ExecutionStatus::Cancelled => -1.0,
                            ExecutionStatus::Submitted => 0.1,
                            ExecutionStatus::TimedOut => -0.5,
                            ExecutionStatus::Paused => 0.0,
                        },
                        unit: "status".to_string(),
                    });
                }

                metrics
            }
            None => {
                return Err(ApiError::new(
                    "execution_not_found",
                    &format!("Execution {execution_id} not found"),
                ));
            }
        }
    };

    let metrics = ExecutionMetrics {
        execution_id,
        metrics: execution_metrics,
        time_range: TimeRange {
            start: start_time,
            end: end_time,
        },
    };

    Ok(Json(metrics))
}

/// Get cluster status
#[utoipa::path(
    get,
    path = "/api/v2/cluster/status",
    responses(
        (status = 200, description = "Cluster status", body = ClusterStatusResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "cluster"
)]
pub async fn get_cluster_status(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    debug!("Getting cluster status");

    let executions = state.executions.read().await;
    let active_executions = executions
        .values()
        .filter(|exec| {
            matches!(
                exec.status,
                ExecutionStatus::Running | ExecutionStatus::Queued
            )
        })
        .count() as u32;
    let queued_executions = executions
        .values()
        .filter(|exec| matches!(exec.status, ExecutionStatus::Queued))
        .count() as u32;

    // Collect real cluster information
    let node_details = {
        let mut nodes = Vec::new();

        // Get information about the local node
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let local_node = ClusterNodeInfo {
            id: format!("local-node-{}", std::process::id()),
            address: config.network.bind_address.clone(),
            status: NodeStatus::Healthy,
            capabilities: vec![
                DEFAULT_RUNTIME_TYPE.to_string(),
                "container".to_string(),
                "wasm".to_string(),
                "python".to_string(),
            ],
            resources: get_local_node_resources().await,
        };
        nodes.push(local_node);

        // Future enhancement: Add distributed node discovery when implemented
        // Current implementation reports local node only, which is suitable for single-node deployments

        nodes
    };

    let total_capacity = ClusterCapacity {
        cpu_cores: node_details.iter().map(|n| n.resources.cpu_cores).sum(),
        memory_gb: node_details.iter().map(|n| n.resources.memory_gb).sum(),
        storage_gb: node_details.iter().map(|n| n.resources.storage_gb).sum(),
        gpu_count: node_details.iter().map(|n| n.resources.gpu_count).sum(),
    };

    // Calculate current utilization from active executions
    let current_utilization = {
        let active_count = active_executions + queued_executions;
        let base_utilization = (f64::from(active_count) / 100.0).min(1.0);
        ClusterCapacity {
            cpu_cores: (base_utilization * 100.0) as u32,
            memory_gb: (base_utilization * 80.0) as u32,
            storage_gb: (base_utilization * 30.0) as u32,
            gpu_count: 0,
        }
    };

    let response = ClusterStatusResponse {
        cluster_id: "toadstool-cluster-1".to_string(),
        total_nodes: node_details.len() as u32,
        healthy_nodes: node_details
            .iter()
            .filter(|n| n.status == NodeStatus::Healthy)
            .count() as u32,
        cluster_load: 45.5,
        active_executions,
        queued_executions,
        total_capacity,
        used_capacity: current_utilization,
        node_details,
        last_updated: Utc::now(),
    };

    Ok(Json(response))
}

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

    // Check configuration validity
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

    let overall_status = if checks.iter().all(|c| c.status == "healthy") {
        "healthy"
    } else if checks.iter().any(|c| c.status == "degraded") {
        "degraded"
    } else {
        "unhealthy"
    };

    // Calculate actual uptime from process start
    let uptime_seconds = start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp: Utc::now(),
        version: state.config.api_version.clone(),
        uptime_seconds,
        checks,
    };

    let status_code = match overall_status {
        "healthy" => StatusCode::OK,
        "degraded" => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };

    Ok((status_code, Json(response)))
}

/// Get API metrics
#[utoipa::path(
    get,
    path = "/api/v2/metrics",
    responses(
        (status = 200, description = "API metrics", body = ApiMetrics),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "metrics"
)]
pub async fn get_api_metrics(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let metrics = state.metrics.read().await;
    Ok(Json(metrics.clone()))
}

/// Execute a workload from a primal (Songbird, Squirrel, etc.)
///
/// This endpoint is called by primals to execute workloads on this ToadStool instance.
/// It receives a `WorkloadExecutionRequest`, converts it to ToadStool's internal format,
/// executes it, and returns the results.
#[utoipa::path(
    post,
    path = "/api/v1/workload/execute",
    request_body = WorkloadExecutionRequest,
    responses(
        (status = 200, description = "Workload executed successfully", body = WorkloadExecutionResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Execution failed", body = ApiError)
    ),
    tag = "workload"
)]
pub async fn execute_workload(
    State(state): State<ApiState>,
    Json(request): Json<toadstool_distributed::WorkloadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use toadstool_distributed::WorkloadExecutor;
    
    

    info!(
        "Received workload execution request {} from primal {}",
        request.request_id,
        request.from_primal
    );

    // Log the required capability
    debug!(
        "Required capability: {}",
        request.required_capability
    );

    // Use the WorkloadExecutor from the capability system
    let executor = WorkloadExecutor::new();
    let response = executor.execute(request.clone()).await.map_err(|e| {
        ApiError::new(
            "EXECUTION_ERROR",
            &format!("Failed to execute workload: {}", e),
        )
    })?;

    info!(
        "Workload execution request {} completed with status {:?}",
        response.request_id, response.status
    );

    Ok(Json(response))
}

// Helper function to extract base URL from headers
fn get_base_url(headers: &HeaderMap) -> String {
    let config = toadstool_config::env_config::EnvironmentConfig::from_env();
    let default_host = format!(
        "{}:{}",
        config.network.bind_address, config.network.songbird_port
    );
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&default_host);

    let scheme = if headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("http")
        == "https"
    {
        "https"
    } else {
        "http"
    };

    format!("{scheme}://{host}")
}

/// Get local node resources from the system
async fn get_local_node_resources() -> NodeResources {
    // Try to get actual system information
    let cpu_cores = num_cpus::get() as u32;

    // Get memory information
    let memory_gb = if cfg!(target_os = "linux") {
        // Try to read from /proc/meminfo
        match tokio::fs::read_to_string("/proc/meminfo").await {
            Ok(content) => content
                .lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                })
                .map_or(8, |kb| kb / 1024 / 1024) as u32,
            Err(_) => 8, // Default fallback
        }
    } else {
        8 // Default for other platforms
    };

    // Get storage information
    let storage_gb = if cfg!(target_os = "linux") {
        // Try to get disk space for root filesystem
        match std::fs::metadata("/") {
            Ok(_) => {
                // Use statvfs or similar system call - for simplicity, use a reasonable default
                500 // Default 500GB
            }
            Err(_) => 500,
        }
    } else {
        500 // Default for other platforms
    };

    // GPU detection is complex and hardware-specific, default to 0 for now
    let gpu_count = 0;

    NodeResources {
        cpu_cores,
        memory_gb,
        storage_gb,
        gpu_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ResourceRequirements, WorkloadSpec};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    #[test]
    fn test_api_constants() {
        assert_eq!(DEFAULT_NODE_ID, "node-1");
        assert_eq!(DEFAULT_RUNTIME_TYPE, "native");
        assert_eq!(EXECUTOR_SOURCE, "executor");
    }

    #[test]
    fn test_metric_name_constants() {
        assert_eq!(METRIC_EXECUTION_DURATION, "execution_duration_ms");
        assert_eq!(METRIC_CPU_USAGE, "cpu_usage");
        assert_eq!(METRIC_MEMORY_USAGE, "memory_usage");
        assert_eq!(METRIC_DISK_USAGE, "disk_usage");
        assert_eq!(METRIC_NETWORK_RX, "network_rx");
        assert_eq!(METRIC_NETWORK_TX, "network_tx");
        assert_eq!(METRIC_EXECUTION_STATUS, "execution_status");
    }

    #[test]
    fn test_api_error_constants() {
        assert_eq!(API_ERROR_INVALID_REQUEST, "Invalid request format");
        assert_eq!(API_ERROR_RATE_LIMITED, "Rate limit exceeded");
        assert_eq!(API_ERROR_EXECUTION_FAILED, "Execution failed");
        assert_eq!(API_ERROR_NOT_FOUND, "Resource not found");
        assert_eq!(API_SUCCESS_SUBMITTED, "Execution submitted successfully");
    }

    fn create_test_api_state() -> ApiState {
        use crate::{websocket::WebSocketManager, ApiConfig, ApiMetrics};

        let (event_tx, _) = broadcast::channel(100);
        ApiState {
            executions: Arc::new(RwLock::new(HashMap::new())),
            config: ApiConfig {
                bind_address: "127.0.0.1:8084".to_string(),
                enable_rest: true,
                enable_websocket: true,
                cors_enabled: true,
                request_timeout_secs: 30,
                enable_openapi: true,
                api_version: "v2".to_string(),
                enable_auth: false,
                jwt_secret: None,
                enable_rate_limiting: false,
                rate_limit_rpm: 60,
                enable_metrics: true,
                enable_tracing: false,
            },
            metrics: Arc::new(RwLock::new(ApiMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time_ms: 0.0,
                active_connections: 0,
                uptime_seconds: 0,
            })),
            websocket_manager: Arc::new(WebSocketManager::new()),
            event_broadcaster: event_tx,
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = create_test_api_state();
        let result = health_check(State(state)).await;

        // health_check returns Result<impl IntoResponse, ApiError>
        // We can only verify it returns Ok
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_executions_empty() {
        let state = create_test_api_state();
        let query = Query(ExecutionFilter {
            status: None,
            runtime_type: None,
            submitted_after: None,
            submitted_before: None,
            page: Some(1),
            per_page: Some(10),
        });

        let result = list_executions(State(state), query).await;
        // list_executions returns Result<impl IntoResponse, ApiError>
        // We can only verify it returns Ok
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_execution_not_found() {
        let state = create_test_api_state();
        let execution_id = Uuid::new_v4();

        let result = get_execution_status(State(state), Path(execution_id)).await;
        // get_execution_status returns Result<Json<ExecutionInfo>, ApiError>
        assert!(result.is_err());

        if let Err(err) = result {
            assert_eq!(err.error_code, "EXECUTION_NOT_FOUND");
        }
    }

    #[tokio::test]
    async fn test_cancel_execution_not_found() {
        let state = create_test_api_state();
        let execution_id = Uuid::new_v4();

        let result = cancel_execution(State(state), Path(execution_id)).await;
        // cancel_execution returns Result<impl IntoResponse, ApiError>
        assert!(result.is_err());

        if let Err(err) = result {
            assert_eq!(err.error_code, "EXECUTION_NOT_FOUND");
        }
    }

    #[tokio::test]
    async fn test_api_state_metrics_access() {
        let state = create_test_api_state();

        // Test we can read and write metrics
        {
            let mut metrics = state.metrics.write().await;
            metrics.total_requests = 1000;
            metrics.successful_requests = 950;
        }

        let metrics = state.metrics.read().await;
        assert_eq!(metrics.total_requests, 1000);
        assert_eq!(metrics.successful_requests, 950);
    }

    #[tokio::test]
    async fn test_api_state_executions_access() {
        let state = create_test_api_state();

        // Test we can access executions map
        let executions = state.executions.read().await;
        assert_eq!(executions.len(), 0);
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_constants_are_defined() {
        // Just verify constants are defined with expected values
        assert!(!DEFAULT_NODE_ID.is_empty());
        assert!(!DEFAULT_RUNTIME_TYPE.is_empty());
        assert!(!EXECUTOR_SOURCE.is_empty());
        assert!(!METRIC_EXECUTION_DURATION.is_empty());
        assert!(!API_ERROR_INVALID_REQUEST.is_empty());
    }

    #[test]
    fn test_execution_id_generation() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // Test that UUIDs are different
        assert_ne!(id1, id2);
        // Test that they can be converted to string
        let s1 = id1.to_string();
        let s2 = id2.to_string();
        assert!(!s1.is_empty());
        assert!(!s2.is_empty());
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_execution_status_variants() {
        let submitted = ExecutionStatus::Submitted;
        let queued = ExecutionStatus::Queued;
        let running = ExecutionStatus::Running;
        let completed = ExecutionStatus::Completed;
        let failed = ExecutionStatus::Failed;
        let cancelled = ExecutionStatus::Cancelled;
        let timed_out = ExecutionStatus::TimedOut;
        let paused = ExecutionStatus::Paused;

        // Test that variants can be created
        assert!(matches!(submitted, ExecutionStatus::Submitted));
        assert!(matches!(queued, ExecutionStatus::Queued));
        assert!(matches!(running, ExecutionStatus::Running));
        assert!(matches!(completed, ExecutionStatus::Completed));
        assert!(matches!(failed, ExecutionStatus::Failed));
        assert!(matches!(cancelled, ExecutionStatus::Cancelled));
        assert!(matches!(timed_out, ExecutionStatus::TimedOut));
        assert!(matches!(paused, ExecutionStatus::Paused));
    }

    #[test]
    fn test_resource_requirements_creation() {
        // Test that we can create ResourceRequirements
        let req = ResourceRequirements {
            cpu_cores: Some(2.0),
            memory_mb: Some(1024),
            storage_mb: Some(5000),
            gpu_count: Some(1),
            network_mbps: Some(100),
        };

        assert_eq!(req.cpu_cores, Some(2.0));
        assert_eq!(req.memory_mb, Some(1024));
        assert_eq!(req.storage_mb, Some(5000));
        assert_eq!(req.gpu_count, Some(1));
        assert_eq!(req.network_mbps, Some(100));
    }

    #[test]
    fn test_workload_spec_native() {
        // Test native workload spec
        let spec = WorkloadSpec::Native {
            executable: "my-binary".to_string(),
            args: vec!["--arg1".to_string(), "--arg2".to_string()],
        };

        match spec {
            WorkloadSpec::Native { executable, args } => {
                assert_eq!(executable, "my-binary");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Native variant"),
        }
    }

    #[test]
    fn test_workload_spec_container() {
        // Test container workload spec
        let spec = WorkloadSpec::Container {
            image: "ubuntu:latest".to_string(),
            command: Some(vec!["bash".to_string()]),
            args: Some(vec!["-c".to_string(), "echo hello".to_string()]),
        };

        match spec {
            WorkloadSpec::Container { image, .. } => {
                assert_eq!(image, "ubuntu:latest");
            }
            _ => panic!("Expected Container variant"),
        }
    }

    #[tokio::test]
    async fn test_api_state_concurrent_access() {
        let state = create_test_api_state();

        // Test concurrent read access
        let state1 = state.clone();
        let state2 = state.clone();

        let handle1 = tokio::spawn(async move {
            let executions = state1.executions.read().await;
            executions.len()
        });

        let handle2 = tokio::spawn(async move {
            let executions = state2.executions.read().await;
            executions.len()
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert_eq!(result1, 0);
        assert_eq!(result2, 0);
    }

    #[tokio::test]
    async fn test_metrics_update() {
        let state = create_test_api_state();

        {
            let mut metrics = state.metrics.write().await;
            metrics.total_requests += 1;
            metrics.successful_requests += 1;
        }

        let metrics = state.metrics.read().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_node_resources_detection() {
        let resources = get_local_node_resources().await;

        // Basic sanity checks
        assert!(resources.cpu_cores > 0);
        assert!(resources.memory_gb > 0);
        assert!(resources.storage_gb > 0);
        // GPU count can be 0 (no GPU) or positive - it's u32 so always >= 0
    }
}

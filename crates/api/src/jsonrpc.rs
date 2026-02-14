//! JSON-RPC 2.0 API Layer
//!
//! **ecoBin Compliance**: JSON-RPC first architecture
//!
//! This module provides JSON-RPC 2.0 endpoints that wrap the existing REST handlers,
//! enabling JSON-RPC as the primary API protocol per ecoBin standards.
//!
//! ## Methods
//!
//! | Method | Description | REST Equivalent |
//! |--------|-------------|-----------------|
//! | `api.health` | Health check | GET /api/v2/health |
//! | `api.execution.submit` | Submit execution | POST /api/v2/executions |
//! | `api.execution.status` | Get execution status | GET /api/v2/executions/:id |
//! | `api.execution.list` | List executions | GET /api/v2/executions |
//! | `api.execution.cancel` | Cancel execution | DELETE /api/v2/executions/:id |
//! | `api.execution.logs` | Get execution logs | GET /api/v2/executions/:id/logs |
//! | `api.execution.metrics` | Get execution metrics | GET /api/v2/executions/:id/metrics |
//! | `api.metrics` | Get API metrics | GET /api/v2/metrics |
//! | `api.cluster.status` | Get cluster status | GET /api/v2/cluster/status |
//! | `api.workload.execute` | Execute workload | POST /api/v2/workload/execute |
//!
//! ## Usage
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "method": "api.execution.submit",
//!   "params": { "workload": "...", "runtime": "native" },
//!   "id": 1
//! }
//! ```

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ApiState;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Method parameters (optional)
    #[serde(default)]
    pub params: Value,
    /// Request ID (for correlation)
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version
    pub jsonrpc: &'static str,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Request ID (echoed from request)
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// Standard JSON-RPC error codes
pub mod error_codes {
    /// Parse error - Invalid JSON
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request - Not a valid JSON-RPC request
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;
    // Application-specific errors: -32000 to -32099
    /// Execution not found
    pub const EXECUTION_NOT_FOUND: i32 = -32001;
    /// Execution failed
    pub const EXECUTION_FAILED: i32 = -32002;
}

impl JsonRpcResponse {
    /// Create success response
    pub fn success(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create error response
    pub fn error(code: i32, message: impl Into<String>, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    /// Create error response with data
    pub fn error_with_data(
        code: i32,
        message: impl Into<String>,
        data: Value,
        id: Option<Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0",
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
            id,
        }
    }
}

/// JSON-RPC 2.0 endpoint handler
///
/// **ecoBin Compliance**: Primary API endpoint
///
/// This single endpoint handles all JSON-RPC requests, routing them to the
/// appropriate handler based on the method name.
pub async fn jsonrpc_handler(
    State(state): State<ApiState>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    // Validate protocol version
    if request.jsonrpc != "2.0" {
        return (
            StatusCode::OK,
            Json(JsonRpcResponse::error(
                error_codes::INVALID_REQUEST,
                "Invalid JSON-RPC version, expected 2.0",
                request.id,
            )),
        );
    }

    // Route to appropriate handler
    let response = match request.method.as_str() {
        // Health
        "api.health" => handle_health(&state).await,

        // Execution lifecycle
        "api.execution.submit" => handle_execution_submit(&state, &request.params).await,
        "api.execution.status" => handle_execution_status(&state, &request.params).await,
        "api.execution.list" => handle_execution_list(&state).await,
        "api.execution.cancel" => handle_execution_cancel(&state, &request.params).await,
        "api.execution.logs" => handle_execution_logs(&state, &request.params).await,
        "api.execution.metrics" => handle_execution_metrics(&state, &request.params).await,

        // Metrics
        "api.metrics" => handle_api_metrics(&state).await,

        // Cluster
        "api.cluster.status" => handle_cluster_status(&state).await,

        // Workload
        "api.workload.execute" => handle_workload_execute(&state, &request.params).await,

        // Unknown method
        _ => JsonRpcResponse::error(
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
            request.id.clone(),
        ),
    };

    // Attach request ID to response
    let response = JsonRpcResponse {
        id: request.id,
        ..response
    };

    (StatusCode::OK, Json(response))
}

// ============================================================================
// Handler implementations
// ============================================================================

async fn handle_health(state: &ApiState) -> JsonRpcResponse {
    let metrics = state.metrics.read().await;
    JsonRpcResponse::success(
        json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "total_requests": metrics.total_requests,
            "uptime_seconds": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        }),
        None,
    )
}

async fn handle_execution_submit(state: &ApiState, params: &Value) -> JsonRpcResponse {
    // Extract parameters
    let _workload = match params.get("workload") {
        Some(w) => w.as_str().unwrap_or("").to_string(),
        None => {
            return JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Missing required parameter: workload",
                None,
            )
        }
    };

    let runtime_str = params
        .get("runtime")
        .and_then(|r| r.as_str())
        .unwrap_or("native");

    let runtime_type = match runtime_str {
        "wasm" => toadstool::RuntimeType::Wasm,
        "container" => toadstool::RuntimeType::Container,
        "python" => toadstool::RuntimeType::Python,
        _ => toadstool::RuntimeType::Native,
    };

    // Create execution
    let execution_id = Uuid::new_v4();
    let execution_info = crate::ExecutionInfo {
        execution_id,
        status: crate::ExecutionStatus::Submitted,
        runtime_type,
        submitted_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: None,
        error_message: None,
        resource_usage: None,
        metadata: std::collections::HashMap::new(),
    };

    // Store execution
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, execution_info);
    }

    // Update metrics
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
    }

    JsonRpcResponse::success(
        json!({
            "execution_id": execution_id.to_string(),
            "status": "submitted"
        }),
        None,
    )
}

async fn handle_execution_status(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match params.get("execution_id").and_then(|id| id.as_str()) {
        Some(id) => match Uuid::parse_str(id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return JsonRpcResponse::error(
                    error_codes::INVALID_PARAMS,
                    "Invalid execution_id format",
                    None,
                )
            }
        },
        None => {
            return JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Missing required parameter: execution_id",
                None,
            )
        }
    };

    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(info) => JsonRpcResponse::success(
            json!({
                "execution_id": info.execution_id.to_string(),
                "status": format!("{:?}", info.status),
                "runtime_type": format!("{:?}", info.runtime_type),
                "submitted_at": info.submitted_at.to_rfc3339(),
                "started_at": info.started_at.map(|t| t.to_rfc3339()),
                "completed_at": info.completed_at.map(|t| t.to_rfc3339()),
                "duration_ms": info.duration_ms,
                "progress": info.progress,
                "error_message": info.error_message
            }),
            None,
        ),
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {}", execution_id),
            None,
        ),
    }
}

async fn handle_execution_list(state: &ApiState) -> JsonRpcResponse {
    let executions = state.executions.read().await;
    let list: Vec<_> = executions
        .values()
        .map(|info| {
            json!({
                "execution_id": info.execution_id.to_string(),
                "status": format!("{:?}", info.status),
                "runtime_type": format!("{:?}", info.runtime_type),
                "submitted_at": info.submitted_at.to_rfc3339()
            })
        })
        .collect();

    JsonRpcResponse::success(json!({ "executions": list, "count": list.len() }), None)
}

async fn handle_execution_cancel(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match params.get("execution_id").and_then(|id| id.as_str()) {
        Some(id) => match Uuid::parse_str(id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return JsonRpcResponse::error(
                    error_codes::INVALID_PARAMS,
                    "Invalid execution_id format",
                    None,
                )
            }
        },
        None => {
            return JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Missing required parameter: execution_id",
                None,
            )
        }
    };

    let mut executions = state.executions.write().await;
    match executions.get_mut(&execution_id) {
        Some(info) => {
            info.status = crate::ExecutionStatus::Cancelled;
            info.completed_at = Some(chrono::Utc::now());
            JsonRpcResponse::success(
                json!({
                    "execution_id": execution_id.to_string(),
                    "status": "cancelled"
                }),
                None,
            )
        }
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {}", execution_id),
            None,
        ),
    }
}

async fn handle_execution_logs(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match params.get("execution_id").and_then(|id| id.as_str()) {
        Some(id) => match Uuid::parse_str(id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return JsonRpcResponse::error(
                    error_codes::INVALID_PARAMS,
                    "Invalid execution_id format",
                    None,
                )
            }
        },
        None => {
            return JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Missing required parameter: execution_id",
                None,
            )
        }
    };

    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(_info) => {
            // ExecutionInfo doesn't store logs directly - logs are fetched from
            // the underlying execution system. Return metadata-based log info.
            JsonRpcResponse::success(
                json!({
                    "execution_id": execution_id.to_string(),
                    "logs": [],
                    "note": "Logs are streamed via WebSocket or retrieved from execution system"
                }),
                None,
            )
        }
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {}", execution_id),
            None,
        ),
    }
}

async fn handle_execution_metrics(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match params.get("execution_id").and_then(|id| id.as_str()) {
        Some(id) => match Uuid::parse_str(id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return JsonRpcResponse::error(
                    error_codes::INVALID_PARAMS,
                    "Invalid execution_id format",
                    None,
                )
            }
        },
        None => {
            return JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Missing required parameter: execution_id",
                None,
            )
        }
    };

    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(info) => {
            let resource_usage = info.resource_usage.as_ref().map(|r| {
                json!({
                    "cpu_percent": r.cpu_percent,
                    "memory_bytes": r.memory_bytes,
                    "disk_bytes": r.disk_bytes,
                    "network_bytes_in": r.network_bytes_in,
                    "network_bytes_out": r.network_bytes_out,
                    "gpu_percent": r.gpu_percent
                })
            });
            JsonRpcResponse::success(
                json!({
                    "execution_id": execution_id.to_string(),
                    "duration_ms": info.duration_ms,
                    "progress": info.progress,
                    "resource_usage": resource_usage
                }),
                None,
            )
        }
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {}", execution_id),
            None,
        ),
    }
}

async fn handle_api_metrics(state: &ApiState) -> JsonRpcResponse {
    let metrics = state.metrics.read().await;
    JsonRpcResponse::success(
        json!({
            "total_requests": metrics.total_requests,
            "successful_requests": metrics.successful_requests,
            "failed_requests": metrics.failed_requests,
            "average_response_time_ms": metrics.average_response_time_ms
        }),
        None,
    )
}

async fn handle_cluster_status(state: &ApiState) -> JsonRpcResponse {
    let executions = state.executions.read().await;
    let active_count = executions
        .values()
        .filter(|e| matches!(e.status, crate::ExecutionStatus::Running))
        .count();

    JsonRpcResponse::success(
        json!({
            "status": "healthy",
            "nodes": 1,
            "active_executions": active_count,
            "total_executions": executions.len()
        }),
        None,
    )
}

async fn handle_workload_execute(state: &ApiState, params: &Value) -> JsonRpcResponse {
    // This delegates to the primal capability system if available
    if state.capability_provider.is_some() {
        // Capability provider is available - create execution via submit
        // The actual workload execution is handled by the primal capability system
        // through the execution lifecycle
        handle_execution_submit(state, params).await
    } else {
        // Fallback: create execution via submit
        handle_execution_submit(state, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(json!({"test": "value"}), Some(json!(1)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.jsonrpc, "2.0");
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(error_codes::METHOD_NOT_FOUND, "Not found", None);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::METHOD_NOT_FOUND);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(error_codes::PARSE_ERROR, -32700);
        assert_eq!(error_codes::INVALID_REQUEST, -32600);
        assert_eq!(error_codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(error_codes::INVALID_PARAMS, -32602);
        assert_eq!(error_codes::INTERNAL_ERROR, -32603);
    }
}

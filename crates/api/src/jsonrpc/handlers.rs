//! JSON-RPC 2.0 method handlers
//!
//! Each handler corresponds to a JSON-RPC method (e.g. api.health, api.execution.submit).

use serde_json::{json, Value};
use uuid::Uuid;

use crate::ApiState;

use super::types::{error_codes, JsonRpcResponse};

/// Extract execution_id from params, returning error response on failure
#[allow(clippy::result_large_err)] // JsonRpcResponse is the API contract for error returns
fn parse_execution_id(params: &Value) -> Result<Uuid, JsonRpcResponse> {
    match params.get("execution_id").and_then(|id| id.as_str()) {
        Some(id) => Uuid::parse_str(id).map_err(|_| {
            JsonRpcResponse::error(
                error_codes::INVALID_PARAMS,
                "Invalid execution_id format",
                None,
            )
        }),
        None => Err(JsonRpcResponse::error(
            error_codes::INVALID_PARAMS,
            "Missing required parameter: execution_id",
            None,
        )),
    }
}

pub async fn handle_health(state: &ApiState) -> JsonRpcResponse {
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

pub async fn handle_execution_submit(state: &ApiState, params: &Value) -> JsonRpcResponse {
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

    let execution_id = Uuid::new_v4();
    let execution_info = crate::ExecutionInfo {
        execution_id,
        status: crate::ExecutionStatus::Submitted,
        runtime_type,
        submitted_at: std::time::SystemTime::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: None,
        error_message: None,
        resource_usage: None,
        metadata: std::collections::HashMap::new(),
    };

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, execution_info);
    }

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

pub async fn handle_execution_status(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match parse_execution_id(params) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(info) => JsonRpcResponse::success(
            json!({
                "execution_id": info.execution_id.to_string(),
                "status": format!("{:?}", info.status),
                "runtime_type": format!("{:?}", info.runtime_type),
                "submitted_at": toadstool_common::system_time_serde::format_rfc3339(info.submitted_at),
                "started_at": info.started_at.map(toadstool_common::system_time_serde::format_rfc3339),
                "completed_at": info.completed_at.map(toadstool_common::system_time_serde::format_rfc3339),
                "duration_ms": info.duration_ms,
                "progress": info.progress,
                "error_message": info.error_message
            }),
            None,
        ),
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {execution_id}"),
            None,
        ),
    }
}

pub async fn handle_execution_list(state: &ApiState) -> JsonRpcResponse {
    let executions = state.executions.read().await;
    let list: Vec<_> = executions
        .values()
        .map(|info| {
            json!({
                "execution_id": info.execution_id.to_string(),
                "status": format!("{:?}", info.status),
                "runtime_type": format!("{:?}", info.runtime_type),
                "submitted_at": toadstool_common::system_time_serde::format_rfc3339(info.submitted_at)
            })
        })
        .collect();

    JsonRpcResponse::success(json!({ "executions": list, "count": list.len() }), None)
}

pub async fn handle_execution_cancel(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match parse_execution_id(params) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let mut executions = state.executions.write().await;
    match executions.get_mut(&execution_id) {
        Some(info) => {
            info.status = crate::ExecutionStatus::Cancelled;
            info.completed_at = Some(std::time::SystemTime::now());
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
            format!("Execution not found: {execution_id}"),
            None,
        ),
    }
}

pub async fn handle_execution_logs(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match parse_execution_id(params) {
        Ok(id) => id,
        Err(r) => return r,
    };

    let executions = state.executions.read().await;
    match executions.get(&execution_id) {
        Some(_info) => JsonRpcResponse::success(
            json!({
                "execution_id": execution_id.to_string(),
                "logs": [],
                "note": "Logs are streamed via WebSocket or retrieved from execution system"
            }),
            None,
        ),
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {execution_id}"),
            None,
        ),
    }
}

pub async fn handle_execution_metrics(state: &ApiState, params: &Value) -> JsonRpcResponse {
    let execution_id = match parse_execution_id(params) {
        Ok(id) => id,
        Err(r) => return r,
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
                    "execution_id": info.execution_id.to_string(),
                    "duration_ms": info.duration_ms,
                    "progress": info.progress,
                    "resource_usage": resource_usage
                }),
                None,
            )
        }
        None => JsonRpcResponse::error(
            error_codes::EXECUTION_NOT_FOUND,
            format!("Execution not found: {execution_id}"),
            None,
        ),
    }
}

pub async fn handle_api_metrics(state: &ApiState) -> JsonRpcResponse {
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

pub async fn handle_cluster_status(state: &ApiState) -> JsonRpcResponse {
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

pub async fn handle_workload_execute(state: &ApiState, params: &Value) -> JsonRpcResponse {
    handle_execution_submit(state, params).await
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

use super::api_types::*;
use super::jsonrpc_server::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerState};

pub(super) async fn handle_request(
    request: JsonRpcRequest,
    state: &ServerState,
) -> JsonRpcResponse {
    if request.jsonrpc != toadstool_common::constants::jsonrpc::VERSION {
        return JsonRpcResponse {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: super::jsonrpc_server::error_codes::INVALID_REQUEST,
                message: "Invalid JSON-RPC version (must be \"2.0\")".to_string(),
                data: None,
            }),
            id: request.id,
        };
    }

    let result = match request.method.as_str() {
        "daemon.health" | "health.liveness" | "health.readiness" | "health.check" => {
            handle_health(state).await
        }
        "identity.get" => handle_identity(state).await,
        "daemon.metrics" => handle_metrics(state).await,
        "daemon.submit_workload" => handle_submit_workload(request.params, state).await,
        "daemon.get_workload" => handle_get_workload(request.params, state).await,
        "daemon.delete_workload" => handle_delete_workload(request.params, state).await,
        "daemon.list_workloads" => handle_list_workloads(state).await,
        #[cfg(feature = "nautilus")]
        method if method.starts_with("ai.nautilus.") => {
            route_nautilus(method, &request.params).await
        }
        _ => Err(JsonRpcError {
            code: super::jsonrpc_server::error_codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            result: Some(value),
            error: None,
            id: request.id,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            result: None,
            error: Some(error),
            id: request.id,
        },
    }
}

async fn handle_identity(_state: &ServerState) -> Result<Value, JsonRpcError> {
    Ok(json!({
        "primal": toadstool_common::constants::PRIMAL_NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "JSON-RPC 2.0",
        "capabilities": ["compute", "workload", "daemon"],
        "transport": "unix-socket",
    }))
}

async fn handle_health(state: &ServerState) -> Result<Value, JsonRpcError> {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let active_workloads = state.workload_manager.active_workload_count().await;

    Ok(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime_secs,
        "active_workloads": active_workloads,
        "ecosystem_connected": false,
    }))
}

async fn handle_metrics(state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_ids = state.workload_manager.list_workloads().await;

    let statuses = futures::future::join_all(
        workload_ids
            .iter()
            .map(|id| state.workload_manager.get_workload_status(id)),
    )
    .await;

    let (mut queued, mut running, mut completed, mut failed) = (0, 0, 0, 0);
    for status_resp in statuses.into_iter().flatten() {
        match status_resp.status {
            WorkloadStatus::Queued => queued += 1,
            WorkloadStatus::Running => running += 1,
            WorkloadStatus::Completed => completed += 1,
            WorkloadStatus::Failed => failed += 1,
            WorkloadStatus::Cancelled => {}
        }
    }

    Ok(json!({
        "uptime_secs": state.start_time.elapsed().as_secs(),
        "workloads": {
            "queued": queued,
            "running": running,
            "completed": completed,
            "failed": failed,
        },
        "ecosystem_connected": false,
    }))
}

async fn handle_submit_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let request: SubmitWorkloadRequest =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: super::jsonrpc_server::error_codes::INVALID_PARAMS,
            message: format!("Invalid params: {e}"),
            data: None,
        })?;

    match state.workload_manager.submit_workload(request).await {
        Ok(response) => serde_json::to_value(response).map_err(|e| JsonRpcError {
            code: super::jsonrpc_server::error_codes::INTERNAL_ERROR,
            message: format!("Serialization failed: {e}"),
            data: None,
        }),
        Err(e) => Err(JsonRpcError {
            code: super::jsonrpc_server::error_codes::WORKLOAD_SUBMIT_FAILED,
            message: format!("Workload submission failed: {e}"),
            data: None,
        }),
    }
}

async fn handle_get_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_id = params["id"].as_str().ok_or_else(|| JsonRpcError {
        code: super::jsonrpc_server::error_codes::INVALID_PARAMS,
        message: "Missing or invalid 'id' parameter".to_string(),
        data: None,
    })?;

    state
        .workload_manager
        .get_workload_status(workload_id)
        .await
        .map_or_else(
            || {
                Err(JsonRpcError {
                    code: super::jsonrpc_server::error_codes::WORKLOAD_NOT_FOUND,
                    message: format!("Workload not found: {workload_id}"),
                    data: None,
                })
            },
            |status| {
                serde_json::to_value(status).map_err(|e| JsonRpcError {
                    code: super::jsonrpc_server::error_codes::INTERNAL_ERROR,
                    message: format!("Serialization failed: {e}"),
                    data: None,
                })
            },
        )
}

async fn handle_delete_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_id = params["id"].as_str().ok_or_else(|| JsonRpcError {
        code: super::jsonrpc_server::error_codes::INVALID_PARAMS,
        message: "Missing or invalid 'id' parameter".to_string(),
        data: None,
    })?;

    match state.workload_manager.cancel_workload(workload_id).await {
        Ok(()) => Ok(json!({"success": true, "workload_id": workload_id})),
        Err(e) => Err(JsonRpcError {
            code: super::jsonrpc_server::error_codes::WORKLOAD_DELETE_FAILED,
            message: format!("Workload deletion failed: {e}"),
            data: None,
        }),
    }
}

async fn handle_list_workloads(state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_ids = state.workload_manager.list_workloads().await;

    let workloads: Vec<_> = futures::future::join_all(
        workload_ids
            .iter()
            .map(|id| state.workload_manager.get_workload_status(id)),
    )
    .await
    .into_iter()
    .flatten()
    .collect();

    Ok(json!({
        "workloads": workloads,
        "count": workloads.len(),
    }))
}

#[cfg(feature = "nautilus")]
async fn route_nautilus(method: &str, params: &Value) -> Result<Value, JsonRpcError> {
    super::nautilus_handlers::proxy_nautilus_rpc(method, params)
        .await
        .map_err(|e| JsonRpcError {
            code: e.code,
            message: e.message,
            data: None,
        })
}

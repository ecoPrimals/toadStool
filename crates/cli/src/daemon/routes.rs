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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    async fn test_state() -> ServerState {
        let wm = Arc::new(
            super::super::workload_manager::WorkloadManager::new(2)
                .await
                .expect("create workload manager"),
        );
        ServerState {
            start_time: Instant::now(),
            workload_manager: wm,
        }
    }

    fn rpc(method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(json!(1)),
        }
    }

    #[tokio::test]
    async fn invalid_jsonrpc_version_returns_error() {
        let state = test_state().await;
        let req = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            method: "daemon.health".to_string(),
            params: json!(null),
            id: Some(json!(1)),
        };
        let resp = handle_request(req, &state).await;
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(
            err.code,
            super::super::jsonrpc_server::error_codes::INVALID_REQUEST
        );
        assert!(err.message.contains("2.0"));
    }

    #[tokio::test]
    async fn identity_get_returns_primal_info() {
        let state = test_state().await;
        let resp = handle_request(rpc("identity.get", json!(null)), &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["primal"], toadstool_common::constants::PRIMAL_NAME);
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
        assert_eq!(result["transport"], "unix-socket");
    }

    #[tokio::test]
    async fn health_returns_ok_status() {
        let state = test_state().await;
        let resp = handle_request(rpc("daemon.health", json!(null)), &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["status"], "ok");
        assert!(result["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn health_aliases_all_work() {
        let state = test_state().await;
        for method in ["health.liveness", "health.readiness", "health.check"] {
            let resp = handle_request(rpc(method, json!(null)), &state).await;
            assert!(resp.error.is_none(), "method {method} failed");
            assert_eq!(resp.result.unwrap()["status"], "ok");
        }
    }

    #[tokio::test]
    async fn metrics_returns_workload_counts() {
        let state = test_state().await;
        let resp = handle_request(rpc("daemon.metrics", json!(null)), &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(result["uptime_secs"].is_number());
        assert_eq!(result["workloads"]["queued"], 0);
        assert_eq!(result["workloads"]["running"], 0);
    }

    #[tokio::test]
    async fn get_workload_missing_id_returns_invalid_params() {
        let state = test_state().await;
        let resp = handle_request(rpc("daemon.get_workload", json!({})), &state).await;
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(
            err.code,
            super::super::jsonrpc_server::error_codes::INVALID_PARAMS
        );
    }

    #[tokio::test]
    async fn get_workload_not_found() {
        let state = test_state().await;
        let resp = handle_request(
            rpc("daemon.get_workload", json!({"id": "nonexistent"})),
            &state,
        )
        .await;
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(
            err.code,
            super::super::jsonrpc_server::error_codes::WORKLOAD_NOT_FOUND
        );
    }

    #[tokio::test]
    async fn delete_workload_missing_id_returns_invalid_params() {
        let state = test_state().await;
        let resp = handle_request(rpc("daemon.delete_workload", json!({})), &state).await;
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.unwrap().code,
            super::super::jsonrpc_server::error_codes::INVALID_PARAMS
        );
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let state = test_state().await;
        let resp = handle_request(rpc("nonexistent.method", json!(null)), &state).await;
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.unwrap().code,
            super::super::jsonrpc_server::error_codes::METHOD_NOT_FOUND
        );
    }

    #[tokio::test]
    async fn list_workloads_empty() {
        let state = test_state().await;
        let resp = handle_request(rpc("daemon.list_workloads", json!(null)), &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["count"], 0);
        assert!(result["workloads"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn submit_workload_invalid_params() {
        let state = test_state().await;
        let resp = handle_request(
            rpc("daemon.submit_workload", json!("not-an-object")),
            &state,
        )
        .await;
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.unwrap().code,
            super::super::jsonrpc_server::error_codes::INVALID_PARAMS
        );
    }
}

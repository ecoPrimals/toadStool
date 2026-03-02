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

mod handlers;
mod types;

use axum::{extract::State, http::StatusCode, Json};

use crate::ApiState;

pub use types::{error_codes, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use handlers::*;

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

    let response = match request.method.as_str() {
        "api.health" => handle_health(&state).await,
        "api.execution.submit" => handle_execution_submit(&state, &request.params).await,
        "api.execution.status" => handle_execution_status(&state, &request.params).await,
        "api.execution.list" => handle_execution_list(&state).await,
        "api.execution.cancel" => handle_execution_cancel(&state, &request.params).await,
        "api.execution.logs" => handle_execution_logs(&state, &request.params).await,
        "api.execution.metrics" => handle_execution_metrics(&state, &request.params).await,
        "api.metrics" => handle_api_metrics(&state).await,
        "api.cluster.status" => handle_cluster_status(&state).await,
        "api.workload.execute" => handle_workload_execute(&state, &request.params).await,
        _ => JsonRpcResponse::error(
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
            request.id.clone(),
        ),
    };

    let response = JsonRpcResponse {
        id: request.id,
        ..response
    };

    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    use super::*;
    use crate::{ApiMetrics, ApiState};

    fn test_api_state() -> ApiState {
        ApiState {
            executions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            event_broadcaster: broadcast::channel(16).0,
            capability_provider: None,
        }
    }

    #[test]
    fn test_jsonrpc_request_deserialize() {
        let json = r#"{"jsonrpc":"2.0","method":"api.health","params":{},"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "api.health");
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_jsonrpc_request_with_params() {
        let json =
            r#"{"jsonrpc":"2.0","method":"api.execution.submit","params":{"workload":"x"},"id":2}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.method, "api.execution.submit");
        assert!(req.params.get("workload").is_some());
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(
            serde_json::json!({"test": "value"}),
            Some(serde_json::json!(1)),
        );
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
    fn test_jsonrpc_response_error_with_data() {
        let response = JsonRpcResponse::error_with_data(
            error_codes::EXECUTION_NOT_FOUND,
            "Not found",
            serde_json::json!({"execution_id": "abc"}),
            Some(serde_json::json!(1)),
        );
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, error_codes::EXECUTION_NOT_FOUND);
        assert!(err.data.is_some());
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(error_codes::PARSE_ERROR, -32700);
        assert_eq!(error_codes::INVALID_REQUEST, -32600);
        assert_eq!(error_codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(error_codes::INVALID_PARAMS, -32602);
        assert_eq!(error_codes::INTERNAL_ERROR, -32603);
        assert_eq!(error_codes::EXECUTION_NOT_FOUND, -32001);
        assert_eq!(error_codes::EXECUTION_FAILED, -32002);
    }

    #[test]
    fn test_jsonrpc_error_serialization() {
        let err = JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(serde_json::json!("extra")),
        };
        let json = serde_json::to_string(&err).expect("serialize");
        assert!(json.contains("Method not found"));
        assert!(json.contains("-32601"));
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_health() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.health".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert_eq!(result["status"], "healthy");
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_invalid_version() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            method: "api.health".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_method_not_found() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.unknown.method".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_submit() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.submit".to_string(),
            params: serde_json::json!({"workload": "test", "runtime": "native"}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert!(result.get("execution_id").is_some());
        assert_eq!(result["status"], "submitted");
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_list() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.list".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert!(result.get("executions").is_some());
        assert!(result.get("count").is_some());
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_api_metrics() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.metrics".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert!(result.get("total_requests").is_some());
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_cluster_status() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.cluster.status".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert_eq!(result["status"], "healthy");
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_status_not_found() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.status".to_string(),
            params: serde_json::json!({"execution_id": "00000000-0000-0000-0000-000000000000"}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(
            response.error.unwrap().code,
            error_codes::EXECUTION_NOT_FOUND
        );
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_submit_missing_workload() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.submit".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_status_invalid_id_format() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.status".to_string(),
            params: serde_json::json!({"execution_id": "not-a-valid-uuid"}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_jsonrpc_handler_execution_status_missing_id() {
        let state = test_api_state();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "api.execution.status".to_string(),
            params: serde_json::json!({}),
            id: Some(serde_json::json!(1)),
        };
        let (status, Json(response)) = jsonrpc_handler(State(state), Json(request)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, error_codes::INVALID_PARAMS);
    }

    #[test]
    fn test_jsonrpc_request_params_optional() {
        let json = r#"{"jsonrpc":"2.0","method":"api.health","id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.method, "api.health");
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_jsonrpc_response_roundtrip() {
        let response = JsonRpcResponse::success(
            serde_json::json!({"key": "value"}),
            Some(serde_json::json!(42)),
        );
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"key\""));
        assert!(json.contains("\"value\""));
    }
}

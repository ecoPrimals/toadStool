//! JSON-RPC 2.0 API server for ToadStool daemon mode (EVOLVED)
//!
//! **DEEP DEBT EVOLUTION**: Replaces HTTP/TCP with JSON-RPC over Unix sockets.
//!
//! ## Philosophy
//!
//! - **Pure Rust**: No HTTP stack (axum, hyper, tower)
//! - **Unix Sockets**: Fast local IPC, zero network overhead
//! - **JSON-RPC 2.0**: Standard protocol, primal-to-primal communication
//! - **Async**: Fully concurrent with tokio
//!
//! ## Before & After
//!
//! **Before (HTTP)**:
//! ```text
//! POST /api/v1/workload/submit HTTP/1.1
//! Content-Type: application/json
//! 
//! {"biome_yaml": "..."}
//! ```
//!
//! **After (JSON-RPC)**:
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "method": "daemon.submit_workload",
//!   "params": {"biome_yaml": "..."},
//!   "id": 1
//! }
//! ```
//!
//! ## Methods
//!
//! - `daemon.health` - Health check and uptime
//! - `daemon.metrics` - Prometheus-compatible metrics
//! - `daemon.submit_workload` - Submit new workload
//! - `daemon.get_workload` - Get workload status
//! - `daemon.delete_workload` - Cancel/delete workload
//! - `daemon.list_workloads` - List all workloads

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info};

use super::api_types::*;
use super::workload_manager::WorkloadManager;

/// Shared server state
#[derive(Clone)]
pub struct ServerState {
    /// Server start time
    pub start_time: Instant,
    
    /// Workload manager
    pub workload_manager: Arc<WorkloadManager>,
}

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used via serde deserialization
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
#[allow(dead_code)] // Fields used via serde serialization
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Error codes (JSON-RPC 2.0 standard + custom)
#[allow(dead_code)]
mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    
    // Custom error codes (application-specific)
    pub const WORKLOAD_NOT_FOUND: i32 = -32000;
    pub const WORKLOAD_SUBMIT_FAILED: i32 = -32001;
    pub const WORKLOAD_DELETE_FAILED: i32 = -32002;
}

/// Start JSON-RPC API server over Unix socket
///
/// # Deep Debt Evolution
///
/// Before: HTTP server on TCP port, requires axum/hyper/tower stack
/// After: JSON-RPC over Unix socket, pure Rust with tokio
///
/// # Errors
///
/// Returns error if socket binding fails or server crashes
pub async fn start_jsonrpc_server(
    socket_path: &Path,
    workload_manager: Arc<WorkloadManager>,
) -> anyhow::Result<()> {
    let state = ServerState {
        start_time: Instant::now(),
        workload_manager,
    };
    
    // Remove existing socket if present
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    
    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Bind Unix socket
    let listener = UnixListener::bind(socket_path)?;
    info!("🍄 JSON-RPC server listening on {}", socket_path.display());
    info!("📊 Methods:");
    info!("   daemon.health");
    info!("   daemon.metrics");
    info!("   daemon.submit_workload");
    info!("   daemon.get_workload");
    info!("   daemon.delete_workload");
    info!("   daemon.list_workloads");
    
    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state_clone).await {
                        error!("Connection handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

/// Handle a single client connection
async fn handle_connection(stream: UnixStream, state: ServerState) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        
        if n == 0 {
            // Connection closed
            break;
        }
        
        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request, &state).await,
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: error_codes::PARSE_ERROR,
                    message: format!("Parse error: {}", e),
                    data: None,
                }),
                id: None,
            },
        };
        
        // Send response
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    
    Ok(())
}

/// Handle JSON-RPC request
async fn handle_request(request: JsonRpcRequest, state: &ServerState) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "daemon.health" => handle_health(state).await,
        "daemon.metrics" => handle_metrics(state).await,
        "daemon.submit_workload" => handle_submit_workload(request.params, state).await,
        "daemon.get_workload" => handle_get_workload(request.params, state).await,
        "daemon.delete_workload" => handle_delete_workload(request.params, state).await,
        "daemon.list_workloads" => handle_list_workloads(state).await,
        _ => Err(JsonRpcError {
            code: error_codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };
    
    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id: request.id,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id: request.id,
        },
    }
}

/// Health check handler
async fn handle_health(state: &ServerState) -> Result<Value, JsonRpcError> {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let active_workloads = state.workload_manager.active_workload_count().await;
    
    Ok(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime_secs,
        "active_workloads": active_workloads,
        "biomeos_connected": false,
    }))
}

/// Metrics handler
async fn handle_metrics(state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_ids = state.workload_manager.list_workloads().await;
    
    let mut queued = 0;
    let mut running = 0;
    let mut completed = 0;
    let mut failed = 0;
    
    for id in &workload_ids {
        if let Some(status_resp) = state.workload_manager.get_workload_status(id).await {
            match status_resp.status {
                WorkloadStatus::Queued => queued += 1,
                WorkloadStatus::Running => running += 1,
                WorkloadStatus::Completed => completed += 1,
                WorkloadStatus::Failed => failed += 1,
                WorkloadStatus::Cancelled => {}
            }
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
        "biomeos_connected": false,
    }))
}

/// Submit workload handler
async fn handle_submit_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let request: SubmitWorkloadRequest = serde_json::from_value(params)
        .map_err(|e| JsonRpcError {
            code: error_codes::INVALID_PARAMS,
            message: format!("Invalid params: {}", e),
            data: None,
        })?;
    
    match state.workload_manager.submit_workload(request).await {
        Ok(response) => Ok(serde_json::to_value(response).unwrap()),
        Err(e) => Err(JsonRpcError {
            code: error_codes::WORKLOAD_SUBMIT_FAILED,
            message: format!("Workload submission failed: {}", e),
            data: None,
        }),
    }
}

/// Get workload handler
async fn handle_get_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_id = params["id"]
        .as_str()
        .ok_or_else(|| JsonRpcError {
            code: error_codes::INVALID_PARAMS,
            message: "Missing or invalid 'id' parameter".to_string(),
            data: None,
        })?;
    
    match state.workload_manager.get_workload_status(workload_id).await {
        Some(status) => Ok(serde_json::to_value(status).unwrap()),
        None => Err(JsonRpcError {
            code: error_codes::WORKLOAD_NOT_FOUND,
            message: format!("Workload not found: {}", workload_id),
            data: None,
        }),
    }
}

/// Delete workload handler
async fn handle_delete_workload(params: Value, state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_id = params["id"]
        .as_str()
        .ok_or_else(|| JsonRpcError {
            code: error_codes::INVALID_PARAMS,
            message: "Missing or invalid 'id' parameter".to_string(),
            data: None,
        })?;
    
    match state.workload_manager.cancel_workload(workload_id).await {
        Ok(()) => Ok(json!({"success": true, "workload_id": workload_id})),
        Err(e) => Err(JsonRpcError {
            code: error_codes::WORKLOAD_DELETE_FAILED,
            message: format!("Workload deletion failed: {}", e),
            data: None,
        }),
    }
}

/// List workloads handler
async fn handle_list_workloads(state: &ServerState) -> Result<Value, JsonRpcError> {
    let workload_ids = state.workload_manager.list_workloads().await;
    
    let mut workloads = Vec::new();
    for id in workload_ids {
        if let Some(status) = state.workload_manager.get_workload_status(&id).await {
            workloads.push(status);
        }
    }
    
    Ok(json!({
        "workloads": workloads,
        "count": workloads.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jsonrpc_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","method":"daemon.health","params":{},"id":1}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "daemon.health");
    }
    
    #[test]
    fn test_jsonrpc_response_serialization() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({"status": "ok"})),
            error: None,
            id: Some(json!(1)),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("result"));
    }
}

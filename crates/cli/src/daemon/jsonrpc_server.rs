// SPDX-License-Identifier: AGPL-3.0-or-later
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
//!
//! ### `ai.nautilus.*` (feature = "nautilus")
//!
//! - `ai.nautilus.status` - Brain status (observations, trained, drifting)
//! - `ai.nautilus.observe` - Feed physics observation
//! - `ai.nautilus.train` - Evolve shell on accumulated observations
//! - `ai.nautilus.predict` - Predict dynamical observables for a beta value
//! - `ai.nautilus.screen` - Score candidate beta values by information content
//! - `ai.nautilus.edges` - Detect concept edges via LOO analysis
//! - `ai.nautilus.shell.export` - Serialize shell to JSON
//! - `ai.nautilus.shell.import` - Restore brain from serialized JSON

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use super::workload_manager::WorkloadManager;

/// Shared server state
#[derive(Clone)]
pub struct ServerState {
    /// Server start time
    pub start_time: Instant,

    /// Workload manager
    pub workload_manager: Arc<WorkloadManager>,
    // Nautilus methods are proxied to the compute service via capability-based IPC.
    // No local state needed — the compute service owns the brain.
}

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub(crate) jsonrpc: String,
    /// Method name (e.g., "daemon.health")
    pub(crate) method: String,
    /// Request parameters (deserialized from JSON)
    pub(crate) params: Value,
    /// Request ID for matching request/response
    pub(crate) id: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcResponse {
    /// Protocol version ("2.0")
    pub(crate) jsonrpc: String,
    /// Success result (present on success, omitted on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result: Option<Value>,
    /// Error object (present on failure, omitted on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<JsonRpcError>,
    /// Request ID from original request
    pub(crate) id: Option<Value>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcError {
    pub(crate) code: i32,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Value>,
}

/// Error codes -- re-exported from shared ecosystem constants
pub(crate) mod error_codes {
    pub use toadstool_common::constants::jsonrpc::error_codes::*;
}

/// Parse a raw JSON line into a request, dispatch it, and return the response.
async fn dispatch_or_parse_error(raw: &[u8], state: &ServerState) -> JsonRpcResponse {
    match serde_json::from_slice::<JsonRpcRequest>(raw) {
        Ok(request) => super::routes::handle_request(request, state).await,
        Err(e) => JsonRpcResponse {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: error_codes::PARSE_ERROR,
                message: format!("Parse error: {e}"),
                data: None,
            }),
            id: None,
        },
    }
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
) -> crate::Result<()> {
    let state = ServerState {
        start_time: Instant::now(),
        workload_manager,
    };

    // Remove existing socket if present — async to avoid blocking the runtime
    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await?;
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Bind Unix socket
    let listener = UnixListener::bind(socket_path)?;
    info!("🍄 JSON-RPC server listening on {}", socket_path.display());
    info!(
        "📊 Methods: daemon.{{health,metrics,submit_workload,get_workload,delete_workload,list_workloads}}"
    );
    #[cfg(feature = "nautilus")]
    info!(
        "🐚 Methods: ai.nautilus.{{status,observe,train,predict,screen,edges,shell.export,shell.import}}"
    );

    let env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
    let btsp_required = toadstool_common::primal_sockets::is_btsp_required(&env);
    if btsp_required {
        info!("🔒 Daemon JSON-RPC: BTSP handshake required (FAMILY_ID set)");
    }

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state_clone = state.clone();
                let btsp = btsp_required;
                tokio::spawn(async move {
                    let result = if btsp {
                        handle_btsp_daemon_connection(stream, state_clone).await
                    } else {
                        handle_connection(stream, state_clone).await
                    };
                    if let Err(e) = result {
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

/// Start JSON-RPC API server over TCP (cross-host access via `--port`).
///
/// Mirrors the Unix socket server but over `0.0.0.0:{port}` for mobile compute
/// sharing and cross-gate communication per UniBin standard.
pub async fn start_tcp_jsonrpc_server(
    port: u16,
    workload_manager: Arc<WorkloadManager>,
) -> crate::Result<()> {
    use tokio::net::TcpListener;

    let state = ServerState {
        start_time: Instant::now(),
        workload_manager,
    };

    let bind_host = std::env::var("TOADSTOOL_BIND_ADDRESS")
        .unwrap_or_else(|_| toadstool_common::constants::network::BIND_ALL_IPV4.into());
    let addr = format!("{bind_host}:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!(
        "🌐 TCP JSON-RPC server listening on {}",
        listener.local_addr()?
    );

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    let (reader, writer) = stream.into_split();
                    if let Err(e) = handle_tcp_connection(reader, writer, state_clone).await {
                        error!("TCP connection from {peer} error: {e}");
                    }
                });
            }
            Err(e) => {
                error!("TCP accept error: {e}");
            }
        }
    }
}

/// Handle a TCP client connection (same protocol as Unix, different transport).
async fn handle_tcp_connection(
    reader: tokio::net::tcp::OwnedReadHalf,
    mut writer: tokio::net::tcp::OwnedWriteHalf,
    state: ServerState,
) -> crate::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let response = dispatch_or_parse_error(line.as_bytes(), &state).await;

        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

/// BTSP production path: handshake then length-prefixed JSON-RPC frames (see `BTSP_PROTOCOL_STANDARD.md`).
#[cfg(feature = "btsp")]
async fn handle_btsp_daemon_connection(
    stream: UnixStream,
    state: ServerState,
) -> crate::Result<()> {
    use toadstool_common::btsp;

    let family_seed = resolve_daemon_family_seed()?;
    let mut stream = stream;

    match btsp::BtspServer::accept_handshake(&mut stream, &family_seed).await {
        Ok(session) => {
            info!(
                "🔒 BTSP daemon handshake complete: cipher={}, session_id={:02x?}",
                session.cipher.as_str(),
                &session.session_id[..4]
            );
        }
        Err(e) => {
            warn!("🔒 BTSP handshake rejected (daemon JSON-RPC): {e}");
            let _ = btsp::BtspServer::send_handshake_error(&mut stream).await;
            return Err(crate::CliError::Other(format!(
                "BTSP handshake failed: {e}"
            )));
        }
    }

    loop {
        match btsp::framing::read_frame(&mut stream).await {
            Ok(frame) => {
                let response = dispatch_or_parse_error(&frame, &state).await;
                let response_json = serde_json::to_string(&response)?;
                if let Err(e) =
                    btsp::framing::write_frame(&mut stream, response_json.as_bytes()).await
                {
                    warn!("BTSP daemon JSON-RPC write error: {e}");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                warn!("BTSP daemon JSON-RPC read error: {e}");
                break;
            }
        }
    }

    Ok(())
}

/// Production path when the `btsp` crate feature is **disabled**.
///
/// When [`toadstool_common::primal_sockets::is_btsp_required`] is true, clients expect BTSP;
/// without the feature we cannot handshake. Logs at target `btsp`, shuts down the stream, and
/// returns (same policy as the server crate tarpc BTSP gate). Unset family ID env vars for
/// development NDJSON on this socket.
#[cfg(not(feature = "btsp"))]
async fn handle_btsp_daemon_connection(
    mut stream: UnixStream,
    _state: ServerState,
) -> crate::Result<()> {
    warn!(
        target: "btsp",
        "BTSP required (FAMILY_ID set) but this binary was built without the `btsp` Cargo feature — closing connection; rebuild with `btsp` enabled or unset family ID env vars for development NDJSON"
    );
    if let Err(e) = stream.shutdown().await {
        warn!(target: "btsp", "shutdown after BTSP-disabled close: {e}");
    }
    Ok(())
}

/// Resolve family seed for BTSP (`FAMILY_SEED` or `.family.seed` in biomeOS dir).
#[cfg(feature = "btsp")]
fn resolve_daemon_family_seed() -> crate::Result<Vec<u8>> {
    if let Ok(seed) = std::env::var("FAMILY_SEED") {
        return Ok(seed.into_bytes());
    }
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let seed_path = biomeos_dir.join(".family.seed");
    if seed_path.exists() {
        return std::fs::read(&seed_path).map_err(|e| {
            crate::CliError::InvalidConfig(format!("Failed to read family seed: {e}"))
        });
    }
    Err(crate::CliError::InvalidConfig(
        "BTSP requires FAMILY_SEED env var or .family.seed file in biomeOS directory".to_string(),
    ))
}

/// Handle a single client connection
async fn handle_connection(stream: UnixStream, state: ServerState) -> crate::Result<()> {
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

        let response = dispatch_or_parse_error(line.as_bytes(), &state).await;
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::time::timeout;

    /// Spawns a test server and returns (temp_dir, socket_path, state).
    /// The temp_dir must be kept in scope for the socket to remain valid.
    async fn spawn_test_server(test_name: &str) -> (tempfile::TempDir, PathBuf, Arc<ServerState>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let socket_path = dir.path().join(format!("{}.sock", test_name));

        let workload_manager = Arc::new(
            WorkloadManager::new(2)
                .await
                .expect("create workload manager"),
        );
        let state = Arc::new(ServerState {
            start_time: Instant::now(),
            workload_manager,
        });

        if socket_path.exists() {
            std::fs::remove_file(&socket_path).expect("remove existing");
        }
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        let listener = UnixListener::bind(&socket_path).expect("bind");

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    let state_clone = Arc::clone(&state_clone);
                    tokio::spawn(async move {
                        let _ =
                            super::handle_connection(stream, state_clone.as_ref().clone()).await;
                    });
                }
            }
        });

        tokio::task::yield_now().await;

        (dir, socket_path, state)
    }

    fn jsonrpc_request(method: &str, params: Value, id: Value) -> String {
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id
        }))
        .expect("serialize request")
    }

    async fn connect_and_send(socket_path: &std::path::Path, request: &str) -> String {
        let stream = timeout(std::time::Duration::from_secs(2), async {
            for _ in 0..50 {
                match UnixStream::connect(socket_path).await {
                    Ok(s) => return Ok(s),
                    Err(_) => tokio::task::yield_now().await,
                }
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "could not connect",
            ))
        })
        .await
        .expect("connect timeout")
        .expect("connect");

        let (reader, mut writer) = stream.into_split();
        writer
            .write_all(request.as_bytes())
            .await
            .expect("write request");
        writer.write_all(b"\n").await.expect("write newline");
        writer.flush().await.expect("flush");

        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        timeout(
            std::time::Duration::from_secs(2),
            reader.read_line(&mut line),
        )
        .await
        .expect("read timeout")
        .expect("read");
        line
    }

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
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            result: Some(json!({"status": "ok"})),
            error: None,
            id: Some(json!(1)),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("result"));
    }

    #[tokio::test]
    async fn test_server_construct_and_health() {
        let (_dir, socket_path, _state) = spawn_test_server("test").await;

        let req = jsonrpc_request("daemon.health", json!({}), json!(1));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert_eq!(parsed["result"]["status"], "ok");
        assert!(parsed["result"]["uptime_secs"].as_u64().is_some());
        assert_eq!(parsed["id"], 1);
    }

    #[tokio::test]
    async fn test_method_routing_metrics() {
        let (_dir, socket_path, _state) = spawn_test_server("test_metrics").await;

        let req = jsonrpc_request("daemon.metrics", json!({}), json!(2));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["result"]["workloads"].is_object());
        assert!(parsed["result"]["uptime_secs"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_method_routing_list_workloads() {
        let (_dir, socket_path, _state) = spawn_test_server("test_list").await;

        let req = jsonrpc_request("daemon.list_workloads", json!({}), json!(3));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["result"]["workloads"].is_array());
        assert!(parsed["result"]["count"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_submit_workload_request_response() {
        let (_dir, socket_path, _state) = spawn_test_server("test_submit").await;

        let params = json!({
            "biome_yaml": "version: 1.0",
            "requester": "test-client",
            "environment": {},
            "timeout_secs": 60,
            "persistent": false
        });
        let req = jsonrpc_request("daemon.submit_workload", params, json!(4));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        if let Some(err) = parsed.get("error") {
            unreachable!("submit_workload failed: {err}");
        }
        // JSON-RPC handler returns workload_id string directly from WorkloadManager
        let workload_id = parsed["result"]
            .as_str()
            .or_else(|| parsed["result"]["workload_id"].as_str());
        assert!(
            workload_id.is_some_and(|id| !id.is_empty()),
            "expected workload_id in result: {}",
            parsed
        );
    }

    #[tokio::test]
    async fn test_get_workload_not_found() {
        let (_dir, socket_path, _state) = spawn_test_server("test_get").await;

        let params = json!({"id": "nonexistent-uuid"});
        let req = jsonrpc_request("daemon.get_workload", params, json!(5));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::WORKLOAD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_parse_error_invalid_json() {
        let (_dir, socket_path, _state) = spawn_test_server("test_parse").await;

        let resp = connect_and_send(&socket_path, "not valid json\n").await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::PARSE_ERROR);
    }

    #[tokio::test]
    async fn test_invalid_jsonrpc_version() {
        let (_dir, socket_path, _state) = spawn_test_server("test_version").await;

        let req = jsonrpc_request("daemon.health", json!({}), json!(1));
        let bad_req = req.replace("\"2.0\"", "\"1.0\"");
        let resp = connect_and_send(&socket_path, &bad_req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let (_dir, socket_path, _state) = spawn_test_server("test_method").await;

        let req = jsonrpc_request("daemon.nonexistent", json!({}), json!(6));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_invalid_params_submit_workload() {
        let (_dir, socket_path, _state) = spawn_test_server("test_invalid_submit").await;

        let params = json!({"invalid": "params"});
        let req = jsonrpc_request("daemon.submit_workload", params, json!(7));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_invalid_params_get_workload_missing_id() {
        let (_dir, socket_path, _state) = spawn_test_server("test_get_missing").await;

        let params = json!({});
        let req = jsonrpc_request("daemon.get_workload", params, json!(8));
        let resp = connect_and_send(&socket_path, &req).await;
        let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
        assert!(parsed["error"].is_object());
        assert_eq!(parsed["error"]["code"], error_codes::INVALID_PARAMS);
    }
}

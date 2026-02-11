//! # Pure Manual JSON-RPC 2.0 Server over Unix Sockets
//!
//! Educational implementation for other primals to learn from.
//! No library dependencies - just tokio, serde_json, and the JSON-RPC 2.0 spec.
//!
//! ## Why Manual Implementation?
//!
//! - **Educational**: Other primals can see exactly how it works
//! - **No Library Lock-in**: jsonrpsee doesn't support Unix sockets
//! - **Deep Debt Compliant**: Full control, no hardcoding
//! - **Lightweight**: Minimal dependencies
//!
//! ## JSON-RPC 2.0 Specification
//!
//! Request:
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "method": "method_name",
//!   "params": {...},
//!   "id": 1
//! }
//! ```
//!
//! Response:
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "result": {...},
//!   "id": 1
//! }
//! ```
//!
//! Error Response:
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "error": {"code": -32600, "message": "Invalid Request"},
//!   "id": null
//! }
//! ```

#[allow(deprecated)]
use toadstool_common::interned_strings::primals;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use super::cross_gate::JobRouter;
use super::gpu_job_queue::{GpuJobQueue, JobQueueConfig, JobQueueError};
use super::ollama::{OllamaClient, OllamaConfig};
use super::resource_estimator::ResourceEstimator;
use super::resource_optimizer::ResourceOptimizer;
use super::resource_validator::ResourceValidator;
use super::tarpc_server::WorkloadExecutor;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response (Success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: Cow<'static, str>,
    pub result: Value,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: Cow<'static, str>,
    pub error: JsonRpcError,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Zero-copy JSON-RPC version for responses (always "2.0")
const JSONRPC_VERSION: Cow<'static, str> =
    Cow::Borrowed(toadstool_common::constants::jsonrpc::VERSION);

/// Fallback error message for serialization failures (avoid per-call allocation)
pub(crate) const SERIALIZATION_FAILED: &str = "serialization failed";

// JSON-RPC 2.0 Error Codes -- re-exported from shared constants
pub use toadstool_common::constants::jsonrpc::error_codes::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};

/// Manual JSON-RPC 2.0 Server over Unix Sockets
pub struct ManualJsonRpcServer {
    pub(crate) executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    pub(crate) version: String,
    pub(crate) estimator: ResourceEstimator,
    pub(crate) validator: ResourceValidator,
    pub(crate) optimizer: ResourceOptimizer,
    pub(crate) job_queue: GpuJobQueue,
    pub(crate) ollama: OllamaClient,
    pub(crate) router: Arc<tokio::sync::RwLock<JobRouter>>,
    pub(crate) error_count: Arc<AtomicU64>,
    pub(crate) start_time: std::time::Instant,
}

impl Clone for ManualJsonRpcServer {
    fn clone(&self) -> Self {
        Self {
            executor: Arc::clone(&self.executor),
            version: self.version.clone(),
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
            job_queue: self.job_queue.clone(),
            ollama: self.ollama.clone(),
            router: Arc::clone(&self.router),
            error_count: Arc::clone(&self.error_count),
            start_time: self.start_time,
        }
    }
}

impl ManualJsonRpcServer {
    /// Create new manual JSON-RPC server
    ///
    /// Pass `error_count` to share the counter with tarpc server for unified monitoring.
    pub fn new(
        executor: Arc<dyn WorkloadExecutor + Send + Sync>,
        version: String,
        error_count: Option<Arc<AtomicU64>>,
    ) -> Self {
        // Derive local gate identity from hostname for cross-gate routing
        let local_gate_id = std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| "local".to_string());
        Self {
            executor,
            version,
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
            job_queue: GpuJobQueue::new(JobQueueConfig::default()),
            ollama: OllamaClient::new(OllamaConfig::default()),
            router: Arc::new(tokio::sync::RwLock::new(JobRouter::new(local_gate_id))),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            start_time: std::time::Instant::now(),
        }
    }

    /// Start server on Unix socket
    ///
    /// Deep debt principle: No hardcoding, Unix socket for multi-instance support
    pub async fn serve(
        self,
        socket_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Starting manual JSON-RPC 2.0 server on Unix socket: {:?}",
            socket_path
        );

        // Ensure parent directory exists (biomeOS requirement for custom socket paths)
        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create socket directory {:?}: {}", parent, e))?;
            info!("Ensured JSON-RPC socket directory exists: {:?}", parent);
        }

        // Clean up old socket if it exists
        if socket_path.exists() {
            warn!("Removing old JSON-RPC socket: {:?}", socket_path);
            tokio::fs::remove_file(&socket_path).await?;
        }

        // Bind to Unix socket
        let listener = UnixListener::bind(&socket_path)?;

        // Set permissions to user-only (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&socket_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&socket_path, perms)?;
            info!("Set JSON-RPC socket permissions to 0600");
        }

        info!(
            "✅ Manual JSON-RPC 2.0 server listening on: {:?}",
            socket_path
        );

        let server = Arc::new(self);

        // Accept connections loop
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Start server on TCP listener (isomorphic fallback)
    ///
    /// **ISOMORPHIC MODE**: Automatic fallback for platforms without Unix sockets.
    ///
    /// This method is used only when Unix sockets fail due to platform constraints
    /// (SELinux, Android, etc.). The listener is pre-bound to 127.0.0.1:0 for security.
    pub async fn serve_tcp(
        self,
        listener: tokio::net::TcpListener,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let local_addr = listener.local_addr()?;
        info!(
            "✅ Manual JSON-RPC 2.0 server listening on TCP: {}",
            local_addr
        );

        let server = Arc::new(self);

        // Accept connections loop
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_tcp_connection(stream).await {
                            error!("TCP connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("TCP accept error: {}", e);
                }
            }
        }
    }

    /// Handle a single TCP connection
    async fn handle_tcp_connection(
        &self,
        stream: tokio::net::TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Peek at first line to detect format
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await?;

        // Parse JSON-RPC request (zero-copy: parse directly from bytes, avoid trim().to_string())
        let (request_result, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            // HTTP-wrapped request - read remaining headers and body (from_slice, no String alloc)
            let (_headers, body) = self.read_http_request_continuation_tcp(&mut reader).await?;
            (serde_json::from_slice::<JsonRpcRequest>(&body), true)
        } else {
            // Raw JSON-RPC request - parse directly from first line (no allocation)
            (
                serde_json::from_slice::<JsonRpcRequest>(first_line.trim().as_bytes()),
                false,
            )
        };

        let response_body = match request_result {
            Ok(request) => {
                // Handle JSON-RPC request
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_vec(&response)?
            }
            Err(e) => {
                // Parse error
                self.error_count.fetch_add(1, Ordering::Relaxed);
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: JSONRPC_VERSION.clone(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: Cow::Owned(format!("Parse error: {}", e)),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_vec(&error_response)?
            }
        };

        // Write response in appropriate format
        if is_http {
            self.write_http_response_tcp(&mut writer, &response_body)
                .await?;
        } else {
            // Raw JSON-RPC - just send the JSON followed by newline
            writer.write_all(&response_body).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Read HTTP request continuation for TCP (after first line already read)
    async fn read_http_request_continuation_tcp(
        &self,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    ) -> Result<(HashMap<String, String>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
        use std::collections::HashMap;
        use tokio::io::AsyncBufReadExt;

        let mut headers = HashMap::new();
        let mut line = String::new();

        // Read headers (request line already consumed)
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break; // End of headers
            }

            // Parse header (Name: Value)
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_lowercase(), value.trim().to_string());
            }
        }

        // Read body (based on Content-Length) - parse directly with from_slice, no UTF-8 validation
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;

        Ok((headers, body))
    }

    /// Write HTTP response for TCP
    async fn write_http_response_tcp(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        body: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::AsyncWriteExt;

        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            body.len()
        );
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(body).await?;
        writer.flush().await?;

        Ok(())
    }

    /// Handle a single connection
    ///
    /// Supports both raw JSON-RPC and HTTP-wrapped JSON-RPC:
    /// - Raw: `{"jsonrpc":"2.0","method":"...","id":1}\n`
    /// - HTTP: `POST / HTTP/1.1\r\nContent-Length: ...\r\n\r\n{...}`
    async fn handle_connection(
        &self,
        stream: UnixStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Peek at first line to detect format
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await?;

        // Parse JSON-RPC request (zero-copy: parse directly from bytes, avoid trim().to_string())
        let (request_result, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            // HTTP-wrapped request - read remaining headers and body (from_slice, no String alloc)
            let (_headers, body) = self.read_http_request_continuation(&mut reader).await?;
            (serde_json::from_slice::<JsonRpcRequest>(&body), true)
        } else {
            // Raw JSON-RPC request - parse directly from first line (no allocation)
            (
                serde_json::from_slice::<JsonRpcRequest>(first_line.trim().as_bytes()),
                false,
            )
        };

        let response_body = match request_result {
            Ok(request) => {
                // Handle JSON-RPC request
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_vec(&response)?
            }
            Err(e) => {
                // Parse error
                self.error_count.fetch_add(1, Ordering::Relaxed);
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: JSONRPC_VERSION.clone(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: Cow::Owned(format!("Parse error: {}", e)),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_vec(&error_response)?
            }
        };

        // Write response in appropriate format
        if is_http {
            self.write_http_response(&mut writer, &response_body)
                .await?;
        } else {
            // Raw JSON-RPC - just send the JSON followed by newline
            writer.write_all(&response_body).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Read HTTP request continuation (after first line already read)
    ///
    /// Called when first line indicates HTTP format
    async fn read_http_request_continuation(
        &self,
        reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    ) -> Result<(HashMap<String, String>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = HashMap::new();
        let mut line = String::new();

        // Read headers (request line already consumed)
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break; // End of headers
            }

            // Parse header (Name: Value)
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_lowercase(), value.trim().to_string());
            }
        }

        // Read body (based on Content-Length) - parse directly with from_slice, no UTF-8 validation
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;

        Ok((headers, body))
    }

    /// Write HTTP response
    async fn write_http_response(
        &self,
        writer: &mut tokio::net::unix::OwnedWriteHalf,
        body: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            body.len()
        );
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(body).await?;
        writer.flush().await?;

        Ok(())
    }

    /// Handle JSON-RPC request
    async fn handle_jsonrpc_request(&self, request: JsonRpcRequest) -> Value {
        // Validate JSON-RPC version
        if request.jsonrpc != toadstool_common::constants::jsonrpc::VERSION {
            return self.error_response(INVALID_REQUEST, "Invalid jsonrpc version", &request);
        }

        // Route to method handler
        match request.method.as_str() {
            "toadstool.health" => self.handle_health(request).await,
            "toadstool.version" => self.handle_version(request).await,
            "toadstool.query_capabilities" => self.handle_query_capabilities(request).await,
            // Collaborative Intelligence methods (semantic naming: toadstool.resources.*)
            "toadstool.resources.estimate" => self.handle_resources_estimate(request).await,
            "toadstool.resources.validate_availability" => {
                self.handle_resources_validate_availability(request).await
            }
            "toadstool.resources.suggest_optimizations" => {
                self.handle_resources_suggest_optimizations(request).await
            }
            "compute.discover_capabilities" => self.handle_discover_capabilities(request).await,
            // GPU Job Queue methods (semantic: compute.*)
            "compute.submit" => self.handle_compute_submit(request).await,
            "compute.status" => self.handle_compute_status(request).await,
            "compute.result" => self.handle_compute_result(request).await,
            "compute.cancel" => self.handle_compute_cancel(request).await,
            "compute.list" => self.handle_compute_list(request).await,
            // GPU info methods (semantic: gpu.*)
            "gpu.info" => self.handle_gpu_info(request).await,
            "gpu.memory" => self.handle_gpu_memory(request).await,
            // Ollama integration methods (semantic: ollama.*)
            "ollama.list_models" => self.handle_ollama_list_models(request).await,
            "ollama.inference" => self.handle_ollama_inference(request).await,
            "ollama.load" => self.handle_ollama_load(request).await,
            "ollama.unload" => self.handle_ollama_unload(request).await,
            // Cross-gate routing methods (semantic: gate.*)
            "gate.update" => self.handle_gate_update(request).await,
            "gate.remove" => self.handle_gate_remove(request).await,
            "gate.list" => self.handle_gate_list(request).await,
            "gate.route" => self.handle_gate_route(request).await,
            _ => self.error_response(
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
                &request,
            ),
        }
    }

    /// Handle health check
    #[allow(deprecated)]
    async fn handle_health(&self, request: JsonRpcRequest) -> Value {
        self.success_response(
            serde_json::json!({
                "healthy": true,
                "service": primals::TOADSTOOL,
                "version": self.version,
                "error_count": self.error_count.load(Ordering::Relaxed),
                "uptime_secs": self.start_time.elapsed().as_secs(),
            }),
            &request,
        )
    }

    /// Handle version query
    async fn handle_version(&self, request: JsonRpcRequest) -> Value {
        self.success_response(
            serde_json::json!({"version": self.version, "protocol": "json-rpc-2.0"}),
            &request,
        )
    }

    /// Handle discover_capabilities - returns all available methods
    #[allow(deprecated)]
    async fn handle_discover_capabilities(&self, request: JsonRpcRequest) -> Value {
        let capabilities = serde_json::json!({
            "capabilities": [
                "toadstool.health",
                "toadstool.version",
                "toadstool.query_capabilities",
                "toadstool.resources.estimate",
                "toadstool.resources.validate_availability",
                "toadstool.resources.suggest_optimizations",
                "compute.discover_capabilities",
                "compute.submit",
                "compute.status",
                "compute.result",
                "compute.cancel",
                "compute.list",
                "gpu.info",
                "gpu.memory",
                "ollama.list_models",
                "ollama.inference",
                "ollama.load",
                "ollama.unload",
                "gate.update",
                "gate.remove",
                "gate.list",
                "gate.route"
            ],
            "version": self.version,
            "primal": primals::TOADSTOOL
        });

        serde_json::to_value(JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result: capabilities,
            id: request.id,
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    // ---- Helper methods ----

    /// Extract job_id from request params
    pub(crate) fn extract_job_id(&self, request: &JsonRpcRequest) -> Result<uuid::Uuid, Value> {
        let job_id_str = request
            .params
            .as_ref()
            .and_then(|p| p.get("job_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                self.error_response(INVALID_PARAMS, "Missing 'job_id' param", request)
            })?;

        uuid::Uuid::parse_str(job_id_str)
            .map_err(|_| self.error_response(INVALID_PARAMS, "Invalid job_id UUID", request))
    }

    /// Build a success JSON-RPC response
    pub(crate) fn success_response(&self, result: Value, request: &JsonRpcRequest) -> Value {
        serde_json::to_value(JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result,
            id: request.id.clone(),
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    /// Build an error JSON-RPC response
    pub(crate) fn error_response(
        &self,
        code: i32,
        message: impl Into<Cow<'static, str>>,
        request: &JsonRpcRequest,
    ) -> Value {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        serde_json::to_value(JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code,
                message: message.into(),
                data: None,
            },
            id: request.id.clone(),
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    /// Map job queue errors to appropriate JSON-RPC error codes
    pub(crate) fn job_queue_error_response(
        &self,
        err: JobQueueError,
        request: &JsonRpcRequest,
    ) -> Value {
        let code = match &err {
            JobQueueError::JobNotFound { .. } => METHOD_NOT_FOUND,
            JobQueueError::QueueFull { .. } => INTERNAL_ERROR,
            _ => INTERNAL_ERROR,
        };
        self.error_response(code, err.to_string(), request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use crate::gpu_job_queue::JobQueueError;

    fn test_server() -> ManualJsonRpcServer {
        let executor = Arc::new(super::super::tarpc_server::StandaloneExecutor::new());
        ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    #[test]
    fn test_jsonrpc_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test");
    }

    #[test]
    fn test_jsonrpc_response_serialization() {
        let response = JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result: serde_json::json!({"status": "ok"}),
            id: Some(serde_json::json!(1)),
        };
        let json = serde_json::to_vec(&response).unwrap();
        let json_str = String::from_utf8_lossy(&json);
        assert!(json_str.contains("2.0"));
        assert!(json_str.contains("result"));
    }

    #[test]
    fn test_success_response() {
        let server = test_server();
        let request = mk_request("test", None, 42);
        let result = server.success_response(serde_json::json!({"key": "value"}), &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert_eq!(obj["result"]["key"], "value");
        assert_eq!(obj["id"], 42);
    }

    #[test]
    fn test_error_response_increments_count() {
        let error_count = Arc::new(AtomicU64::new(0));
        let executor = Arc::new(super::super::tarpc_server::StandaloneExecutor::new());
        let server =
            ManualJsonRpcServer::new(executor, "test".to_string(), Some(Arc::clone(&error_count)));
        let request = mk_request("test", None, 1);

        let result = server.error_response(-32600, "Test error", &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], -32600);
        assert_eq!(obj["error"]["message"], "Test error");
        assert_eq!(error_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_job_queue_error_response_job_not_found() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let err = JobQueueError::JobNotFound {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn test_job_queue_error_response_queue_full() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let err = JobQueueError::QueueFull { max: 100 };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn test_extract_job_id_valid() {
        let server = test_server();
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let request = mk_request("test", Some(serde_json::json!({"job_id": uuid_str})), 1);
        let job_id = server.extract_job_id(&request).expect("valid uuid");
        assert_eq!(job_id.to_string(), uuid_str);
    }

    #[test]
    fn test_extract_job_id_missing_param() {
        let server = test_server();
        let request = mk_request("test", Some(serde_json::json!({})), 1);
        let result = server.extract_job_id(&request);
        assert!(result.is_err());
        let err_val = result.unwrap_err();
        let obj = err_val.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing"));
    }

    #[test]
    fn test_extract_job_id_invalid_uuid() {
        let server = test_server();
        let request = mk_request("test", Some(serde_json::json!({"job_id": "not-a-uuid"})), 1);
        let result = server.extract_job_id(&request);
        assert!(result.is_err());
        let err_val = result.unwrap_err();
        let obj = err_val.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_method_dispatch_health() {
        let server = test_server();
        let request = mk_request("toadstool.health", None, 1);
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj["result"]["healthy"].as_bool().unwrap());
        assert_eq!(obj["result"]["service"], "toadstool");
    }

    #[tokio::test]
    async fn test_method_dispatch_version() {
        let server = test_server();
        let request = mk_request("toadstool.version", None, 2);
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["result"]["version"], "test-1.0.0");
        assert_eq!(obj["result"]["protocol"], "json-rpc-2.0");
    }

    #[tokio::test]
    async fn test_method_dispatch_invalid_version() {
        let server = test_server();
        let request = JsonRpcRequest {
            jsonrpc: "3.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_method_dispatch_unknown() {
        let server = test_server();
        let request = mk_request("unknown.method", None, 99);
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
    }

    // ---- Parse functions (from_slice paths) ----

    #[test]
    fn test_parse_request_from_slice_valid() {
        let json = br#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
        let request: JsonRpcRequest = serde_json::from_slice(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "toadstool.health");
        assert_eq!(request.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_parse_request_from_slice_malformed_json() {
        let json = b"{invalid json}";
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_from_slice_missing_method() {
        // JSON without required "method" field fails deserialization
        let json = br#"{"jsonrpc":"2.0","id":1}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_from_slice_missing_jsonrpc() {
        let json = br#"{"method":"test","id":1}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_batch_array_fails() {
        // JSON-RPC batch is array; this impl expects single object
        let json = br#"[{"jsonrpc":"2.0","method":"test","id":1}]"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_with_params_from_slice() {
        let json = br#"{"jsonrpc":"2.0","method":"compute.submit","params":{"x":42},"id":2}"#;
        let request: JsonRpcRequest = serde_json::from_slice(json).unwrap();
        assert_eq!(request.params, Some(serde_json::json!({"x": 42})));
    }

    // ---- Response construction ----

    #[test]
    fn test_success_response_with_null_id() {
        let server = test_server();
        let request = JsonRpcRequest {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            method: "test".to_string(),
            params: None,
            id: None,
        };
        let result = server.success_response(serde_json::json!({"ok": true}), &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert_eq!(obj["result"]["ok"], true);
        assert!(obj["id"].is_null());
    }

    #[test]
    fn test_error_response_cow_owned() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let dynamic_msg = format!("Dynamic error: {}", 42);
        let result = server.error_response(-32600, dynamic_msg.clone(), &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["message"], dynamic_msg);
    }

    #[test]
    fn test_error_response_cow_borrowed() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let result = server.error_response(-32600, "Static error message", &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["message"], "Static error message");
    }

    #[test]
    fn test_error_response_preserves_id() {
        let server = test_server();
        let request = mk_request("test", None, 123);
        let result = server.error_response(INVALID_REQUEST, "Bad request", &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["id"], 123);
    }

    // ---- Zero-copy to_vec / from_slice ----

    #[test]
    fn test_jsonrpc_error_response_to_vec_roundtrip() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: PARSE_ERROR,
                message: Cow::Borrowed("Parse error"),
                data: None,
            },
            id: None,
        };
        let vec = serde_json::to_vec(&err).unwrap();
        let parsed: JsonRpcErrorResponse = serde_json::from_slice(&vec).unwrap();
        assert_eq!(parsed.error.code, PARSE_ERROR);
        assert_eq!(parsed.error.message.as_ref(), "Parse error");
    }

    #[test]
    fn test_jsonrpc_response_to_vec_roundtrip() {
        let resp = JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result: serde_json::json!({"nested": {"a": 1}}),
            id: Some(serde_json::json!("abc")),
        };
        let vec = serde_json::to_vec(&resp).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_slice(&vec).unwrap();
        assert_eq!(parsed.result["nested"]["a"], 1);
        assert_eq!(parsed.id, Some(serde_json::json!("abc")));
    }

    // ---- job_queue_error_response other branch ----

    #[test]
    fn test_job_queue_error_response_job_not_complete() {
        let server = test_server();
        let request = mk_request("compute.result", None, 1);
        let err = JobQueueError::JobNotComplete {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn test_job_queue_error_response_job_failed() {
        let server = test_server();
        let request = mk_request("compute.result", None, 1);
        let err = JobQueueError::JobFailed {
            id: uuid::Uuid::nil(),
            error: "GPU OOM".to_string(),
        };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    }

    // ---- Method dispatch: discover_capabilities ----

    #[tokio::test]
    async fn test_method_dispatch_discover_capabilities() {
        let server = test_server();
        let request = mk_request("compute.discover_capabilities", None, 1);
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        let caps = obj["result"]["capabilities"].as_array().expect("array");
        assert!(caps.iter().any(|v| v == "toadstool.health"));
        assert!(caps.iter().any(|v| v == "compute.submit"));
        assert_eq!(obj["result"]["primal"], "toadstool");
    }

    // ---- Edge case: METHOD_NOT_FOUND uses format! (Cow::Owned) ----

    #[tokio::test]
    async fn test_method_not_found_includes_method_name() {
        let server = test_server();
        let request = mk_request("nonexistent.weird_method", None, 1);
        let response = server.handle_jsonrpc_request(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
        assert!(obj["error"]["message"]
            .as_str()
            .unwrap()
            .contains("nonexistent.weird_method"));
    }
}

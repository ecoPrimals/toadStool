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

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use super::graph_types::ExecutionGraph;
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
    pub jsonrpc: String,
    pub result: Value,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub error: JsonRpcError,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// JSON-RPC 2.0 Error Codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

/// Manual JSON-RPC 2.0 Server over Unix Sockets
pub struct ManualJsonRpcServer {
    executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    version: String,
    estimator: ResourceEstimator,
    validator: ResourceValidator,
    optimizer: ResourceOptimizer,
}

impl Clone for ManualJsonRpcServer {
    fn clone(&self) -> Self {
        Self {
            executor: Arc::clone(&self.executor),
            version: self.version.clone(),
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
        }
    }
}

impl ManualJsonRpcServer {
    /// Create new manual JSON-RPC server
    pub fn new(executor: Arc<dyn WorkloadExecutor + Send + Sync>, version: String) -> Self {
        Self {
            executor,
            version,
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
        }
    }

    /// Start server on Unix socket
    ///
    /// Deep debt principle: No hardcoding, Unix socket for multi-instance support
    pub async fn serve(self, socket_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let local_addr = listener.local_addr()?;
        info!("✅ Manual JSON-RPC 2.0 server listening on TCP: {}", local_addr);

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
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Peek at first line to detect format
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await?;

        let (body, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            // HTTP-wrapped request - read remaining headers and body
            let (_headers, body) = self.read_http_request_continuation_tcp(&mut reader).await?;
            (body, true)
        } else {
            // Raw JSON-RPC request - the first line IS the request
            (first_line.trim().to_string(), false)
        };

        // Parse JSON-RPC request
        let response_body = match serde_json::from_str::<JsonRpcRequest>(&body) {
            Ok(request) => {
                // Handle JSON-RPC request
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_string(&response)?
            }
            Err(e) => {
                // Parse error
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: format!("Parse error: {}", e),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_string(&error_response)?
            }
        };

        // Write response in appropriate format
        if is_http {
            self.write_http_response_tcp(&mut writer, &response_body)
                .await?;
        } else {
            // Raw JSON-RPC - just send the JSON followed by newline
            writer.write_all(response_body.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Read HTTP request continuation for TCP (after first line already read)
    async fn read_http_request_continuation_tcp(
        &self,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    ) -> Result<(HashMap<String, String>, String), Box<dyn std::error::Error>> {
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

        // Read body (based on Content-Length)
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;
        let body = String::from_utf8(body)?;

        Ok((headers, body))
    }

    /// Write HTTP response for TCP
    async fn write_http_response_tcp(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::io::AsyncWriteExt;

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );

        writer.write_all(response.as_bytes()).await?;
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Peek at first line to detect format
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await?;

        let (body, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            // HTTP-wrapped request - read remaining headers and body
            let (_headers, body) = self.read_http_request_continuation(&mut reader).await?;
            (body, true)
        } else {
            // Raw JSON-RPC request - the first line IS the request
            (first_line.trim().to_string(), false)
        };

        // Parse JSON-RPC request
        let response_body = match serde_json::from_str::<JsonRpcRequest>(&body) {
            Ok(request) => {
                // Handle JSON-RPC request
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_string(&response)?
            }
            Err(e) => {
                // Parse error
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: format!("Parse error: {}", e),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_string(&error_response)?
            }
        };

        // Write response in appropriate format
        if is_http {
            self.write_http_response(&mut writer, &response_body)
                .await?;
        } else {
            // Raw JSON-RPC - just send the JSON followed by newline
            writer.write_all(response_body.as_bytes()).await?;
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
    ) -> Result<(HashMap<String, String>, String), Box<dyn std::error::Error>> {
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

        // Read body (based on Content-Length)
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;
        let body = String::from_utf8(body)?;

        Ok((headers, body))
    }

    /// Write HTTP response
    async fn write_http_response(
        &self,
        writer: &mut tokio::net::unix::OwnedWriteHalf,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );

        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }

    /// Handle JSON-RPC request
    async fn handle_jsonrpc_request(&self, request: JsonRpcRequest) -> Value {
        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return serde_json::to_value(JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                error: JsonRpcError {
                    code: INVALID_REQUEST,
                    message: "Invalid jsonrpc version".to_string(),
                    data: None,
                },
                id: request.id,
            })
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
        }

        // Route to method handler
        match request.method.as_str() {
            "toadstool.health" => self.handle_health(request).await,
            "toadstool.version" => self.handle_version(request).await,
            "toadstool.query_capabilities" => self.handle_query_capabilities(request).await,
            // Collaborative Intelligence methods
            "resources.estimate" => self.handle_resources_estimate(request).await,
            "resources.validate_availability" => {
                self.handle_resources_validate_availability(request).await
            }
            "resources.suggest_optimizations" => {
                self.handle_resources_suggest_optimizations(request).await
            }
            _ => {
                // Method not found
                serde_json::to_value(JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    },
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
            }
        }
    }

    /// Handle health check
    async fn handle_health(&self, request: JsonRpcRequest) -> Value {
        let result = serde_json::json!({
            "healthy": true,
            "service": "toadstool",
            "version": self.version,
        });

        serde_json::to_value(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result,
            id: request.id,
        })
        .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
    }

    /// Handle version query
    async fn handle_version(&self, request: JsonRpcRequest) -> Value {
        let result = serde_json::json!({
            "version": self.version,
            "protocol": "json-rpc-2.0",
        });

        serde_json::to_value(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result,
            id: request.id,
        })
        .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
    }

    /// Handle capabilities query
    async fn handle_query_capabilities(&self, request: JsonRpcRequest) -> Value {
        match self.executor.query_capabilities().await {
            Ok(caps) => {
                let result = serde_json::to_value(caps)
                    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                serde_json::to_value(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result,
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
            }
            Err(e) => serde_json::to_value(JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                error: JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Failed to query capabilities: {}", e),
                    data: None,
                },
                id: request.id,
            })
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})),
        }
    }

    /// Handle resources.estimate - Estimate resource requirements for a graph
    ///
    /// Request params: { "graph": ExecutionGraph }
    /// Response: ResourceEstimate
    async fn handle_resources_estimate(&self, request: JsonRpcRequest) -> Value {
        // Parse params
        let graph: ExecutionGraph = match request.params {
            Some(params) => {
                match serde_json::from_value(params.get("graph").cloned().unwrap_or(Value::Null)) {
                    Ok(g) => g,
                    Err(e) => {
                        return serde_json::to_value(JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            error: JsonRpcError {
                                code: INVALID_PARAMS,
                                message: format!("Invalid graph parameter: {}", e),
                                data: None,
                            },
                            id: request.id,
                        })
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                    }
                }
            }
            None => {
                return serde_json::to_value(JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: INVALID_PARAMS,
                        message: "Missing 'graph' parameter".to_string(),
                        data: None,
                    },
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
            }
        };

        // Estimate resources
        match self.estimator.estimate(&graph) {
            Ok(estimate) => {
                let result = serde_json::to_value(estimate)
                    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                serde_json::to_value(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result,
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
            }
            Err(e) => serde_json::to_value(JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                error: JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Estimation failed: {}", e),
                    data: None,
                },
                id: request.id,
            })
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})),
        }
    }

    /// Handle resources.validate_availability - Check if system can execute graph
    ///
    /// Request params: { "graph": ExecutionGraph }
    /// Response: AvailabilityResult
    async fn handle_resources_validate_availability(&self, request: JsonRpcRequest) -> Value {
        // Parse params
        let graph: ExecutionGraph = match request.params {
            Some(params) => {
                match serde_json::from_value(params.get("graph").cloned().unwrap_or(Value::Null)) {
                    Ok(g) => g,
                    Err(e) => {
                        return serde_json::to_value(JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            error: JsonRpcError {
                                code: INVALID_PARAMS,
                                message: format!("Invalid graph parameter: {}", e),
                                data: None,
                            },
                            id: request.id,
                        })
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                    }
                }
            }
            None => {
                return serde_json::to_value(JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: INVALID_PARAMS,
                        message: "Missing 'graph' parameter".to_string(),
                        data: None,
                    },
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
            }
        };

        // Validate availability
        match self.validator.validate_availability(&graph).await {
            Ok(result) => {
                let result_value = serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                serde_json::to_value(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: result_value,
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
            }
            Err(e) => serde_json::to_value(JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                error: JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Validation failed: {}", e),
                    data: None,
                },
                id: request.id,
            })
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})),
        }
    }

    /// Handle resources.suggest_optimizations - Suggest optimizations for graph
    ///
    /// Request params: { "graph": ExecutionGraph }
    /// Response: OptimizationSuggestions
    async fn handle_resources_suggest_optimizations(&self, request: JsonRpcRequest) -> Value {
        // Parse params
        let graph: ExecutionGraph = match request.params {
            Some(params) => {
                match serde_json::from_value(params.get("graph").cloned().unwrap_or(Value::Null)) {
                    Ok(g) => g,
                    Err(e) => {
                        return serde_json::to_value(JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            error: JsonRpcError {
                                code: INVALID_PARAMS,
                                message: format!("Invalid graph parameter: {}", e),
                                data: None,
                            },
                            id: request.id,
                        })
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                    }
                }
            }
            None => {
                return serde_json::to_value(JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: INVALID_PARAMS,
                        message: "Missing 'graph' parameter".to_string(),
                        data: None,
                    },
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
            }
        };

        // Suggest optimizations
        match self.optimizer.suggest_optimizations(&graph).await {
            Ok(suggestions) => {
                let result = serde_json::to_value(suggestions)
                    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                serde_json::to_value(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result,
                    id: request.id,
                })
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}))
            }
            Err(e) => serde_json::to_value(JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                error: JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Optimization failed: {}", e),
                    data: None,
                },
                id: request.id,
            })
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            jsonrpc: "2.0".to_string(),
            result: serde_json::json!({"status": "ok"}),
            id: Some(serde_json::json!(1)),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("result"));
    }
}

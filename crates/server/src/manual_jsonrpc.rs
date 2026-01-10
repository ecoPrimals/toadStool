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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{info, warn, error};

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
}

impl ManualJsonRpcServer {
    /// Create new manual JSON-RPC server
    pub fn new(executor: Arc<dyn WorkloadExecutor + Send + Sync>, version: String) -> Self {
        Self { executor, version }
    }
    
    /// Start server on Unix socket
    ///
    /// Deep debt principle: No hardcoding, Unix socket for multi-instance support
    pub async fn serve(self, socket_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting manual JSON-RPC 2.0 server on Unix socket: {:?}", socket_path);
        
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
        
        info!("✅ Manual JSON-RPC 2.0 server listening on: {:?}", socket_path);
        
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
    
    /// Handle a single connection
    async fn handle_connection(&self, stream: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        
        // Read HTTP request
        let (_headers, body) = self.read_http_request(&mut reader).await?;
        
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
        
        // Write HTTP response
        self.write_http_response(&mut writer, &response_body).await?;
        
        Ok(())
    }
    
    /// Read HTTP request (simple HTTP/1.1 parser)
    async fn read_http_request(
        &self,
        reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    ) -> Result<(HashMap<String, String>, String), Box<dyn std::error::Error>> {
        let mut headers = HashMap::new();
        let mut line = String::new();
        
        // Read request line (e.g., "POST / HTTP/1.1")
        reader.read_line(&mut line).await?;
        
        // Read headers
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break; // End of headers
            }
            
            // Parse header (Name: Value)
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(
                    name.trim().to_lowercase(),
                    value.trim().to_string(),
                );
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
            }).unwrap();
        }
        
        // Route to method handler
        match request.method.as_str() {
            "toadstool.health" => self.handle_health(request).await,
            "toadstool.version" => self.handle_version(request).await,
            "toadstool.query_capabilities" => self.handle_query_capabilities(request).await,
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
                }).unwrap()
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
        }).unwrap()
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
        }).unwrap()
    }
    
    /// Handle capabilities query
    async fn handle_query_capabilities(&self, request: JsonRpcRequest) -> Value {
        match self.executor.query_capabilities().await {
            Ok(caps) => {
                let result = serde_json::to_value(caps).unwrap();
                serde_json::to_value(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result,
                    id: request.id,
                }).unwrap()
            }
            Err(e) => {
                serde_json::to_value(JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    error: JsonRpcError {
                        code: INTERNAL_ERROR,
                        message: format!("Failed to query capabilities: {}", e),
                        data: None,
                    },
                    id: request.id,
                }).unwrap()
            }
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


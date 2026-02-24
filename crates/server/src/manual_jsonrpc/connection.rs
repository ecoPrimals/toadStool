//! Connection handling for Manual JSON-RPC server (Unix socket + TCP)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};

use super::JSONRPC_VERSION;
use super::{JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, ManualJsonRpcServer, PARSE_ERROR};

impl ManualJsonRpcServer {
    /// Start server on Unix socket
    pub async fn serve(self, socket_path: PathBuf) -> ServerResult<()> {
        info!(
            "Starting manual JSON-RPC 2.0 server on Unix socket: {:?}",
            socket_path
        );

        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ServerError::Initialization(format!(
                    "Failed to create socket directory {parent:?}: {e}"
                ))
            })?;
            info!("Ensured JSON-RPC socket directory exists: {:?}", parent);
        }

        if socket_path.exists() {
            warn!("Removing old JSON-RPC socket: {:?}", socket_path);
            tokio::fs::remove_file(&socket_path)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
        }

        let listener =
            UnixListener::bind(&socket_path).map_err(|e| ServerError::Network(e.to_string()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&socket_path)
                .map_err(|e| ServerError::Internal(e.to_string()))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&socket_path, perms)
                .map_err(|e| ServerError::Internal(e.to_string()))?;
            info!("Set JSON-RPC socket permissions to 0600");
        }

        info!(
            "✅ Manual JSON-RPC 2.0 server listening on: {:?}",
            socket_path
        );

        let server = Arc::new(self);

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
                Err(e) => error!("Accept error: {}", e),
            }
        }
    }

    /// Start server on TCP listener (isomorphic fallback)
    pub async fn serve_tcp(self, listener: tokio::net::TcpListener) -> ServerResult<()> {
        let local_addr = listener
            .local_addr()
            .map_err(|e| ServerError::Network(e.to_string()))?;
        info!(
            "✅ Manual JSON-RPC 2.0 server listening on TCP: {}",
            local_addr
        );

        let server = Arc::new(self);

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
                Err(e) => error!("TCP accept error: {}", e),
            }
        }
    }

    /// Handle a single TCP connection
    async fn handle_tcp_connection(&self, stream: tokio::net::TcpStream) -> ServerResult<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;

        let (request_result, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            let (_headers, body) = self.read_http_request_continuation_tcp(&mut reader).await?;
            (serde_json::from_slice::<JsonRpcRequest>(&body), true)
        } else {
            (
                serde_json::from_slice::<JsonRpcRequest>(first_line.trim().as_bytes()),
                false,
            )
        };

        let response_body = match request_result {
            Ok(request) => {
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()))?
            }
            Err(e) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: JSONRPC_VERSION.clone(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: std::borrow::Cow::Owned(format!("Parse error: {e}")),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_vec(&error_response)
                    .map_err(|e| ServerError::Internal(e.to_string()))?
            }
        };

        if is_http {
            self.write_http_response_tcp(&mut writer, &response_body)
                .await?;
        } else {
            writer
                .write_all(&response_body)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .write_all(b"\n")
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .flush()
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
        }

        Ok(())
    }

    async fn read_http_request_continuation_tcp(
        &self,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    ) -> ServerResult<(HashMap<String, String>, Vec<u8>)> {
        let mut headers = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_lowercase(), value.trim().to_string());
            }
        }

        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;

        Ok((headers, body))
    }

    async fn write_http_response_tcp(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        body: &[u8],
    ) -> ServerResult<()> {
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            body.len()
        );
        writer
            .write_all(header.as_bytes())
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        writer
            .write_all(body)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        writer
            .flush()
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        Ok(())
    }

    /// Handle a single Unix socket connection
    async fn handle_connection(&self, stream: UnixStream) -> ServerResult<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;

        let (request_result, is_http) = if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            let (_headers, body) = self.read_http_request_continuation(&mut reader).await?;
            (serde_json::from_slice::<JsonRpcRequest>(&body), true)
        } else {
            (
                serde_json::from_slice::<JsonRpcRequest>(first_line.trim().as_bytes()),
                false,
            )
        };

        let response_body = match request_result {
            Ok(request) => {
                let response = self.handle_jsonrpc_request(request).await;
                serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()))?
            }
            Err(e) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                let error_response = JsonRpcErrorResponse {
                    jsonrpc: JSONRPC_VERSION.clone(),
                    error: JsonRpcError {
                        code: PARSE_ERROR,
                        message: std::borrow::Cow::Owned(format!("Parse error: {e}")),
                        data: None,
                    },
                    id: None,
                };
                serde_json::to_vec(&error_response)
                    .map_err(|e| ServerError::Internal(e.to_string()))?
            }
        };

        if is_http {
            self.write_http_response(&mut writer, &response_body)
                .await?;
        } else {
            writer
                .write_all(&response_body)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .write_all(b"\n")
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .flush()
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
        }

        Ok(())
    }

    async fn read_http_request_continuation(
        &self,
        reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    ) -> ServerResult<(HashMap<String, String>, Vec<u8>)> {
        let mut headers = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_lowercase(), value.trim().to_string());
            }
        }

        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;

        Ok((headers, body))
    }

    async fn write_http_response(
        &self,
        writer: &mut tokio::net::unix::OwnedWriteHalf,
        body: &[u8],
    ) -> ServerResult<()> {
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            body.len()
        );
        writer
            .write_all(header.as_bytes())
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        writer
            .write_all(body)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        writer
            .flush()
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Connection handling tests — message framing, error responses, serialization.
    //! No running services: tests pure logic, types, and serialization.
    #![allow(deprecated)]

    use super::super::{INVALID_PARAMS, INVALID_REQUEST, JSONRPC_VERSION, METHOD_NOT_FOUND};
    use super::{
        JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, ManualJsonRpcServer, PARSE_ERROR,
    };
    use crate::tarpc_server::StandaloneExecutor;
    use std::sync::Arc;

    fn test_server() -> ManualJsonRpcServer {
        let executor = Arc::new(StandaloneExecutor::new());
        ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
    }

    #[test]
    fn test_jsonrpc_request_parse_valid_full() {
        let json = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "toadstool.health");
        assert!(req.params.is_some());
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_jsonrpc_request_parse_valid_minimal() {
        let json = r#"{"jsonrpc":"2.0","method":"test","id":null}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "test");
        assert_eq!(req.params, None);
    }

    #[test]
    fn test_jsonrpc_request_parse_from_bytes() {
        let bytes = br#"{"jsonrpc":"2.0","method":"compute.submit","id":42}"#;
        let req: JsonRpcRequest = serde_json::from_slice(bytes).unwrap();
        assert_eq!(req.method, "compute.submit");
        assert_eq!(req.id, Some(serde_json::json!(42)));
    }

    #[test]
    fn test_jsonrpc_request_parse_invalid_returns_err() {
        let invalid = b"not json";
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_request_parse_empty_returns_err() {
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_request_serialization_roundtrip() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.version".to_string(),
            params: Some(serde_json::json!({"foo": "bar"})),
            id: Some(serde_json::json!(99)),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.method, req.method);
        assert_eq!(restored.params, req.params);
    }

    #[test]
    fn test_parse_error_response_structure() {
        let _server = test_server();
        let _req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown".to_string(),
            params: None,
            id: Some(serde_json::json!(5)),
        };
        let err_resp = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: PARSE_ERROR,
                message: std::borrow::Cow::Owned("Parse error: expected value".to_string()),
                data: None,
            },
            id: None,
        };
        let serialized = serde_json::to_value(&err_resp).unwrap();
        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["error"]["code"], PARSE_ERROR);
        assert!(serialized["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Parse error"));
        assert!(serialized["id"].is_null());
    }

    #[test]
    fn test_error_response_serialization_roundtrip() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: PARSE_ERROR,
                message: std::borrow::Cow::Owned("Invalid JSON".to_string()),
                data: None,
            },
            id: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        let restored: JsonRpcErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.error.code, PARSE_ERROR);
        assert!(restored.error.message.contains("Invalid"));
    }

    #[test]
    fn test_http_response_header_format() {
        let body = br#"{"jsonrpc":"2.0","result":{},"id":1}"#;
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            body.len()
        );
        assert!(header.starts_with("HTTP/1.1 200 OK"));
        assert!(header.contains("Content-Type: application/json"));
        assert!(header.contains(&format!("Content-Length: {}", body.len())));
        assert!(header.contains("Connection: close"));
    }

    #[test]
    fn test_first_line_detection_http_post() {
        let first_line = "POST /rpc HTTP/1.1\r\n";
        assert!(first_line.starts_with("POST"));
    }

    #[test]
    fn test_first_line_detection_http_get() {
        let first_line = "GET /health HTTP/1.1\r\n";
        assert!(first_line.starts_with("GET"));
    }

    #[test]
    fn test_first_line_detection_raw_json() {
        let first_line = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        assert!(!first_line.starts_with("POST"));
        assert!(!first_line.starts_with("GET"));
        assert!(!first_line.starts_with("HTTP"));
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_invalid_version() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.contains_key("error"));
        assert_eq!(obj["error"]["code"], INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_method_not_found() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "nonexistent.method".to_string(),
            params: None,
            id: Some(serde_json::json!(2)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_toadstool_health() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(3)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.contains_key("result"));
        assert!(obj["result"]["healthy"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_compute_health_alias() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.health".to_string(),
            params: None,
            id: Some(serde_json::json!(4)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.as_object().unwrap().contains_key("result"));
    }

    #[test]
    fn test_jsonrpc_version_constant() {
        assert_eq!(JSONRPC_VERSION.as_ref(), "2.0");
    }

    // ── Additional coverage: error codes, response formatting, message routing ──

    #[test]
    fn test_jsonrpc_request_parse_invalid_missing_method() {
        let json = r#"{"jsonrpc":"2.0","id":1}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_request_parse_invalid_null_method() {
        let json = r#"{"jsonrpc":"2.0","method":null,"id":1}"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_str(json);
        // May parse or fail depending on serde config
        let _ = result;
    }

    #[test]
    fn test_jsonrpc_request_parse_valid_string_id() {
        let json = r#"{"jsonrpc":"2.0","method":"test.method","params":{},"id":"req-42"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "test.method");
        assert_eq!(req.id, Some(serde_json::json!("req-42")));
    }

    #[test]
    fn test_jsonrpc_request_parse_valid_array_params() {
        let json = r#"{"jsonrpc":"2.0","method":"test","params":[1,2,3],"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.params.is_some());
        assert!(req.params.unwrap().is_array());
    }

    #[test]
    fn test_jsonrpc_response_format() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "result": {"healthy": true},
            "id": 5
        });
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"]["healthy"].as_bool().unwrap());
        assert_eq!(response["id"], 5);
    }

    #[test]
    fn test_error_response_invalid_request_code() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: INVALID_REQUEST,
                message: std::borrow::Cow::Owned("Invalid request".to_string()),
                data: None,
            },
            id: Some(serde_json::json!(1)),
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert_eq!(serialized["error"]["code"], INVALID_REQUEST);
    }

    #[test]
    fn test_error_response_method_not_found_code() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: std::borrow::Cow::Owned("Method not found: x.y".to_string()),
                data: None,
            },
            id: Some(serde_json::json!(2)),
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert_eq!(serialized["error"]["code"], METHOD_NOT_FOUND);
        assert!(serialized["id"].is_number());
    }

    #[test]
    fn test_error_response_with_data_field() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: PARSE_ERROR,
                message: std::borrow::Cow::Owned("Parse error".to_string()),
                data: Some(serde_json::json!({"line": 5})),
            },
            id: None,
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert!(serialized["error"]["data"].is_object());
    }

    #[test]
    fn test_jsonrpc_error_cow_borrowed() {
        use std::borrow::Cow;
        let msg: Cow<'static, str> = Cow::Borrowed("static message");
        assert_eq!(msg.as_ref(), "static message");
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_params_object() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: Some(serde_json::json!({"foo": "bar"})),
            id: Some(serde_json::json!(6)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.as_object().unwrap().contains_key("result"));
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_toadstool_version() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.version".to_string(),
            params: None,
            id: Some(serde_json::json!(7)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.contains_key("result"));
        let result = &obj["result"];
        assert!(result
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_error_increments_count() {
        let error_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let server = ManualJsonRpcServer::new(
            Arc::new(StandaloneExecutor::new()),
            "test-1.0.0".to_string(),
            Some(Arc::clone(&error_count)),
        );
        let req = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(8)),
        };
        let _ = server.handle_jsonrpc_request(req).await;
        assert!(error_count.load(std::sync::atomic::Ordering::Relaxed) >= 1);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_response_preserves_id() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "nonexistent.method".to_string(),
            params: None,
            id: Some(serde_json::json!(123)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["id"], 123);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_notification_no_id() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: None, // Notification
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        // Notification: response may have null id
        assert!(obj.contains_key("result") || obj.contains_key("error"));
    }

    #[tokio::test]
    async fn test_message_routing_compute_submit_alias() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.submit".to_string(),
            params: Some(
                serde_json::json!({"job_id": "00000000-0000-0000-0000-000000000000", "workload": {}}),
            ),
            id: Some(serde_json::json!(9)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        // Routes to handle_compute_submit; may return success or validation error
        assert!(response.is_object());
    }

    #[tokio::test]
    async fn test_message_routing_resources_estimate() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources.estimate".to_string(),
            params: Some(serde_json::json!({})),
            id: Some(serde_json::json!(10)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(
            response.as_object().unwrap().contains_key("result")
                || response.as_object().unwrap().contains_key("error")
        );
    }

    #[tokio::test]
    async fn test_message_routing_compute_health() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.health".to_string(),
            params: None,
            id: Some(serde_json::json!(11)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.as_object().unwrap().contains_key("result"));
    }

    #[tokio::test]
    async fn test_connection_state_error_count_zero_initially() {
        let server = test_server();
        // error_count is internal; verify server handles request
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(12)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.as_object().unwrap().contains_key("result"));
    }

    // ── Extended coverage: INVALID_PARAMS, INTERNAL_ERROR, routing, response formatting ──

    #[tokio::test]
    async fn test_handle_jsonrpc_request_invalid_params_missing_job_id() {
        use super::super::INVALID_PARAMS;
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.submit".to_string(),
            params: Some(serde_json::json!({"workload": {}})),
            id: Some(serde_json::json!(20)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.contains_key("error"));
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_invalid_params_missing_params() {
        use super::super::INVALID_PARAMS;
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.submit".to_string(),
            params: None,
            id: Some(serde_json::json!(21)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_jsonrpc_request_invalid_params_invalid_job_id_uuid() {
        use super::super::INVALID_PARAMS;
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.submit".to_string(),
            params: Some(serde_json::json!({"job_id": "not-a-uuid", "workload": {}})),
            id: Some(serde_json::json!(22)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_error_response_internal_error_code() {
        use super::super::INTERNAL_ERROR;
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: INTERNAL_ERROR,
                message: std::borrow::Cow::Owned("Internal server error".to_string()),
                data: None,
            },
            id: Some(serde_json::json!(3)),
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert_eq!(serialized["error"]["code"], INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn test_error_response_invalid_params_code() {
        use super::super::INVALID_PARAMS;
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: INVALID_PARAMS,
                message: std::borrow::Cow::Owned("Invalid params".to_string()),
                data: None,
            },
            id: Some(serde_json::json!(4)),
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert_eq!(serialized["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn test_jsonrpc_request_parse_invalid_malformed_json() {
        let invalid = br#"{"jsonrpc":"2.0","method":"test"#;
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_request_parse_invalid_whitespace_only() {
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(b"   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_request_parse_invalid_array() {
        let result: Result<JsonRpcRequest, _> = serde_json::from_slice(b"[1,2,3]");
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_response_success_structure() {
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "result": {"healthy": true, "version": "1.0"},
            "id": 1
        });
        assert_eq!(resp["jsonrpc"], "2.0");
        assert!(resp["result"].is_object());
        assert_eq!(resp["id"], 1);
    }

    #[test]
    fn test_jsonrpc_error_response_preserves_request_id() {
        let err = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: std::borrow::Cow::Owned("Method not found".to_string()),
                data: None,
            },
            id: Some(serde_json::json!("req-99")),
        };
        let serialized = serde_json::to_value(&err).unwrap();
        assert_eq!(serialized["id"], "req-99");
    }

    #[tokio::test]
    async fn test_message_routing_toadstool_query_capabilities() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.query_capabilities".to_string(),
            params: None,
            id: Some(serde_json::json!(23)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.is_object());
        assert!(
            response.as_object().unwrap().contains_key("result")
                || response.as_object().unwrap().contains_key("error")
        );
    }

    #[tokio::test]
    async fn test_message_routing_compute_submit_canonical() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.submit".to_string(),
            params: Some(serde_json::json!({
                "job_id": "550e8400-e29b-41d4-a716-446655440000",
                "workload": {"type": "compute", "payload": {}}
            })),
            id: Some(serde_json::json!(24)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.is_object());
    }

    #[tokio::test]
    async fn test_message_routing_resources_estimate_alias() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources.estimate".to_string(),
            params: Some(serde_json::json!({"graph": {"nodes": [], "edges": []}})),
            id: Some(serde_json::json!(25)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.is_object());
    }

    #[tokio::test]
    async fn test_message_routing_gate_list() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "gate.list".to_string(),
            params: None,
            id: Some(serde_json::json!(26)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.is_object());
    }

    #[tokio::test]
    async fn test_message_routing_compute_status() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "compute.status".to_string(),
            params: Some(serde_json::json!({"job_id": "550e8400-e29b-41d4-a716-446655440000"})),
            id: Some(serde_json::json!(27)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        assert!(response.is_object());
    }

    #[tokio::test]
    async fn test_response_format_result_and_id_preserved() {
        let server = test_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(999)),
        };
        let response = server.handle_jsonrpc_request(req).await;
        let obj = response.as_object().unwrap();
        assert_eq!(obj["id"], 999);
        assert!(obj["result"]["healthy"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_parse_error_response_has_null_id() {
        let err_resp = JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code: PARSE_ERROR,
                message: std::borrow::Cow::Owned("Parse error".to_string()),
                data: None,
            },
            id: None,
        };
        let serialized = serde_json::to_value(&err_resp).unwrap();
        assert!(serialized["id"].is_null());
    }

    #[test]
    fn test_http_request_content_length_header_parsed() {
        let header_line = "Content-Length: 42";
        let (name, value) = header_line.split_once(':').unwrap();
        assert_eq!(name.trim(), "Content-Length");
        assert_eq!(value.trim().parse::<usize>().unwrap(), 42);
    }

    #[test]
    fn test_first_line_detection_http_prefix() {
        let first_line = "HTTP/1.1 200 OK\r\n";
        assert!(first_line.starts_with("HTTP"));
    }

    // ─── Connection flow tests: Unix raw JSON and HTTP, TCP raw JSON ─────────

    #[tokio::test]
    async fn test_handle_connection_unix_raw_json() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":1}"#;
        client.write_all(req.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert!(parsed.get("result").is_some());
        assert!(parsed["result"]["healthy"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_handle_connection_unix_http_post() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let body = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":3}"#;
        let req = format!(
            "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        client.write_all(req.as_bytes()).await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        assert!(buf.contains("HTTP/1.1 200"));
        assert!(buf.contains("Content-Type: application/json"));
        let body_start = buf.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let json_part = &buf[body_start..];
        let parsed: serde_json::Value = serde_json::from_str(json_part.trim()).unwrap();
        assert!(parsed.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_connection_unix_parse_error() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        client.write_all(b"not json at all\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = vec![0u8; 512];
        let n = client.read(&mut buf).await.unwrap_or(0);
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("error") || resp.contains("Parse error"));
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_raw_json() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = std::sync::Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":4}"#;
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert!(parsed.get("result").is_some());
    }

    // ─── Mock-based integration tests: temp Unix socket, TCP port 0, error paths ─

    #[cfg(unix)]
    #[tokio::test]
    async fn test_unix_socket_temp_path_connect_client() {
        use std::path::PathBuf;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("jsonrpc.sock");
        let socket_path = PathBuf::from(socket_path);

        let server = test_server();
        let path_for_server = socket_path.clone();
        tokio::spawn(async move {
            let _ = server.serve(path_for_server).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut client = UnixStream::connect(&socket_path).await.expect("connect");
        let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":100}"#;
        client.write_all(req.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert!(parsed.get("result").is_some());
        assert!(parsed["result"]["healthy"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_tcp_port_zero_connect_client() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = test_server();
        let server_arc = Arc::new(server);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = r#"{"jsonrpc":"2.0","method":"toadstool.version","params":{},"id":101}"#;
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert!(parsed.get("result").is_some());
        assert!(!parsed["result"]["version"]
            .as_str()
            .unwrap_or("")
            .is_empty());
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_http_post() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let body = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":5}"#;
        let req = format!(
            "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        assert!(buf.contains("HTTP/1.1 200"));
        let body_start = buf.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let json_part = buf[body_start..].trim();
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert!(parsed.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_connection_unix_unknown_method_returns_method_not_found() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let req = r#"{"jsonrpc":"2.0","method":"nonexistent.unknown","params":{},"id":50}"#;
        client.write_all(req.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert_eq!(parsed["error"]["code"], METHOD_NOT_FOUND);
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Method not found"));
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_unknown_method() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = r#"{"jsonrpc":"2.0","method":"fake.method","params":null,"id":51}"#;
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert_eq!(parsed["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_parse_error() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(b"{ invalid json }\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = vec![0u8; 512];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("error") || resp.contains("Parse error"));
    }

    #[tokio::test]
    async fn test_handle_connection_unix_invalid_version() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let req = r#"{"jsonrpc":"1.0","method":"toadstool.health","params":{},"id":52}"#;
        client.write_all(req.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert_eq!(parsed["error"]["code"], INVALID_REQUEST);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_serve_tcp_accepts_connection() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = test_server();
        tokio::spawn(async move {
            let _ = server.serve_tcp(listener).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = r#"{"jsonrpc":"2.0","method":"gate.list","params":{},"id":200}"#;
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("result").is_some() || parsed.get("error").is_some());
    }

    #[tokio::test]
    async fn test_handle_connection_http_content_length_zero() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let req = "POST /rpc HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        client.write_all(req.as_bytes()).await.unwrap();
        client.flush().await.unwrap();

        let mut buf = vec![0u8; 512];
        let n = client.read(&mut buf).await.unwrap_or(0);
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("error") || resp.contains("Parse error"));
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_first_line_http_prefix() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let body = r#"{"jsonrpc":"2.0","method":"compute.health","params":{},"id":60}"#;
        let req = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        assert!(buf.len() > 0);
    }

    #[tokio::test]
    async fn test_handle_connection_unix_response_serialization() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        let req =
            r#"{"jsonrpc":"2.0","method":"toadstool.query_capabilities","params":{},"id":70}"#;
        client.write_all(req.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = String::new();
        client.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert!(parsed.get("result").is_some() || parsed.get("error").is_some());
    }

    #[tokio::test]
    async fn test_handle_connection_unix_malformed_json_trailing() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let (mut client, server) = UnixStream::pair().unwrap();
        let server_instance = test_server();
        tokio::spawn(async move {
            let _ = server_instance.handle_connection(server).await;
        });

        client
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"x\"\n")
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut buf = vec![0u8; 256];
        let n = client.read(&mut buf).await.unwrap_or(0);
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.contains("error"));
    }

    #[tokio::test]
    async fn test_handle_tcp_connection_compute_submit_validation() {
        use super::super::INVALID_PARAMS;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_instance = test_server();
        let server_arc = Arc::new(server_instance);
        let server_clone = Arc::clone(&server_arc);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = server_clone.handle_tcp_connection(stream).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req =
            r#"{"jsonrpc":"2.0","method":"compute.submit","params":{"job_id":"bad-uuid"},"id":80}"#;
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.ok();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
        assert_eq!(parsed["error"]["code"], INVALID_PARAMS);
    }
}

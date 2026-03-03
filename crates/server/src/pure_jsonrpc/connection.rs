// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection handling for Pure JSON-RPC server (Unix socket + TCP)
//!
//! Generic over JsonRpcHandler. Parses requests from owned bytes so that
//! JsonRpcRequest's Cow<'a, str> can borrow from the slice during deserialization.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::types::JsonRpcError;
use crate::pure_jsonrpc::{JsonRpcHandler, JsonRpcRequest, JsonRpcResponse};

/// Serve JSON-RPC on a Unix socket.
///
/// Accepts connections, parses JSON-RPC requests (raw JSON or HTTP/JSON hybrid),
/// dispatches to the handler, and writes responses.
///
/// # Errors
///
/// Returns [`ServerError`] if directory creation, socket bind, or permission setting fails.
pub async fn serve_unix(handler: Arc<JsonRpcHandler>, socket_path: PathBuf) -> ServerResult<()> {
    info!(
        "Starting pure JSON-RPC 2.0 server on Unix socket: {:?}",
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
        "✅ Pure JSON-RPC 2.0 server listening on: {:?}",
        socket_path
    );

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_unix_connection(handler, stream).await {
                        error!("Unix connection error: {}", e);
                    }
                });
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

/// Serve JSON-RPC on a TCP listener (isomorphic fallback).
///
/// # Errors
///
/// Returns [`ServerError`] if getting local address fails.
pub async fn serve_tcp(handler: Arc<JsonRpcHandler>, listener: TcpListener) -> ServerResult<()> {
    let local_addr = listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;
    info!(
        "✅ Pure JSON-RPC 2.0 server listening on TCP: {}",
        local_addr
    );

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(handler, stream).await {
                        error!("TCP connection error: {}", e);
                    }
                });
            }
            Err(e) => error!("TCP accept error: {}", e),
        }
    }
}

async fn handle_unix_connection(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
) -> ServerResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    let (body, is_http) = if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        let (_headers, body) = read_http_request_continuation_unix(&mut reader).await?;
        (body, true)
    } else {
        (first_line.trim().as_bytes().to_vec(), false)
    };

    let response_body = process_request(&handler, &body).await?;

    if is_http {
        write_http_response_unix(&mut writer, &response_body).await?;
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

async fn handle_tcp_connection(
    handler: Arc<JsonRpcHandler>,
    stream: TcpStream,
) -> ServerResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    let (body, is_http) = if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        let (_headers, body) = read_http_request_continuation_tcp(&mut reader).await?;
        (body, true)
    } else {
        (first_line.trim().as_bytes().to_vec(), false)
    };

    let response_body = process_request(&handler, &body).await?;

    if is_http {
        write_http_response_tcp(&mut writer, &response_body).await?;
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

/// Parse request from body bytes, dispatch to handler, return serialized response.
///
/// Uses owned body so JsonRpcRequest can borrow from it via `serde_json::from_slice`.
#[cfg_attr(test, allow(dead_code))]
pub(crate) async fn process_request(
    handler: &JsonRpcHandler,
    body: &[u8],
) -> ServerResult<Vec<u8>> {
    let request: JsonRpcRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            let response = JsonRpcResponse {
                jsonrpc: std::borrow::Cow::Borrowed(toadstool_common::constants::jsonrpc::VERSION),
                result: None,
                error: Some(JsonRpcError::parse_error(format!("Parse error: {e}"))),
                id: serde_json::Value::Null,
            };
            return serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()));
        }
    };

    let response = handler.handle_request(&request).await;

    serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()))
}

async fn read_http_request_continuation_unix(
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
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    Ok((headers, body))
}

async fn read_http_request_continuation_tcp(
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
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    Ok((headers, body))
}

async fn write_http_response_unix(
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

async fn write_http_response_tcp(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tarpc_server::StandaloneExecutor;

    fn test_handler() -> JsonRpcHandler {
        let executor = Arc::new(StandaloneExecutor::new());
        JsonRpcHandler::new(executor, "test-conn-1.0.0".to_string(), None)
    }

    #[tokio::test]
    async fn test_process_request_valid_health() {
        let handler = test_handler();
        let body = br#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
        let result = process_request(&handler, body).await;
        assert!(result.is_ok());
        let bytes = result.expect("ok");
        let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(resp["result"]["healthy"].as_bool().is_some());
        assert_eq!(resp["result"]["version"], "test-conn-1.0.0");
    }

    #[tokio::test]
    async fn test_process_request_invalid_json() {
        let handler = test_handler();
        let body = b"this is not json";
        let result = process_request(&handler, body).await;
        assert!(result.is_ok());
        let bytes = result.expect("ok");
        let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(resp.get("error").is_some());
        assert_eq!(resp["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn test_process_request_empty_body() {
        let handler = test_handler();
        let body = b"";
        let result = process_request(&handler, body).await;
        assert!(result.is_ok());
        let bytes = result.expect("ok");
        let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(resp.get("error").is_some());
        assert_eq!(resp["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn test_process_request_method_not_found() {
        let handler = test_handler();
        let body = br#"{"jsonrpc":"2.0","method":"nonexistent.method","id":42}"#;
        let result = process_request(&handler, body).await;
        assert!(result.is_ok());
        let bytes = result.expect("ok");
        let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(resp.get("error").is_some());
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_serve_tcp_accepts_raw_json() {
        let handler = Arc::new(test_handler());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server_handler = Arc::clone(&handler);
        let _server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let handler = server_handler;
            handle_tcp_connection(handler, stream).await.expect("ok");
        });

        let mut client = TcpStream::connect(addr).await.expect("connect");
        let request = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n";
        client.write_all(request).await.expect("write");
        client.shutdown().await.ok();

        let mut buf = Vec::new();
        client.read_to_end(&mut buf).await.expect("read");
        let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
        assert!(resp["result"]["healthy"].as_bool().is_some());
    }

    #[tokio::test]
    async fn test_serve_tcp_accepts_http_post() {
        let handler = Arc::new(test_handler());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server_handler = Arc::clone(&handler);
        let _server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_tcp_connection(server_handler, stream)
                .await
                .expect("ok");
        });

        let body = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
        let http = format!(
            "POST /jsonrpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut client = TcpStream::connect(addr).await.expect("connect");
        client.write_all(http.as_bytes()).await.expect("write");
        client.shutdown().await.ok();

        let mut buf = Vec::new();
        client.read_to_end(&mut buf).await.expect("read");
        let response = String::from_utf8_lossy(&buf);
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("test-conn-1.0.0"));
    }
}

//! Connection handling for Manual JSON-RPC server (Unix socket + TCP)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};

use super::{
    JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, ManualJsonRpcServer, JSONRPC_VERSION,
    PARSE_ERROR,
};

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
mod tests;
#[cfg(test)]
mod tests_extended;

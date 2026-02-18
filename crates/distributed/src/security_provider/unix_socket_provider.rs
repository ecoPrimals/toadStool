//! Unix Socket Security Provider
//!
//! Communicates with a remote security provider over Unix domain sockets.
//! This is the preferred transport for inter-primal IPC:
//!
//! - **Pure Rust**: No TLS/HTTP dependencies (ecoBin compliant)
//! - **Fast**: Direct kernel IPC, minimal overhead
//! - **Secure**: File-system permissions for access control
//! - **Local**: Ideal for primals on same machine
//!
//! ## Protocol
//!
//! Uses JSON-RPC 2.0 over Unix domain sockets:
//! - Request: `{"jsonrpc":"2.0","method":"security.encrypt","params":{...},"id":1}`
//! - Response: `{"jsonrpc":"2.0","result":{...},"id":1}`
//!
//! ## Security
//!
//! - Socket permissions enforced by filesystem
//! - Each request is independent (stateless)
//! - Connection timeout prevents resource exhaustion

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::provider::*;
use super::types::*;
use toadstool_common::constants::timeouts;

/// Unix socket security provider
///
/// Forwards security operations to a remote provider over Unix domain sockets.
pub struct UnixSocketSecurityProvider {
    /// Path to the Unix socket
    socket_path: PathBuf,
    /// Request ID counter (atomic for thread safety)
    request_id: AtomicU64,
    /// Connection timeout in seconds
    timeout_secs: u64,
}

impl UnixSocketSecurityProvider {
    /// Create a new Unix socket provider
    pub fn new(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            request_id: AtomicU64::new(1),
            timeout_secs: timeouts::DEFAULT_REQUEST_TIMEOUT.as_secs(),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(socket_path: impl AsRef<Path>, timeout_secs: u64) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            request_id: AtomicU64::new(1),
            timeout_secs,
        }
    }

    /// Check if the socket exists
    pub fn socket_exists(&self) -> bool {
        self.socket_path.exists()
    }

    /// Get next request ID
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Send JSON-RPC request and receive response
    async fn send_request<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: P,
    ) -> ToadStoolResult<R> {
        // Connect to socket with timeout
        let connect_future = UnixStream::connect(&self.socket_path);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            connect_future,
        )
        .await
        .map_err(|_| {
            ToadStoolError::timeout(format!(
                "Connection to {} timed out after {}s",
                self.socket_path.display(),
                self.timeout_secs
            ))
        })?
        .map_err(|e| {
            ToadStoolError::network(format!(
                "Failed to connect to {}: {}",
                self.socket_path.display(),
                e
            ))
        })?;

        let (reader, mut writer) = stream.into_split();

        // Build JSON-RPC request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id(),
        };

        // Serialize and send
        let request_json = serde_json::to_string(&request)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize request: {}", e)))?;

        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send request: {}", e)))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send newline: {}", e)))?;
        writer
            .flush()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to flush: {}", e)))?;

        // Read response with timeout
        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();

        tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            buf_reader.read_line(&mut response_line),
        )
        .await
        .map_err(|_| {
            ToadStoolError::timeout(format!(
                "Response from {} timed out after {}s",
                self.socket_path.display(),
                self.timeout_secs
            ))
        })?
        .map_err(|e| ToadStoolError::network(format!("Failed to read response: {}", e)))?;

        // Parse response
        let response: JsonRpcResponse<R> = serde_json::from_str(&response_line).map_err(|e| {
            ToadStoolError::runtime(format!(
                "Failed to parse response: {} (raw: {})",
                e,
                response_line.trim()
            ))
        })?;

        // Check for error
        if let Some(error) = response.error {
            return Err(ToadStoolError::runtime(format!(
                "Remote error ({}): {}",
                error.code, error.message
            )));
        }

        response.result.ok_or_else(|| {
            ToadStoolError::runtime("Response contained neither result nor error".to_string())
        })
    }
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest<P> {
    jsonrpc: String,
    method: String,
    params: P,
    id: u64,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<R> {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<R>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: u64,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// Request/Response types for security operations
#[derive(Debug, Serialize)]
struct EncryptRequest<'a> {
    data: &'a [u8],
    options: Option<&'a EncryptionOptions>,
}

#[derive(Debug, Serialize)]
struct DecryptRequest<'a> {
    ciphertext: &'a [u8],
    metadata: &'a EncryptionMetadata,
}

#[derive(Debug, Serialize)]
struct SignRequest<'a> {
    data: &'a [u8],
    options: Option<&'a SigningOptions>,
}

#[derive(Debug, Serialize)]
struct VerifyRequest<'a> {
    data: &'a [u8],
    signature: &'a [u8],
    public_key_id: &'a str,
}

#[derive(Debug, Serialize)]
struct RevokeRequest<'a> {
    permission_id: &'a uuid::Uuid,
    reason: &'a str,
}

#[async_trait]
impl SecurityProvider for UnixSocketSecurityProvider {
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>> {
        self.send_request("security.capabilities", ()).await
    }

    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata> {
        self.send_request("security.metadata", ()).await
    }

    async fn encrypt(
        &self,
        data: &[u8],
        options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult> {
        self.send_request(
            "security.encrypt",
            EncryptRequest {
                data,
                options: options.as_ref(),
            },
        )
        .await
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        self.send_request(
            "security.decrypt",
            DecryptRequest {
                ciphertext,
                metadata,
            },
        )
        .await
    }

    async fn sign(
        &self,
        data: &[u8],
        options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        self.send_request(
            "security.sign",
            SignRequest {
                data,
                options: options.as_ref(),
            },
        )
        .await
    }

    async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        self.send_request(
            "security.verify",
            VerifyRequest {
                data,
                signature,
                public_key_id,
            },
        )
        .await
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        self.send_request("security.createPermission", request)
            .await
    }

    async fn validate_permission(
        &self,
        permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        self.send_request("security.validatePermission", permission)
            .await
    }

    async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        self.send_request(
            "security.revokePermission",
            RevokeRequest {
                permission_id,
                reason,
            },
        )
        .await
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        self.send_request("security.healthCheck", ()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
        assert_eq!(provider.socket_path, PathBuf::from("/tmp/test.sock"));
        assert_eq!(provider.timeout_secs, 30);
    }

    #[test]
    fn test_provider_with_timeout() {
        let provider = UnixSocketSecurityProvider::with_timeout("/tmp/test.sock", 60);
        assert_eq!(provider.timeout_secs, 60);
    }

    #[test]
    fn test_socket_exists() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/path.sock");
        assert!(!provider.socket_exists());
    }

    #[test]
    fn test_request_id_increment() {
        let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
        assert_eq!(provider.next_id(), 1);
        assert_eq!(provider.next_id(), 2);
        assert_eq!(provider.next_id(), 3);
    }
}

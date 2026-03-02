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
                "Failed to connect to {}: {e}",
                self.socket_path.display()
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
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize request: {e}")))?;

        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send request: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send newline: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to flush: {e}")))?;

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
        .map_err(|e| ToadStoolError::network(format!("Failed to read response: {e}")))?;

        // Parse response
        let response: JsonRpcResponse<R> = serde_json::from_str(&response_line).map_err(|e| {
            ToadStoolError::runtime(format!(
                "Failed to parse response: {e} (raw: {raw})",
                raw = response_line.trim()
            ))
        })?;

        // Check for error
        if let Some(error) = response.error {
            return Err(ToadStoolError::runtime(format!(
                "Remote error ({code}): {message}",
                code = error.code,
                message = error.message
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
    #[allow(dead_code)] // Required for JSON-RPC 2.0 shape; not read after deserialization
    jsonrpc: String,
    result: Option<R>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)] // Required for JSON-RPC 2.0 shape; not read after deserialization
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

// TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
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

    #[test]
    fn test_provider_creation_with_pathbuf() {
        let path = PathBuf::from("/var/run/security.sock");
        let provider = UnixSocketSecurityProvider::new(&path);
        assert_eq!(provider.socket_path, path);
    }

    #[test]
    fn test_provider_with_timeout_custom_path() {
        let provider = UnixSocketSecurityProvider::with_timeout("/custom/path.sock", 5);
        assert_eq!(provider.socket_path, PathBuf::from("/custom/path.sock"));
        assert_eq!(provider.timeout_secs, 5);
    }

    #[test]
    fn test_socket_exists_when_file_exists() {
        let provider = UnixSocketSecurityProvider::new("/");
        assert!(provider.socket_exists());
    }

    #[test]
    fn test_request_id_thread_safety() {
        let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
        let ids: Vec<u64> = (0..10).map(|_| provider.next_id()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 10);
    }

    #[tokio::test]
    async fn test_capabilities_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.capabilities().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to connect") || err.to_string().contains("Connection")
        );
    }

    #[tokio::test]
    async fn test_metadata_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.metadata().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encrypt_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.encrypt(b"data", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let metadata = EncryptionMetadata {
            algorithm: "AES-256-GCM".to_string(),
            key_id: "key-1".to_string(),
            encrypted_at: std::time::SystemTime::now(),
        };
        let result = provider.decrypt(&[0u8; 32], &metadata).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.health_check().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sign_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.sign(b"data", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let result = provider.verify(b"data", &[0u8; 64], "key-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_permission_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let request = PermissionRequest {
            requester_id: "test".to_string(),
            target: ExternalTarget::CloudProvider {
                provider: CloudProvider::AWS,
                regions: vec!["us-east-1".to_string()],
                services: vec!["s3".to_string()],
            },
            scope: PermissionScope {
                operations: vec!["read".to_string()],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: vec![],
            },
            validity_duration: std::time::Duration::from_secs(3600),
            delegation_info: None,
        };
        let result = provider.create_permission(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_permission_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let now = std::time::SystemTime::now();
        let permission = SecurityPermission {
            permission_id: uuid::Uuid::new_v4(),
            holder_id: "test".to_string(),
            target: ExternalTarget::CloudProvider {
                provider: CloudProvider::AWS,
                regions: vec![],
                services: vec![],
            },
            scope: PermissionScope {
                operations: vec![],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: vec![],
            },
            valid_from: now,
            valid_until: now,
            proof: SecurityProof {
                signature: vec![],
                algorithm: SignatureAlgorithm::Ed25519,
                public_key_id: "key".to_string(),
                signed_at: now,
            },
            provider_metadata: ProviderMetadata {
                provider_id: "p".to_string(),
                provider_type: "test".to_string(),
                provider_version: "1.0".to_string(),
                metadata: std::collections::HashMap::new(),
            },
        };
        let result = provider.validate_permission(&permission).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_permission_connection_refused() {
        let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
        let perm_id = uuid::Uuid::new_v4();
        let result = provider.revoke_permission(&perm_id, "test reason").await;
        assert!(result.is_err());
    }
}

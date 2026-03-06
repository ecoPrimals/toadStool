// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP Security Provider
//!
//! Communicates with a remote security provider over TCP using JSON-RPC 2.0.
//! For local communication, Unix sockets are preferred (lower latency, no TCP
//! overhead). TCP is for cross-machine security provider access.
//!
//! ## Protocol
//!
//! Uses JSON-RPC 2.0 over TCP with newline-delimited messages:
//! - Request: `{"jsonrpc":"2.0","method":"security.encrypt","params":{...},"id":1}\n`
//! - Response: `{"jsonrpc":"2.0","result":{...},"id":1}\n`

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::provider::*;
use super::types::*;
use toadstool_common::constants::timeouts;

/// TCP security provider
///
/// Forwards security operations to a remote provider over TCP.
pub struct TcpSecurityProvider {
    host: String,
    port: u16,
    request_id: AtomicU64,
    timeout_secs: u64,
}

impl TcpSecurityProvider {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            request_id: AtomicU64::new(1),
            timeout_secs: timeouts::DEFAULT_REQUEST_TIMEOUT.as_secs(),
        }
    }

    pub fn with_timeout(host: &str, port: u16, timeout_secs: u64) -> Self {
        Self {
            host: host.to_string(),
            port,
            request_id: AtomicU64::new(1),
            timeout_secs,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    async fn send_request<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: P,
    ) -> ToadStoolResult<R> {
        let addr = self.addr();
        let timeout = std::time::Duration::from_secs(self.timeout_secs);

        let stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| {
                ToadStoolError::timeout(format!(
                    "Connection to {addr} timed out after {}s",
                    self.timeout_secs
                ))
            })?
            .map_err(|e| ToadStoolError::network(format!("Failed to connect to {addr}: {e}")))?;

        let (reader, mut writer) = stream.into_split();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id(),
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize request: {e}")))?;

        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send request: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send delimiter: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to flush: {e}")))?;

        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();

        tokio::time::timeout(timeout, buf_reader.read_line(&mut response_line))
            .await
            .map_err(|_| {
                ToadStoolError::timeout(format!(
                    "Response from {addr} timed out after {}s",
                    self.timeout_secs
                ))
            })?
            .map_err(|e| ToadStoolError::network(format!("Failed to read response: {e}")))?;

        let response: JsonRpcResponse<R> = serde_json::from_str(&response_line).map_err(|e| {
            ToadStoolError::runtime(format!(
                "Failed to parse response: {e} (raw: {raw})",
                raw = response_line.trim()
            ))
        })?;

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

#[derive(Debug, Serialize)]
struct JsonRpcRequest<P> {
    jsonrpc: String,
    method: String,
    params: P,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<R> {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<R>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

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

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl SecurityProvider for TcpSecurityProvider {
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
        let provider = TcpSecurityProvider::new("localhost", 9090);
        assert_eq!(provider.host, "localhost");
        assert_eq!(provider.port, 9090);
    }

    #[test]
    fn test_provider_with_timeout() {
        let provider = TcpSecurityProvider::with_timeout("example.com", 443, 60);
        assert_eq!(provider.timeout_secs, 60);
    }

    #[test]
    fn test_addr_format() {
        let provider = TcpSecurityProvider::new("host.example", 8080);
        assert_eq!(provider.addr(), "host.example:8080");
    }

    #[test]
    fn test_request_id_increment() {
        let provider = TcpSecurityProvider::new("localhost", 9090);
        assert_eq!(provider.next_id(), 1);
        assert_eq!(provider.next_id(), 2);
        assert_eq!(provider.next_id(), 3);
    }

    #[tokio::test]
    async fn test_connection_refused() {
        let provider = TcpSecurityProvider::new("127.0.0.1", 1);
        let result = provider.capabilities().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_connection_refused() {
        let provider = TcpSecurityProvider::new("127.0.0.1", 1);
        let result = provider.health_check().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "test.method".to_string(),
            params: serde_json::json!({"key": "value"}),
            id: 1,
        };
        let json = serde_json::to_string(&req);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("test.method"));
    }

    #[test]
    fn test_json_rpc_response_deserialization_with_result() {
        let json = r#"{"jsonrpc":"2.0","result":{"healthy":true},"id":1}"#;
        let parsed: Result<JsonRpcResponse<serde_json::Value>, _> = serde_json::from_str(json);
        assert!(parsed.is_ok());
        let resp = parsed.unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_deserialization_with_error() {
        let json =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid request"},"id":1}"#;
        let parsed: Result<JsonRpcResponse<serde_json::Value>, _> = serde_json::from_str(json);
        assert!(parsed.is_ok());
        let resp = parsed.unwrap();
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert!(err.message.contains("Invalid"));
    }

    #[test]
    fn test_encrypt_request_structure() {
        let data = [1u8, 2, 3, 4, 5];
        let req = EncryptRequest {
            data: &data,
            options: None,
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_decrypt_request_structure() {
        use crate::security_provider::types::EncryptionMetadata;
        use std::time::SystemTime;
        let ciphertext = [0xaau8, 0xbb];
        let metadata = EncryptionMetadata {
            algorithm: "AES-256-GCM".to_string(),
            key_id: "key-1".to_string(),
            encrypted_at: SystemTime::now(),
        };
        let req = DecryptRequest {
            ciphertext: &ciphertext,
            metadata: &metadata,
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_verify_request_structure() {
        let data = [1u8, 2, 3];
        let sig = [0x11u8, 0x22];
        let req = VerifyRequest {
            data: &data,
            signature: &sig,
            public_key_id: "pk-1",
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_revoke_request_structure() {
        let perm_id = uuid::Uuid::new_v4();
        let req = RevokeRequest {
            permission_id: &perm_id,
            reason: "test revocation",
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }
}

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

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::provider::{
    EncryptionOptions, PermissionValidationResult, ProviderHealth, SecurityCapability,
    SecurityProvider, SigningOptions,
};
use super::types::{
    DecryptionResult, EncryptionMetadata, EncryptionResult, PermissionRequest, ProviderMetadata,
    SecurityPermission, SignatureResult, VerificationResult,
};
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
    /// Connects to `host:port` with the default request timeout.
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            request_id: AtomicU64::new(1),
            timeout_secs: timeouts::DEFAULT_REQUEST_TIMEOUT.as_secs(),
        }
    }

    /// Like [`Self::new`] but sets the TCP read/write timeout in seconds.
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
    #[expect(
        dead_code,
        reason = "JSON-RPC 2.0 response shape; required for serde deserialization"
    )]
    jsonrpc: String,
    result: Option<R>,
    error: Option<JsonRpcError>,
    #[expect(
        dead_code,
        reason = "JSON-RPC 2.0 response shape; required for serde deserialization"
    )]
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

impl SecurityProvider for TcpSecurityProvider {
    fn capabilities(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<SecurityCapability>>> + Send + '_>> {
        Box::pin(async { self.send_request("security.capabilities", ()).await })
    }

    fn metadata(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ProviderMetadata>> + Send + '_>> {
        Box::pin(async { self.send_request("security.metadata", ()).await })
    }

    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<EncryptionOptions>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<EncryptionResult>> + Send + 'a>> {
        Box::pin(async move {
            self.send_request(
                "security.encrypt",
                EncryptRequest {
                    data,
                    options: options.as_ref(),
                },
            )
            .await
        })
    }

    fn decrypt<'a>(
        &'a self,
        ciphertext: &'a [u8],
        metadata: &'a EncryptionMetadata,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<DecryptionResult>> + Send + 'a>> {
        Box::pin(async move {
            self.send_request(
                "security.decrypt",
                DecryptRequest {
                    ciphertext,
                    metadata,
                },
            )
            .await
        })
    }

    fn sign<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<SigningOptions>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SignatureResult>> + Send + 'a>> {
        Box::pin(async move {
            self.send_request(
                "security.sign",
                SignRequest {
                    data,
                    options: options.as_ref(),
                },
            )
            .await
        })
    }

    fn verify<'a>(
        &'a self,
        data: &'a [u8],
        signature: &'a [u8],
        public_key_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VerificationResult>> + Send + 'a>> {
        Box::pin(async move {
            self.send_request(
                "security.verify",
                VerifyRequest {
                    data,
                    signature,
                    public_key_id,
                },
            )
            .await
        })
    }

    fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SecurityPermission>> + Send + '_>> {
        Box::pin(async move {
            self.send_request("security.createPermission", request)
                .await
        })
    }

    fn validate_permission<'a>(
        &'a self,
        permission: &'a SecurityPermission,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<PermissionValidationResult>> + Send + 'a>>
    {
        Box::pin(async move {
            self.send_request("security.validatePermission", permission)
                .await
        })
    }

    fn revoke_permission<'a>(
        &'a self,
        permission_id: &'a uuid::Uuid,
        reason: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            self.send_request(
                "security.revokePermission",
                RevokeRequest {
                    permission_id,
                    reason,
                },
            )
            .await
        })
    }

    fn health_check(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_>> {
        Box::pin(async { self.send_request("security.healthCheck", ()).await })
    }
}

#[cfg(test)]
#[path = "tcp_provider_tests.rs"]
mod tests;

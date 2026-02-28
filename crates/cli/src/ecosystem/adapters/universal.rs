//! Universal service adapter - protocol-agnostic service invocation
//!
//! The universal adapter discovers services by capability and invokes them
//! using the appropriate protocol, completely abstracting away service identity.
//!
//! ## Protocol Priority (UNIVERSAL_IPC_STANDARD_V3)
//!
//! JSON-RPC 2.0 over Unix sockets is preferred (pure Rust, no tonic/protobuf).
//! HTTP is deprecated for primal-to-primal; use Songbird for external HTTP.

use crate::{CliContextExt, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use toadstool_common::ToadStoolError;
use tokio::time::Duration;

use crate::ecosystem::capabilities::{CapabilityId, CapabilityResolver, ServiceProvider};

/// Universal service adapter
///
/// This adapter:
/// 1. Resolves capabilities to service providers
/// 2. Negotiates protocols with discovered services
/// 3. Invokes services using the negotiated protocol
/// 4. Returns results in a standard format
///
/// # Example
/// ```ignore
/// // Forward-looking example - API under development
/// use toadstool_cli::ecosystem::adapters::UniversalServiceAdapter;
/// use toadstool_cli::ecosystem::capabilities::StandardCapability;
///
/// # async fn example(adapter: UniversalServiceAdapter) -> anyhow::Result<()> {
/// // Discover and invoke a crypto service (could be any provider!)
/// let response = adapter.invoke(
///     StandardCapability::CryptoSignatureEd25519.id(),
///     Request::new("verify", serde_json::json!({
///         "public_key": "...",
///         "message": "...",
///         "signature": "..."
///     }))
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct UniversalServiceAdapter {
    /// Capability resolver
    resolver: Arc<CapabilityResolver>,

    /// Request timeout
    timeout: Duration,

    /// Enable request/response logging
    enable_logging: bool,
}

impl UniversalServiceAdapter {
    /// Create a new universal service adapter
    pub fn new(resolver: Arc<CapabilityResolver>) -> Self {
        Self {
            resolver,
            timeout: Duration::from_secs(30),
            enable_logging: false,
        }
    }

    /// Configure request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable request/response logging
    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.enable_logging = enabled;
        self
    }

    /// Invoke a capability with a generic request
    ///
    /// This method:
    /// 1. Resolves the capability to a service provider
    /// 2. Determines the best protocol to use
    /// 3. Serializes the request for that protocol
    /// 4. Sends the request and awaits response
    /// 5. Deserializes and returns the response
    pub async fn invoke(
        &self,
        capability: impl Into<CapabilityId>,
        request: Request,
    ) -> Result<Response> {
        let capability = capability.into();

        if self.enable_logging {
            tracing::debug!(
                capability = %capability,
                operation = %request.operation,
                "Invoking capability"
            );
        }

        // Resolve capability to provider
        let provider = self
            .resolver
            .resolve(capability.clone())
            .await
            .context(format!("Failed to resolve capability: {capability}"))?;

        // Invoke via appropriate protocol
        let response = self.invoke_provider(&provider, request).await?;

        if self.enable_logging {
            tracing::debug!(
                capability = %capability,
                status = ?response.status,
                "Capability invocation complete"
            );
        }

        Ok(response)
    }

    /// Invoke a specific service provider
    async fn invoke_provider(
        &self,
        provider: &ServiceProvider,
        request: Request,
    ) -> Result<Response> {
        // Determine protocol (prefer JSON-RPC / Unix socket per UNIVERSAL_IPC_STANDARD_V3)
        let protocol = Self::select_protocol(&provider.protocols)?;

        match protocol.as_str() {
            "jsonrpc" | "unix" => self.invoke_jsonrpc(provider, request).await,
            "http" | "https" => self.invoke_http(provider, request).await,
            "grpc" => {
                tracing::error!(
                    "gRPC protocol deprecated (UNIVERSAL_IPC_STANDARD_V3). Migrate to JSON-RPC over Unix socket."
                );
                Err(crate::CliError::Other(
                    "gRPC protocol not supported. Migrate to JSON-RPC over Unix socket: \
                     use UnixJsonRpcClient from toadstool_common::unix_jsonrpc_client. \
                     (UNIVERSAL_IPC_STANDARD_V3). For external HTTP, route through Songbird."
                        .to_string(),
                ))?
            }
            _ => Err(crate::CliError::Other(format!(
                "Unsupported protocol: {protocol} (use jsonrpc or unix)"
            )))?,
        }
    }

    /// Select the best protocol from available options
    ///
    /// Preference order (UNIVERSAL_IPC_STANDARD_V3): jsonrpc, unix > http > grpc
    fn select_protocol(protocols: &[String]) -> Result<String> {
        // Prefer JSON-RPC (canonical) or json-rpc (alias)
        for preferred in &["jsonrpc", "json-rpc"] {
            if protocols.iter().any(|p| p == preferred) {
                return Ok("jsonrpc".to_string());
            }
        }
        // Prefer Unix socket (canonical) or unix-socket (alias)
        for preferred in &["unix", "unix-socket"] {
            if protocols.iter().any(|p| p == preferred) {
                return Ok("unix".to_string());
            }
        }

        // Fall back to HTTP (deprecated for primal-to-primal)
        for preferred in &["http", "https"] {
            if protocols.iter().any(|p| p == preferred) {
                return Ok(preferred.to_string());
            }
        }

        // Last resort: gRPC (requires tonic, violates ecoBin)
        for preferred in &["grpc"] {
            if protocols.iter().any(|p| p == preferred) {
                return Ok(preferred.to_string());
            }
        }

        // Fall back to first available
        protocols
            .first()
            .map(|s| s.to_string())
            .ok_or_else(|| crate::CliError::Other("No protocols available".to_string()))
    }

    /// Extract Unix socket path from provider endpoint
    ///
    /// Supports: `unix:///path/to/sock`, `unix://path`, or bare `/path/to/sock`
    pub(crate) fn socket_path_from_endpoint(endpoint: &str) -> Result<PathBuf> {
        let path = if endpoint.starts_with("unix://") {
            endpoint
                .strip_prefix("unix://")
                .map(|s| s.to_string())
                .unwrap_or_else(|| endpoint.to_string())
        } else if endpoint.starts_with('/') {
            endpoint.to_string()
        } else {
            return Err(crate::CliError::Other(format!(
                "Endpoint is not a Unix socket path (expected unix:///path or /path): {endpoint}"
            )));
        };

        Ok(PathBuf::from(path))
    }

    /// Invoke via Unix socket JSON-RPC 2.0 (UNIVERSAL_IPC_STANDARD_V3)
    ///
    /// Pure Rust - no tonic, no protobuf, no C dependencies.
    async fn invoke_jsonrpc(
        &self,
        provider: &ServiceProvider,
        request: Request,
    ) -> Result<Response> {
        let socket_path = Self::socket_path_from_endpoint(&provider.endpoint).context(format!(
            "Provider endpoint {} is not a Unix socket for JSON-RPC",
            provider.endpoint
        ))?;

        let client = UnixJsonRpcClient::new(socket_path);

        match client
            .call(request.operation.as_str(), request.payload.clone())
            .await
        {
            Ok(result) => Ok(Response {
                status: ResponseStatus::Success,
                data: Some(result),
                error: None,
            }),
            Err(e) => {
                // JSON-RPC server returned error object → Response with status Error
                if matches!(e, ToadStoolError::Execution(_)) {
                    return Ok(Response {
                        status: ResponseStatus::Error,
                        data: None,
                        error: Some(e.to_string()),
                    });
                }
                // Connection, serialization, network errors → propagate
                Err(e.into())
            }
        }
    }

    /// Invoke via HTTP/REST
    ///
    /// DEEP DEBT: HTTP adapter removed - use Unix socket RPC instead!
    #[allow(clippy::unused_async)] // Trait method; deprecated, returns Err immediately
    async fn invoke_http(
        &self,
        _provider: &ServiceProvider,
        _request: Request,
    ) -> Result<Response> {
        // External HTTP should go through Songbird (Concentrated Gap architecture)
        tracing::error!(
            "HTTP invoke deprecated - use Unix socket RPC for primal-to-primal communication"
        );

        Err(crate::CliError::Other(
            "HTTP adapter removed. Use Unix socket RPC instead. \
             For external HTTP, route through Songbird (Concentrated Gap architecture)."
                .to_string(),
        ))?
    }
}

/// Generic service request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Operation name (e.g., "verify", "encrypt", "store")
    pub operation: String,

    /// Request payload (operation-specific)
    pub payload: serde_json::Value,
}

impl Request {
    /// Create a new request
    pub fn new(operation: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            operation: operation.into(),
            payload,
        }
    }
}

/// Generic service response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Response status
    pub status: ResponseStatus,

    /// Response data (if successful)
    pub data: Option<serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl Response {
    /// Check if response was successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, ResponseStatus::Success)
    }

    /// Get response data, returning error if failed
    pub fn data(self) -> Result<serde_json::Value> {
        match self.status {
            ResponseStatus::Success => self.data.ok_or_else(|| {
                crate::CliError::Other("No data in successful response".to_string())
            }),
            ResponseStatus::Error => {
                let error = self.error.unwrap_or_else(|| "Unknown error".to_string());
                Err(crate::CliError::Other(format!("Service error: {error}")))
            }
        }
    }
}

/// Response status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Request succeeded
    Success,

    /// Request failed
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let request = Request::new(
            "verify",
            serde_json::json!({
                "signature": "abc123"
            }),
        );

        assert_eq!(request.operation, "verify");
        assert!(request.payload.is_object());
    }

    #[test]
    fn test_response_success() {
        let response = Response {
            status: ResponseStatus::Success,
            data: Some(serde_json::json!({"result": true})),
            error: None,
        };

        assert!(response.is_success());
        assert!(response.data.is_some());
    }

    #[test]
    fn test_protocol_selection_prefers_jsonrpc() {
        // jsonrpc and unix-socket preferred over http/grpc
        let protocols = vec![
            "grpc".to_string(),
            "http".to_string(),
            "jsonrpc".to_string(),
        ];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(selected, "jsonrpc");
    }

    #[test]
    fn test_protocol_selection_prefers_unix_over_http() {
        let protocols = vec!["http".to_string(), "unix".to_string()];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(selected, "unix");
    }

    #[test]
    fn test_protocol_selection_unix_socket_alias_normalizes_to_unix() {
        let protocols = vec!["http".to_string(), "unix-socket".to_string()];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(
            selected, "unix",
            "unix-socket alias should normalize to canonical unix"
        );
    }

    #[test]
    fn test_protocol_selection_http_over_grpc() {
        let protocols = vec!["grpc".to_string(), "http".to_string()];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(selected, "http");
    }

    #[test]
    fn test_protocol_selection_grpc_fallback() {
        let protocols = vec!["grpc".to_string()];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(selected, "grpc");
    }

    #[test]
    fn test_protocol_selection_empty() {
        let protocols: Vec<String> = vec![];
        let result = UniversalServiceAdapter::select_protocol(&protocols);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_path_from_endpoint_unix_prefix() {
        let path =
            UniversalServiceAdapter::socket_path_from_endpoint("unix:///var/run/toadstool.sock")
                .unwrap();
        assert_eq!(path, std::path::Path::new("/var/run/toadstool.sock"));
    }

    #[test]
    fn test_socket_path_from_endpoint_bare_path() {
        let path = UniversalServiceAdapter::socket_path_from_endpoint("/tmp/service.sock").unwrap();
        assert_eq!(path, std::path::Path::new("/tmp/service.sock"));
    }

    #[test]
    fn test_socket_path_from_endpoint_http_fails() {
        let result = UniversalServiceAdapter::socket_path_from_endpoint("http://localhost:8080");
        assert!(result.is_err());
    }
}

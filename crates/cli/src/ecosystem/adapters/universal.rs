//! Universal service adapter - protocol-agnostic service invocation
//!
//! The universal adapter discovers services by capability and invokes them
//! using the appropriate protocol, completely abstracting away service identity.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
            .with_context(|| format!("Failed to resolve capability: {}", capability))?;

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
        // Determine protocol (prefer HTTP if available)
        let protocol = Self::select_protocol(&provider.protocols)?;

        match protocol.as_str() {
            "http" | "https" => self.invoke_http(provider, request).await,
            "grpc" => self.invoke_grpc(provider, request).await,
            _ => anyhow::bail!("Unsupported protocol: {}", protocol),
        }
    }

    /// Select the best protocol from available options
    fn select_protocol(protocols: &[String]) -> Result<String> {
        // Preference order: http, grpc, others
        for preferred in &["http", "https", "grpc"] {
            if protocols.iter().any(|p| p == preferred) {
                return Ok(preferred.to_string());
            }
        }

        // Fall back to first available
        protocols
            .first()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No protocols available"))
    }

    /// Invoke via HTTP/REST
    ///
    /// DEEP DEBT: HTTP adapter removed - use Unix socket RPC instead!
    async fn invoke_http(
        &self,
        _provider: &ServiceProvider,
        _request: Request,
    ) -> Result<Response> {
        // External HTTP should go through Songbird (Concentrated Gap architecture)
        tracing::error!(
            "HTTP invoke deprecated - use Unix socket RPC for primal-to-primal communication"
        );

        anyhow::bail!(
            "HTTP adapter removed. Use Unix socket RPC instead. \
             For external HTTP, route through Songbird (Concentrated Gap architecture)."
        )
    }

    /// Invoke via gRPC (stub implementation)
    async fn invoke_grpc(
        &self,
        _provider: &ServiceProvider,
        _request: Request,
    ) -> Result<Response> {
        // FUTURE: Implement gRPC invocation (planned for v0.3.0)
        // This requires adding tonic or similar gRPC client dependency.
        // Currently, HTTP/REST is sufficient for all existing primal integrations.
        // See: docs/planning/GRPC_INTEGRATION_PLAN.md (to be created)
        anyhow::bail!("gRPC protocol not yet implemented - use HTTP/REST for now")
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
            ResponseStatus::Success => self
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in successful response")),
            ResponseStatus::Error => {
                let error = self.error.unwrap_or_else(|| "Unknown error".to_string());
                anyhow::bail!("Service error: {}", error)
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
    fn test_protocol_selection() {
        let protocols = vec!["grpc".to_string(), "http".to_string()];
        let selected = UniversalServiceAdapter::select_protocol(&protocols).unwrap();
        assert_eq!(selected, "http"); // HTTP preferred
    }
}

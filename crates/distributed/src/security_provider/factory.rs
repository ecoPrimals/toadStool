// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security Provider Factory
//!
//! Creates security provider instances from Universal Adapter capability handles.
//! This is the glue between capability discovery and actual provider usage.
//!
//! ## Supported Transports
//!
//! - **InProcess**: Direct provider instantiation (BearDog, etc.) - ✅ Working
//! - **UnixSocket**: IPC via Unix domain sockets - ✅ Implemented
//! - **TCP**: TCP socket connection - ✅ Implemented (JSON-RPC over TCP, cross-machine)
//! - **HTTP**: REST/JSON-RPC over HTTP - Not supported (not ecoBin-compliant; use Unix/TCP)
//!
//! ## ecoBin Compliance
//!
//! Unix sockets are the preferred transport for inter-primal communication:
//! - Pure Rust: No TLS/HTTP stack required
//! - Fast: Direct kernel IPC, no TCP overhead
//! - Secure: File-system permissions for access control
//! - Local: Ideal for primals on same machine

use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::universal_adapter::{CapabilityHandle, ServiceEndpoint};

use super::provider::SecurityProvider;
use super::unix_socket_provider::UnixSocketSecurityProvider;

/// Factory for creating security providers
pub struct SecurityProviderFactory;

impl SecurityProviderFactory {
    /// Create a security provider from a Universal Adapter capability handle
    ///
    /// This is the key method that bridges capability discovery and provider usage:
    /// 1. Universal Adapter discovers a security capability
    /// 2. Returns a CapabilityHandle
    /// 3. Factory creates the appropriate SecurityProvider impl
    /// 4. Code uses SecurityProvider trait (doesn't know which impl!)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use toadstool_distributed::security_provider::*;
    /// use toadstool_common::universal_adapter::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Discover security capability
    /// let adapter = UniversalAdapter::new().await?;
    /// let handle = adapter.request_capability(
    ///     CapabilityType::Security {
    ///         features: vec![SecurityFeature::Encryption],
    ///         min_trust_level: TrustLevel::High,
    ///     }
    /// ).await?;
    ///
    /// // Create provider from handle (discovered at runtime!)
    /// let provider = SecurityProviderFactory::create_from_handle(&handle).await?;
    ///
    /// // Use provider (don't know which impl!)
    /// let encrypted = provider.encrypt(b"data", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_from_handle(
        handle: &CapabilityHandle,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // Inspect endpoint to determine provider type
        match handle.endpoint() {
            ServiceEndpoint::Http(url) => Self::create_http_provider(url).await,
            ServiceEndpoint::UnixSocket(path) => Self::create_unix_socket_provider(path).await,
            ServiceEndpoint::Tcp { host, port } => Self::create_tcp_provider(host, *port).await,
            ServiceEndpoint::InProcess => Self::create_in_process_provider().await,
            ServiceEndpoint::Custom { protocol, address } => {
                Self::create_custom_provider(protocol, address).await
            }
        }
    }

    /// Create provider from HTTP endpoint
    ///
    /// NOTE: HTTP is not the preferred transport for security operations.
    /// Use Unix sockets for local IPC or mDNS for remote discovery.
    async fn create_http_provider(url: &str) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // HTTP transport is not ecoBin-compliant (requires TLS/HTTP stack)
        // Prefer Unix sockets for local communication
        Err(ToadStoolError::runtime(format!(
            "HTTP security provider not supported (not ecoBin-compliant). \
             Use Unix socket transport instead. Attempted URL: {url}. \
             For remote discovery, use mDNS to find Unix socket paths.",
        )))
    }

    /// Create provider from Unix socket endpoint
    ///
    /// Uses the UnixSocketSecurityProvider to communicate over Unix domain sockets.
    /// This is the preferred transport for inter-primal IPC (ecoBin compliant).
    async fn create_unix_socket_provider(
        path: &std::path::Path,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // Verify socket exists
        if !path.exists() {
            return Err(ToadStoolError::not_found(format!(
                "Security provider socket not found: {}",
                path.display()
            )));
        }

        let provider = UnixSocketSecurityProvider::new(path);

        // Verify connectivity with a health check
        match provider.health_check().await {
            Ok(_) => {
                tracing::info!(
                    "✅ Connected to security provider via Unix socket: {}",
                    path.display()
                );
                Ok(Arc::new(provider) as Arc<dyn SecurityProvider>)
            }
            Err(e) => Err(ToadStoolError::runtime(format!(
                "Security provider at {} not responding: {e}",
                path.display()
            ))),
        }
    }

    /// Create provider from TCP endpoint
    ///
    /// Connects to remote security service over TCP using JSON-RPC 2.0.
    /// For local communication, Unix sockets are preferred (lower latency).
    async fn create_tcp_provider(
        host: &str,
        port: u16,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        use crate::security_provider::tcp_provider::TcpSecurityProvider;

        let provider = TcpSecurityProvider::new(host, port);

        match provider.health_check().await {
            Ok(_) => {
                tracing::info!("Connected to security provider via TCP: {}:{}", host, port);
                Ok(Arc::new(provider) as Arc<dyn SecurityProvider>)
            }
            Err(e) => Err(ToadStoolError::runtime(format!(
                "Security provider at {host}:{port} not responding: {e}",
            ))),
        }
    }

    /// Create in-process provider
    async fn create_in_process_provider() -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // For in-process, we can try to instantiate providers directly

        // Try BearDog implementation first
        use crate::security_provider::beardog_impl::BearDogSecurityProvider;
        match BearDogSecurityProvider::new().await {
            Ok(provider) => return Ok(Arc::new(provider) as Arc<dyn SecurityProvider>),
            Err(_) => {
                // BearDog not available, try other providers
            }
        }

        // Try LocalKeyringProvider — uses OS keyring when D-Bus is available,
        // falls back to in-memory gracefully.
        use crate::security_provider::local_keyring::LocalKeyringProvider;
        let keyring = LocalKeyringProvider::new();
        if keyring.backend() != &crate::security_provider::local_keyring::KeyringBackend::InMemory {
            tracing::info!("LocalKeyringProvider available (OS keyring)");
            return Ok(Arc::new(keyring) as Arc<dyn SecurityProvider>);
        }

        // SoftwareHsmProvider — pure in-process fallback, always succeeds.
        // Keys are ephemeral (lost on restart) — suitable for development/CI.
        use crate::security_provider::software_hsm::SoftwareHsmProvider;
        tracing::info!("Using SoftwareHsmProvider (in-memory, ephemeral keys)");
        Ok(Arc::new(SoftwareHsmProvider::new()) as Arc<dyn SecurityProvider>)
    }

    /// Create custom protocol provider
    async fn create_custom_provider(
        _protocol: &str,
        _address: &str,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        Err(ToadStoolError::not_found(
            "Custom protocol security provider not yet implemented".to_string(),
        ))
    }

    /// Create a mock provider for testing
    #[cfg(test)]
    pub fn create_mock() -> Arc<dyn SecurityProvider> {
        Arc::new(super::provider::MockSecurityProvider::new())
    }
}

/// Helper to get security provider via Universal Adapter
///
/// This is a convenience function that combines discovery and creation.
///
/// # Example
///
/// ```rust,no_run
/// use toadstool_distributed::security_provider::*;
/// use toadstool_common::universal_adapter::*;
/// use toadstool::error::ToadStoolResult;
///
/// # async fn example() -> ToadStoolResult<()> {
/// // One-liner: discover and create provider
/// let provider = discover_security_provider(vec![SecurityFeature::Encryption]).await?;
///
/// // Use provider (discovered at runtime!)
/// let encrypted = provider.encrypt(b"data", None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn discover_security_provider(
    features: Vec<toadstool_common::universal_adapter::SecurityFeature>,
) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
    use toadstool_common::universal_adapter::*;

    // Discover security capability via Universal Adapter
    let adapter = UniversalAdapter::new().await?;

    let handle = adapter
        .request_capability(CapabilityType::Security {
            features,
            min_trust_level: TrustLevel::Medium,
        })
        .await?;

    // Create provider from discovered capability
    SecurityProviderFactory::create_from_handle(&handle).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use toadstool_common::universal_adapter::{
        CapabilityHandle, CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint, TrustLevel,
    };

    #[test]
    fn test_factory_creation() {
        // Just verify factory can be constructed
        let _factory = SecurityProviderFactory;
    }

    #[test]
    fn test_mock_provider_creation() {
        let provider = SecurityProviderFactory::create_mock();
        assert!(Arc::strong_count(&provider) >= 1);
    }

    #[tokio::test]
    async fn test_mock_provider_usage() {
        let provider = SecurityProviderFactory::create_mock();

        // Test basic operations
        let caps = provider.capabilities().await.unwrap();
        assert!(!caps.is_empty());

        let health = provider.health_check().await.unwrap();
        assert_eq!(health, super::super::provider::ProviderHealth::Healthy);
    }

    #[tokio::test]
    async fn test_http_provider_not_implemented() {
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Http(
                    toadstool_common::constants::network::default_http_url(),
                ),
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("HTTP security provider not supported"));
    }

    #[tokio::test]
    async fn test_tcp_provider_connection_failure_without_server() {
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Tcp {
                    host: "localhost".to_string(),
                    port: 38443, // Unlikely to have security provider on this port
                },
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("not responding")
                || err_msg.contains("Failed to connect")
                || err_msg.contains("timed out"),
            "Expected connection failure, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_custom_provider_not_implemented() {
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Custom {
                    protocol: "custom".to_string(),
                    address: "custom://localhost".to_string(),
                },
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Custom protocol"));
    }

    #[tokio::test]
    async fn test_unix_socket_provider_socket_not_found() {
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::UnixSocket(
                    Path::new("/nonexistent/path/security.sock").into(),
                ),
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("socket"));
    }

    #[tokio::test]
    async fn test_in_process_provider_succeeds() {
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::InProcess,
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_ok());
        let provider = result.unwrap();
        let caps = provider.capabilities().await.unwrap();
        assert!(!caps.is_empty());
    }

    #[tokio::test]
    async fn test_mock_provider_encrypt_decrypt() {
        let provider = SecurityProviderFactory::create_mock();
        let encrypted = provider.encrypt(b"secret data", None).await.unwrap();
        assert!(!encrypted.ciphertext.is_empty());
        let decrypted = provider
            .decrypt(&encrypted.ciphertext, &encrypted.metadata)
            .await
            .unwrap();
        assert_eq!(decrypted.plaintext, b"secret data");
    }

    #[tokio::test]
    async fn test_mock_provider_metadata() {
        let provider = SecurityProviderFactory::create_mock();
        let metadata = provider.metadata().await.unwrap();
        assert!(!metadata.provider_id.is_empty());
        assert!(!metadata.provider_type.is_empty());
    }
}

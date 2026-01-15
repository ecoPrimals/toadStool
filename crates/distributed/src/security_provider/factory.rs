//! Security Provider Factory
//!
//! Creates security provider instances from Universal Adapter capability handles.
//! This is the glue between capability discovery and actual provider usage.

use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::universal_adapter::{CapabilityHandle, ServiceEndpoint};

use super::provider::SecurityProvider;

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
            ServiceEndpoint::Http(url) => {
                Self::create_http_provider(url).await
            }
            ServiceEndpoint::UnixSocket(path) => {
                Self::create_unix_socket_provider(path).await
            }
            ServiceEndpoint::Tcp { host, port } => {
                Self::create_tcp_provider(host, *port).await
            }
            ServiceEndpoint::InProcess => {
                Self::create_in_process_provider().await
            }
            ServiceEndpoint::Custom { protocol, address } => {
                Self::create_custom_provider(protocol, address).await
            }
        }
    }

    /// Create provider from HTTP endpoint
    async fn create_http_provider(url: &str) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // In the future, this would create an HTTP client to the security provider
        // For now, return error indicating not yet implemented
        Err(ToadStoolError::not_found(format!(
            "HTTP security provider not yet implemented: {}",
            url
        )))
    }

    /// Create provider from Unix socket endpoint
    async fn create_unix_socket_provider(
        _path: &std::path::Path,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // Future: Create Unix socket client
        Err(ToadStoolError::not_found(
            "Unix socket security provider not yet implemented".to_string(),
        ))
    }

    /// Create provider from TCP endpoint
    async fn create_tcp_provider(
        _host: &str,
        _port: u16,
    ) -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // Future: Create TCP client
        Err(ToadStoolError::not_found(
            "TCP security provider not yet implemented".to_string(),
        ))
    }

    /// Create in-process provider
    async fn create_in_process_provider() -> ToadStoolResult<Arc<dyn SecurityProvider>> {
        // For in-process, we can try to instantiate providers directly
        
        // ✅ COMPLETED: BearDog SecurityProvider fully implemented (Phase 1B)
        // Try BearDog implementation first
        use crate::security_provider::beardog_impl::BearDogSecurityProvider;
        match BearDogSecurityProvider::new().await {
            Ok(provider) => return Ok(Arc::new(provider) as Arc<dyn SecurityProvider>),
            Err(_) => {
                // BearDog not available, try other providers
            }
        }
        
        // TODO(future): Try LocalKeyringProvider
        // TODO(future): Try SoftwareHSMProvider

        Err(ToadStoolError::not_found(
            "No in-process security provider available (tried BearDog)".to_string(),
        ))
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
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
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
        use toadstool_common::universal_adapter::*;
        
        let handle = CapabilityHandle::new(
            CapabilityInfo {
                provider_id: "test".to_string(),
                capability: CapabilityType::Security {
                    features: vec![],
                    min_trust_level: TrustLevel::Low,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Http("http://localhost:8080".to_string()),
                health: HealthStatus::Healthy,
            },
            CapabilityType::Security {
                features: vec![],
                min_trust_level: TrustLevel::Low,
            },
        );

        let result = SecurityProviderFactory::create_from_handle(&handle).await;
        assert!(result.is_err());
    }
}

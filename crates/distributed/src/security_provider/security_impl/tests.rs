// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration Tests for Security SecurityProvider
//!
//! Tests the complete integration between:
//! - DistributedSecurityProvider (trait impl)
//! - SecurityClient (HTTP client)
//! - SecurityDiscovery (service discovery)

#[cfg(test)]
#[expect(
    clippy::module_inception,
    reason = "module name matches parent for API clarity"
)]
mod tests {
    use super::super::client::DistributedSecurityProvider;
    use crate::security_provider::SecurityProviderDispatch;
    use crate::security_provider::provider::*;
    use crate::security_provider::types::*;

    #[tokio::test]
    async fn test_security_provider_creation() {
        // This may fail if Security is not running (expected)
        let result = DistributedSecurityProvider::new().await;

        // Provider creation should succeed even if Security unavailable
        assert!(result.is_ok());

        let provider = result.unwrap();

        // Check capabilities
        let caps = provider.capabilities().await;
        assert!(caps.is_ok());

        let caps_vec = caps.unwrap();
        assert!(caps_vec.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps_vec.contains(&SecurityCapability::DigitalSignatures));
        assert!(caps_vec.contains(&SecurityCapability::PermissionIssuance));
    }

    #[tokio::test]
    async fn test_security_provider_metadata() {
        let provider = DistributedSecurityProvider::new().await.unwrap();
        let metadata = provider.metadata().await.unwrap();

        assert_eq!(metadata.provider_type, "crypto");
        assert_eq!(metadata.provider_version, "2.0.0");
        assert!(metadata.metadata.contains_key("capability"));
        assert_eq!(metadata.metadata.get("capability").unwrap(), "crypto");
    }

    #[tokio::test]
    async fn test_security_provider_health_check() {
        let provider = DistributedSecurityProvider::new().await.unwrap();

        // Health check should return Unhealthy if Security not running
        let health = provider.health_check().await;
        assert!(health.is_ok());

        // Will be Unhealthy if no security service available
        let status = health.unwrap();
        assert!(matches!(
            status,
            ProviderHealth::Healthy | ProviderHealth::Unhealthy
        ));
    }

    #[tokio::test]
    async fn test_security_provider_capabilities_list() {
        let provider = DistributedSecurityProvider::new().await.unwrap();
        let caps = provider.capabilities().await.unwrap();

        // Verify all expected capabilities
        assert_eq!(caps.len(), 6);
        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::AsymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
        assert!(caps.contains(&SecurityCapability::KeyManagement));
        assert!(caps.contains(&SecurityCapability::PermissionIssuance));
        assert!(caps.contains(&SecurityCapability::AuditLogging));
    }

    /// Test encryption flow (will fail if Security not available)
    /// This is expected behavior - graceful degradation
    #[tokio::test]
    async fn test_security_encryption_graceful_fail() {
        let provider = DistributedSecurityProvider::new().await.unwrap();

        let data = b"test data for encryption";
        let result = provider.encrypt(data, None).await;

        // If Security not available, this should fail gracefully
        // (not panic, not hang, just return error)
        if let Err(e) = result {
            // Expected: service not found or connection failed
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("not found")
                    || err_str.contains("connection")
                    || err_str.contains("network")
            );
        }
        // If Security IS available, verify result structure
        else if let Ok(encrypted) = result {
            assert!(!encrypted.ciphertext.is_empty());
            assert!(!encrypted.metadata.key_id.is_empty());
            assert!(!encrypted.metadata.algorithm.is_empty());
        }
    }

    /// Test signing flow (will fail if Security not available)
    #[tokio::test]
    async fn test_security_signing_graceful_fail() {
        let provider = DistributedSecurityProvider::new().await.unwrap();

        let data = b"test data for signing";
        let result = provider.sign(data, None).await;

        // Graceful failure expected if no Security
        if let Err(e) = result {
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("not found")
                    || err_str.contains("connection")
                    || err_str.contains("network")
            );
        } else if let Ok(signature) = result {
            assert!(!signature.signature.is_empty());
            assert!(!signature.key_id.is_empty());
        }
    }

    /// Test permission creation flow
    #[tokio::test]
    async fn test_security_permission_creation_graceful_fail() {
        let provider = DistributedSecurityProvider::new().await.unwrap();

        let request = PermissionRequest {
            requester_id: "test-user".to_string(),
            target: ExternalTarget::ExternalTool {
                tool_name: "test-tool".to_string(),
                api_endpoints: vec!["https://test.example.com".to_string()],
                feature_set: vec![],
            },
            scope: PermissionScope {
                operations: vec!["read".to_string(), "write".to_string()],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: vec![],
            },
            validity_duration: std::time::Duration::from_secs(3600),
            delegation_info: None,
        };

        let result = provider.create_permission(request).await;

        // Graceful failure expected if no Security
        if let Err(e) = result {
            let err_str = format!("{:?}", e);
            assert!(
                err_str.contains("not found")
                    || err_str.contains("connection")
                    || err_str.contains("network")
            );
        } else if let Ok(permission) = result {
            assert_eq!(permission.holder_id, "test-user");
        }
    }

    /// Test that provider implements Send + Sync (required for async)
    #[test]
    fn test_security_provider_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DistributedSecurityProvider>();
        assert_sync::<DistributedSecurityProvider>();
    }

    /// Test provider behind dispatch enum
    #[tokio::test]
    async fn test_security_provider_dispatch_wrapper() {
        let provider = SecurityProviderDispatch::Distributed(
            DistributedSecurityProvider::new().await.unwrap(),
        );

        let caps = provider.capabilities().await.unwrap();
        assert!(!caps.is_empty());

        let metadata = provider.metadata().await.unwrap();
        assert_eq!(metadata.provider_type, "crypto");
    }

    /// Test multiple providers can coexist
    #[tokio::test]
    async fn test_multiple_security_providers() {
        let provider1 = DistributedSecurityProvider::new().await.unwrap();
        let provider2 = DistributedSecurityProvider::new().await.unwrap();

        let caps1 = provider1.capabilities().await.unwrap();
        let caps2 = provider2.capabilities().await.unwrap();

        // Both should report same capabilities
        assert_eq!(caps1, caps2);

        let meta1 = provider1.metadata().await.unwrap();
        let meta2 = provider2.metadata().await.unwrap();

        // Metadata should be consistent
        assert_eq!(meta1.provider_type, meta2.provider_type);
        assert_eq!(meta1.provider_version, meta2.provider_version);
    }
}

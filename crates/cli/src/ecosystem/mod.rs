// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem Integration - Sovereign Science Network
//!
//! Integration with the ecoPrimals ecosystem for distributed sovereign computing:
//! - Songbird: Service discovery and coordination
//! - `BearDog`: Cryptographic security and permissions
//! - `NestGate`: Distributed storage and data management
//!
//! ## Module Structure (Refactored by Protocol)
//!
//! - `types`: Type definitions (EcosystemIntegrator, ServiceEndpoint, etc.)
//! - `discovery`: Service discovery and scanning logic
//! - `connection`: Connection management
//! - `services/`: Service-specific integrations (Songbird, BearDog, NestGate)
//! - `integrator_impl`: Core EcosystemIntegrator implementation

// Public modules
pub mod adapters;
pub mod capabilities;
pub mod config;
pub mod constants; // Zero-copy constants
pub mod service_type;
pub mod services;
pub mod types;

// Internal modules
mod connection;
mod discovery;

// Public re-exports
#[allow(deprecated)] // Re-exporting deprecated EcosystemService for backward compatibility
pub use types::{
    BearDogPermission, CryptoVerificationContext, DiscoveredService, DiscoveryResult,
    EcosystemIntegrator, EcosystemService, NestGateMount, ServiceEndpoint, ServiceSignature,
    ServiceType, SignedServiceResponse, TrustLevel,
};

// Internal types
use types::{ConnectionStatus, EcosystemStatus, ServiceConnection};

impl Default for EcosystemIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

// Include the implementation
include!("integrator_impl.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// ✅ MIGRATED: Replaced EcosystemService enum with ServiceType
    /// Old test: test_ecosystem_service_variants
    /// New test: test_service_type_capabilities
    #[test]
    fn test_service_type_capabilities() {
        use crate::ecosystem::capabilities::StandardCapability;
        use crate::ecosystem::service_type::ServiceType;

        // Test crypto service type (replaces EcosystemService::BearDog)
        let crypto_caps = vec![StandardCapability::CryptoSignatureEd25519.id()];
        let crypto_service = ServiceType::from_capability_list(crypto_caps);
        assert!(crypto_service.provides_crypto());
        assert!(!crypto_service.provides_coordination());
        assert!(!crypto_service.provides_storage());

        // Test coordination service type (replaces EcosystemService::Songbird)
        let coord_caps = vec![StandardCapability::CoordinationServiceRegistry.id()];
        let coord_service = ServiceType::from_capability_list(coord_caps);
        assert!(!coord_service.provides_crypto());
        assert!(coord_service.provides_coordination());
        assert!(!coord_service.provides_storage());

        // Test storage service type (replaces EcosystemService::NestGate)
        let storage_caps = vec![StandardCapability::StorageDistributedFilesystem.id()];
        let storage_service = ServiceType::from_capability_list(storage_caps);
        assert!(!storage_service.provides_crypto());
        assert!(!storage_service.provides_coordination());
        assert!(storage_service.provides_storage());

        // Test custom service type (replaces EcosystemService::Unknown)
        let custom_service = ServiceType::default().with_legacy_name("custom");
        assert_eq!(custom_service.legacy_name(), Some("custom"));
    }

    /// ✅ MIGRATED: Replaced EcosystemService::name() with ServiceType::display_name()
    /// Old test: test_ecosystem_service_name
    /// New test: test_service_type_names
    #[test]
    fn test_service_type_names() {
        use crate::ecosystem::capabilities::StandardCapability;
        use crate::ecosystem::service_type::ServiceType;

        // Test crypto service display name (replaces "beardog")
        let crypto_caps = vec![StandardCapability::CryptoSignatureEd25519.id()];
        let crypto_service = ServiceType::from_capability_list(crypto_caps);
        assert_eq!(crypto_service.display_name(), "crypto-service");

        // Test coordination service display name (replaces "songbird")
        let coord_caps = vec![StandardCapability::CoordinationServiceRegistry.id()];
        let coord_service = ServiceType::from_capability_list(coord_caps);
        assert_eq!(coord_service.display_name(), "coordination-service");

        // Test storage service display name (replaces "nestgate")
        let storage_caps = vec![StandardCapability::StorageDistributedFilesystem.id()];
        let storage_service = ServiceType::from_capability_list(storage_caps);
        assert_eq!(storage_service.display_name(), "storage-service");

        // Test custom legacy name
        let custom_service = ServiceType::default().with_legacy_name("test");
        assert_eq!(custom_service.display_name(), "test");
    }

    #[test]
    fn test_trust_level_variants() {
        assert!(matches!(TrustLevel::Unknown, TrustLevel::Unknown));
        assert!(matches!(TrustLevel::Discovered, TrustLevel::Discovered));
        assert!(matches!(TrustLevel::Advertised, TrustLevel::Advertised));
        assert!(matches!(TrustLevel::Verified, TrustLevel::Verified));
        assert!(matches!(TrustLevel::Sovereign, TrustLevel::Sovereign));
    }

    #[test]
    #[allow(deprecated)] // Testing deprecated ServiceType during migration
    fn test_service_type_variants() {
        assert!(matches!(ServiceType::Songbird, ServiceType::Songbird));
        assert!(matches!(ServiceType::BearDog, ServiceType::BearDog));
        assert!(matches!(ServiceType::NestGate, ServiceType::NestGate));
        assert!(matches!(ServiceType::ToadStool, ServiceType::ToadStool));
        assert!(matches!(ServiceType::Generic, ServiceType::Generic));

        // Test capability mapping
        assert_eq!(ServiceType::Songbird.to_capability(), "orchestration");
        assert_eq!(ServiceType::BearDog.to_capability(), "pki");
        assert_eq!(ServiceType::NestGate.to_capability(), "storage");

        // Test from_name migration helper
        assert!(matches!(
            ServiceType::from_name("songbird"),
            ServiceType::Songbird
        ));
        assert!(matches!(
            ServiceType::from_name("orchestration"),
            ServiceType::Songbird
        ));
        assert!(matches!(
            ServiceType::from_name("pki"),
            ServiceType::BearDog
        ));
        assert!(matches!(
            ServiceType::from_name("unknown"),
            ServiceType::Generic
        ));
    }

    #[test]
    #[allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService
    fn test_service_endpoint_creation() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: Arc::from("1.0.0"),
            capabilities: vec!["discovery".to_string(), "coordination".to_string()],
            trust_level: TrustLevel::Verified,
        };

        assert!(matches!(endpoint.service_type, EcosystemService::Songbird));
        assert_eq!(endpoint.version.as_ref(), "1.0.0");
        assert_eq!(endpoint.capabilities.len(), 2);
        assert!(matches!(endpoint.trust_level, TrustLevel::Verified));
    }

    #[test]
    #[allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService
    fn test_service_endpoint_serialization() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::BearDog,
            address: format!(
                "{}:{}",
                toadstool_common::constants::LOCALHOST_IPV4,
                6000 // Test fixture: deterministic port for serialization round-trip test
            )
            .parse()
            .unwrap(),
            version: Arc::from("2.0.0"),
            capabilities: vec!["auth".to_string()],
            trust_level: TrustLevel::Sovereign,
        };

        let json = serde_json::to_string(&endpoint).unwrap();
        let deserialized: ServiceEndpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version.as_ref(), "2.0.0");
        assert!(matches!(deserialized.trust_level, TrustLevel::Sovereign));
    }

    #[test]
    fn test_discovery_result_creation() {
        let result = DiscoveryResult {
            services: vec![],
            scan_duration: Duration::from_secs(5),
            total_discovered: 10,
            verified_count: 7,
        };

        assert_eq!(result.total_discovered, 10);
        assert_eq!(result.verified_count, 7);
        assert_eq!(result.scan_duration.as_secs(), 5);
    }

    #[test]
    fn test_beardog_permission_creation() {
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test-service".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
            signature: "test-signature".to_string(),
        };

        assert_eq!(permission.granted_to, "test-service");
        assert_eq!(permission.capabilities.len(), 2);
        assert!(!permission.signature.is_empty());
    }

    #[test]
    fn test_nestgate_mount_creation() {
        let mount = NestGateMount {
            dataset_name: "research-data".to_string(),
            mount_point: PathBuf::from("/mnt/data"),
            endpoint: "127.0.0.1:9000".to_string(),
            zfs_dataset: Some("tank/research".to_string()),
            access_mode: "read".to_string(),
            encryption_key: Some("key123".to_string()),
        };

        assert_eq!(mount.dataset_name, "research-data");
        assert_eq!(mount.access_mode, "read");
        assert!(mount.zfs_dataset.is_some());
    }

    #[allow(deprecated)] // Using ServiceType during migration
    #[test]
    fn test_discovered_service_creation() {
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "1.0.0".to_string());

        let service = DiscoveredService {
            service_type: ServiceType::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            trust_level: TrustLevel::Verified,
            capabilities,
            last_seen: std::time::SystemTime::now(),
        };

        assert!(matches!(service.service_type, ServiceType::Songbird));
        assert_eq!(service.capabilities.len(), 1);
    }

    #[test]
    fn test_service_signature_creation() {
        let signature = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "base64-signature".to_string(),
            public_key: "base64-public-key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "random-nonce".to_string(),
        };

        assert_eq!(signature.algorithm, "ed25519");
        assert!(!signature.nonce.is_empty());
    }

    #[test]
    fn test_signed_service_response_creation() {
        let signature = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig123".to_string(),
            public_key: "key123".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "nonce123".to_string(),
        };

        let response = SignedServiceResponse {
            service_id: "songbird-001".to_string(),
            service_type: "songbird".to_string(),
            status: "active".to_string(),
            capabilities: vec!["discovery".to_string()],
            timestamp: std::time::SystemTime::now(),
            signature,
        };

        assert_eq!(response.service_id, "songbird-001");
        assert_eq!(response.status, "active");
        assert_eq!(response.capabilities.len(), 1);
    }

    #[test]
    fn test_crypto_verification_context_default() {
        let context = CryptoVerificationContext::default();

        // Should have empty trusted keys if no env vars set
        assert_eq!(context.revoked_keys.len(), 0);
        assert_eq!(context.max_age_minutes, 5);
    }

    #[test]
    fn test_crypto_verification_context_with_key() {
        let context = CryptoVerificationContext::new().with_trusted_key("songbird", "test-key");

        assert!(context.trusted_public_keys.contains_key("songbird"));
        assert_eq!(
            context.trusted_public_keys.get("songbird"),
            Some(&"test-key".to_string())
        );
    }

    #[test]
    fn test_crypto_verification_context_multiple_keys() {
        let context = CryptoVerificationContext::new()
            .with_trusted_key("songbird", "key1")
            .with_trusted_key("beardog", "key2");

        assert_eq!(context.trusted_public_keys.len(), 2);
        assert!(context.trusted_public_keys.contains_key("songbird"));
        assert!(context.trusted_public_keys.contains_key("beardog"));
    }

    #[test]
    fn test_crypto_verification_context_custom_config() {
        let mut trusted_keys = HashMap::new();
        trusted_keys.insert("songbird".to_string(), "test-key-1".to_string());
        trusted_keys.insert("beardog".to_string(), "test-key-2".to_string());

        let context = CryptoVerificationContext {
            trusted_public_keys: trusted_keys,
            verification_timestamp: std::time::SystemTime::now(),
            revoked_keys: vec![],
            max_age_minutes: 60,
        };

        assert_eq!(context.trusted_public_keys.len(), 2);
        assert!(context.trusted_public_keys.contains_key("songbird"));
        assert_eq!(context.max_age_minutes, 60);
    }

    // ========================================================================
    // Week 14: Implementation Function Tests
    // ========================================================================

    #[test]
    fn test_ecosystem_integrator_new() {
        let integrator = EcosystemIntegrator::new();
        assert!(integrator.endpoints.is_empty());
        assert!(integrator.connections.is_empty());
        assert!(integrator.credentials.is_none());
    }

    #[test]
    fn test_ecosystem_integrator_default() {
        let integrator = EcosystemIntegrator::default();
        assert!(integrator.endpoints.is_empty());
        assert!(integrator.connections.is_empty());
    }

    // ✅ REMOVED: test_get_standard_service_ports_legacy (December 2, 2025)
    // The deprecated get_standard_service_ports() function has been removed.
    // Service discovery now uses PortRegistry and ServiceRegistry for dynamic configuration.

    #[test]
    fn test_create_permission_message() {
        // NOTE: This test is kept for backward compatibility but the underlying
        // implementation now uses capability-based crypto adapters.
        // The `BearDogPermission` type is maintained for legacy compatibility.

        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test-service".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
            signature: "test-signature".to_string(),
        };

        // Create canonical message (service-agnostic format)
        let mut data = std::collections::BTreeMap::new();
        data.insert("permission_id", permission.permission_id.to_string());
        data.insert("granted_to", permission.granted_to.clone());
        data.insert(
            "valid_until",
            toadstool_common::system_time_serde::format_rfc3339(permission.valid_until),
        );

        let capabilities_json = serde_json::to_string(&permission.capabilities).unwrap();
        data.insert("capabilities", capabilities_json);

        let canonical_json = serde_json::to_string(&data).unwrap();
        let message = canonical_json.into_bytes();

        assert!(!message.is_empty());

        // Verify message contains expected fields
        let parsed: serde_json::Value = serde_json::from_slice(&message).unwrap();
        assert!(parsed.get("permission_id").is_some());
        assert!(parsed.get("granted_to").is_some());
        assert!(parsed.get("capabilities").is_some());
        assert!(parsed.get("valid_until").is_some());
    }

    #[test]
    #[allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService
    fn test_discovery_result_with_services() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: Arc::from("1.0.0"),
            capabilities: vec!["discovery".to_string()],
            trust_level: TrustLevel::Verified,
        };

        let result = DiscoveryResult {
            services: vec![endpoint],
            scan_duration: Duration::from_secs(3),
            total_discovered: 1,
            verified_count: 1,
        };

        assert_eq!(result.services.len(), 1);
        assert_eq!(result.total_discovered, 1);
        assert_eq!(result.verified_count, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discover_services_empty() {
        let mut integrator = EcosystemIntegrator::new();

        // With 1 second timeout, should quickly return no services
        let result = integrator.discover_services(vec![], 1).await;

        // May timeout or succeed with empty list depending on network
        if let Ok(_discovery) = result {
            // total_discovered is unsigned, always >= 0
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_with_orchestrator_invalid_endpoint() {
        let mut integrator = EcosystemIntegrator::new();

        // Invalid endpoint format
        let result = integrator
            .register_with_orchestrator("invalid-endpoint".to_string(), None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_install_crypto_permissions_nonexistent_file() {
        let mut integrator = EcosystemIntegrator::new();

        // Nonexistent file should return error
        let result = integrator
            .install_crypto_permissions(PathBuf::from("/nonexistent/permissions.json"), false)
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_beardog_permission_expiration() {
        let now = std::time::SystemTime::now();
        let future = now + std::time::Duration::from_secs(2 * 3600);

        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test".to_string(),
            capabilities: vec!["read".to_string()],
            valid_until: future,
            signature: "sig".to_string(),
        };

        assert!(permission.valid_until > now);
        assert!(permission.valid_until > std::time::SystemTime::now());
    }

    #[test]
    fn test_nestgate_mount_with_encryption() {
        let mount = NestGateMount {
            dataset_name: "encrypted-data".to_string(),
            mount_point: PathBuf::from("/mnt/secure"),
            endpoint: "127.0.0.1:9000".to_string(),
            zfs_dataset: Some("tank/encrypted".to_string()),
            access_mode: "readwrite".to_string(),
            encryption_key: Some("super-secret-key-123".to_string()),
        };

        assert!(mount.encryption_key.is_some());
        assert_eq!(mount.access_mode, "readwrite");
        assert!(mount.zfs_dataset.is_some());
    }

    #[test]
    fn test_nestgate_mount_without_encryption() {
        let mount = NestGateMount {
            dataset_name: "public-data".to_string(),
            mount_point: PathBuf::from("/mnt/public"),
            endpoint: "127.0.0.1:9001".to_string(),
            zfs_dataset: None,
            access_mode: "read".to_string(),
            encryption_key: None,
        };

        assert!(mount.encryption_key.is_none());
        assert_eq!(mount.access_mode, "read");
    }

    #[test]
    fn test_service_signature_timestamp_validation() {
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let signature = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig".to_string(),
            public_key: "key".to_string(),
            timestamp: old_time,
            nonce: "nonce".to_string(),
        };

        let age_minutes = std::time::SystemTime::now()
            .duration_since(signature.timestamp)
            .map(|d| d.as_secs() / 60)
            .unwrap_or(0);
        assert!(age_minutes >= 59); // Should be about an hour old
    }

    #[test]
    fn test_discovery_result_verification_ratio() {
        let result = DiscoveryResult {
            services: vec![],
            scan_duration: Duration::from_secs(10),
            total_discovered: 10,
            verified_count: 7,
        };

        let ratio = result.verified_count as f64 / result.total_discovered as f64;
        assert_eq!(ratio, 0.7); // 70% verification rate
    }

    #[test]
    fn test_discovery_result_all_verified() {
        let result = DiscoveryResult {
            services: vec![],
            scan_duration: Duration::from_secs(5),
            total_discovered: 5,
            verified_count: 5,
        };

        assert_eq!(result.verified_count, result.total_discovered);
    }

    #[test]
    fn test_discovery_result_none_verified() {
        let result = DiscoveryResult {
            services: vec![],
            scan_duration: Duration::from_secs(30),
            total_discovered: 10,
            verified_count: 0,
        };

        assert_eq!(result.verified_count, 0);
        assert!(result.total_discovered > 0);
    }
}

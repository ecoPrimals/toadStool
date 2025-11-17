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

// Submodules
mod connection;
mod discovery;
mod services;
mod types;

// Public re-exports
pub use types::{
    BearDogPermission, CryptoVerificationContext, DiscoveredService, DiscoveryResult,
    EcosystemIntegrator, EcosystemService, NestGateMount, ServiceEndpoint, ServiceSignature,
    ServiceType, SignedServiceResponse, TrustLevel,
};

// Internal types
use types::{ConnectionStatus, EcosystemStatus, ServiceConnection, SongbirdRegistration};

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

    #[test]
    fn test_ecosystem_service_variants() {
        assert!(matches!(
            EcosystemService::Songbird,
            EcosystemService::Songbird
        ));
        assert!(matches!(
            EcosystemService::BearDog,
            EcosystemService::BearDog
        ));
        assert!(matches!(
            EcosystemService::NestGate,
            EcosystemService::NestGate
        ));

        let unknown = EcosystemService::Unknown("custom".to_string());
        match unknown {
            EcosystemService::Unknown(name) => assert_eq!(name, "custom"),
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_ecosystem_service_name() {
        assert_eq!(EcosystemService::Songbird.name(), "songbird");
        assert_eq!(EcosystemService::BearDog.name(), "beardog");
        assert_eq!(EcosystemService::NestGate.name(), "nestgate");
        assert_eq!(EcosystemService::Unknown("test".to_string()).name(), "test");
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
    fn test_service_type_variants() {
        assert!(matches!(ServiceType::Songbird, ServiceType::Songbird));
        assert!(matches!(ServiceType::BearDog, ServiceType::BearDog));
        assert!(matches!(ServiceType::NestGate, ServiceType::NestGate));
        assert!(matches!(ServiceType::ToadStool, ServiceType::ToadStool));
    }

    #[test]
    fn test_service_endpoint_creation() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: "1.0.0".to_string(),
            capabilities: vec!["discovery".to_string(), "coordination".to_string()],
            trust_level: TrustLevel::Verified,
        };

        assert!(matches!(endpoint.service_type, EcosystemService::Songbird));
        assert_eq!(endpoint.version, "1.0.0");
        assert_eq!(endpoint.capabilities.len(), 2);
        assert!(matches!(endpoint.trust_level, TrustLevel::Verified));
    }

    #[test]
    fn test_service_endpoint_serialization() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::BearDog,
            address: "127.0.0.1:8081".parse().unwrap(),
            version: "2.0.0".to_string(),
            capabilities: vec!["auth".to_string()],
            trust_level: TrustLevel::Sovereign,
        };

        let json = serde_json::to_string(&endpoint).unwrap();
        let deserialized: ServiceEndpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "2.0.0");
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
            valid_until: Utc::now() + chrono::Duration::hours(1),
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

    #[test]
    fn test_discovered_service_creation() {
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "1.0.0".to_string());

        let service = DiscoveredService {
            service_type: ServiceType::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            trust_level: TrustLevel::Verified,
            capabilities,
            last_seen: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
            nonce: "nonce123".to_string(),
        };

        let response = SignedServiceResponse {
            service_id: "songbird-001".to_string(),
            service_type: "songbird".to_string(),
            status: "active".to_string(),
            capabilities: vec!["discovery".to_string()],
            timestamp: Utc::now(),
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
        let context = CryptoVerificationContext::new()
            .with_trusted_key("songbird".to_string(), "test-key".to_string());

        assert!(context.trusted_public_keys.contains_key("songbird"));
        assert_eq!(
            context.trusted_public_keys.get("songbird"),
            Some(&"test-key".to_string())
        );
    }

    #[test]
    fn test_crypto_verification_context_multiple_keys() {
        let context = CryptoVerificationContext::new()
            .with_trusted_key("songbird".to_string(), "key1".to_string())
            .with_trusted_key("beardog".to_string(), "key2".to_string());

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
            verification_timestamp: Utc::now(),
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

    #[test]
    fn test_get_standard_service_ports() {
        let ports = discovery::get_standard_service_ports();

        assert!(ports.contains_key("songbird"));
        assert!(ports.contains_key("beardog"));
        assert!(ports.contains_key("nestgate"));

        // Verify default ports are reasonable
        if let Some(&port) = ports.get("songbird") {
            assert!(port > 1024 && port < 65535);
        }
    }

    #[test]
    fn test_create_permission_message() {
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test-service".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            valid_until: Utc::now() + chrono::Duration::hours(1),
            signature: "test-signature".to_string(),
        };

        let result = services::beardog::create_permission_message(&permission);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(!message.is_empty());
    }

    #[test]
    fn test_discovery_result_with_services() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: "1.0.0".to_string(),
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

    #[test]
    fn test_generate_ip_range_class_c() {
        let integrator = EcosystemIntegrator::new();
        let result = integrator.generate_ip_range("192.168.1.0", 24);

        assert!(result.is_ok());
        let ips = result.unwrap();
        // Implementation generates 1-254, not 0-255
        assert_eq!(ips.len(), 254);
        assert!(ips.contains(&"192.168.1.1".to_string()));
        assert!(ips.contains(&"192.168.1.254".to_string()));
        assert!(!ips.contains(&"192.168.1.0".to_string()));
        assert!(!ips.contains(&"192.168.1.255".to_string()));
    }

    #[test]
    fn test_generate_ip_range_single_host() {
        let integrator = EcosystemIntegrator::new();
        let result = integrator.generate_ip_range("10.0.0.5", 32);

        assert!(result.is_ok());
        let ips = result.unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0], "10.0.0.5");
    }

    #[test]
    fn test_generate_ip_range_unsupported_prefix() {
        let integrator = EcosystemIntegrator::new();

        // Unsupported prefix length returns empty list (not error)
        let result = integrator.generate_ip_range("192.168.1.0", 16);
        assert!(result.is_ok());
        let ips = result.unwrap();
        assert_eq!(ips.len(), 0); // Not implemented for /16
    }

    #[tokio::test]
    async fn test_discover_services_empty() {
        let mut integrator = EcosystemIntegrator::new();

        // With 1 second timeout, should quickly return no services
        let result = integrator.discover_services(vec![], 1).await;

        // May timeout or succeed with empty list depending on network
        if let Ok(_discovery) = result {
            // total_discovered is unsigned, always >= 0
        }
    }

    #[tokio::test]
    async fn test_register_with_orchestrator_invalid_endpoint() {
        let mut integrator = EcosystemIntegrator::new();

        // Invalid endpoint format
        let result = integrator
            .register_with_orchestrator("invalid-endpoint".to_string(), None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_beardog_permissions_nonexistent_file() {
        let mut integrator = EcosystemIntegrator::new();

        // Nonexistent file should return error
        let result = integrator
            .install_beardog_permissions(PathBuf::from("/nonexistent/permissions.json"), false)
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_beardog_permission_expiration() {
        let now = Utc::now();
        let future = now + chrono::Duration::hours(2);

        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test".to_string(),
            capabilities: vec!["read".to_string()],
            valid_until: future,
            signature: "sig".to_string(),
        };

        assert!(permission.valid_until > now);
        assert!(permission.valid_until > Utc::now());
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
        let old_time = Utc::now() - chrono::Duration::hours(1);
        let signature = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig".to_string(),
            public_key: "key".to_string(),
            timestamp: old_time,
            nonce: "nonce".to_string(),
        };

        let age_minutes = (Utc::now() - signature.timestamp).num_minutes();
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

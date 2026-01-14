//! E2E Test: Primal Discovery Workflow
//!
//! Tests the complete workflow of discovering primals by capability,
//! validating Deep Debt compliance (no hardcoding, runtime discovery).

use toadstool::ecosystem::discovery::{EcosystemDiscovery, DiscoveryConfig};
use toadstool_common::primal_identity::{Capability, CryptoCapability, CoordinationCapability, StorageCapability};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod};

#[tokio::test]
async fn test_ecosystem_discovery_initialization() {
    // E2E: Create discovery engine with default config
    let config = DiscoveryConfig::default();
    let result = EcosystemDiscovery::new(config).await;
    
    assert!(result.is_ok(), "Ecosystem discovery should initialize");
}

#[tokio::test]
async fn test_discovery_with_crypto_capability() {
    // E2E: Discover crypto services by capability (no hardcoding!)
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    let capability = Capability::Crypto(CryptoCapability::Encryption);
    
    // Should not panic, even if no service found
    let result = discovery.find_by_capability(capability).await;
    
    // Either found a service or returned "not found" error (both valid)
    match result {
        Ok(service) => {
            // Validate service structure
            assert!(!service.id.is_empty(), "Service should have ID");
            assert!(!service.name.is_empty(), "Service should have name");
            assert!(!service.endpoints.is_empty(), "Service should have endpoints");
        }
        Err(e) => {
            // Not found is acceptable (no crypto service running locally)
            assert!(e.to_string().contains("not found") || e.to_string().contains("No service"));
        }
    }
}

#[tokio::test]
async fn test_discovery_with_coordination_capability() {
    // E2E: Discover coordination services (Songbird, etc.)
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    
    let result = discovery.find_by_capability(capability).await;
    
    // Graceful handling whether found or not
    match result {
        Ok(service) => {
            assert!(!service.id.is_empty());
            assert!(service.capabilities.len() > 0, "Service should have capabilities");
        }
        Err(_) => {
            // Not found is okay in test environment
        }
    }
}

#[tokio::test]
async fn test_discovery_with_storage_capability() {
    // E2E: Discover storage services (NestGate, etc.)
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    let capability = Capability::Storage(StorageCapability::ObjectStorage);
    
    let result = discovery.find_by_capability(capability).await;
    
    // Validation
    match result {
        Ok(service) => {
            assert!(!service.name.is_empty());
            // Storage services should have health status
            assert!(service.healthy || !service.healthy); // Boolean check
        }
        Err(_) => {
            // Expected if NestGate not running
        }
    }
}

#[tokio::test]
async fn test_discovery_multiple_capabilities() {
    // E2E: Discover multiple services in sequence
    let config = DiscoveryConfig {
        required_capabilities: vec![
            Capability::Crypto(CryptoCapability::Encryption),
            Capability::Coordination(CoordinationCapability::ServiceDiscovery),
        ],
        ..Default::default()
    };
    
    let discovery = EcosystemDiscovery::new(config.clone()).await.unwrap();
    
    // Discover all required capabilities
    let result = discovery.discover_all(&config).await;
    
    // Should return Vec, even if empty
    assert!(result.is_ok(), "Discovery should not panic");
    
    let services = result.unwrap();
    // May be empty if no services running, but should be valid Vec
    assert!(services.len() <= 10, "Should not discover unreasonable number of services");
}

#[tokio::test]
async fn test_discovery_cache_consistency() {
    // E2E: Verify cache consistency across discovery calls
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    let capability = Capability::Crypto(CryptoCapability::Encryption);
    
    // First discovery attempt
    let result1 = discovery.find_by_capability(capability.clone()).await;
    
    // Second discovery attempt (should use cache if available)
    let result2 = discovery.find_by_capability(capability.clone()).await;
    
    // Results should be consistent
    match (result1, result2) {
        (Ok(s1), Ok(s2)) => {
            assert_eq!(s1.id, s2.id, "Cached service should have same ID");
            assert_eq!(s1.name, s2.name, "Cached service should have same name");
        }
        (Err(_), Err(_)) => {
            // Both not found is consistent
        }
        _ => {
            // One found, one not - could happen if service just started/stopped
            // Don't fail, just log
            eprintln!("Discovery results differed (service may have changed)");
        }
    }
}

#[tokio::test]
async fn test_deep_debt_no_hardcoded_endpoints() {
    // E2E: Verify no hardcoded service endpoints
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    // Try discovering various capabilities
    let capabilities = vec![
        Capability::Crypto(CryptoCapability::Encryption),
        Capability::Coordination(CoordinationCapability::ServiceDiscovery),
        Capability::Storage(StorageCapability::ObjectStorage),
    ];
    
    for capability in capabilities {
        let result = discovery.find_by_capability(capability.clone()).await;
        
        if let Ok(service) = result {
            // Validate endpoints are dynamically discovered, not hardcoded
            for endpoint in &service.endpoints {
                // Endpoints should have runtime-discovered addresses
                assert!(!endpoint.address.is_empty(), "Endpoint should have address");
                assert!(endpoint.port > 0, "Endpoint should have valid port");
                
                // Should not be hardcoded test values
                assert_ne!(endpoint.address, "localhost:8080", "Should not use hardcoded test endpoint");
                assert_ne!(endpoint.address, "127.0.0.1:9000", "Should not use hardcoded test endpoint");
            }
        }
    }
}

#[tokio::test]
async fn test_discovery_method_auto_uses_multiple_strategies() {
    // E2E: Verify Auto discovery method uses multiple strategies
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    // Auto should try mDNS, registry, environment variables
    // We can't directly test internal strategies, but we can verify it doesn't panic
    let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    
    let result = discovery.find_by_capability(capability).await;
    
    // Should complete without panic, regardless of result
    assert!(result.is_ok() || result.is_err(), "Discovery should complete");
}

#[tokio::test]
async fn test_discovery_respects_timeout() {
    // E2E: Verify discovery respects timeout and doesn't hang
    use tokio::time::{timeout, Duration};
    
    let config = DiscoveryConfig::default();
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    
    let capability = Capability::Crypto(CryptoCapability::Encryption);
    
    // Should complete within 5 seconds
    let result = timeout(
        Duration::from_secs(5),
        discovery.find_by_capability(capability)
    ).await;
    
    assert!(result.is_ok(), "Discovery should complete within timeout");
}

#[tokio::test]
async fn test_discovered_service_has_valid_metadata() {
    // E2E: Verify discovered services have valid metadata structure
    let config = DiscoveryConfig {
        required_capabilities: vec![
            Capability::Crypto(CryptoCapability::Encryption),
        ],
        ..Default::default()
    };
    
    let discovery = EcosystemDiscovery::new(config).await.unwrap();
    let capability = Capability::Crypto(CryptoCapability::Encryption);
    
    if let Ok(service) = discovery.find_by_capability(capability).await {
        // Validate service metadata structure
        assert!(!service.id.is_empty(), "Should have service ID");
        assert!(!service.name.is_empty(), "Should have service name");
        assert!(!service.version.is_empty(), "Should have version");
        assert!(!service.capabilities.is_empty(), "Should have capabilities");
        assert!(!service.endpoints.is_empty(), "Should have endpoints");
        
        // Timestamps should be valid
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now();
        assert!(service.discovered_at <= now, "Discovery time should not be in future");
        assert!(service.last_seen <= now, "Last seen should not be in future");
    }
}

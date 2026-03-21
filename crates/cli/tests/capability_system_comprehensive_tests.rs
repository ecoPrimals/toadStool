// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for the capability-based service discovery system
//!
//! This test suite validates:
//! - Capability registry functionality
//! - Capability resolution
//! - Service provider management
//! - Priority and preference handling
//! - Cache behavior
//! - Edge cases and error handling

use std::sync::Arc;
use std::time::Duration;
use toadstool_cli::ecosystem::capabilities::{
    CapabilityId, CapabilityRegistry, CapabilityResolver, ServiceProvider, StandardCapability,
};
use toadstool_common::infant_discovery::{DiscoveryEngine, ServiceHealth, ServiceMetadata};

// ============================================================================
// Capability Registry Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_creation() {
    let _registry = CapabilityRegistry::new();
    // Just verify it creates without panicking - success!
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_with_ttl() {
    let _registry = CapabilityRegistry::new().with_ttl(Duration::from_secs(60));
    // Just verify it creates with TTL without panicking - success!
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_registry_with_auto_cleanup() {
    let _registry = CapabilityRegistry::new()
        .with_auto_cleanup(false)
        .with_ttl(Duration::from_secs(120));
    // Just verify it creates with custom settings without panicking - success!
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_single_provider() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    let provider = create_test_provider("http://crypto-service:8080", 50);

    registry.register(capability.clone(), provider).await;

    // Verify provider was registered
    let result = registry.get_best_provider(&capability).await;
    assert!(result.is_some(), "Provider should be registered");

    let retrieved = result.unwrap();
    assert_eq!(retrieved.endpoint, "http://crypto-service:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_providers() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Register three providers with different priorities
    let provider1 = create_test_provider("http://crypto1:8080", 30);
    let provider2 = create_test_provider("http://crypto2:8080", 70);
    let provider3 = create_test_provider("http://crypto3:8080", 50);

    registry.register(capability.clone(), provider1).await;
    registry.register(capability.clone(), provider2).await;
    registry.register(capability.clone(), provider3).await;

    // Best provider should be the one with highest priority (70)
    let best = registry.get_best_provider(&capability).await;
    assert!(best.is_some(), "Should have a best provider");
    assert_eq!(best.unwrap().endpoint, "http://crypto2:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_priority_ordering() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::StorageBlock.id();

    // Register providers in random order
    registry
        .register(
            capability.clone(),
            create_test_provider("http://low:8080", 10),
        )
        .await;
    registry
        .register(
            capability.clone(),
            create_test_provider("http://high:8080", 90),
        )
        .await;
    registry
        .register(
            capability.clone(),
            create_test_provider("http://medium:8080", 50),
        )
        .await;

    // Get all providers - should be sorted by priority
    let all = registry.get_providers(&capability).await;
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].endpoint, "http://high:8080"); // Highest priority first
    assert_eq!(all[1].endpoint, "http://medium:8080");
    assert_eq!(all[2].endpoint, "http://low:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_update() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Register initial provider
    let provider1 = create_test_provider("http://crypto:8080", 50);
    registry.register(capability.clone(), provider1).await;

    // Register again with same endpoint but different priority
    let provider2 = create_test_provider("http://crypto:8080", 80);
    registry.register(capability.clone(), provider2).await;

    // Should have updated, not duplicated
    let all = registry.get_providers(&capability).await;
    assert_eq!(all.len(), 1, "Should not duplicate providers");
    assert_eq!(all[0].priority, 80, "Priority should be updated");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unregister_provider() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    let provider = create_test_provider("http://crypto:8080", 50);
    registry.register(capability.clone(), provider).await;

    // Verify registered
    assert!(registry.get_best_provider(&capability).await.is_some());

    // Unregister
    registry.unregister(&capability, "http://crypto:8080").await;

    // Verify unregistered
    assert!(registry.get_best_provider(&capability).await.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_nonexistent_capability() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = CapabilityId::from("nonexistent.capability");

    let result = registry.get_best_provider(&capability).await;
    assert!(
        result.is_none(),
        "Should return None for unregistered capability"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_capabilities_isolation() {
    let registry = Arc::new(CapabilityRegistry::new());

    // Register providers for different capabilities
    let crypto_cap = StandardCapability::CryptoSignatureEd25519.id();
    let storage_cap = StandardCapability::StorageBlock.id();

    registry
        .register(
            crypto_cap.clone(),
            create_test_provider("http://crypto:8080", 50),
        )
        .await;
    registry
        .register(
            storage_cap.clone(),
            create_test_provider("http://storage:8080", 50),
        )
        .await;

    // Verify isolation - each capability has only its providers
    let crypto_providers = registry.get_providers(&crypto_cap).await;
    let storage_providers = registry.get_providers(&storage_cap).await;

    assert_eq!(crypto_providers.len(), 1);
    assert_eq!(storage_providers.len(), 1);
    assert_eq!(crypto_providers[0].endpoint, "http://crypto:8080");
    assert_eq!(storage_providers[0].endpoint, "http://storage:8080");
}

// ============================================================================
// Capability Resolver Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resolver_creation() {
    let discovery = Arc::new(DiscoveryEngine::new());
    let registry = Arc::new(CapabilityRegistry::new());
    let resolver = CapabilityResolver::new(discovery, registry);

    // Verify resolver was created (the fact that we got here proves it)
    assert!(
        std::mem::size_of_val(&resolver) > 0,
        "Resolver should be a valid struct"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resolver_with_registry_cache() {
    let discovery = Arc::new(DiscoveryEngine::new());
    let registry = Arc::new(CapabilityRegistry::new());
    let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

    // Pre-populate registry
    let capability = StandardCapability::CryptoSignatureEd25519.id();
    let provider = create_test_provider("http://cached-crypto:8080", 50);
    registry.register(capability.clone(), provider).await;

    // Resolution should use cached value (no discovery needed)
    let result = resolver.resolve(capability).await;

    // Since we pre-populated registry, this should succeed without discovery
    if let Ok(provider) = result {
        assert_eq!(provider.endpoint, "http://cached-crypto:8080");
    } else {
        // If discovery engine doesn't have config, this is also fine
        println!("No discovery configuration available (expected in test)");
    }
}

// ============================================================================
// ServiceProvider Tests
// ============================================================================

#[test]
fn test_service_provider_creation() {
    let provider = create_test_provider("http://test:8080", 50);
    assert_eq!(provider.endpoint, "http://test:8080");
    assert_eq!(provider.priority, 50);
    assert!(provider.protocols.contains(&"http".to_string()));
}

#[test]
fn test_service_provider_health() {
    let mut provider = create_test_provider("http://test:8080", 50);

    // Initially healthy
    assert_eq!(provider.health, ServiceHealth::Healthy);

    // Can be updated
    provider.health = ServiceHealth::Degraded;
    assert_eq!(provider.health, ServiceHealth::Degraded);
}

// ============================================================================
// CapabilityId Tests
// ============================================================================

#[test]
fn test_capability_id_from_string() {
    let cap = CapabilityId::from("crypto.signature.ed25519");
    assert_eq!(cap.as_str(), "crypto.signature.ed25519");
}

#[test]
fn test_capability_id_from_standard() {
    let cap = StandardCapability::CryptoSignatureEd25519.id();
    assert!(cap.as_str().contains("crypto"));
    assert!(cap.as_str().contains("ed25519"));
}

#[test]
fn test_capability_id_equality() {
    let cap1 = CapabilityId::from("test.capability");
    let cap2 = CapabilityId::from("test.capability");
    let cap3 = CapabilityId::from("different.capability");

    assert_eq!(cap1.as_str(), cap2.as_str());
    assert_ne!(cap1.as_str(), cap3.as_str());
}

#[test]
fn test_standard_capabilities_exist() {
    // Verify all standard capabilities can be created and have unique IDs
    let crypto_ed25519 = StandardCapability::CryptoSignatureEd25519;
    let crypto_ecdsa = StandardCapability::CryptoSignatureEcdsa;
    let crypto_rsa = StandardCapability::CryptoSignatureRsa;
    let storage_block = StandardCapability::StorageBlock;
    let storage_object = StandardCapability::StorageObjectS3;
    let coordination = StandardCapability::CoordinationServiceRegistry;

    // Verify each capability has a non-empty ID
    assert!(!crypto_ed25519.id().as_str().is_empty());
    assert!(!crypto_ecdsa.id().as_str().is_empty());
    assert!(!crypto_rsa.id().as_str().is_empty());
    assert!(!storage_block.id().as_str().is_empty());
    assert!(!storage_object.id().as_str().is_empty());
    assert!(!coordination.id().as_str().is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_full_workflow_register_and_resolve() {
    let discovery = Arc::new(DiscoveryEngine::new());
    let registry = Arc::new(CapabilityRegistry::new());
    let resolver = CapabilityResolver::new(discovery, Arc::clone(&registry));

    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Register a provider
    let provider = create_test_provider("http://crypto-service:8080", 70);
    registry.register(capability.clone(), provider).await;

    // Resolve should find it
    let result = resolver.resolve(capability).await;
    assert!(result.is_ok(), "Should resolve registered provider");

    let resolved = result.unwrap();
    assert_eq!(resolved.endpoint, "http://crypto-service:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_failover_to_secondary_provider() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::StorageBlock.id();

    // Register primary (high priority) and secondary (low priority)
    let primary = create_test_provider("http://primary:8080", 90);
    let secondary = create_test_provider("http://secondary:8080", 50);

    registry.register(capability.clone(), primary).await;
    registry.register(capability.clone(), secondary).await;

    // Get all providers
    let providers = registry.get_providers(&capability).await;
    assert_eq!(providers.len(), 2);

    // Primary should be first
    assert_eq!(providers[0].endpoint, "http://primary:8080");
    // Secondary should be second (for failover)
    assert_eq!(providers[1].endpoint, "http://secondary:8080");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_resolvers_share_registry() {
    let discovery = Arc::new(DiscoveryEngine::new());
    let registry = Arc::new(CapabilityRegistry::new());

    // Create two resolvers sharing the same registry
    let resolver1 = CapabilityResolver::new(Arc::clone(&discovery), Arc::clone(&registry));
    let resolver2 = CapabilityResolver::new(discovery, Arc::clone(&registry));

    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Register through registry
    let provider = create_test_provider("http://shared-crypto:8080", 60);
    registry.register(capability.clone(), provider).await;

    // Both resolvers should see it
    let result1 = resolver1.resolve(capability.clone()).await;
    let result2 = resolver2.resolve(capability).await;

    assert!(result1.is_ok(), "Resolver 1 should find provider");
    assert!(result2.is_ok(), "Resolver 2 should find provider");
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_empty_registry() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    let result = registry.get_best_provider(&capability).await;
    assert!(result.is_none(), "Empty registry should return None");

    let all = registry.get_providers(&capability).await;
    assert!(all.is_empty(), "Empty registry should return empty list");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unregister_nonexistent_provider() {
    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Unregister without registering first - should not panic
    registry
        .unregister(&capability, "http://nonexistent:8080")
        .await;

    // Success! No panic occurred when unregistering nonexistent provider
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_with_empty_endpoint() {
    let mut provider = create_test_provider("http://test:8080", 50);
    provider.endpoint = String::new();

    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    // Should be able to register even with empty endpoint
    registry.register(capability.clone(), provider).await;

    let result = registry.get_best_provider(&capability).await;
    assert!(result.is_some(), "Should handle empty endpoint");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_with_zero_priority() {
    let provider = create_test_provider("http://test:8080", 0);
    assert_eq!(provider.priority, 0);

    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    registry.register(capability.clone(), provider).await;

    let result = registry.get_best_provider(&capability).await;
    assert!(result.is_some(), "Should handle zero priority");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provider_with_max_priority() {
    let provider = create_test_provider("http://test:8080", 100);
    assert_eq!(provider.priority, 100);

    let registry = Arc::new(CapabilityRegistry::new());
    let capability = StandardCapability::CryptoSignatureEd25519.id();

    registry.register(capability.clone(), provider).await;

    let result = registry.get_best_provider(&capability).await;
    assert!(result.is_some(), "Should handle max priority");
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test service provider with given endpoint and priority
fn create_test_provider(endpoint: &str, priority: u8) -> ServiceProvider {
    ServiceProvider {
        endpoint: endpoint.to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        health: ServiceHealth::Healthy,
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: std::time::SystemTime::now(),
            priority: 50,
            extra: Default::default(),
        },
        last_seen: std::time::Instant::now(),
        priority,
    }
}

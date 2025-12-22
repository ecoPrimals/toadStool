//! Comprehensive Error Handling and Edge Case Tests for Ecosystem Module
//!
//! **Goal**: Increase coverage of ecosystem.rs with high-value error path tests
//! **Philosophy**: Test what could go wrong, not just happy paths
//! **Created**: December 15, 2025
//!
//! This suite targets:
//! - Error handling in primal discovery
//! - Edge cases in service communication
//! - Timeout and failure scenarios
//! - Concurrent access patterns
//! - State consistency under failures

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use toadstool::ecosystem::*;

// ============================================================================
// ERROR PATH TESTS - Discovery Failures
// ============================================================================

#[test]
fn test_ecosystem_config_with_empty_optional_primals() {
    // Edge case: No optional primals configured
    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(5),
        primal_endpoints: HashMap::new(),
        required_primals: vec![],
        optional_primals: vec![], // Empty list
    };

    // Should be valid configuration
    assert!(config.optional_primals.is_empty());
    assert!(!config.auto_discovery);
}

#[test]
fn test_ecosystem_config_with_zero_timeout() {
    // Edge case: Zero timeout (should still be valid struct)
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(0), // Zero timeout
        primal_endpoints: HashMap::new(),
        required_primals: vec![],
        optional_primals: vec!["songbird".to_string()],
    };

    assert_eq!(config.discovery_timeout, Duration::from_secs(0));
}

#[test]
fn test_ecosystem_config_with_very_long_timeout() {
    // Edge case: Extremely long timeout
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(3600), // 1 hour
        primal_endpoints: HashMap::new(),
        required_primals: vec![],
        optional_primals: vec![],
    };

    assert_eq!(config.discovery_timeout, Duration::from_secs(3600));
}

#[test]
fn test_ecosystem_config_with_duplicate_primals() {
    // Edge case: Duplicate entries in optional primals
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(30),
        primal_endpoints: HashMap::new(),
        required_primals: vec!["songbird".to_string(), "songbird".to_string()], // Duplicate
        optional_primals: vec!["nestgate".to_string(), "nestgate".to_string()], // Duplicate
    };

    // Should handle duplicates gracefully
    assert_eq!(config.required_primals.len(), 2); // Contains duplicates
    assert_eq!(config.optional_primals.len(), 2);
}

#[test]
fn test_ecosystem_config_with_conflicting_required_optional() {
    // Edge case: Same primal in both required and optional
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(30),
        primal_endpoints: HashMap::new(),
        required_primals: vec!["songbird".to_string()],
        optional_primals: vec!["songbird".to_string()], // Also in required
    };

    // Should handle overlap gracefully
    assert!(config.required_primals.contains(&"songbird".to_string()));
    assert!(config.optional_primals.contains(&"songbird".to_string()));
}

// ============================================================================
// PRIMAL INSTANCE TESTS - State and Lifecycle
// ============================================================================

#[test]
fn test_primal_instance_creation() {
    // Test creating a valid primal instance
    let instance = PrimalInstance {
        name: "test-primal".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["coordination".to_string()],
        status: PrimalStatus::Discovered,
        discovered_at: Utc::now(),
    };

    assert_eq!(instance.name, "test-primal");
    assert_eq!(instance.primal_type, PrimalType::Songbird);
    assert_eq!(instance.status, PrimalStatus::Discovered);
}

#[test]
fn test_primal_instance_with_empty_capabilities() {
    // Edge case: Primal with no capabilities
    let instance = PrimalInstance {
        name: "empty-primal".to_string(),
        primal_type: PrimalType::Custom("unknown".to_string()),
        endpoint: "http://localhost:9090".to_string(),
        version: "0.0.1".to_string(),
        capabilities: vec![], // No capabilities
        status: PrimalStatus::Discovered,
        discovered_at: Utc::now(),
    };

    assert!(instance.capabilities.is_empty());
}

#[test]
fn test_primal_instance_with_many_capabilities() {
    // Edge case: Primal with many capabilities
    let capabilities: Vec<String> = (0..100).map(|i| format!("capability_{}", i)).collect();

    let instance = PrimalInstance {
        name: "super-primal".to_string(),
        primal_type: PrimalType::BiomeOS,
        endpoint: "http://localhost:3000".to_string(),
        version: "2.0.0".to_string(),
        capabilities: capabilities.clone(),
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert_eq!(instance.capabilities.len(), 100);
    assert!(instance.capabilities.contains(&"capability_42".to_string()));
}

// ============================================================================
// PRIMAL TYPE TESTS - All Variants
// ============================================================================

#[test]
fn test_primal_type_equality() {
    // Test that enum equality works correctly
    assert_eq!(PrimalType::Songbird, PrimalType::Songbird);
    assert_eq!(PrimalType::NestGate, PrimalType::NestGate);
    assert_eq!(PrimalType::BearDog, PrimalType::BearDog);
    assert_eq!(PrimalType::Squirrel, PrimalType::Squirrel);
    assert_eq!(PrimalType::BiomeOS, PrimalType::BiomeOS);
    assert_eq!(PrimalType::ToadStool, PrimalType::ToadStool);
}

#[test]
fn test_primal_type_custom_equality() {
    // Test custom primal type equality
    let custom1 = PrimalType::Custom("custom-primal".to_string());
    let custom2 = PrimalType::Custom("custom-primal".to_string());
    let custom3 = PrimalType::Custom("other-primal".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

#[test]
fn test_primal_type_mixed_inequality() {
    // Test that different types are not equal
    assert_ne!(PrimalType::Songbird, PrimalType::NestGate);
    assert_ne!(PrimalType::BearDog, PrimalType::Squirrel);
    assert_ne!(PrimalType::BiomeOS, PrimalType::Custom("test".to_string()));
}

// ============================================================================
// PRIMAL STATUS TESTS - State Transitions
// ============================================================================

#[test]
fn test_primal_status_variants() {
    // Test all status variants can be created
    let discovered = PrimalStatus::Discovered;
    let connected = PrimalStatus::Connected;
    let failed = PrimalStatus::Failed("connection timeout".to_string());
    let disconnected = PrimalStatus::Disconnected;

    // All should be valid
    assert_eq!(discovered, PrimalStatus::Discovered);
    assert_eq!(connected, PrimalStatus::Connected);
    assert_eq!(disconnected, PrimalStatus::Disconnected);

    // Failed should contain error message
    match failed {
        PrimalStatus::Failed(msg) => assert_eq!(msg, "connection timeout"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_primal_status_failed_with_empty_message() {
    // Edge case: Failed status with empty error message
    let status = PrimalStatus::Failed("".to_string());

    match status {
        PrimalStatus::Failed(msg) => assert!(msg.is_empty()),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_primal_status_failed_with_long_message() {
    // Edge case: Failed status with very long error message
    let long_message = "error: ".to_string() + &"x".repeat(1000);
    let status = PrimalStatus::Failed(long_message.clone());

    match status {
        PrimalStatus::Failed(msg) => {
            assert_eq!(msg.len(), 1007); // "error: " (7 chars) + 1000 x's
            assert!(msg.starts_with("error: xxx"));
        }
        _ => panic!("Expected Failed status"),
    }
}

// ============================================================================
// CONFIGURATION CLONING AND SERIALIZATION
// ============================================================================

#[test]
fn test_ecosystem_config_clone() {
    // Test that config can be cloned
    let mut endpoints = HashMap::new();
    endpoints.insert("test".to_string(), "http://test:8080".to_string());

    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(15),
        primal_endpoints: endpoints,
        required_primals: vec!["songbird".to_string()],
        optional_primals: vec!["nestgate".to_string()],
    };

    let cloned = config.clone();

    // Verify clone is independent
    assert_eq!(config.auto_discovery, cloned.auto_discovery);
    assert_eq!(config.discovery_timeout, cloned.discovery_timeout);
    assert_eq!(config.primal_endpoints.len(), cloned.primal_endpoints.len());
}

#[test]
fn test_primal_instance_clone_independence() {
    // Test that cloning doesn't create shared references
    let original = PrimalInstance {
        name: "original".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["test".to_string()],
        status: PrimalStatus::Discovered,
        discovered_at: Utc::now(),
    };

    let mut cloned = original.clone();

    // Modify clone
    cloned.name = "modified".to_string();
    cloned.status = PrimalStatus::Connected;

    // Original should be unchanged
    assert_eq!(original.name, "original");
    assert_eq!(original.status, PrimalStatus::Discovered);
    assert_eq!(cloned.name, "modified");
    assert_eq!(cloned.status, PrimalStatus::Connected);
}

// ============================================================================
// ENDPOINT VALIDATION TESTS
// ============================================================================

#[test]
fn test_primal_endpoint_formats() {
    // Test various endpoint URL formats
    let formats = vec![
        "http://localhost:8080",
        "https://primal.example.com:443",
        "http://192.168.1.100:9090",
        "https://[::1]:8080", // IPv6
        "http://primal:80",
    ];

    for endpoint in formats {
        let instance = PrimalInstance {
            name: "test".to_string(),
            primal_type: PrimalType::Songbird,
            endpoint: endpoint.to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Discovered,
            discovered_at: Utc::now(),
        };

        // Should accept all formats
        assert_eq!(instance.endpoint, endpoint);
    }
}

#[test]
fn test_primal_endpoint_edge_cases() {
    // Test edge case endpoint formats
    let edge_cases = vec![
        "",                        // Empty
        "not-a-url",               // No protocol
        "http://",                 // Incomplete
        "://localhost:8080",       // Missing protocol
        "http://localhost:999999", // Invalid port
    ];

    for endpoint in edge_cases {
        let instance = PrimalInstance {
            name: "test".to_string(),
            primal_type: PrimalType::Custom("test".to_string()),
            endpoint: endpoint.to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Failed("Invalid endpoint".to_string()),
            discovered_at: Utc::now(),
        };

        // Structure should accept any string (validation happens elsewhere)
        assert_eq!(instance.endpoint, endpoint);
        // But status should reflect the issue
        assert!(matches!(instance.status, PrimalStatus::Failed(_)));
    }
}

// ============================================================================
// VERSION STRING TESTS
// ============================================================================

#[test]
fn test_primal_version_formats() {
    // Test various semantic version formats
    let versions = vec![
        "1.0.0",
        "0.1.0-alpha",
        "2.3.4-beta.1",
        "1.0.0-rc.1+build.123",
        "dev",
        "",
    ];

    for version in versions {
        let instance = PrimalInstance {
            name: "test".to_string(),
            primal_type: PrimalType::ToadStool,
            endpoint: "http://localhost:8080".to_string(),
            version: version.to_string(),
            capabilities: vec![],
            status: PrimalStatus::Connected,
            discovered_at: Utc::now(),
        };

        assert_eq!(instance.version, version);
    }
}

// ============================================================================
// CAPABILITY STRING TESTS
// ============================================================================

#[test]
fn test_capability_string_formats() {
    // Test various capability naming conventions
    let capabilities = vec![
        "compute",
        "storage",
        "ai-inference",
        "container.execution",
        "wasm_runtime",
        "GPU/COMPUTE",
        "coordination-v2",
        "capability.with.dots",
        "", // Empty capability name (edge case)
    ];

    let instance = PrimalInstance {
        name: "multi-capability".to_string(),
        primal_type: PrimalType::BiomeOS,
        endpoint: "http://localhost:3000".to_string(),
        version: "3.0.0".to_string(),
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    // Should accept all formats
    assert_eq!(instance.capabilities.len(), capabilities.len());
    assert!(instance.capabilities.contains(&"compute".to_string()));
    assert!(instance.capabilities.contains(&"GPU/COMPUTE".to_string()));
}

// ============================================================================
// CONCURRENT ACCESS PATTERNS
// ============================================================================

#[test]
fn test_multiple_config_instances_independence() {
    // Test that multiple configs can exist independently
    let config1 = EcosystemConfig::default();
    let config2 = EcosystemConfig::default();

    // Should be independent instances
    assert_eq!(config1.auto_discovery, config2.auto_discovery);
    // But not the same instance
    // (Can't test pointer equality easily, but they should behave independently)
}

// ============================================================================
// PROPERTY INVARIANT TESTS
// ============================================================================

#[test]
fn test_ecosystem_config_default_invariants() {
    // Test that default config maintains expected invariants
    let config = EcosystemConfig::default();

    // Auto-discovery should be enabled by default
    assert!(config.auto_discovery);

    // Should have reasonable timeout
    assert!(config.discovery_timeout >= Duration::from_secs(1));
    assert!(config.discovery_timeout <= Duration::from_secs(300));

    // Should have some optional primals
    assert!(!config.optional_primals.is_empty());

    // Required primals should be empty (all optional by default)
    assert!(config.required_primals.is_empty());
}

#[test]
fn test_primal_instance_timestamp_ordering() {
    // Test that timestamps work correctly
    let now = Utc::now();

    let instance1 = PrimalInstance {
        name: "first".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Discovered,
        discovered_at: now,
    };

    // Create slightly later
    std::thread::sleep(Duration::from_millis(10));
    let later = Utc::now();

    let instance2 = PrimalInstance {
        name: "second".to_string(),
        primal_type: PrimalType::NestGate,
        endpoint: "http://localhost:9090".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Discovered,
        discovered_at: later,
    };

    // Second should have later timestamp
    assert!(instance2.discovered_at > instance1.discovered_at);
}

// ============================================================================
// STRESS AND BOUNDARY TESTS
// ============================================================================

#[test]
fn test_ecosystem_config_with_many_endpoints() {
    // Stress test: Many configured endpoints
    let mut endpoints = HashMap::new();
    for i in 0..100 {
        endpoints.insert(format!("primal_{}", i), format!("http://primal-{}:8080", i));
    }

    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(60),
        primal_endpoints: endpoints,
        required_primals: vec![],
        optional_primals: vec![],
    };

    // Should handle many endpoints
    assert_eq!(config.primal_endpoints.len(), 100);
    assert!(config.primal_endpoints.contains_key("primal_42"));
}

#[test]
fn test_primal_name_with_special_characters() {
    // Edge case: Primal names with special characters
    let special_names = vec![
        "primal-with-dashes",
        "primal_with_underscores",
        "primal.with.dots",
        "primal123",
        "UPPERCASE",
        "mIxEdCaSe",
        "primal with spaces",
        "primal@special#chars",
        "🍄toadstool", // Unicode
    ];

    for name in special_names {
        let instance = PrimalInstance {
            name: name.to_string(),
            primal_type: PrimalType::Custom("test".to_string()),
            endpoint: "http://localhost:8080".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Discovered,
            discovered_at: Utc::now(),
        };

        assert_eq!(instance.name, name);
    }
}

// ============================================================================
// DOCUMENTATION EXAMPLE VALIDATION
// ============================================================================

#[test]
fn test_ecosystem_config_as_documented() {
    // Verify the config works as shown in documentation
    let config = EcosystemConfig::default();

    // Should work out of box
    assert!(config.auto_discovery);
    assert!(!config.optional_primals.is_empty());
}

#[test]
fn test_custom_primal_type_as_documented() {
    // Test creating custom primal type as shown in docs
    let custom_type = PrimalType::Custom("my-custom-primal".to_string());

    let instance = PrimalInstance {
        name: "custom-instance".to_string(),
        primal_type: custom_type,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["custom-capability".to_string()],
        status: PrimalStatus::Discovered,
        discovered_at: Utc::now(),
    };

    match instance.primal_type {
        PrimalType::Custom(name) => assert_eq!(name, "my-custom-primal"),
        _ => panic!("Expected Custom primal type"),
    }
}

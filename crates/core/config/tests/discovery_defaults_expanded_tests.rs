//! Expanded tests for discovery_defaults module
//!
//! Coverage expansion: discovery_defaults.rs needs expanded coverage
//! Testing capability-based discovery patterns and defaults

use std::time::Duration;
use toadstool_config::discovery_defaults::*;

/// Test DiscoveryDefaults default values
#[test]
fn test_discovery_defaults_values() {
    let defaults = DiscoveryDefaults::default();

    assert_eq!(defaults.discovery_timeout, Duration::from_secs(5));
    assert_eq!(defaults.refresh_interval, Duration::from_secs(30));
    assert_eq!(defaults.cache_ttl, Duration::from_secs(300));
    assert_eq!(defaults.max_retries, 3);
    assert_eq!(defaults.retry_delay, Duration::from_secs(1));
}

/// Test DiscoveryDefaults clone
#[test]
fn test_discovery_defaults_clone() {
    let defaults1 = DiscoveryDefaults::default();
    let defaults2 = defaults1.clone();

    assert_eq!(defaults1.discovery_timeout, defaults2.discovery_timeout);
    assert_eq!(defaults1.max_retries, defaults2.max_retries);
}

/// Test DiscoveryDefaults debug format
#[test]
fn test_discovery_defaults_debug() {
    let defaults = DiscoveryDefaults::default();
    let debug_str = format!("{:?}", defaults);

    assert!(debug_str.contains("DiscoveryDefaults"));
    assert!(debug_str.contains("discovery_timeout"));
}

/// Test capability constants exist
#[test]
fn test_capability_constants() {
    assert_eq!(capabilities::MESSAGE_ROUTING, "message-routing");
    assert_eq!(capabilities::COORDINATION, "coordination");
    assert_eq!(capabilities::STORAGE, "storage");
    assert_eq!(capabilities::COMPUTE, "compute");
    assert_eq!(capabilities::IDENTITY, "identity");
    assert_eq!(capabilities::MONITORING, "monitoring");
    assert_eq!(capabilities::CONFIGURATION, "configuration");
    assert_eq!(capabilities::AI_ORCHESTRATION, "ai-orchestration");
}

/// Test all capability constants are non-empty
#[test]
fn test_capability_constants_non_empty() {
    assert!(!capabilities::MESSAGE_ROUTING.is_empty());
    assert!(!capabilities::COORDINATION.is_empty());
    assert!(!capabilities::STORAGE.is_empty());
    assert!(!capabilities::COMPUTE.is_empty());
    assert!(!capabilities::IDENTITY.is_empty());
    assert!(!capabilities::MONITORING.is_empty());
    assert!(!capabilities::CONFIGURATION.is_empty());
    assert!(!capabilities::AI_ORCHESTRATION.is_empty());
}

/// Test capability constants are lowercase
#[test]
fn test_capability_constants_lowercase() {
    assert_eq!(
        capabilities::MESSAGE_ROUTING,
        capabilities::MESSAGE_ROUTING.to_lowercase()
    );
    assert_eq!(
        capabilities::COORDINATION,
        capabilities::COORDINATION.to_lowercase()
    );
    assert_eq!(capabilities::STORAGE, capabilities::STORAGE.to_lowercase());
    assert_eq!(capabilities::COMPUTE, capabilities::COMPUTE.to_lowercase());
}

/// Test capability constants use kebab-case
#[test]
fn test_capability_constants_format() {
    // All capability names should use kebab-case for consistency
    assert!(capabilities::MESSAGE_ROUTING.contains('-'));
    // Some may be single words (no dash required)
    assert!(!capabilities::COORDINATION.contains('_'));
    assert!(!capabilities::STORAGE.contains('_'));
}

/// Test discovery timeout is reasonable
#[test]
fn test_discovery_timeout_reasonable() {
    let defaults = DiscoveryDefaults::default();

    // Should be at least 1 second
    assert!(defaults.discovery_timeout >= Duration::from_secs(1));

    // Should be less than 1 minute
    assert!(defaults.discovery_timeout <= Duration::from_secs(60));
}

/// Test refresh interval is reasonable
#[test]
fn test_refresh_interval_reasonable() {
    let defaults = DiscoveryDefaults::default();

    // Should be at least 10 seconds
    assert!(defaults.refresh_interval >= Duration::from_secs(10));

    // Should be less than 5 minutes
    assert!(defaults.refresh_interval <= Duration::from_secs(300));
}

/// Test cache TTL is reasonable
#[test]
fn test_cache_ttl_reasonable() {
    let defaults = DiscoveryDefaults::default();

    // Should be at least 1 minute
    assert!(defaults.cache_ttl >= Duration::from_secs(60));

    // Should be less than 1 hour
    assert!(defaults.cache_ttl <= Duration::from_secs(3600));
}

/// Test max retries is reasonable
#[test]
fn test_max_retries_reasonable() {
    let defaults = DiscoveryDefaults::default();

    // Should be at least 1
    assert!(defaults.max_retries >= 1);

    // Should be less than 10
    assert!(defaults.max_retries <= 10);
}

/// Test retry delay is reasonable
#[test]
fn test_retry_delay_reasonable() {
    let defaults = DiscoveryDefaults::default();

    // Should be at least 100ms
    assert!(defaults.retry_delay >= Duration::from_millis(100));

    // Should be less than 10 seconds
    assert!(defaults.retry_delay <= Duration::from_secs(10));
}

/// Test ServiceDiscoveryHelper default creation
#[test]
fn test_service_discovery_helper_default() {
    let helper = ServiceDiscoveryHelper::default();
    assert_eq!(helper.discovery_timeout(), Duration::from_secs(5));
}

/// Test ServiceDiscoveryHelper with custom defaults
#[test]
fn test_service_discovery_helper_custom() {
    let custom = DiscoveryDefaults {
        discovery_timeout: Duration::from_secs(10),
        refresh_interval: Duration::from_secs(45),
        cache_ttl: Duration::from_secs(400),
        max_retries: 5,
        retry_delay: Duration::from_millis(500),
    };

    let helper = ServiceDiscoveryHelper::with_defaults(custom);
    assert_eq!(helper.discovery_timeout(), Duration::from_secs(10));
    assert_eq!(helper.refresh_interval(), Duration::from_secs(45));
    assert_eq!(helper.cache_ttl(), Duration::from_secs(400));
}

/// Test FallbackEndpoints default values
#[test]
fn test_fallback_endpoints_defaults() {
    let fallback = FallbackEndpoints::default();
    assert!(fallback.enable_localhost_fallback);
    assert_eq!(fallback.localhost_base_port, 9080);
}

/// Test FallbackEndpoints localhost_endpoint
#[test]
fn test_fallback_localhost_endpoint() {
    let fallback = FallbackEndpoints::default();

    assert_eq!(
        fallback.localhost_endpoint(0).unwrap(),
        "http://localhost:9080"
    );
    assert_eq!(
        fallback.localhost_endpoint(1).unwrap(),
        "http://localhost:9081"
    );
    assert_eq!(
        fallback.localhost_endpoint(10).unwrap(),
        "http://localhost:9090"
    );
}

/// Test FallbackEndpoints with disabled fallback
#[test]
fn test_fallback_disabled() {
    let fallback = FallbackEndpoints {
        enable_localhost_fallback: false,
        localhost_base_port: 9080,
    };

    let result = fallback.localhost_endpoint(0);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("fallback disabled"));
}

/// Test FallbackEndpoints clone
#[test]
fn test_fallback_endpoints_clone() {
    let fallback1 = FallbackEndpoints::default();
    let fallback2 = fallback1.clone();

    assert_eq!(
        fallback1.enable_localhost_fallback,
        fallback2.enable_localhost_fallback
    );
    assert_eq!(fallback1.localhost_base_port, fallback2.localhost_base_port);
}

/// Test custom DiscoveryDefaults creation
#[test]
fn test_custom_discovery_defaults() {
    let custom = DiscoveryDefaults {
        discovery_timeout: Duration::from_secs(10),
        refresh_interval: Duration::from_secs(60),
        cache_ttl: Duration::from_secs(600),
        max_retries: 5,
        retry_delay: Duration::from_secs(2),
    };

    assert_eq!(custom.discovery_timeout, Duration::from_secs(10));
    assert_eq!(custom.max_retries, 5);
}

/// Test all capabilities follow wateringHole standard
#[test]
fn test_capabilities_wateringhole_compliant() {
    // All capabilities should be lowercase kebab-case or single word
    let caps = vec![
        capabilities::MESSAGE_ROUTING,
        capabilities::COORDINATION,
        capabilities::STORAGE,
        capabilities::COMPUTE,
        capabilities::IDENTITY,
        capabilities::MONITORING,
        capabilities::CONFIGURATION,
        capabilities::AI_ORCHESTRATION,
    ];

    for cap in caps {
        // Should not have uppercase
        assert_eq!(cap, cap.to_lowercase());

        // Should not have underscores
        assert!(!cap.contains('_'));

        // Should use dashes or be single word
        assert!(cap.contains('-') || !cap.contains(' '));
    }
}

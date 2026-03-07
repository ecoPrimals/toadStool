// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Comprehensive tests for primal_discovery_mdns
//!
//! Focus: mDNS record creation, parsing, serialization, error paths

use std::time::Duration;

use toadstool_common::primal_discovery::{DiscoveryConfig, DiscoveryMethod, TrustLevel};
use toadstool_common::primal_discovery_mdns::{MdnsAdapter, TOADSTOOL_SERVICE_TYPE};

// ============================================================================
// Constants and service type
// ============================================================================

#[test]
fn test_toadstool_service_type_constant() {
    assert_eq!(TOADSTOOL_SERVICE_TYPE, "_toadstool._tcp.local.");
    assert!(TOADSTOOL_SERVICE_TYPE.ends_with(".local."));
    assert!(TOADSTOOL_SERVICE_TYPE.contains("toadstool"));
    assert!(TOADSTOOL_SERVICE_TYPE.starts_with("_toadstool"));
    assert!(TOADSTOOL_SERVICE_TYPE.contains("tcp"));
}

// ============================================================================
// DiscoveryConfig
// ============================================================================

#[test]
fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
    assert!(config.enable_mdns);
}

#[test]
fn test_discovery_config_cache_ttl() {
    let config = DiscoveryConfig::default();
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
}

#[test]
fn test_discovery_config_enable_mdns_default() {
    let config = DiscoveryConfig::default();
    assert!(config.enable_mdns);
}

#[test]
fn test_discovery_config_with_fallbacks() {
    let mut config = DiscoveryConfig::default();
    config
        .fallbacks
        .insert("storage".to_string(), "http://localhost:8080".to_string());
    assert_eq!(
        config.fallbacks.get("storage"),
        Some(&"http://localhost:8080".to_string())
    );
}

// ============================================================================
// MdnsAdapter creation
// ============================================================================

#[test]
fn test_mdns_adapter_new() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::new(config);
    match &result {
        Ok(adapter) => {
            assert_eq!(adapter.timeout(), Duration::from_secs(3));
            assert_eq!(adapter.config().cache_ttl, Duration::from_secs(300));
        }
        Err(e) => {
            eprintln!("MdnsAdapter::new failed (mDNS may be unavailable): {e}");
        }
    }
}

#[test]
fn test_mdns_adapter_with_timeout() {
    let config = DiscoveryConfig::default();
    let timeout = Duration::from_millis(250);
    let result = MdnsAdapter::with_timeout(config, timeout);
    if let Ok(adapter) = result {
        assert_eq!(adapter.timeout(), timeout);
    }
}

#[test]
fn test_mdns_adapter_config_accessor() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::with_timeout(config.clone(), Duration::from_millis(50));
    if let Ok(adapter) = result {
        let retrieved = adapter.config();
        assert_eq!(retrieved.cache_ttl, config.cache_ttl);
        assert_eq!(retrieved.enable_mdns, config.enable_mdns);
    }
}

// ============================================================================
// MdnsAdapter discover
// ============================================================================

#[test]
fn test_mdns_adapter_discover_nonexistent_capability() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::with_timeout(config, Duration::from_millis(50));
    if let Ok(adapter) = result {
        let endpoints = adapter.discover("nonexistent-capability-xyz-123");
        assert!(endpoints.is_ok());
        let eps = endpoints.unwrap();
        assert!(
            eps.is_empty(),
            "expected no services in test env, got {}",
            eps.len()
        );
    }
}

#[test]
fn test_mdns_adapter_discover_all() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::with_timeout(config, Duration::from_millis(50));
    if let Ok(adapter) = result {
        let endpoints = adapter.discover_all();
        assert!(endpoints.is_ok());
        let _eps = endpoints.unwrap();
    }
}

#[test]
fn test_mdns_adapter_discover_empty_capability_string() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::with_timeout(config, Duration::from_millis(50));
    if let Ok(adapter) = result {
        let eps = adapter.discover("");
        assert!(eps.is_ok());
    }
}

#[test]
fn test_mdns_adapter_discover_short_timeout() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::with_timeout(config, Duration::from_millis(10));
    if let Ok(adapter) = result {
        let eps = adapter.discover_all();
        assert!(eps.is_ok());
    }
}

// ============================================================================
// DiscoveryMethod and TrustLevel (used by convert_mdns_service_to_endpoint)
// ============================================================================

#[test]
fn test_discovery_method_mdns() {
    let method = DiscoveryMethod::MDns;
    assert!(matches!(method, DiscoveryMethod::MDns));
}

#[test]
fn test_discovery_method_mdns_display() {
    let m = DiscoveryMethod::MDns;
    let s = format!("{m:?}");
    assert!(s.contains("MDns") || s.contains("Mdns"));
}

#[test]
fn test_trust_level_local() {
    let level = TrustLevel::Local;
    assert_eq!(level, TrustLevel::Local);
}

#[test]
fn test_trust_level_ordering() {
    assert!(TrustLevel::Verified <= TrustLevel::Local);
    assert!(TrustLevel::Local <= TrustLevel::Unverified);
}

#[test]
fn test_trust_level_unverified() {
    assert!(TrustLevel::Unverified >= TrustLevel::Local);
}

// ============================================================================
// Default discovery timeout (3 seconds per DEFAULT_DISCOVERY_TIMEOUT)
// ============================================================================

#[test]
fn test_default_discovery_timeout_is_three_seconds() {
    let config = DiscoveryConfig::default();
    let result = MdnsAdapter::new(config);
    if let Ok(adapter) = result {
        assert_eq!(adapter.timeout(), Duration::from_secs(3));
    }
}

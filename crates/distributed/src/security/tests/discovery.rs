// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security discovery behavior tests — async endpoint selection and health

use crate::security::types::{SecurityCapability, SecurityEndpoint};
use crate::security::{
    DistributedCryptoProvider, SecurityClient, SecurityConfig, SecurityDiscovery, ServiceLocation,
};
use std::sync::Arc;
use toadstool::CryptoProvider;

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();
    assert!(config.auto_discover);
    assert_eq!(config.discovery_timeout_ms, 5000);
    assert_eq!(config.preferred_location, ServiceLocation::Local);
    assert!(config.fallback_enabled);
}

#[test]
fn test_security_discovery_new() {
    let config = SecurityConfig::default();
    let discovery = SecurityDiscovery::new(config);
    assert!(discovery.config().auto_discover);
}

#[test]
fn test_security_client_new() {
    let config = SecurityConfig::default();
    let _client = SecurityClient::new_test(config);
}

#[tokio::test]
async fn test_security_discovery_get_best_endpoint_empty() {
    let config = SecurityConfig::default();
    let discovery = SecurityDiscovery::new(config);
    let result = discovery.get_best_endpoint().await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No security/crypto endpoints")
    );
}

#[tokio::test]
async fn test_security_discovery_preferred_location_local() {
    let config = SecurityConfig {
        preferred_location: ServiceLocation::Local,
        ..Default::default()
    };
    let discovery = SecurityDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[tokio::test]
async fn test_security_discovery_preferred_location_any() {
    let config = SecurityConfig {
        preferred_location: ServiceLocation::Any,
        ..Default::default()
    };
    let discovery = SecurityDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[tokio::test]
async fn test_security_discovery_preferred_location_network() {
    let config = SecurityConfig {
        preferred_location: ServiceLocation::Network,
        ..Default::default()
    };
    let discovery = SecurityDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[test]
fn test_security_client_provider_id() {
    let config = SecurityConfig::default();
    let client = Arc::new(SecurityClient::new_test(config).unwrap());
    let crypto = DistributedCryptoProvider::Security(Arc::clone(&client));
    assert_eq!(crypto.provider_id(), "crypto");
}

#[test]
fn test_security_client_capabilities() {
    let config = SecurityConfig::default();
    let client = Arc::new(SecurityClient::new_test(config).unwrap());
    let crypto = DistributedCryptoProvider::Security(Arc::clone(&client));
    assert!(!crypto.capabilities().algorithms.is_empty());
}

#[tokio::test]
async fn test_security_discovery_get_best_endpoint_returns_lowest_latency() {
    let config = SecurityConfig::default();
    let endpoints = vec![
        SecurityEndpoint {
            service_id: "ep-slow".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![SecurityCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(50),
        },
        SecurityEndpoint {
            service_id: "ep-fast".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8082".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![SecurityCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(5),
        },
    ];
    let discovery = SecurityDiscovery::with_endpoints(config, endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "ep-fast");
    assert_eq!(best.latency_ms, Some(5));
}

#[tokio::test]
async fn test_security_discovery_get_best_endpoint_all_unhealthy_returns_error() {
    let config = SecurityConfig::default();
    let endpoints = vec![SecurityEndpoint {
        service_id: "ep-unhealthy".to_string(),
        protocol: "http".to_string(),
        address: "127.0.0.1:8081".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![SecurityCapability::KeyManagement],
        healthy: false,
        latency_ms: Some(100),
    }];
    let discovery = SecurityDiscovery::with_endpoints(config, endpoints);
    let result = discovery.get_best_endpoint().await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("healthy")
    );
}

#[tokio::test]
async fn test_security_discovery_get_best_endpoint_no_latency_uses_max() {
    let config = SecurityConfig::default();
    let endpoints = vec![
        SecurityEndpoint {
            service_id: "ep-a".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![SecurityCapability::KeyManagement],
            healthy: true,
            latency_ms: None,
        },
        SecurityEndpoint {
            service_id: "ep-b".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8082".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![SecurityCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(1),
        },
    ];
    let discovery = SecurityDiscovery::with_endpoints(config, endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "ep-b");
}

#[test]
fn test_security_client_creation_with_custom_config() {
    let config = SecurityConfig {
        auto_discover: false,
        discovery_timeout_ms: 10000,
        preferred_location: ServiceLocation::Network,
        fallback_enabled: false,
    };
    assert!(SecurityClient::new_test(config).is_ok());
}

#[test]
fn test_security_client_creation_default_config() {
    let config = SecurityConfig::default();
    let client = Arc::new(SecurityClient::new_test(config).unwrap());
    let crypto = DistributedCryptoProvider::Security(Arc::clone(&client));
    assert_eq!(crypto.provider_id(), "crypto");
}

#[tokio::test]
async fn test_security_discovery_with_endpoints_injects_data() {
    let config = SecurityConfig::default();
    let mock_endpoints = vec![SecurityEndpoint {
        service_id: "mock-1".to_string(),
        protocol: "unix".to_string(),
        address: "127.0.0.1:9090".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![SecurityCapability::KeyManagement],
        healthy: true,
        latency_ms: Some(2),
    }];
    let discovery = SecurityDiscovery::with_endpoints(config, mock_endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "mock-1");
}

#[test]
fn test_bear_dog_config_variations() {
    let config = SecurityConfig {
        auto_discover: false,
        discovery_timeout_ms: 10000,
        preferred_location: ServiceLocation::Network,
        fallback_enabled: false,
    };
    assert!(!config.auto_discover);
    assert_eq!(config.discovery_timeout_ms, 10000);
    assert_eq!(config.preferred_location, ServiceLocation::Network);
}

#[test]
fn test_security_config_timeout_variations() {
    let config = SecurityConfig {
        auto_discover: true,
        discovery_timeout_ms: 1,
        preferred_location: ServiceLocation::Local,
        fallback_enabled: true,
    };
    assert_eq!(config.discovery_timeout_ms, 1);
}

#[test]
fn test_security_config_timeout_affects_discovery_timeout() {
    let config = SecurityConfig {
        discovery_timeout_ms: 2500,
        ..Default::default()
    };
    assert_eq!(config.discovery_timeout_ms, 2500);
}

#[test]
fn test_security_config_fallback_disabled() {
    let config = SecurityConfig {
        fallback_enabled: false,
        ..Default::default()
    };
    assert!(!config.fallback_enabled);
}

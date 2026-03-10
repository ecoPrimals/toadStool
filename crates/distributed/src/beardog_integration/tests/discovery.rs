// SPDX-License-Identifier: AGPL-3.0-only
//! BearDog discovery behavior tests — async endpoint selection and health

use crate::beardog_integration::types::{BearDogCapability, BearDogEndpoint};
use crate::beardog_integration::{BearDogClient, BearDogConfig, BearDogDiscovery, ServiceLocation};
use toadstool::CryptoProvider;

#[test]
fn test_beardog_config_default() {
    let config = BearDogConfig::default();
    assert!(config.auto_discover);
    assert_eq!(config.discovery_timeout_ms, 5000);
    assert_eq!(config.preferred_location, ServiceLocation::Local);
    assert!(config.fallback_enabled);
}

#[test]
#[allow(deprecated)]
fn test_beardog_discovery_new() {
    let config = BearDogConfig::default();
    let discovery = BearDogDiscovery::new(config);
    assert!(discovery.config().auto_discover);
}

#[test]
#[allow(deprecated)]
fn test_beardog_client_new() {
    let config = BearDogConfig::default();
    let _client = BearDogClient::new(config);
}

#[tokio::test]
async fn test_beardog_discovery_get_best_endpoint_empty() {
    let config = BearDogConfig::default();
    let discovery = BearDogDiscovery::new(config);
    let result = discovery.get_best_endpoint().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No BearDog endpoints"));
}

#[tokio::test]
async fn test_beardog_discovery_preferred_location_local() {
    let config = BearDogConfig {
        preferred_location: ServiceLocation::Local,
        ..Default::default()
    };
    let discovery = BearDogDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[tokio::test]
async fn test_beardog_discovery_preferred_location_any() {
    let config = BearDogConfig {
        preferred_location: ServiceLocation::Any,
        ..Default::default()
    };
    let discovery = BearDogDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[tokio::test]
async fn test_beardog_discovery_preferred_location_network() {
    let config = BearDogConfig {
        preferred_location: ServiceLocation::Network,
        ..Default::default()
    };
    let discovery = BearDogDiscovery::new(config);
    assert!(discovery.discover().await.is_ok());
}

#[test]
#[allow(deprecated)]
fn test_beardog_client_provider_id() {
    let config = BearDogConfig::default();
    let client = BearDogClient::new(config).unwrap();
    assert_eq!(client.provider_id(), "crypto");
}

#[test]
#[allow(deprecated)]
fn test_beardog_client_capabilities() {
    let config = BearDogConfig::default();
    let client = BearDogClient::new(config).unwrap();
    assert!(!client.capabilities().algorithms.is_empty());
}

#[tokio::test]
async fn test_beardog_discovery_get_best_endpoint_returns_lowest_latency() {
    let config = BearDogConfig::default();
    let endpoints = vec![
        BearDogEndpoint {
            service_id: "ep-slow".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(50),
        },
        BearDogEndpoint {
            service_id: "ep-fast".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8082".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(5),
        },
    ];
    let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "ep-fast");
    assert_eq!(best.latency_ms, Some(5));
}

#[tokio::test]
async fn test_beardog_discovery_get_best_endpoint_all_unhealthy_returns_error() {
    let config = BearDogConfig::default();
    let endpoints = vec![BearDogEndpoint {
        service_id: "ep-unhealthy".to_string(),
        protocol: "http".to_string(),
        address: "127.0.0.1:8081".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![BearDogCapability::KeyManagement],
        healthy: false,
        latency_ms: Some(100),
    }];
    let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
    let result = discovery.get_best_endpoint().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .to_lowercase()
        .contains("healthy"));
}

#[tokio::test]
async fn test_beardog_discovery_get_best_endpoint_no_latency_uses_max() {
    let config = BearDogConfig::default();
    let endpoints = vec![
        BearDogEndpoint {
            service_id: "ep-a".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: None,
        },
        BearDogEndpoint {
            service_id: "ep-b".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8082".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(1),
        },
    ];
    let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "ep-b");
}

#[test]
#[allow(deprecated)]
fn test_beardog_client_creation_with_custom_config() {
    let config = BearDogConfig {
        auto_discover: false,
        discovery_timeout_ms: 10000,
        preferred_location: ServiceLocation::Network,
        fallback_enabled: false,
    };
    assert!(BearDogClient::new(config).is_ok());
}

#[test]
#[allow(deprecated)]
fn test_beardog_client_creation_default_config() {
    let config = BearDogConfig::default();
    let client = BearDogClient::new(config).unwrap();
    assert_eq!(client.provider_id(), "crypto");
}

#[tokio::test]
async fn test_beardog_discovery_with_endpoints_injects_data() {
    let config = BearDogConfig::default();
    let mock_endpoints = vec![BearDogEndpoint {
        service_id: "mock-1".to_string(),
        protocol: "unix".to_string(),
        address: "127.0.0.1:9090".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![BearDogCapability::KeyManagement],
        healthy: true,
        latency_ms: Some(2),
    }];
    let discovery = BearDogDiscovery::with_endpoints(config, mock_endpoints);
    let best = discovery.get_best_endpoint().await.unwrap();
    assert_eq!(best.service_id, "mock-1");
}

#[test]
fn test_bear_dog_config_variations() {
    let config = BearDogConfig {
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
fn test_beardog_config_timeout_variations() {
    let config = BearDogConfig {
        auto_discover: true,
        discovery_timeout_ms: 1,
        preferred_location: ServiceLocation::Local,
        fallback_enabled: true,
    };
    assert_eq!(config.discovery_timeout_ms, 1);
}

#[test]
fn test_beardog_config_timeout_affects_discovery_timeout() {
    let config = BearDogConfig {
        discovery_timeout_ms: 2500,
        ..Default::default()
    };
    assert_eq!(config.discovery_timeout_ms, 2500);
}

#[test]
fn test_beardog_config_fallback_disabled() {
    let config = BearDogConfig {
        fallback_enabled: false,
        ..Default::default()
    };
    assert!(!config.fallback_enabled);
}

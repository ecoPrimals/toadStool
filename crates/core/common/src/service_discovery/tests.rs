//! Service discovery tests

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::discovery_defaults::DiscoveryConfig;
use crate::primal_identity::{Capability, ServiceEndpoint};

use super::*;

#[tokio::test]
async fn test_service_discovery_creation() {
    let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await;
    assert!(discovery.is_ok());
}

#[tokio::test]
async fn test_discover_from_env() {
    std::env::remove_var("TOADSTOOL_SERVICE_CACHE_URL");
    std::env::set_var("TOADSTOOL_SERVICE_TEST_URL", "http://localhost:9000");
    std::env::set_var("TOADSTOOL_SERVICE_TEST_CAPABILITIES", "coordination");

    let discovery = ServiceDiscovery::new(DiscoveryMethod::Environment)
        .await
        .unwrap();
    let services = discovery.discover_from_env().await.unwrap();
    assert!(!services.is_empty());
    assert_eq!(services[0].name, "test");

    std::env::remove_var("TOADSTOOL_SERVICE_TEST_URL");
    std::env::remove_var("TOADSTOOL_SERVICE_TEST_CAPABILITIES");
}

#[tokio::test]
async fn test_service_endpoint_from_url() {
    let endpoint = ServiceEndpoint::from_url_string("http://localhost:8080").unwrap();
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.address, "localhost");
    assert_eq!(endpoint.port, 8080);
    assert_eq!(endpoint.url(), "http://localhost:8080");
}

#[tokio::test]
async fn test_discovered_service_has_capability() {
    let service = DiscoveredService {
        id: "test".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability::Coordination(
            crate::primal_identity::CoordinationCapability::ServiceDiscovery,
        )],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert!(service.has_capability(&Capability::Coordination(
        crate::primal_identity::CoordinationCapability::ServiceDiscovery
    )));
}

#[tokio::test]
async fn test_service_freshness() {
    let service = DiscoveredService {
        id: "test".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert!(service.is_fresh(Duration::from_secs(3600)));
}

#[test]
fn test_parse_capabilities() {
    let discovery = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(ServiceDiscovery::new(DiscoveryMethod::Auto))
        .unwrap();
    let caps = discovery.parse_capabilities("coordination,storage,compute");
    assert_eq!(caps.len(), 3);
}

#[test]
fn test_discovery_error_no_service() {
    use crate::primal_identity::ComputeCapability;
    let err = DiscoveryError::NoServiceFound {
        capability: Capability::Compute(ComputeCapability::NativeExecution),
    };
    assert!(err.to_string().contains("No services found"));
}

#[test]
fn test_discovery_error_timeout() {
    let err = DiscoveryError::Timeout {
        duration: Duration::from_secs(5),
    };
    assert!(err.to_string().contains("timeout"));
}

#[test]
fn test_discovery_error_method_unavailable() {
    let err = DiscoveryError::MethodUnavailable {
        method: "consul".to_string(),
    };
    assert!(err.to_string().contains("unavailable"));
}

#[test]
fn test_discovery_error_invalid_response() {
    let err = DiscoveryError::InvalidResponse {
        reason: "malformed JSON".to_string(),
    };
    assert!(err.to_string().contains("Invalid"));
}

#[test]
fn test_discovery_error_config_error() {
    let err = DiscoveryError::ConfigError {
        reason: "missing field".to_string(),
    };
    assert!(err.to_string().contains("Configuration"));
}

#[test]
fn test_discovery_method_auto() {
    assert_eq!(DiscoveryMethod::Auto, DiscoveryMethod::Auto);
}

#[test]
fn test_discovery_method_config_file() {
    let method = DiscoveryMethod::ConfigFile {
        path: "/etc/toadstool.conf".to_string(),
    };
    if let DiscoveryMethod::ConfigFile { path } = method {
        assert_eq!(path, "/etc/toadstool.conf");
    } else {
        panic!("Expected ConfigFile variant");
    }
}

#[test]
fn test_discovery_method_registry() {
    let method = DiscoveryMethod::Registry {
        endpoint: "http://consul:8500".to_string(),
    };
    if let DiscoveryMethod::Registry { endpoint } = method {
        assert!(endpoint.contains("consul"));
    } else {
        panic!("Expected Registry variant");
    }
}

#[test]
fn test_discovery_method_multi() {
    let method = DiscoveryMethod::Multi(vec![
        DiscoveryMethod::Environment,
        DiscoveryMethod::Mdns,
        DiscoveryMethod::Auto,
    ]);
    if let DiscoveryMethod::Multi(methods) = method {
        assert_eq!(methods.len(), 3);
    } else {
        panic!("Expected Multi variant");
    }
}

#[tokio::test]
async fn test_discovered_service_primary_endpoint() {
    let service = DiscoveredService {
        id: "test".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![
            ServiceEndpoint::http("localhost", 8080),
            ServiceEndpoint::http("localhost", 8081),
        ],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let primary = service.primary_endpoint();
    assert!(primary.is_some());
    assert_eq!(primary.unwrap().port, 8080);
}

#[tokio::test]
async fn test_discovered_service_no_endpoints() {
    let service = DiscoveredService {
        id: "test".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert!(service.primary_endpoint().is_none());
}

#[tokio::test]
async fn test_discovered_service_not_fresh() {
    let old_time = SystemTime::now()
        .checked_sub(Duration::from_secs(7200))
        .unwrap();
    let service = DiscoveredService {
        id: "test".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: old_time,
        last_seen: old_time,
        healthy: true,
    };
    assert!(!service.is_fresh(Duration::from_secs(3600)));
}

#[tokio::test]
async fn test_service_discovery_mdns() {
    let discovery = ServiceDiscovery::new(DiscoveryMethod::Mdns).await;
    assert!(discovery.is_ok());
}

#[tokio::test]
async fn test_service_discovery_with_config() {
    let config = DiscoveryConfig::default();
    let discovery = ServiceDiscovery::with_config(DiscoveryMethod::Auto, config).await;
    assert!(discovery.is_ok());
}

#[tokio::test]
async fn test_service_endpoint_unix() {
    let endpoint = ServiceEndpoint::from_url_string("unix:///tmp/test.sock").unwrap();
    assert_eq!(endpoint.protocol, "unix");
}

#[tokio::test]
async fn test_service_endpoint_from_url_invalid_format() {
    let result = ServiceEndpoint::from_url_string("invalid-no-protocol");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid") || err.to_string().contains("URL"));
}

#[tokio::test]
async fn test_service_endpoint_from_url_with_port_default() {
    let endpoint = ServiceEndpoint::from_url_string("http://example.com").unwrap();
    assert_eq!(endpoint.port, 80);
    assert_eq!(endpoint.address, "example.com");
}

#[tokio::test]
async fn test_parse_capabilities_empty() {
    let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await.unwrap();
    assert_eq!(discovery.parse_capabilities("").len(), 0);
}

#[tokio::test]
async fn test_parse_capabilities_whitespace() {
    let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await.unwrap();
    let caps = discovery.parse_capabilities("  coordination  ,  storage  ");
    assert_eq!(caps.len(), 2);
}

#[tokio::test]
async fn test_find_service_by_capability_no_services_returns_error() {
    use crate::primal_identity::CoordinationCapability;
    let discovery = ServiceDiscovery::new(DiscoveryMethod::Mdns).await.unwrap();
    let cap = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
    let result = discovery.find_service_by_capability(cap).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("No services found") || msg.contains("NoServiceFound"),
        "Expected no service error, got: {}",
        msg
    );
}

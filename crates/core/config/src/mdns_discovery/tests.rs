// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::net::IpAddr;

use toadstool_common::primal_identity::{
    AuthCapability, Capability, ComputeCapability, CoordinationCapability, DiscoveredService,
    DiscoveryCapability, ServiceEndpoint, StorageCapability,
};
use toadstool_common::runtime_discovery::DiscoveryClient;

use super::MdnsDiscoveryClient;
use std::time::Duration;

#[tokio::test]
async fn test_mdns_client_creation() {
    let client = MdnsDiscoveryClient::new();
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_mdns_client_with_ttl() {
    let client = MdnsDiscoveryClient::with_ttl(Duration::from_secs(60));
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_cache_service() {
    let client = MdnsDiscoveryClient::new().unwrap();

    let service = DiscoveredService {
        id: Some("test-service".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.cache_service(service.clone()).await;

    let cache = client.cache.read().await;
    assert!(cache.contains_key("test-service"));
}

#[tokio::test]
async fn test_discover_by_capability_empty() {
    let client = MdnsDiscoveryClient::new().unwrap();

    let services = client
        .discover_by_capability(&Capability::Coordination(CoordinationCapability::default()))
        .await
        .unwrap();

    // Should return empty when no services cached
    assert!(services.is_empty());
}

#[tokio::test]
async fn test_parse_capabilities() {
    let txt_records = vec![
        "capability=coordination:service-discovery".to_string(),
        "capability=storage:object".to_string(),
        "version=1.0".to_string(),
    ];

    let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
    assert_eq!(capabilities.len(), 2);

    // Verify parsed capabilities
    assert!(capabilities.contains(&Capability::Coordination(
        CoordinationCapability::ServiceDiscovery
    )));
    assert!(capabilities.contains(&Capability::Storage(StorageCapability::ObjectStorage)));
}

#[tokio::test]
async fn test_parse_capabilities_compute_authentication_discovery() {
    let txt_records = vec![
        "capability=compute:native".to_string(),
        "capability=authentication:user".to_string(),
        "capability=discovery:mdns".to_string(),
        "capability=discovery:other".to_string(),
    ];

    let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
    assert!(capabilities.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(capabilities.contains(&Capability::Authentication(AuthCapability::UserAuth)));
    assert!(capabilities.contains(&Capability::Discovery(DiscoveryCapability::MdnsDiscovery)));
    assert!(capabilities.contains(&Capability::Discovery(
        DiscoveryCapability::CapabilityDiscovery
    )));
}

#[tokio::test]
async fn test_parse_capabilities_unknown_skipped() {
    let txt_records = vec![
        "capability=unknown:variant".to_string(),
        "capability=coordination:service-discovery".to_string(),
    ];
    let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
    assert_eq!(capabilities.len(), 1);
}

#[tokio::test]
async fn test_register_service_with_endpoints() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let service = DiscoveredService {
        id: Some("svc-1".to_string()),
        capabilities: vec![Capability::Storage(StorageCapability::ObjectStorage)],
        endpoints: vec![ServiceEndpoint {
            address: "192.168.1.10".to_string(),
            port: 8080,
            protocol: "http".to_string(),
            path: None,
            metadata: HashMap::new(),
        }],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.register_service(&service).await.unwrap();

    let services = client
        .discover_by_capability(&Capability::Storage(StorageCapability::ObjectStorage))
        .await
        .unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].id.as_deref(), Some("svc-1"));
}

#[tokio::test]
async fn test_deregister_service() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let service = DiscoveredService {
        id: Some("to-remove".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.cache_service(service).await;
    client.deregister_service("to-remove").await.unwrap();

    let services = client
        .discover_by_capability(&Capability::Compute(ComputeCapability::NativeExecution))
        .await
        .unwrap();
    assert!(services.is_empty());
}

#[tokio::test]
async fn test_discover_all_with_services() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let service = DiscoveredService {
        id: Some("all-svc".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.cache_service(service).await;

    let all = client.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn test_health_check_cached_service() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let service = DiscoveredService {
        id: Some("health-svc".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.cache_service(service).await;
    let healthy = client.health_check("health-svc").await.unwrap();
    assert!(healthy);
}

#[tokio::test]
async fn test_health_check_unknown_service() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let healthy = client.health_check("nonexistent-svc-id").await.unwrap();
    assert!(!healthy);
}

#[tokio::test]
async fn test_mdns_to_discovered_service() {
    let txt = vec![
        "capability=coordination:service-discovery".to_string(),
        "capability=storage:object".to_string(),
    ];
    let service = MdnsDiscoveryClient::mdns_to_discovered_service(
        "mdns-svc".to_string(),
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1)),
        9000,
        &txt,
    );
    assert_eq!(service.id.as_deref(), Some("mdns-svc"));
    assert_eq!(service.endpoints.len(), 1);
    assert_eq!(service.endpoints[0].address, "192.168.1.1");
    assert_eq!(service.endpoints[0].port, 9000);
    assert!(service.healthy);
    assert_eq!(service.capabilities.len(), 2);
}

#[tokio::test]
async fn test_cache_service_id_from_endpoint_when_no_id() {
    let client = MdnsDiscoveryClient::new().unwrap();
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint {
            address: "10.0.0.1".to_string(),
            port: 7777,
            protocol: "http".to_string(),
            path: None,
            metadata: HashMap::new(),
        }],
        healthy: true,
        metadata: HashMap::new(),
    };

    client.cache_service(service).await;

    let cache = client.cache.read().await;
    assert!(cache.contains_key("10.0.0.1:7777"));
}

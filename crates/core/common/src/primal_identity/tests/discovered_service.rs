// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use super::super::*;

#[test]
fn test_capability_matching() {
    let service = DiscoveredService {
        id: Some("test".to_string()),
        capabilities: vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
        ],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_compute_capability());
    assert!(service.has_storage_capability());
    assert!(!service.has_auth_capability());
}

#[test]
fn test_discovered_service_has_capability() {
    let service = DiscoveredService {
        id: Some("test".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_capability(&Capability::Compute(ComputeCapability::GpuCompute)));
    assert!(!service.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
}

#[test]
fn test_discovered_service_has_compute_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![
            Capability::Compute(ComputeCapability::WasmExecution),
            Capability::Storage(StorageCapability::Cache),
        ],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_compute_capability());
}

#[test]
fn test_discovered_service_has_storage_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Storage(StorageCapability::Database)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_storage_capability());
    assert!(!service.has_compute_capability());
}

#[test]
fn test_discovered_service_has_auth_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Authentication(AuthCapability::UserAuth)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_auth_capability());
    assert!(!service.has_compute_capability());
    assert!(!service.has_storage_capability());
}

#[test]
fn test_discovered_service_endpoints_for_protocol() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![
            ServiceEndpoint::http("api1.example.com", 8080),
            ServiceEndpoint::https("api2.example.com", 443),
            ServiceEndpoint::http("api3.example.com", 8081),
        ],
        healthy: true,
        metadata: HashMap::new(),
    };

    let http_endpoints = service.endpoints_for_protocol("http");
    assert_eq!(http_endpoints.len(), 2);

    let tls_endpoints = service.endpoints_for_protocol("https");
    assert_eq!(tls_endpoints.len(), 1);

    let grpc_endpoints = service.endpoints_for_protocol("grpc");
    assert_eq!(grpc_endpoints.len(), 0);
}

#[test]
fn test_discovered_service_with_no_id() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![],
        healthy: false,
        metadata: HashMap::new(),
    };

    assert!(service.id.is_none());
    assert!(!service.healthy);
}

#[test]
fn test_discovered_service_with_crypto_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    assert!(!service.has_compute_capability());
    assert!(!service.has_storage_capability());
    assert!(!service.has_auth_capability());
}

#[test]
fn test_discovered_service_with_discovery_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Discovery(
            DiscoveryCapability::CapabilityDiscovery,
        )],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_capability(&Capability::Discovery(
        DiscoveryCapability::CapabilityDiscovery
    )));
}

#[test]
fn test_discovered_service_with_coordination_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Coordination(
            CoordinationCapability::LoadBalancing,
        )],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };

    assert!(service.has_capability(&Capability::Coordination(
        CoordinationCapability::LoadBalancing
    )));
}

#[test]
fn test_discovered_service_debug_formatting() {
    let service = DiscoveredService {
        id: Some("id".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    let debug_str = format!("{service:?}");
    assert!(!debug_str.is_empty());
}

#[test]
fn test_discovered_service_endpoints_for_protocol_empty() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    let http_eps = service.endpoints_for_protocol("http");
    assert!(http_eps.is_empty());
}

#[test]
fn test_discovered_service_has_capability_empty_caps() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![],
        healthy: false,
        metadata: HashMap::new(),
    };
    assert!(!service.has_capability(&Capability::Compute(ComputeCapability::GpuCompute)));
    assert!(!service.has_compute_capability());
    assert!(!service.has_storage_capability());
    assert!(!service.has_auth_capability());
}

#[test]
fn test_discovered_service_endpoints_for_protocol_mixed() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![
            ServiceEndpoint::grpc("grpc1", 50051),
            ServiceEndpoint::grpc("grpc2", 50052),
            ServiceEndpoint::http("http1", 8080),
        ],
        healthy: true,
        metadata: HashMap::new(),
    };
    let grpc_eps = service.endpoints_for_protocol("grpc");
    assert_eq!(grpc_eps.len(), 2);
}

#[test]
fn test_discovered_service_clone() {
    let service = DiscoveredService {
        id: Some("x".into()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![ServiceEndpoint::http("a", 1)],
        healthy: true,
        metadata: HashMap::new(),
    };
    let cloned = service.clone();
    assert_eq!(service.id, cloned.id);
    assert_eq!(service.capabilities, cloned.capabilities);
}

#[test]
fn test_discovered_service_has_coordination_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        )],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    assert!(service.has_capability(&Capability::Coordination(
        CoordinationCapability::ServiceDiscovery
    )));
}

#[test]
fn test_discovered_service_has_discovery_capability() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![Capability::Discovery(DiscoveryCapability::MdnsDiscovery)],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    assert!(service.has_capability(&Capability::Discovery(DiscoveryCapability::MdnsDiscovery)));
}

#[test]
fn test_discovered_service_has_capability_multiple_same_type() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Compute(ComputeCapability::GpuCompute),
        ],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    assert!(service.has_capability(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(service.has_capability(&Capability::Compute(ComputeCapability::GpuCompute)));
}

#[test]
fn test_discovered_service_endpoints_for_protocol_nonexistent() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        healthy: true,
        metadata: HashMap::new(),
    };
    let grpc_eps = service.endpoints_for_protocol("grpc");
    assert!(grpc_eps.is_empty());
}

#[test]
fn test_discovered_service_with_id_some() {
    let service = DiscoveredService {
        id: Some("svc-123".to_string()),
        capabilities: vec![],
        endpoints: vec![],
        healthy: true,
        metadata: HashMap::new(),
    };
    assert_eq!(service.id.as_deref(), Some("svc-123"));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for primal identity system

use std::collections::HashMap;

use super::*;

#[test]
fn test_toadstool_identity() {
    let identity = ToadStoolIdentity::new();

    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
    assert!(!identity.capabilities().is_empty());
}

#[test]
fn test_service_endpoint_url() {
    let endpoint = ServiceEndpoint::http("localhost", 8080).with_path("/api/v1");

    assert_eq!(endpoint.url(), "http://localhost:8080/api/v1");
}

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
fn test_service_endpoint_http() {
    let endpoint = ServiceEndpoint::http("example.com", 8080);
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.address, "example.com");
    assert_eq!(endpoint.port, 8080);
    assert_eq!(endpoint.url(), "http://example.com:8080");
}

#[test]
fn test_service_endpoint_https() {
    let endpoint = ServiceEndpoint::https("secure.example.com", 443);
    assert_eq!(endpoint.protocol, "https");
    assert_eq!(endpoint.port, 443);
    assert_eq!(endpoint.url(), "https://secure.example.com:443");
}

#[test]
fn test_service_endpoint_grpc() {
    let endpoint = ServiceEndpoint::grpc("grpc.example.com", 9090);
    assert_eq!(endpoint.protocol, "grpc");
    assert_eq!(endpoint.url(), "grpc://grpc.example.com:9090");
}

#[test]
fn test_service_endpoint_jsonrpc_polling() {
    // JSON-RPC 2.0 polling (replacement for deprecated WebSocket)
    let endpoint = ServiceEndpoint::http("api.example.com", 8081).with_path("/jsonrpc");
    assert_eq!(endpoint.protocol, "http");
    assert_eq!(endpoint.url(), "http://api.example.com:8081/jsonrpc");
}

#[test]
fn test_service_endpoint_with_path() {
    let endpoint = ServiceEndpoint::http("api.example.com", 8080).with_path("/v2/compute");
    assert_eq!(endpoint.url(), "http://api.example.com:8080/v2/compute");
}

#[test]
fn test_service_endpoint_with_metadata() {
    let endpoint = ServiceEndpoint::http("api.example.com", 8080)
        .with_metadata("region", "us-west")
        .with_metadata("tier", "production");

    assert_eq!(
        endpoint.metadata.get("region"),
        Some(&"us-west".to_string())
    );
    assert_eq!(
        endpoint.metadata.get("tier"),
        Some(&"production".to_string())
    );
}

#[test]
fn test_toadstool_identity_default_capabilities() {
    let identity = ToadStoolIdentity::new();
    let caps = identity.capabilities();

    assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::WasmExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::GpuCompute)));
}

#[test]
fn test_toadstool_identity_add_endpoint() {
    let mut identity = ToadStoolIdentity::new();
    identity.add_endpoint(ServiceEndpoint::http("localhost", 8080));

    let endpoints = identity.endpoints();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].protocol, "http");
}

#[test]
fn test_toadstool_identity_with_endpoints() {
    let endpoints = vec![
        ServiceEndpoint::http("localhost", 8080),
        ServiceEndpoint::grpc("localhost", 9090),
    ];
    let identity = ToadStoolIdentity::new().with_endpoints(endpoints);

    assert_eq!(identity.endpoints().len(), 2);
}

#[test]
fn test_toadstool_identity_add_capability() {
    let mut identity = ToadStoolIdentity::new();
    let initial_count = identity.capabilities().len();

    identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
    assert_eq!(identity.capabilities().len(), initial_count + 1);

    identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
    assert_eq!(identity.capabilities().len(), initial_count + 1);
}

#[test]
fn test_toadstool_identity_add_metadata() {
    let mut identity = ToadStoolIdentity::new();
    identity.add_metadata("custom_key".to_string(), "custom_value".to_string());

    let metadata = identity.metadata();
    assert_eq!(
        metadata.get("custom_key"),
        Some(&"custom_value".to_string())
    );
}

#[test]
fn test_toadstool_identity_metadata_includes_platform() {
    let identity = ToadStoolIdentity::new();
    let metadata = identity.metadata();

    assert!(metadata.contains_key("platform"));
    assert!(metadata.contains_key("arch"));
    assert!(metadata.contains_key("description"));
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

    let https_endpoints = service.endpoints_for_protocol("https");
    assert_eq!(https_endpoints.len(), 1);

    let grpc_endpoints = service.endpoints_for_protocol("grpc");
    assert_eq!(grpc_endpoints.len(), 0);
}

#[test]
fn test_capability_equality() {
    let cap1 = Capability::Compute(ComputeCapability::NativeExecution);
    let cap2 = Capability::Compute(ComputeCapability::NativeExecution);
    let cap3 = Capability::Compute(ComputeCapability::WasmExecution);

    assert_eq!(cap1, cap2);
    assert_ne!(cap1, cap3);
}

#[test]
fn test_capability_custom() {
    let cap1 = Capability::Custom {
        name: "custom-service".to_string(),
        version: "1.0".to_string(),
    };
    let cap2 = Capability::Custom {
        name: "custom-service".to_string(),
        version: "1.0".to_string(),
    };

    assert_eq!(cap1, cap2);
}

#[test]
fn test_coordination_capability_default() {
    let cap = CoordinationCapability::default();
    assert_eq!(cap, CoordinationCapability::ServiceDiscovery);
}

#[test]
fn test_service_endpoint_clone() {
    let endpoint1 = ServiceEndpoint::http("localhost", 8080);
    let endpoint2 = endpoint1.clone();

    assert_eq!(endpoint1.protocol, endpoint2.protocol);
    assert_eq!(endpoint1.address, endpoint2.address);
    assert_eq!(endpoint1.port, endpoint2.port);
}

#[test]
fn test_toadstool_identity_default() {
    let identity = ToadStoolIdentity::default();
    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
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
fn test_all_compute_capabilities() {
    let caps = vec![
        ComputeCapability::NativeExecution,
        ComputeCapability::ContainerOrchestration,
        ComputeCapability::WasmExecution,
        ComputeCapability::PythonExecution,
        ComputeCapability::GpuCompute,
        ComputeCapability::EdgeExecution,
        ComputeCapability::SpecialtyHardware,
    ];

    assert_eq!(caps.len(), 7);
}

#[test]
fn test_all_storage_capabilities() {
    let caps = vec![
        StorageCapability::ObjectStorage,
        StorageCapability::BlockStorage,
        StorageCapability::FileStorage,
        StorageCapability::Database,
        StorageCapability::Cache,
        StorageCapability::ArtifactStorage,
    ];

    assert_eq!(caps.len(), 6);
}

#[test]
fn test_all_auth_capabilities() {
    let caps = vec![
        AuthCapability::UserAuth,
        AuthCapability::ServiceAuth,
        AuthCapability::TokenManagement,
        AuthCapability::OAuthProvider,
        AuthCapability::SamlProvider,
    ];

    assert_eq!(caps.len(), 5);
}

#[test]
fn test_all_crypto_capabilities() {
    let caps = vec![
        CryptoCapability::Encryption,
        CryptoCapability::KeyManagement,
        CryptoCapability::CertificateAuthority,
        CryptoCapability::SecretsManagement,
        CryptoCapability::HardwareSecurity,
        CryptoCapability::GeneticEntropy,
        CryptoCapability::DigitalSignatures,
        CryptoCapability::Hashing,
    ];

    assert_eq!(caps.len(), 8);
}

#[test]
fn test_all_coordination_capabilities() {
    let caps = vec![
        CoordinationCapability::ServiceDiscovery,
        CoordinationCapability::LoadBalancing,
        CoordinationCapability::HealthChecking,
        CoordinationCapability::ConfigManagement,
        CoordinationCapability::WorkflowOrchestration,
    ];

    assert_eq!(caps.len(), 5);
}

#[test]
fn test_all_discovery_capabilities() {
    let caps = vec![
        DiscoveryCapability::CapabilityDiscovery,
        DiscoveryCapability::DnsDiscovery,
        DiscoveryCapability::MdnsDiscovery,
        DiscoveryCapability::RegistryDiscovery,
    ];

    assert_eq!(caps.len(), 4);
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
fn test_capability_debug_formatting() {
    let cap = Capability::Compute(ComputeCapability::WasmExecution);
    let debug_str = format!("{:?}", cap);
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("Compute"));
    assert!(debug_str.contains("WasmExecution"));

    let custom = Capability::Custom {
        name: "test".to_string(),
        version: "1.0".to_string(),
    };
    let custom_debug = format!("{:?}", custom);
    assert!(!custom_debug.is_empty());
    assert!(custom_debug.contains("Custom"));
}

#[test]
fn test_service_endpoint_debug_formatting() {
    let ep = ServiceEndpoint::http("localhost", 8080);
    let debug_str = format!("{:?}", ep);
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("localhost"));
}

#[test]
fn test_toadstool_identity_debug_formatting() {
    let identity = ToadStoolIdentity::new();
    let debug_str = format!("{:?}", identity);
    assert!(!debug_str.is_empty());
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
    let debug_str = format!("{:?}", service);
    assert!(!debug_str.is_empty());
}

#[test]
fn test_capability_serialize_deserialize() {
    let cap = Capability::Compute(ComputeCapability::GpuCompute);
    let json = serde_json::to_string(&cap).unwrap();
    let deserialized: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(cap, deserialized);

    let custom = Capability::Custom {
        name: "custom-service".to_string(),
        version: "2.0".to_string(),
    };
    let json = serde_json::to_string(&custom).unwrap();
    let deserialized: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(custom, deserialized);
}

#[test]
fn test_toadstool_identity_builder_pattern() {
    let identity = ToadStoolIdentity::new().with_endpoints(vec![
        ServiceEndpoint::http("localhost", 8080),
        ServiceEndpoint::grpc("localhost", 9090),
    ]);
    assert_eq!(identity.endpoints().len(), 2);
}

#[test]
fn test_toadstool_identity_add_capability_no_duplicate() {
    let mut identity = ToadStoolIdentity::new();
    let cap = Capability::Storage(StorageCapability::ObjectStorage);
    let count_before = identity.capabilities().len();
    identity.add_capability(cap.clone());
    identity.add_capability(cap);
    assert_eq!(identity.capabilities().len(), count_before + 1);
}

#[test]
fn test_primal_identity_trait_object() {
    let identity = ToadStoolIdentity::new();
    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
    assert!(!identity.capabilities().is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Additional deep tests for all methods and edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_toadstool_identity_default_capabilities_contains_all() {
    let identity = ToadStoolIdentity::new();
    let caps = identity.capabilities();

    assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(caps.contains(&Capability::Compute(
        ComputeCapability::ContainerOrchestration
    )));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::WasmExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::PythonExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::GpuCompute)));
    assert_eq!(caps.len(), 5);
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
fn test_discovered_service_serialize_deserialize() {
    let service = DiscoveredService {
        id: Some("svc-1".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        healthy: true,
        metadata: {
            let mut m = HashMap::new();
            m.insert("version".into(), "1.0".into());
            m
        },
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(service.id, restored.id);
    assert_eq!(service.healthy, restored.healthy);
    assert_eq!(service.capabilities.len(), restored.capabilities.len());
}

#[test]
fn test_service_endpoint_with_path_none_remains_none() {
    let endpoint = ServiceEndpoint::http("localhost", 8080);
    assert!(endpoint.path.is_none());
}

#[test]
fn test_capability_compute_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cap1 = Capability::Compute(ComputeCapability::WasmExecution);
    let cap2 = Capability::Compute(ComputeCapability::WasmExecution);
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    cap1.hash(&mut h1);
    cap2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_toadstool_identity_add_capability_dedup() {
    let mut identity = ToadStoolIdentity::new();
    let cap = Capability::Compute(ComputeCapability::EdgeExecution);
    let len_before = identity.capabilities().len();
    identity.add_capability(cap.clone());
    identity.add_capability(cap);
    assert_eq!(identity.capabilities().len(), len_before + 1);
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
fn test_service_endpoint_grpc_with_path() {
    let endpoint = ServiceEndpoint::grpc("grpc.example.com", 50051).with_path("/service.Greeter");
    assert_eq!(
        endpoint.url(),
        "grpc://grpc.example.com:50051/service.Greeter"
    );
}

#[test]
fn test_service_endpoint_https_with_path() {
    let endpoint = ServiceEndpoint::https("api.secure.com", 443).with_path("/v2/graphql");
    assert_eq!(endpoint.url(), "https://api.secure.com:443/v2/graphql");
}

#[test]
fn test_coordination_capability_all_variants() {
    let _ = CoordinationCapability::ServiceDiscovery;
    let _ = CoordinationCapability::LoadBalancing;
    let _ = CoordinationCapability::HealthChecking;
    let _ = CoordinationCapability::ConfigManagement;
    let _ = CoordinationCapability::WorkflowOrchestration;
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
fn test_capability_coordination_default() {
    let default = CoordinationCapability::default();
    assert_eq!(default, CoordinationCapability::ServiceDiscovery);
}

#[test]
fn test_primal_identity_metadata_platform_arch() {
    let identity = ToadStoolIdentity::new();
    let meta = identity.metadata();
    assert!(meta.contains_key("platform"));
    assert!(meta.contains_key("arch"));
    assert!(meta.get("platform").map(|s| !s.is_empty()).unwrap_or(false));
}

#[test]
fn test_service_endpoint_eq() {
    let ep1 = ServiceEndpoint::http("localhost", 8080);
    let ep2 = ServiceEndpoint::http("localhost", 8080);
    assert_eq!(ep1, ep2);
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
fn test_toadstool_identity_capabilities_clone() {
    let identity = ToadStoolIdentity::new();
    let caps1 = identity.capabilities();
    let caps2 = identity.capabilities();
    assert_eq!(caps1.len(), caps2.len());
}

#[test]
fn test_toadstool_identity_endpoints_clone() {
    let identity =
        ToadStoolIdentity::new().with_endpoints(vec![ServiceEndpoint::http("localhost", 8080)]);
    let eps1 = identity.endpoints();
    let eps2 = identity.endpoints();
    assert_eq!(eps1.len(), eps2.len());
    assert_eq!(eps1[0].protocol, eps2[0].protocol);
}

#[test]
fn test_discovered_service_serde_roundtrip_empty() {
    let service = DiscoveredService {
        id: None,
        capabilities: vec![],
        endpoints: vec![],
        healthy: false,
        metadata: HashMap::new(),
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(service.id, restored.id);
    assert_eq!(service.healthy, restored.healthy);
    assert!(restored.capabilities.is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Additional edge case tests for coverage
// ═══════════════════════════════════════════════════════════════════

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
fn test_toadstool_identity_version_contains_semver() {
    let identity = ToadStoolIdentity::new();
    let version = identity.version();
    assert!(!version.is_empty());
    assert!(
        version
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            || version.contains('.')
    );
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

#[test]
fn test_capability_compute_edge_execution() {
    let cap = Capability::Compute(ComputeCapability::EdgeExecution);
    let json = serde_json::to_string(&cap).expect("serialize");
    let restored: Capability = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(
        restored,
        Capability::Compute(ComputeCapability::EdgeExecution)
    ));
}

#[test]
fn test_capability_compute_specialty_hardware() {
    let cap = Capability::Compute(ComputeCapability::SpecialtyHardware);
    let json = serde_json::to_string(&cap).expect("serialize");
    let restored: Capability = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(
        restored,
        Capability::Compute(ComputeCapability::SpecialtyHardware)
    ));
}

#[test]
fn test_capability_storage_all_variants_serde() {
    let variants = [
        StorageCapability::BlockStorage,
        StorageCapability::FileStorage,
        StorageCapability::Database,
        StorageCapability::Cache,
        StorageCapability::ArtifactStorage,
    ];
    for cap in &variants {
        let cap_enum = Capability::Storage(cap.clone());
        let json = serde_json::to_string(&cap_enum).expect("serialize");
        let restored: Capability = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&cap_enum, &restored);
    }
}

#[test]
fn test_discovered_service_metadata_roundtrip() {
    let mut meta = HashMap::new();
    meta.insert("region".to_string(), "us-east".to_string());
    meta.insert("version".to_string(), "2.0".to_string());
    let service = DiscoveredService {
        id: Some("m".to_string()),
        capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
        endpoints: vec![ServiceEndpoint::http("x", 80)],
        healthy: true,
        metadata: meta.clone(),
    };
    let json = serde_json::to_string(&service).unwrap();
    let restored: DiscoveredService = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.metadata.get("region"),
        Some(&"us-east".to_string())
    );
    assert_eq!(restored.metadata.get("version"), Some(&"2.0".to_string()));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::primal_identity::{
    Capability, ComputeCapability, CoordinationCapability, ServiceEndpoint, StorageCapability,
};

use super::*;

#[test]
fn test_parse_capabilities_empty() {
    let caps = ServiceDiscovery::parse_capabilities("");
    assert!(caps.is_empty());
}

#[test]
fn test_parse_capabilities_coordination() {
    let caps = ServiceDiscovery::parse_capabilities("coordination");
    assert_eq!(caps.len(), 1);
    assert!(matches!(
        caps[0],
        Capability::Coordination(CoordinationCapability::ServiceDiscovery)
    ));
}

#[test]
fn test_parse_capabilities_storage() {
    let caps = ServiceDiscovery::parse_capabilities("storage");
    assert_eq!(caps.len(), 1);
    assert!(matches!(
        caps[0],
        Capability::Storage(StorageCapability::ObjectStorage)
    ));
}

#[test]
fn test_parse_capabilities_compute() {
    let caps = ServiceDiscovery::parse_capabilities("compute");
    assert_eq!(caps.len(), 1);
    assert!(matches!(
        caps[0],
        Capability::Compute(ComputeCapability::NativeExecution)
    ));
}

#[test]
fn test_parse_capabilities_multiple() {
    let caps = ServiceDiscovery::parse_capabilities("coordination, storage, compute");
    assert_eq!(caps.len(), 3);
}

#[test]
fn test_parse_capabilities_unknown_filtered() {
    let caps = ServiceDiscovery::parse_capabilities("coordination, unknown_cap, storage");
    assert_eq!(caps.len(), 2);
}

#[test]
fn test_discovered_service_has_capability() {
    let service = DiscoveredService {
        id: "test-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
        ],
        endpoints: vec![],
        metadata: std::collections::HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert!(service.has_capability(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(service.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
    assert!(!service.has_capability(&Capability::Coordination(
        CoordinationCapability::ServiceDiscovery
    )));
}

#[test]
fn test_discovered_service_is_fresh() {
    let now = SystemTime::now();
    let service = DiscoveredService {
        id: "test-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: std::collections::HashMap::new(),
        discovered_at: now,
        last_seen: now,
        healthy: true,
    };
    assert!(service.is_fresh(Duration::from_secs(60)));
}

#[test]
fn test_discovered_service_is_stale() {
    let old = UNIX_EPOCH;
    let service = DiscoveredService {
        id: "test-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: std::collections::HashMap::new(),
        discovered_at: old,
        last_seen: old,
        healthy: true,
    };
    assert!(!service.is_fresh(Duration::from_secs(1)));
}

#[test]
fn test_discovered_service_primary_endpoint() {
    let endpoint =
        ServiceEndpoint::from_url_string("http://localhost:8080").expect("valid url");
    let service = DiscoveredService {
        id: "test-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![endpoint.clone()],
        metadata: std::collections::HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    assert_eq!(service.primary_endpoint(), Some(&endpoint));
}

#[test]
fn test_discovered_service_healthy_endpoints() {
    let endpoint =
        ServiceEndpoint::from_url_string("http://localhost:8080").expect("valid url");
    let service = DiscoveredService {
        id: "test-1".to_string(),
        name: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![endpoint],
        metadata: std::collections::HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let healthy = service.healthy_endpoints();
    assert_eq!(healthy.len(), 1);
}

#[test]
fn test_discovery_method_variants() {
    assert_eq!(DiscoveryMethod::Auto, DiscoveryMethod::Auto);
    assert_eq!(DiscoveryMethod::Mdns, DiscoveryMethod::Mdns);
    assert_eq!(DiscoveryMethod::Environment, DiscoveryMethod::Environment);
    assert_ne!(DiscoveryMethod::Auto, DiscoveryMethod::Mdns);
}

#[test]
fn test_discover_from_fallbacks_prefers_eco_primals_unix_socket() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let eco = tmp.path().join("ecoPrimals");
    std::fs::create_dir_all(&eco).expect("mkdir");
    std::fs::File::create(eco.join("toadstool.sock")).expect("touch");

    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(tmp.path().to_str().unwrap()),
        || {
            let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
            let services = disc.discover_from_fallbacks().expect("ok");
            assert_eq!(services.len(), 1);
            assert_eq!(services[0].endpoints[0].protocol, "unix");
            assert!(services[0].endpoints[0].address.contains("toadstool.sock"));
            assert_eq!(
                services[0].metadata.get("source").map(String::as_str),
                Some("fallback-unix-socket")
            );
        },
    );
}

#[test]
fn test_discover_from_fallbacks_tcp_when_no_socket() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let eco = tmp.path().join("ecoPrimals");
    std::fs::create_dir_all(&eco).expect("mkdir");

    temp_env::with_vars(
        [
            ("XDG_RUNTIME_DIR", Some(tmp.path().to_str().unwrap())),
            ("TOADSTOOL_URL", Some("http://localhost:8084")),
        ],
        || {
            let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
            let services = disc.discover_from_fallbacks().expect("ok");
            assert_eq!(services.len(), 1);
            assert_eq!(services[0].endpoints[0].protocol, "http");
            assert_eq!(
                services[0].metadata.get("source").map(String::as_str),
                Some("fallback-tcp")
            );
            assert_eq!(
                services[0].metadata.get("deprecation").map(String::as_str),
                Some("tcp_url_fallback")
            );
        },
    );
}

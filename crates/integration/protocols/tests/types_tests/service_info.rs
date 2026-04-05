// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_service_info_creation() {
    let endpoint = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: Arc::from("svc-123"),
        name: Arc::from("ToadStool Compute"),
        version: "1.0.0".to_string(),
        endpoints: vec![endpoint],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec!["execute".to_string(), "schedule".to_string()],
    };

    assert_eq!(&*service.id, "svc-123");
    assert_eq!(&*service.name, "ToadStool Compute");
    assert_eq!(service.health_status, HealthStatus::Healthy);
    assert_eq!(service.capabilities.len(), 2);
    assert_eq!(service.endpoints.len(), 1);
}

#[test]
fn test_service_info_multiple_endpoints() {
    let ep1 = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "localhost".to_string(),
        port: 8080,
        path: None,
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let ep2 = ServiceEndpoint {
        id: "ep-2".to_string(),
        transport: TransportType::TRpc,
        address: "localhost".to_string(),
        port: 9000,
        path: Some("/ws".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };

    let service = ServiceInfo {
        id: Arc::from("multi-endpoint-service"),
        name: Arc::from("Multi-Endpoint Service"),
        version: "2.0.0".to_string(),
        endpoints: vec![ep1, ep2],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.endpoints.len(), 2);
    assert_eq!(service.endpoints[0].transport, TransportType::Http);
    assert_eq!(service.endpoints[1].transport, TransportType::TRpc);
}

#[test]
fn test_service_info_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("region".to_string(), "us-west-2".to_string());
    metadata.insert("zone".to_string(), "a".to_string());

    let service = ServiceInfo {
        id: Arc::from("svc-meta"),
        name: Arc::from("Metadata Service"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata,
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert_eq!(service.metadata.len(), 2);
    assert_eq!(
        service.metadata.get("region"),
        Some(&"us-west-2".to_string())
    );
}

#[test]
fn test_service_info_with_capabilities() {
    let service = ServiceInfo {
        id: Arc::from("capable-svc"),
        name: Arc::from("Capable Service"),
        version: "2.1.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![
            "compute".to_string(),
            "storage".to_string(),
            "ml".to_string(),
        ],
    };

    assert_eq!(service.capabilities.len(), 3);
    assert!(service.capabilities.contains(&"compute".to_string()));
}

#[test]
fn test_service_info_version_parsing() {
    let service = ServiceInfo {
        id: Arc::from("versioned"),
        name: Arc::from("Versioned Service"),
        version: "3.2.1-beta".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert!(service.version.contains("beta"));
    assert!(service.version.starts_with("3.2.1"));
}

#[test]
fn test_service_info_no_endpoints() {
    let service = ServiceInfo {
        id: Arc::from("no-endpoints"),
        name: Arc::from("Configuring Service"),
        version: "0.1.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Unknown,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    assert!(service.endpoints.is_empty());
}

#[test]
fn test_service_info_timestamp() {
    let now = std::time::SystemTime::now();
    let service = ServiceInfo {
        id: Arc::from("timestamp-test"),
        name: Arc::from("Timestamp Test"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: now,
        capabilities: vec![],
    };

    assert_eq!(service.last_seen, now);
}

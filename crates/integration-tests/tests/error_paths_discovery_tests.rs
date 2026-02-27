//! Error path tests for runtime discovery
//!
//! Tests error handling and failure scenarios in the primal discovery system,
//! exercising `RuntimeDiscovery`'s register, find, and stats API paths.

use std::collections::HashMap;
use std::sync::Arc;
use toadstool::runtime_discovery::{DiscoveryConfig, RuntimeDiscovery};
use toadstool::self_identity::{Capability, DiscoveredService, SelfIdentity};
use uuid::Uuid;

fn make_capability(name: &str) -> Capability {
    Capability {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        features: vec![],
        characteristics: HashMap::new(),
    }
}

fn make_service(primal_type: &str, endpoint: &str, caps: Vec<Capability>) -> DiscoveredService {
    let now = std::time::SystemTime::now();
    DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: primal_type.to_string(),
        version: "1.0.0".to_string(),
        capabilities: caps,
        endpoint: endpoint.to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: now,
        last_seen: now,
    }
}

#[tokio::test]
async fn test_discovery_with_invalid_endpoint() {
    // Registration should succeed; validation happens only on use.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service(
        "storage",
        "not-a-valid-url",
        vec![make_capability("storage")],
    );

    let result = discovery.register_service(service).await;
    assert!(
        result.is_ok(),
        "Service registration should succeed even with an invalid endpoint"
    );
}

#[tokio::test]
async fn test_discovery_network_timeout() {
    // A very short timeout doesn't prevent startup — no services exist yet.
    let identity = SelfIdentity::new();
    let config = DiscoveryConfig {
        service_timeout: std::time::Duration::from_millis(10),
        ..Default::default()
    };

    let discovery = RuntimeDiscovery::with_config(identity, config);
    discovery
        .start()
        .await
        .expect("Discovery should start with minimal timeout");

    let stats = discovery.get_stats().await;
    assert_eq!(
        stats.active_services, 0,
        "No services should be active initially"
    );
}

#[tokio::test]
async fn test_discovery_connection_refused() {
    // Register a service whose endpoint is not listening.
    // Connection error surfaces only when actually used, not at registration time.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service(
        "compute",
        "http://localhost:9999",
        vec![make_capability("compute")],
    );

    discovery
        .register_service(service)
        .await
        .expect("Service registration should succeed");

    let found = discovery
        .find_by_capability("compute")
        .await
        .expect("find_by_capability should succeed");
    assert_eq!(found.len(), 1, "Should find one registered compute service");
}

#[tokio::test]
async fn test_discovery_no_matching_capability() {
    // find_by_capability returns an empty Vec — not an error — for unknown cap names.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let not_found = discovery
        .find_by_capability("nonexistent")
        .await
        .expect("Querying an unknown capability should return an empty list");
    assert_eq!(
        not_found.len(),
        0,
        "Should find no services for unknown capability"
    );
}

#[tokio::test]
async fn test_discovery_minimal_service() {
    // A service with empty metadata/features should be accepted.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service(
        "minimal",
        "http://localhost:8080",
        vec![make_capability("minimal")],
    );

    let result = discovery.register_service(service).await;
    assert!(
        result.is_ok(),
        "Should accept a service with only required fields"
    );

    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 1, "Should have one registered service");
}

#[tokio::test]
async fn test_discovery_with_empty_capability_list() {
    // Services with no capabilities are registered but never matched by find_by_capability.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service("empty", "http://localhost:8080", vec![]);

    let result = discovery.register_service(service).await;
    assert!(
        result.is_ok(),
        "Should accept a service with no capabilities"
    );

    let found = discovery
        .find_by_capability("any")
        .await
        .expect("Search should succeed");
    assert_eq!(found.len(), 0, "No capabilities → no match");
}

#[tokio::test]
async fn test_discovery_concurrent_registrations() {
    // Ten concurrent registration tasks should all succeed and be visible.
    let identity = SelfIdentity::new();
    let discovery = Arc::new(RuntimeDiscovery::new(identity));

    let mut handles = vec![];

    for i in 0..10 {
        let d = Arc::clone(&discovery);
        let handle = tokio::spawn(async move {
            let service = make_service(
                &format!("service-{i}"),
                &format!("http://localhost:808{i}"),
                vec![make_capability(&format!("cap-{i}"))],
            );
            d.register_service(service)
                .await
                .expect("Concurrent registration should succeed");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task should complete successfully");
    }

    let all = discovery.get_all_services().await;
    assert_eq!(
        all.len(),
        10,
        "All 10 concurrently registered services should be visible"
    );
}

#[tokio::test]
async fn test_discovery_cache_corruption() {
    // Resilience against corrupted/malformed data in discovery.
    // RuntimeDiscovery stores services in-memory; we verify the system remains
    // operational when services have malformed or edge-case endpoint data.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register services with various "corrupted-looking" endpoint values
    let long_string = "x".repeat(10000);
    let malformed_endpoints: [&str; 5] = [
        "",
        ":::",
        "http://",
        "not-a-url-at-all!!!@@###",
        &long_string,
    ];

    for (i, ep) in malformed_endpoints.iter().enumerate() {
        let service = make_service(
            &format!("corrupt-{i}"),
            ep,
            vec![make_capability(&format!("corrupt-cap-{i}"))],
        );
        let result = discovery.register_service(service).await;
        assert!(
            result.is_ok(),
            "Discovery should accept service with malformed endpoint '{}'",
            &ep[..ep.len().min(50)]
        );
    }

    // Discovery should remain operational: get_all_services and find_by_capability
    // must complete without panicking
    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 5, "All malformed services should be stored");

    for i in 0..5 {
        let found = discovery
            .find_by_capability(&format!("corrupt-cap-{i}"))
            .await
            .expect("find_by_capability should succeed");
        assert_eq!(found.len(), 1);
    }
}

#[tokio::test]
async fn test_discovery_dns_resolution_failure() {
    // Unresolvable hostname error path: registering a service whose endpoint
    // cannot be resolved. Discovery accepts registration (optimistic); we
    // verify that attempting to connect to such an endpoint yields a proper
    // DNS/connection error (not a panic).
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service(
        "unresolvable",
        "this-hostname-definitely-does-not-exist-12345.invalid:9999",
        vec![make_capability("unresolvable")],
    );

    discovery
        .register_service(service)
        .await
        .expect("Registration should succeed");

    let found = discovery
        .find_by_capability("unresolvable")
        .await
        .expect("find_by_capability should succeed");
    assert_eq!(found.len(), 1);

    // Attempt connection; must fail with a proper error (DNS resolution or connection)
    let endpoint = &found[0].endpoint;
    let connect_result = tokio::net::TcpStream::connect(endpoint).await;

    assert!(
        connect_result.is_err(),
        "Connection to unresolvable hostname should fail"
    );

    let err = connect_result.unwrap_err();
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("no address")
            || err_str.contains("dns")
            || err_str.contains("name")
            || err_str.contains("resolve")
            || err_str.contains("not found")
            || err_str.contains("connection"),
        "Error should indicate DNS/connection failure: {}",
        err
    );
}

#[tokio::test]
async fn test_discovery_ssl_certificate_error() {
    // TLS certificate error path: a service endpoint with invalid/self-signed
    // certificate should be rejected when connecting via HTTPS.
    // Uses badssl.com endpoints known to have certificate issues.
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = make_service(
        "badssl",
        "https://self-signed.badssl.com:443",
        vec![make_capability("badssl")],
    );

    discovery
        .register_service(service)
        .await
        .expect("Registration should succeed");

    let found = discovery
        .find_by_capability("badssl")
        .await
        .expect("find_by_capability should succeed");
    assert_eq!(found.len(), 1);

    // Attempt HTTPS request; TLS verification should fail
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
        .build()
        .expect("reqwest client");

    let connect_result = client.get("https://self-signed.badssl.com/").send().await;

    assert!(
        connect_result.is_err(),
        "HTTPS request to self-signed cert should fail with TLS error"
    );

    let err = connect_result.unwrap_err();
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("certificate")
            || err_str.contains("tls")
            || err_str.contains("ssl")
            || err_str.contains("invalid")
            || err_str.contains("verify"),
        "Error should indicate TLS/certificate failure: {}",
        err
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;
use std::pin::Pin;

use super::*;
use toadstool_common::primal_identity::{CoordinationCapability, ServiceEndpoint};
use toadstool_common::runtime_discovery::{DiscoveryClient, LocalhostDiscoveryClient};

/// Test fallback endpoint (port 50001)
const TEST_FALLBACK_50001: &str = "http://localhost:50001";
/// Test fallback endpoint (port 9999)
const TEST_FALLBACK_9999: &str = "http://localhost:9999";
/// Test fallback endpoint (port 8888)
const TEST_FALLBACK_8888: &str = "http://localhost:8888";
/// Test fallback endpoint (port 7777)
const TEST_FALLBACK_7777: &str = "http://localhost:7777";

/// Test discovery client that returns configurable results
struct TestDiscoveryClient {
    services: Vec<DiscoveredService>,
    fail: bool,
}

impl DiscoveryClient for TestDiscoveryClient {
    fn discover_by_capability<'a>(
        &'a self,
        _capability: &'a Capability,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move {
            if self.fail {
                return Err(toadstool_common::ToadStoolError::Integration(
                    toadstool_common::error::IntegrationError::ServiceUnavailable {
                        service: "test".to_string(),
                        reason: "forced failure".to_string(),
                    },
                ));
            }
            Ok(self.services.clone())
        })
    }

    fn discover_all<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move { Ok(self.services.clone()) })
    }

    fn register_service<'a>(
        &'a self,
        _service: &'a DiscoveredService,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn deregister_service<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn health_check<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>> {
        Box::pin(async move { Ok(true) })
    }
}

#[tokio::test]
async fn test_discover_or_fallback_uses_fallback() {
    // When discovery fails or returns no results, should use fallback
    let client = Arc::new(LocalhostDiscoveryClient::new());
    let discovery = RuntimeDiscovery::new(client);
    let fallback = TEST_FALLBACK_50001;

    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();

    assert_eq!(result, fallback);
}

#[tokio::test]
async fn test_discover_or_fallback_uses_discovered_service_with_endpoints() {
    let service_with_endpoint = DiscoveredService {
        id: Some("coord-1".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![ServiceEndpoint::http("discovered.host", 9000)],
        healthy: true,
        metadata: std::collections::HashMap::new(),
    };
    let client = Arc::new(TestDiscoveryClient {
        services: vec![service_with_endpoint],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);
    let fallback = TEST_FALLBACK_50001;

    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();

    assert_eq!(result, "http://discovered.host:9000");
}

#[tokio::test]
async fn test_discover_or_fallback_uses_fallback_when_service_has_no_endpoints() {
    let service_no_endpoints = DiscoveredService {
        id: Some("coord-1".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![],
        healthy: true,
        metadata: std::collections::HashMap::new(),
    };
    let client = Arc::new(TestDiscoveryClient {
        services: vec![service_no_endpoints],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);
    let fallback = TEST_FALLBACK_9999;

    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();

    assert_eq!(result, fallback);
}

#[tokio::test]
async fn test_discover_or_fallback_uses_fallback_when_no_services() {
    let client = Arc::new(TestDiscoveryClient {
        services: vec![],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);
    let fallback = TEST_FALLBACK_8888;

    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();

    assert_eq!(result, fallback);
}

#[tokio::test]
async fn test_discover_or_fallback_uses_fallback_on_discovery_error() {
    let client = Arc::new(TestDiscoveryClient {
        services: vec![],
        fail: true,
    });
    let discovery = RuntimeDiscovery::new(client);
    let fallback = TEST_FALLBACK_7777;

    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();

    assert_eq!(result, fallback);
}

#[tokio::test]
async fn test_discover_all_by_capability() {
    let service = DiscoveredService {
        id: Some("coord-1".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![ServiceEndpoint::http("host1", 8080)],
        healthy: true,
        metadata: std::collections::HashMap::new(),
    };
    let client = Arc::new(TestDiscoveryClient {
        services: vec![service],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);

    let services = discover_all_by_capability(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
    )
    .await
    .unwrap();

    assert_eq!(services.len(), 1);
    assert_eq!(services[0].id.as_deref(), Some("coord-1"));
}

#[tokio::test]
async fn test_discover_with_load_balancing_uses_discovered_service() {
    let service = DiscoveredService {
        id: Some("coord-1".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![ServiceEndpoint::http("lb.host", 9001)],
        healthy: true,
        metadata: std::collections::HashMap::new(),
    };
    let client = Arc::new(TestDiscoveryClient {
        services: vec![service],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);

    let result = discover_with_load_balancing(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        "http://fallback:5000",
    )
    .await
    .unwrap();

    assert_eq!(result, "http://lb.host:9001");
}

#[tokio::test]
async fn test_discover_with_load_balancing_uses_fallback_when_no_endpoints() {
    let service = DiscoveredService {
        id: Some("coord-1".to_string()),
        capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
        endpoints: vec![],
        healthy: true,
        metadata: std::collections::HashMap::new(),
    };
    let client = Arc::new(TestDiscoveryClient {
        services: vec![service],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);

    let result = discover_with_load_balancing(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        "http://fallback:6000",
    )
    .await
    .unwrap();

    assert_eq!(result, "http://fallback:6000");
}

#[tokio::test]
async fn test_discover_with_load_balancing_uses_fallback_when_no_services() {
    let client = Arc::new(TestDiscoveryClient {
        services: vec![],
        fail: false,
    });
    let discovery = RuntimeDiscovery::new(client);

    let result = discover_with_load_balancing(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        "http://fallback:7000",
    )
    .await
    .unwrap();

    assert_eq!(result, "http://fallback:7000");
}

#[tokio::test]
async fn test_discover_with_load_balancing_uses_fallback_on_error() {
    let client = Arc::new(TestDiscoveryClient {
        services: vec![],
        fail: true,
    });
    let discovery = RuntimeDiscovery::new(client);

    let result = discover_with_load_balancing(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        "http://fallback:8000",
    )
    .await
    .unwrap();

    assert_eq!(result, "http://fallback:8000");
}

#[tokio::test]
async fn test_create_discovery() {
    let discovery = create_discovery();
    assert!(discovery.is_ok());
}

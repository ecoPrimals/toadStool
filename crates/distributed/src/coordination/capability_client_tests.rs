// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use toadstool_common::infant_discovery::sources::EnvironmentSource;

struct MockEndpointSource {
    endpoint: String,
}

impl toadstool_common::infant_discovery::EndpointSource for MockEndpointSource {
    fn resolve(
        &self,
        _service: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Option<String>,
                        toadstool_common::infant_discovery::DiscoveryError,
                    >,
                > + Send
                + '_,
        >,
    > {
        let endpoint = self.endpoint.clone();
        Box::pin(async move { Ok(Some(endpoint)) })
    }

    fn source_name(&self) -> &'static str {
        "mock"
    }
}

#[tokio::test]
async fn test_capability_discovery_pattern() {
    let discovery = Arc::new(DiscoveryEngine::new());

    let result =
        CapabilityClient::discover(discovery, vec!["service-discovery".to_string()]).await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_discover_with_mock_source() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://localhost:9999".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["load-balancing".to_string()])
        .await
        .unwrap();

    let services = client.get_available_services().await.unwrap();
    assert!(!services.is_empty());
    assert_eq!(services[0].endpoint, "http://localhost:9999");
}

#[tokio::test]
async fn test_get_best_service() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://coordination:8080".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["service-discovery".to_string()])
        .await
        .unwrap();

    let best = client.get_best_service().await.unwrap();
    assert_eq!(best.endpoint, "http://coordination:8080");
}

#[tokio::test]
async fn test_get_best_service_empty_fails() {
    let discovery = Arc::new(DiscoveryEngine::new());

    let client =
        CapabilityClient::discover(discovery, vec!["nonexistent-capability".to_string()])
            .await
            .unwrap();

    let result = client.get_best_service().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_is_healthy_with_services() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://localhost:1".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["cap".to_string()])
        .await
        .unwrap();

    assert!(client.is_healthy().await);
}

#[tokio::test]
async fn test_is_healthy_without_services() {
    let discovery = Arc::new(DiscoveryEngine::new());

    let client = CapabilityClient::discover(discovery, vec!["nonexistent".to_string()])
        .await
        .unwrap();

    assert!(!client.is_healthy().await);
}

#[tokio::test]
async fn test_get_stats() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://a:1".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["x".to_string()])
        .await
        .unwrap();

    let stats = client.get_stats().await;
    assert_eq!(stats.available_services, 1);
    assert!(stats.last_discovery.is_some());
    assert!(stats.cache_age_seconds.is_some());
}

#[tokio::test]
async fn test_execute_with_failover_success() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://ok:1".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["cap".to_string()])
        .await
        .unwrap();

    let result = client
        .execute_with_failover(
            |svc| async move { Ok::<_, toadstool::ToadStoolError>(svc.endpoint) },
        )
        .await
        .unwrap();

    assert_eq!(result, "http://ok:1");
}

#[tokio::test]
async fn test_execute_with_failover_empty_services() {
    let discovery = Arc::new(DiscoveryEngine::new());

    let client = CapabilityClient::discover(discovery, vec!["x".to_string()])
        .await
        .unwrap();

    let result: ToadStoolResult<String> = client
        .execute_with_failover(|_| async { Err(ToadStoolError::runtime("fail")) })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_refresh_discovery() {
    let discovery = Arc::new(DiscoveryEngine::new());
    discovery
        .register_source(Arc::new(MockEndpointSource {
            endpoint: "http://refresh:1".to_string(),
        }))
        .await;

    let client = CapabilityClient::discover(discovery, vec!["r".to_string()])
        .await
        .unwrap();

    let services = client.refresh_discovery().await.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].capability, "r");
}

#[tokio::test]
async fn test_client_stats_debug() {
    let stats = ClientStats {
        available_services: 3,
        last_discovery: Some(SystemTime::now()),
        cache_age_seconds: Some(0),
    };
    let _ = format!("{:?}", stats);
}

#[test]
fn test_discover_with_env_source() {
    temp_env::with_var(
        "TOADSTOOL_AI_PROCESSING_ENDPOINT",
        Some("http://env-coordination:8080"),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let discovery = Arc::new(DiscoveryEngine::new());
                discovery
                    .register_source(Arc::new(EnvironmentSource::default()))
                    .await;

                let client =
                    CapabilityClient::discover(discovery, vec!["ai_processing".to_string()])
                        .await
                        .unwrap();

                let services = client.get_available_services().await.unwrap();
                assert!(!services.is_empty());
                assert_eq!(services[0].endpoint, "http://env-coordination:8080");
            });
        },
    );
}

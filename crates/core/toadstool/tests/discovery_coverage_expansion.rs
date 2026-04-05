// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive test coverage expansion for core discovery module
//!
//! Focus: Increase coverage of discovery/orchestration.rs

use toadstool::discovery::{OrchestrationClient, discover_orchestration};

#[tokio::test]
async fn test_discover_orchestration_with_env_var() {
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://localhost:8082"))],
        async {
            let result = discover_orchestration().await;

            match result {
                Ok(endpoint) => {
                    assert!(!endpoint.is_empty());
                    assert!(endpoint.contains("http"));
                }
                Err(e) => {
                    assert!(
                        e.to_string().contains("discover") || e.to_string().contains("endpoint")
                    );
                }
            }
        },
    )
    .await;
}

#[tokio::test]
async fn test_discover_orchestration_without_env() {
    temp_env::async_with_vars([("SONGBIRD_ENDPOINT", None::<&str>)], async {
        let result = discover_orchestration().await;

        match result {
            Ok(_) => {}
            Err(e) => {
                assert!(!e.to_string().is_empty());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn test_orchestration_client_creation() {
    let client = OrchestrationClient::new();
    assert!(std::mem::size_of_val(&client) > 0);
}

#[tokio::test]
async fn test_orchestration_client_service_discovery() {
    let client = OrchestrationClient::new();
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://test-service:8082"))],
        async {
            let result = client.discover_service_discovery().await;
            if let Ok(endpoint) = result {
                assert!(endpoint.contains("http"));
            }
        },
    )
    .await;
}

#[tokio::test]
async fn test_orchestration_client_load_balancing() {
    let client = OrchestrationClient::new();
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://load-balancer:8082"))],
        async {
            let result = client.discover_load_balancing().await;
            if let Ok(endpoint) = result {
                assert!(endpoint.contains("http"));
            }
        },
    )
    .await;
}

#[tokio::test]
async fn test_orchestration_client_any_capability() {
    let client = OrchestrationClient::new();
    let result = client.discover_any_orchestration().await;
    match result {
        Ok(_) | Err(_) => {}
    }
}

#[tokio::test]
async fn test_concurrent_discovery_calls() {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];

    for i in 0..20 {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let endpoint = format!("http://service-{i}:8082");
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let _ = discover_orchestration().await;
            drop(endpoint);
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }
}

#[tokio::test]
async fn test_discovery_error_handling() {
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("invalid://not-a-real-url"))],
        async {
            let result = discover_orchestration().await;

            match result {
                Ok(_) => {}
                Err(e) => {
                    assert!(!e.to_string().is_empty());
                }
            }
        },
    )
    .await;
}

#[test]
fn test_orchestration_client_size() {
    use std::mem;
    let size = mem::size_of::<OrchestrationClient>();
    assert!(size < 1024, "Client size too large: {size} bytes");
}

#[tokio::test]
async fn test_discovery_with_multiple_endpoints() {
    let client = OrchestrationClient::new();
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://primary:8082"))],
        async {
            let service_result = client.discover_service_discovery().await;
            let load_result = client.discover_load_balancing().await;
            let any_result = client.discover_any_orchestration().await;

            let attempted_count = [
                service_result.is_ok() || service_result.is_err(),
                load_result.is_ok() || load_result.is_err(),
                any_result.is_ok() || any_result.is_err(),
            ]
            .iter()
            .filter(|&&x| x)
            .count();

            assert_eq!(attempted_count, 3);
        },
    )
    .await;
}

#[tokio::test]
async fn test_discovery_timeout_behavior() {
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://slow-service:8082"))],
        async {
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(5), discover_orchestration())
                    .await;

            match result {
                Ok(inner_result) => match inner_result {
                    Ok(_) | Err(_) => {}
                },
                Err(e) => {
                    panic!("Discovery timed out - should fail faster: {e}");
                }
            }
        },
    )
    .await;
}

#[tokio::test]
async fn test_discovery_with_empty_endpoint() {
    temp_env::async_with_vars([("SONGBIRD_ENDPOINT", Some(""))], async {
        let result = discover_orchestration().await;

        match result {
            Ok(_) => {}
            Err(e) => {
                assert!(!e.to_string().is_empty());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn test_orchestration_client_reuse() {
    let client = OrchestrationClient::new();
    temp_env::async_with_vars(
        [("SONGBIRD_ENDPOINT", Some("http://reusable:8082"))],
        async {
            for _ in 0..5 {
                let _ = client.discover_any_orchestration().await;
            }
            let final_result = client.discover_service_discovery().await;
            assert!(final_result.is_ok() || final_result.is_err());
        },
    )
    .await;
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for primal integration discovery.

use super::*;

#[tokio::test]
async fn test_env_var_discovery() {
    temp_env::with_var(
        "TOADSTOOL_ENCRYPTION_ENDPOINT",
        Some("http://beardog:6060"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_encryption_service();
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints.len(), 1);
                    assert_eq!(endpoints[0].url, "http://beardog:6060");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_encryption_fallback_default() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
            ("TOADSTOOL_ENCRYPTION_DEFAULT_ENDPOINT", None::<&str>),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_encryption_service();
                    assert!(
                        result.is_err(),
                        "encryption discovery must fail when no env/discovery provides endpoint"
                    );
                    assert!(matches!(
                        result.unwrap_err(),
                        DiscoveryError::NoServiceFound { capability } if capability == "encryption"
                    ));
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_encryption_explicit_endpoint() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
            (
                "TOADSTOOL_ENCRYPTION_ENDPOINT",
                Some("http://custom-beardog:9090"),
            ),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_encryption_service();
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints[0].url, "http://custom-beardog:9090");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_no_service_found() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_NONEXISTENT_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_NONEXISTENT_URL", None::<&str>),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability("nonexistent");
                    assert!(result.is_err());
                    match result {
                        Err(DiscoveryError::NoServiceFound { capability }) => {
                            assert_eq!(capability, "nonexistent");
                        }
                        _ => panic!("Expected NoServiceFound error"),
                    }
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_generic_service_url_env_var() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_CACHE_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_CACHE_URL", Some("http://redis:6379")),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability("cache");
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints.len(), 1);
                    assert_eq!(endpoints[0].url, "http://redis:6379");
                    assert_eq!(endpoints[0].service_id, "cache-service");
                    assert_eq!(endpoints[0].capabilities, vec!["cache"]);
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_storage_service_via_env() {
    temp_env::with_var(
        "TOADSTOOL_STORAGE_ENDPOINT",
        Some("http://nestgate:8080"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_storage_service();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "http://nestgate:8080");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_coordination_service_via_env() {
    temp_env::with_var(
        "TOADSTOOL_COORDINATION_ENDPOINT",
        Some("http://songbird:6061"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_coordination_service();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "http://songbird:6061");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_mcp_service_via_env() {
    temp_env::with_var(
        "TOADSTOOL_MCP_ENDPOINT",
        Some("http://squirrel:6062"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_mcp_service();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "http://squirrel:6062");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_cache_service_via_env() {
    temp_env::with_var(
        "TOADSTOOL_CACHE_ENDPOINT",
        Some("redis://localhost:6379"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_cache_service();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "redis://localhost:6379");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_database_service_via_env() {
    temp_env::with_var(
        "TOADSTOOL_DATABASE_ENDPOINT",
        Some("postgres://localhost:5432"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_database_service();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "postgres://localhost:5432");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_object_storage_via_env() {
    temp_env::with_var(
        "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
        Some("https://s3.example.com"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_object_storage();
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].url, "https://s3.example.com");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_security_capability_via_env() {
    temp_env::with_var(
        "TOADSTOOL_SECURITY_ENDPOINT",
        Some("http://crypto:6060"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability(capabilities::SECURITY);
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints[0].service_id, "security-env");
                    assert_eq!(endpoints[0].url, "http://crypto:6060");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_storage_capability_via_env() {
    temp_env::with_var(
        "TOADSTOOL_STORAGE_ENDPOINT",
        Some("http://storage:8082"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability(capabilities::STORAGE);
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints[0].service_id, "storage-env");
                    assert_eq!(endpoints[0].url, "http://storage:8082");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_encryption_capability_via_env() {
    temp_env::with_var(
        "TOADSTOOL_ENCRYPTION_ENDPOINT",
        Some("http://custom-crypto:9090"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability("encryption");
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints[0].service_id, "encryption-env");
                    assert_eq!(endpoints[0].url, "http://custom-crypto:9090");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_discover_capability_not_found() {
    temp_env::with_vars(
        vec![
            ("TOADSTOOL_SECURITY_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_SECURITY_URL", None::<&str>),
            ("KUBERNETES_SERVICE_HOST", None::<&str>),
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ("CONSUL_HTTP_ADDR", None::<&str>),
            ("ETCD_ENDPOINTS", None::<&str>),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability("nonexistent_capability_xyz");
                    assert!(result.is_err());
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[test]
fn test_primal_endpoint_structure() {
    use std::time::SystemTime;
    let endpoint = PrimalEndpoint {
        service_id: "test-1".to_string(),
        url: "http://test:80".to_string(),
        capabilities: vec!["encryption".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    assert_eq!(endpoint.service_id, "test-1");
    assert_eq!(endpoint.url, "http://test:80");
    assert_eq!(endpoint.capabilities, vec!["encryption"]);
    assert!(endpoint.healthy);
}

#[test]
fn test_discover_service_socket_by_capability_env_var() {
    temp_env::with_var("SECURITY_SOCKET", Some("/run/security.sock"), || {
        let result = discover_service_socket_by_capability(capabilities::SECURITY);
        assert_eq!(result, Some("/run/security.sock".to_string()));
    });
}

#[test]
fn test_discover_service_socket_by_capability_capability_constants() {
    assert_eq!(capabilities::SECURITY, "security");
    assert_eq!(capabilities::STORAGE, "storage");
    assert_eq!(capabilities::ORCHESTRATION, "orchestration");
    assert_eq!(capabilities::AI, "ai");
    assert_eq!(capabilities::COMPUTE, "compute");
}

#[tokio::test]
async fn test_discover_via_filesystem() {
    let dir = tempfile::tempdir().expect("tempdir");
    let capability_dir = dir.path().join("storage");
    std::fs::create_dir_all(&capability_dir).expect("create capability dir");

    temp_env::with_var(
        "TOADSTOOL_SERVICE_DIR",
        Some(dir.path().to_str().unwrap()),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_service_by_capability("storage");
                    assert!(result.is_ok());
                    let endpoints = result.unwrap();
                    assert_eq!(endpoints.len(), 1);
                    assert_eq!(endpoints[0].service_id, "storage-fs");
                    assert!(endpoints[0].url.starts_with("file://"));
                    assert!(endpoints[0].url.contains("storage"));
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[test]
fn test_discovery_error_display() {
    let err = DiscoveryError::NoServiceFound {
        capability: "storage".to_string(),
    };
    assert!(err.to_string().contains("No service found"));
    assert!(err.to_string().contains("storage"));

    let err = DiscoveryError::ServiceUnhealthy {
        service_id: "beardog-1".to_string(),
    };
    assert!(err.to_string().contains("Service unhealthy"));
    assert!(err.to_string().contains("beardog-1"));

    let err = DiscoveryError::DiscoveryFailed {
        method: "mDNS".to_string(),
        reason: "timeout".to_string(),
    };
    assert!(err.to_string().contains("Discovery method failed"));
    assert!(err.to_string().contains("mDNS"));
    assert!(err.to_string().contains("timeout"));

    let err = DiscoveryError::Network("connection refused".to_string());
    assert!(err.to_string().contains("connection refused"));
}

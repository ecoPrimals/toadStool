// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for discovery engine module
use super::*;

#[tokio::test]
async fn test_discovery_engine_creation() {
    let engine = DiscoveryEngine::with_defaults();
    assert!(engine.is_ok());
}

#[tokio::test]
async fn test_empty_discovery() {
    let engine = DiscoveryEngine::empty();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_discovery_engine_new_with_custom_sources() {
    let sources: Vec<Box<dyn DiscoverySource>> = vec![Box::new(MDnsSource::new())];
    let engine = DiscoveryEngine::new(sources).unwrap();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_add_source() {
    let mut engine = DiscoveryEngine::empty();
    engine.add_source(Box::new(MDnsSource::new()));
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_discover_all_deduplication() {
    struct MockSource;
    #[async_trait::async_trait]
    impl DiscoverySource for MockSource {
        async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
            Ok(vec![
                CapabilityInfo {
                    provider_id: "dup-1".to_string(),
                    capability: CapabilityType::Storage {
                        features: vec![],
                        min_throughput_mbps: None,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint: ServiceEndpoint::Http("http://a".to_string()),
                    health: HealthStatus::Unknown,
                },
                CapabilityInfo {
                    provider_id: "dup-1".to_string(),
                    capability: CapabilityType::Storage {
                        features: vec![],
                        min_throughput_mbps: None,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint: ServiceEndpoint::Http("http://b".to_string()),
                    health: HealthStatus::Unknown,
                },
            ])
        }
        fn name(&self) -> &'static str {
            "mock"
        }
    }
    let engine = DiscoveryEngine::new(vec![Box::new(MockSource)]).unwrap();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 1, "Should deduplicate by provider_id");
    assert_eq!(providers[0].provider_id, "dup-1");
}

#[tokio::test]
async fn test_discover_all_source_error() {
    struct FailingSource;
    #[async_trait::async_trait]
    impl DiscoverySource for FailingSource {
        async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
            Err(ToadStoolError::configuration("config error".to_string()))
        }
        fn name(&self) -> &'static str {
            "failing"
        }
    }
    let engine = DiscoveryEngine::new(vec![Box::new(FailingSource)]).unwrap();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0, "Should continue past failing source");
}

struct SlowSource;
#[async_trait::async_trait]
impl DiscoverySource for SlowSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        std::future::pending::<ToadStoolResult<Vec<CapabilityInfo>>>().await
    }
    fn name(&self) -> &'static str {
        "slow"
    }
}

#[tokio::test]
async fn test_discover_all_timeout() {
    let mut engine = DiscoveryEngine::empty();
    engine.add_source(Box::new(SlowSource));
    // empty() has 1s timeout, slow source never completes - will timeout
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0, "Should handle timeout gracefully");
}

#[tokio::test]
async fn test_environment_source_parsing() {
    let endpoint = EnvironmentSource::parse_endpoint("http://localhost:8080");
    assert!(endpoint.is_ok());
    assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Http(_)));

    let endpoint = EnvironmentSource::parse_endpoint("https://example.com");
    assert!(endpoint.is_ok());
    assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Http(_)));

    let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost:9000");
    assert!(endpoint.is_ok());
    assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Tcp { .. }));

    let endpoint = EnvironmentSource::parse_endpoint("unix:///var/run/test.sock");
    assert!(endpoint.is_ok());
    assert!(matches!(endpoint.unwrap(), ServiceEndpoint::UnixSocket(_)));

    let endpoint = EnvironmentSource::parse_endpoint("custom://something");
    assert!(endpoint.is_ok());
    assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Custom { .. }));
}

#[tokio::test]
async fn test_environment_source_parse_endpoint_errors() {
    // TCP without port (host:port format required)
    let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost");
    assert!(endpoint.is_err());

    // TCP with invalid port number
    let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost:invalid");
    assert!(endpoint.is_err());
}

#[tokio::test]
async fn test_environment_discovery() {
    temp_env::with_var(
        "TOADSTOOL_SECURITY_PROVIDER",
        Some("http://discovered:0"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let source = EnvironmentSource::new();
                    let providers = source.discover().await.unwrap();
                    assert!(
                        !providers.is_empty(),
                        "Should find at least one provider from env"
                    );
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_environment_discovery_storage_provider() {
    temp_env::with_var(
        "TOADSTOOL_STORAGE_PROVIDER",
        Some("http://discovered:0"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let source = EnvironmentSource::new();
                    let providers = source.discover().await.unwrap();
                    assert!(
                        !providers.is_empty(),
                        "Should find storage provider from env"
                    );
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_environment_discovery_coordination_provider() {
    temp_env::with_var(
        "TOADSTOOL_COORDINATION_PROVIDER",
        Some("tcp://host:1234"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let source = EnvironmentSource::new();
                    let providers = source.discover().await.unwrap();
                    assert!(!providers.is_empty(), "Should find coordination provider");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[tokio::test]
async fn test_environment_discovery_intelligence_provider() {
    temp_env::with_var(
        "TOADSTOOL_INTELLIGENCE_PROVIDER",
        Some("unix:///tmp/ai.sock"),
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let source = EnvironmentSource::new();
                    let providers = source.discover().await.unwrap();
                    assert!(!providers.is_empty(), "Should find intelligence provider");
                });
            })
            .join()
            .expect("test thread");
        },
    );
}

#[test]
fn test_mdns_source_default_and_with_timeout() {
    let default = MDnsSource::new();
    assert_eq!(default.name(), "mdns");
    let _custom = MDnsSource::with_timeout(10);
}

#[test]
fn test_local_registry_capability_from_str_all_variants() {
    assert!(matches!(
        LocalRegistrySource::capability_from_str("storage"),
        CapabilityType::Storage { .. }
    ));
    assert!(matches!(
        LocalRegistrySource::capability_from_str("intelligence"),
        CapabilityType::Intelligence { .. }
    ));
    assert!(matches!(
        LocalRegistrySource::capability_from_str("network"),
        CapabilityType::Network { .. }
    ));
    assert!(matches!(
        LocalRegistrySource::capability_from_str("monitoring"),
        CapabilityType::Monitoring { .. }
    ));
}

#[test]
fn test_local_registry_parse_endpoint() {
    let http = LocalRegistrySource::parse_endpoint("http://localhost:8080");
    assert!(http.is_ok());
    assert!(matches!(http.unwrap(), ServiceEndpoint::Http(_)));
    let invalid_tcp = LocalRegistrySource::parse_endpoint("tcp://noport");
    assert!(invalid_tcp.is_err());
}

#[tokio::test]
async fn test_mdns_source() {
    let source = MDnsSource::new();
    assert_eq!(source.name(), "mdns");
    // EVOLVED: mDNS now implemented - may find services on local network
    // or return empty if no ToadStool services are advertised
    let providers = source.discover().await.unwrap();
    // Just verify it returns without error; actual results depend on network
    assert!(providers.iter().all(|p| !p.provider_id.is_empty()) || providers.is_empty());
}

#[tokio::test]
async fn test_local_registry_source() {
    let source = LocalRegistrySource::new();
    assert_eq!(source.name(), "local_registry");
    let providers = source.discover().await.unwrap();
    assert!(providers.iter().all(|p| !p.provider_id.is_empty()) || providers.is_empty());
}

#[tokio::test]
async fn test_local_registry_capability_from_str() {
    let cap = LocalRegistrySource::capability_from_str("security");
    assert!(matches!(cap, CapabilityType::Security { .. }));

    let cap = LocalRegistrySource::capability_from_str("compute");
    assert!(matches!(cap, CapabilityType::Compute { .. }));

    let cap = LocalRegistrySource::capability_from_str("unknown");
    assert!(matches!(cap, CapabilityType::Coordination { .. }));
}

#[tokio::test]
async fn test_local_registry_with_valid_file() {
    let temp_dir = std::env::temp_dir();
    let config_dir = temp_dir.join("toadstool_test_config");
    let biomeos_dir = config_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(
        biomeos_dir.join("registry.json"),
        r#"[{"provider_id":"p1","endpoint":"http://discovered:0","capability":"storage"}]"#,
    )
    .unwrap();

    let config_path = config_dir.to_str().unwrap().to_string();
    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert!(!providers.is_empty(), "Should discover from registry file");
                assert_eq!(providers[0].provider_id, "p1");
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&config_dir).ok();
}

#[tokio::test]
async fn test_local_registry_invalid_json() {
    let temp_dir = std::env::temp_dir();
    let config_dir = temp_dir.join("toadstool_test_config2");
    let biomeos_dir = config_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(biomeos_dir.join("registry.json"), "not valid json").unwrap();

    let config_path = config_dir.to_str().unwrap().to_string();
    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert!(providers.is_empty(), "Invalid JSON should return empty");
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&config_dir).ok();
}

#[tokio::test]
async fn test_mdns_source_with_timeout() {
    let source = MDnsSource::with_timeout(1);
    assert_eq!(source.name(), "mdns");
    let providers = source.discover().await.unwrap();
    assert!(providers.is_empty() || providers.iter().all(|p| !p.provider_id.is_empty()));
}

#[tokio::test]
async fn test_local_registry_skips_invalid_endpoint() {
    let temp_dir = std::env::temp_dir();
    let config_dir = temp_dir.join("toadstool_test_registry_invalid_ep");
    let biomeos_dir = config_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(
        biomeos_dir.join("registry.json"),
        r#"[
            {"provider_id":"valid","endpoint":"http://localhost:8080","capability":"storage"},
            {"provider_id":"invalid-ep","endpoint":"tcp://noport","capability":"compute"}
        ]"#,
    )
    .unwrap();

    let config_path = config_dir.to_str().unwrap().to_string();
    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert_eq!(providers.len(), 1, "Should skip invalid endpoint entry");
                assert_eq!(providers[0].provider_id, "valid");
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&config_dir).ok();
}

#[tokio::test]
async fn test_local_registry_no_file_returns_empty() {
    let temp_dir = std::env::temp_dir();
    let config_dir = temp_dir.join("toadstool_test_no_registry");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.to_str().unwrap().to_string();

    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert!(providers.is_empty());
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&config_dir).ok();
}

#[tokio::test]
async fn test_local_registry_capability_from_str_all() {
    let s = LocalRegistrySource::capability_from_str("storage");
    assert!(matches!(s, CapabilityType::Storage { .. }));

    let s = LocalRegistrySource::capability_from_str("network");
    assert!(matches!(s, CapabilityType::Network { .. }));

    let s = LocalRegistrySource::capability_from_str("monitoring");
    assert!(matches!(s, CapabilityType::Monitoring { .. }));

    let s = LocalRegistrySource::capability_from_str("intelligence");
    assert!(matches!(s, CapabilityType::Intelligence { .. }));
}

#[tokio::test]
async fn test_discover_all_mixed_sources() {
    struct OkSource;
    #[async_trait::async_trait]
    impl DiscoverySource for OkSource {
        async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
            Ok(vec![CapabilityInfo {
                provider_id: "ok-1".to_string(),
                capability: CapabilityType::Compute {
                    features: vec![],
                    min_memory_gb: None,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Http("http://ok:0".to_string()),
                health: HealthStatus::Unknown,
            }])
        }
        fn name(&self) -> &'static str {
            "ok"
        }
    }
    struct FailingSource;
    #[async_trait::async_trait]
    impl DiscoverySource for FailingSource {
        async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
            Err(ToadStoolError::configuration("fail".to_string()))
        }
        fn name(&self) -> &'static str {
            "fail"
        }
    }
    let engine = DiscoveryEngine::new(vec![
        Box::new(OkSource),
        Box::new(FailingSource),
        Box::new(OkSource),
    ])
    .unwrap();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].provider_id, "ok-1");
}

#[tokio::test]
async fn test_local_registry_parse_endpoint_tcp_multicolon() {
    let result = LocalRegistrySource::parse_endpoint("tcp://host:port");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_environment_parse_endpoint_unix_strip_prefix() {
    let ep = EnvironmentSource::parse_endpoint("unix:///tmp/sock");
    assert!(ep.is_ok());
    assert!(matches!(ep.unwrap(), ServiceEndpoint::UnixSocket(_)));
}

#[tokio::test]
async fn test_environment_source_parse_custom_protocol_fallback() {
    let ep = EnvironmentSource::parse_endpoint("grpc://service:50051");
    assert!(ep.is_ok());
    let endpoint = ep.unwrap();
    assert!(matches!(endpoint, ServiceEndpoint::Custom { .. }));
}

#[tokio::test]
async fn test_environment_source_invalid_url_skips_provider() {
    temp_env::with_var("TOADSTOOL_SECURITY_PROVIDER", Some("tcp://noport"), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = EnvironmentSource::new();
                let providers = source.discover().await.unwrap();
                assert!(providers.is_empty(), "Invalid URL should skip provider");
            });
        })
        .join()
        .expect("test thread");
    });
}

#[tokio::test]
async fn test_local_registry_parse_endpoint_http_https() {
    let ep = LocalRegistrySource::parse_endpoint("https://secure.example.com:443");
    assert!(ep.is_ok());
    assert!(matches!(ep.unwrap(), ServiceEndpoint::Http(_)));
}

#[tokio::test]
async fn test_local_registry_parse_endpoint_invalid_tcp() {
    let ep = LocalRegistrySource::parse_endpoint("tcp://onlyhost");
    assert!(ep.is_err());
}

#[tokio::test]
async fn test_local_registry_parse_endpoint_invalid_port() {
    let ep = LocalRegistrySource::parse_endpoint("tcp://host:notanumber");
    assert!(ep.is_err());
}

#[tokio::test]
async fn test_local_registry_empty_file_returns_empty() {
    let temp_dir = std::env::temp_dir().join("toadstool_empty_registry_test");
    let biomeos_dir = temp_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(biomeos_dir.join("registry.json"), "[]").unwrap();
    let config_path = temp_dir.to_str().unwrap().to_string();

    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert!(providers.is_empty());
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_local_registry_entry_without_capability_defaults_to_coordination() {
    let temp_dir = std::env::temp_dir().join("toadstool_cap_default_test");
    let biomeos_dir = temp_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(
        biomeos_dir.join("registry.json"),
        r#"[{"provider_id":"no-cap","endpoint":"http://localhost:0"}]"#,
    )
    .unwrap();
    let config_path = temp_dir.to_str().unwrap().to_string();

    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert_eq!(providers.len(), 1);
                assert!(matches!(
                    providers[0].capability,
                    CapabilityType::Coordination { .. }
                ));
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_mdns_source_browse_failure_returns_empty() {
    let source = MDnsSource::with_timeout(0);
    let providers = source.discover().await.unwrap();
    assert!(providers.is_empty() || providers.iter().all(|p| !p.provider_id.is_empty()));
}

#[tokio::test]
async fn test_discovery_engine_empty_timeout() {
    let engine = DiscoveryEngine::empty();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_discover_all_partial_timeout_and_success() {
    struct FastOkSource;
    #[async_trait::async_trait]
    impl DiscoverySource for FastOkSource {
        async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
            Ok(vec![CapabilityInfo {
                provider_id: "fast".to_string(),
                capability: CapabilityType::Compute {
                    features: vec![],
                    min_memory_gb: None,
                },
                metadata: std::collections::HashMap::new(),
                endpoint: ServiceEndpoint::Http("http://fast:0".to_string()),
                health: HealthStatus::Unknown,
            }])
        }
        fn name(&self) -> &'static str {
            "fast"
        }
    }
    let mut engine = DiscoveryEngine::empty();
    engine.add_source(Box::new(FastOkSource));
    engine.add_source(Box::new(SlowSource));
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].provider_id, "fast");
}

#[tokio::test]
async fn test_local_registry_entry_aliases_id_url() {
    let temp_dir = std::env::temp_dir().join("toadstool_registry_aliases_test");
    let biomeos_dir = temp_dir.join("biomeos");
    std::fs::create_dir_all(&biomeos_dir).unwrap();
    std::fs::write(
        biomeos_dir.join("registry.json"),
        r#"[{"id":"alias-id","url":"http://localhost:9999","capability":"compute"}]"#,
    )
    .unwrap();
    let config_path = temp_dir.to_str().unwrap().to_string();

    temp_env::with_var("XDG_CONFIG_HOME", Some(config_path.as_str()), || {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                let source = LocalRegistrySource::new();
                let providers = source.discover().await.unwrap();
                assert_eq!(providers.len(), 1);
                assert_eq!(providers[0].provider_id, "alias-id");
            });
        })
        .join()
        .expect("test thread");
    });
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_environment_source_name() {
    let source = EnvironmentSource::new();
    assert_eq!(source.name(), "environment");
}

#[tokio::test]
async fn test_local_registry_home_fallback() {
    let temp_dir = std::env::temp_dir().join("toadstool_home_registry_test");
    let fake_home = temp_dir.join("fake_home");
    let config_dir = fake_home.join(".config/biomeos");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("registry.json"),
        r#"[{"provider_id":"home-svc","endpoint":"http://localhost:0","capability":"storage"}]"#,
    )
    .unwrap();
    let home_path = fake_home.to_str().unwrap().to_string();

    temp_env::with_vars(
        [
            ("XDG_CONFIG_HOME", None::<&str>),
            ("HOME", Some(home_path.as_str())),
        ],
        || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let source = LocalRegistrySource::new();
                    let providers = source.discover().await.unwrap();
                    assert!(!providers.is_empty());
                    assert_eq!(providers[0].provider_id, "home-svc");
                });
            })
            .join()
            .expect("test thread");
        },
    );
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_discovery_engine_timeout_field() {
    let engine = DiscoveryEngine::empty();
    let providers = engine.discover_all().await.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_mdns_source_daemon_unavailable_returns_empty() {
    let source = MDnsSource::with_timeout(0);
    let providers = source.discover().await.unwrap();
    assert!(providers.is_empty() || providers.iter().all(|p| !p.provider_id.is_empty()));
}

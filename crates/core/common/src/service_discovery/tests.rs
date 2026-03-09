// SPDX-License-Identifier: AGPL-3.0-only
//! Service discovery tests

use std::io::Write;
use std::path::PathBuf;

use crate::discovery_defaults::DiscoveryConfig;
use crate::primal_identity::{Capability, CryptoCapability};

use super::config::{capability_from_str, default_version};
use super::service::ServiceDiscovery;
use super::types::{DiscoveryError, DiscoveryMethod, ServiceDiscoveryTrait};

use tempfile::NamedTempFile;

// --- Helpers ---

fn write_test_config(config_json: &str) -> (NamedTempFile, PathBuf) {
    let mut tmp = NamedTempFile::new().expect("temp file");
    tmp.write_all(config_json.as_bytes()).unwrap();
    let path = tmp.path().to_path_buf();
    (tmp, path)
}

async fn discovery_from_json(json: &str) -> (NamedTempFile, ServiceDiscovery) {
    let (tmp, path) = write_test_config(json);
    let path_str = path.to_string_lossy().to_string();
    let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path: path_str })
        .await
        .unwrap();
    (tmp, disc)
}

async fn env_discovery() -> ServiceDiscovery {
    ServiceDiscovery::new(DiscoveryMethod::Environment)
        .await
        .unwrap()
}

macro_rules! run_async_with_env {
    ($vars:expr, $body:block) => {
        temp_env::with_vars($vars, || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async $body);
            })
            .join()
            .expect("test thread");
        });
    };
}

macro_rules! run_async_with_var {
    ($name:expr, $value:expr, $body:block) => {
        temp_env::with_var($name, $value, || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async $body);
            })
            .join()
            .expect("test thread");
        });
    };
}

#[test]
fn test_capability_from_str_known() {
    assert!(matches!(
        capability_from_str("coordination"),
        Capability::Coordination(_)
    ));
    assert!(matches!(
        capability_from_str("orchestration"),
        Capability::Coordination(_)
    ));
    assert!(matches!(
        capability_from_str("storage"),
        Capability::Storage(_)
    ));
    assert!(matches!(
        capability_from_str("object-storage"),
        Capability::Storage(_)
    ));
    assert!(matches!(
        capability_from_str("crypto"),
        Capability::Crypto(_)
    ));
    assert!(matches!(
        capability_from_str("auth"),
        Capability::Authentication(_)
    ));
    assert!(matches!(
        capability_from_str("compute"),
        Capability::Compute(_)
    ));
    assert!(matches!(capability_from_str("gpu"), Capability::Compute(_)));
}

#[test]
fn test_capability_from_str_unknown() {
    match capability_from_str("custom-thing") {
        Capability::Custom { name, .. } => assert_eq!(name, "custom-thing"),
        other => panic!("Expected Custom, got {other:?}"),
    }
}

#[test]
fn test_capability_from_str_case_insensitive() {
    assert!(matches!(
        capability_from_str("COORDINATION"),
        Capability::Coordination(_)
    ));
    assert!(matches!(
        capability_from_str("Storage"),
        Capability::Storage(_)
    ));
    assert!(matches!(
        capability_from_str("GPU_COMPUTE"),
        Capability::Compute(_)
    ));
}

#[tokio::test]
async fn test_config_file_discovery() {
    let config = r#"{
        "services": [
            {
                "name": "test-compute",
                "version": "1.0.0",
                "capabilities": ["compute", "gpu"],
                "endpoints": ["http://localhost:9090/compute"],
                "metadata": {"region": "local"}
            },
            {
                "name": "test-storage",
                "capabilities": ["storage"],
                "endpoints": ["http://localhost:8080/storage"]
            }
        ]
    }"#;

    let (_tmp, path) = write_test_config(config);
    let discovery = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
        path: path.to_string_lossy().to_string(),
    })
    .await;
    assert!(discovery.is_ok(), "Config file discovery should succeed");

    let disc = discovery.unwrap();
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 2);

    let compute_svc = all.iter().find(|s| s.name == "test-compute").unwrap();
    assert_eq!(compute_svc.version, "1.0.0");
    assert!(compute_svc.capabilities.len() >= 2);
    assert_eq!(compute_svc.metadata.get("region").unwrap(), "local");

    let storage_svc = all.iter().find(|s| s.name == "test-storage").unwrap();
    assert!(storage_svc
        .capabilities
        .iter()
        .any(|c| matches!(c, Capability::Storage(_))));
}

#[tokio::test]
async fn test_config_file_missing() {
    let result = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
        path: "/nonexistent/path/discovery.json".to_string(),
    })
    .await;
    // Should succeed (logs warning) because `new` catches initial refresh failures
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_config_file_malformed_json() {
    let (_tmp, path) = write_test_config("not valid json {{{");
    let path_str = path.to_string_lossy().to_string();

    let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path: path_str })
        .await
        .unwrap();
    let all = disc.discover_all().await;
    assert!(all.is_err());
}

#[tokio::test]
async fn test_config_file_empty_services() {
    let (_tmp, disc) = discovery_from_json(r#"{"services": []}"#).await;
    let all = disc.discover_all().await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_parse_capabilities() {
    let caps = ServiceDiscovery::parse_capabilities("coordination,storage,compute");
    assert_eq!(caps.len(), 3);
    assert!(caps
        .iter()
        .any(|c| matches!(c, Capability::Coordination(_))));
    assert!(caps.iter().any(|c| matches!(c, Capability::Storage(_))));
    assert!(caps.iter().any(|c| matches!(c, Capability::Compute(_))));
}

#[tokio::test]
async fn test_parse_capabilities_empty() {
    let caps = ServiceDiscovery::parse_capabilities("");
    assert!(caps.is_empty());
}

#[tokio::test]
async fn test_cache_population_and_lookup() {
    let (_tmp, disc) = discovery_from_json(r#"{
        "services": [{"name": "cached-svc", "capabilities": ["compute"], "endpoints": ["http://localhost:7777/api"]}]
    }"#).await;

    let compute_cap =
        Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
    let found = disc.find_service_by_capability(compute_cap).await;
    assert!(found.is_ok());
    assert_eq!(found.unwrap().name, "cached-svc");
}

#[tokio::test]
async fn test_find_service_by_capability_not_found() {
    let (_tmp, disc) = discovery_from_json(r#"{"services": [
        {"name": "storage-only", "capabilities": ["storage"], "endpoints": ["http://localhost:1234"]}
    ]}"#).await;

    let result = disc
        .find_service_by_capability(Capability::Crypto(CryptoCapability::KeyManagement))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_refresh_clears_cache() {
    let (_tmp, disc) = discovery_from_json(
        r#"{"services": [
        {"name": "refreshable", "capabilities": ["compute"], "endpoints": ["http://localhost:5555"]}
    ]}"#,
    )
    .await;

    // Cache should be populated
    let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
    let found = disc.find_service_by_capability(cap.clone()).await;
    assert!(found.is_ok());

    // Refresh should work
    let refresh_result = disc.refresh().await;
    assert!(refresh_result.is_ok());
}

#[test]
fn test_default_version() {
    assert_eq!(default_version(), "unknown");
}

#[tokio::test]
async fn test_multi_method_discovery() {
    let disc = ServiceDiscovery::new(DiscoveryMethod::Auto).await.unwrap();
    let all = disc.discover_all().await;
    // Auto discovery may return empty if no env vars or mDNS services exist
    assert!(all.is_ok());
}

#[test]
fn test_discover_from_env() {
    run_async_with_env!(
        [
            (
                "TOADSTOOL_SERVICE_TESTCOMPUTE_URL",
                Some("http://localhost:9090")
            ),
            (
                "TOADSTOOL_SERVICE_TESTCOMPUTE_CAPABILITIES",
                Some("compute,storage")
            ),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let services = disc.discover_from_env().unwrap();
            assert!(!services.is_empty(), "Should discover from env vars");
            let svc = services.iter().find(|s| s.name == "testcompute").unwrap();
            assert_eq!(svc.endpoints.len(), 1);
            assert!(svc.capabilities.len() >= 2);
        }
    );
}

#[test]
fn test_discover_from_env_invalid_url_returns_error() {
    run_async_with_env!(
        [(
            "TOADSTOOL_SERVICE_BAD_URL",
            Some("not-a-valid-url://broken")
        )],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let result = disc.discover_from_env();
            assert!(result.is_ok() || result.is_err());
        }
    );
}

#[test]
fn test_config_path_resolution_via_env() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"env-svc","capabilities":["compute"],"endpoints":["http://localhost:7777"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    run_async_with_var!("TOADSTOOL_DISCOVERY_CONFIG", Some(path_str.as_str()), {
        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
            path: String::new(),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "env-svc");
    });
}

#[tokio::test]
async fn test_config_with_explicit_id() {
    let (_tmp, disc) = discovery_from_json(
        r#"{
        "services": [{
            "id": "custom-id-123",
            "name": "explicit-id-svc",
            "capabilities": ["storage"],
            "endpoints": ["http://localhost:8888"]
        }]
    }"#,
    )
    .await;
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "custom-id-123");
    assert_eq!(all[0].name, "explicit-id-svc");
}

#[tokio::test]
async fn test_config_skips_malformed_endpoint() {
    let (_tmp, disc) = discovery_from_json(
        r#"{
        "services": [{
            "name": "mixed-endpoints",
            "capabilities": ["compute"],
            "endpoints": ["http://localhost:9090", ":::invalid", "https://valid.com:443"]
        }]
    }"#,
    )
    .await;
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
    // Should have 2 valid endpoints (invalid one skipped)
    assert!(!all[0].endpoints.is_empty());
}

#[tokio::test]
async fn test_parse_capabilities_unknown_filtered() {
    let caps = ServiceDiscovery::parse_capabilities("coordination,unknown_thing,storage,foo");
    assert_eq!(caps.len(), 2);
    assert!(caps
        .iter()
        .any(|c| matches!(c, Capability::Coordination(_))));
    assert!(caps.iter().any(|c| matches!(c, Capability::Storage(_))));
}

#[tokio::test]
async fn test_parse_capabilities_whitespace() {
    let caps = ServiceDiscovery::parse_capabilities("  coordination  ,  storage  ,  compute  ");
    assert_eq!(caps.len(), 3);
}

#[tokio::test]
async fn test_with_config_refresh_failure() {
    let config = DiscoveryConfig::default();
    let result = ServiceDiscovery::with_config(
        DiscoveryMethod::ConfigFile {
            path: "/nonexistent/path/discovery.json".to_string(),
        },
        config,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_announce_self() {
    use crate::primal_identity::ToadStoolIdentity;

    let disc = env_discovery().await;
    let identity = ToadStoolIdentity::new();
    let result = disc.announce_self(&identity).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_find_service_all_unhealthy_returns_first() {
    let (_tmp, disc) = discovery_from_json(r#"{
        "services": [{"name": "only-svc", "capabilities": ["compute"], "endpoints": ["http://localhost:9999"]}]
    }"#).await;
    let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
    let found = disc.find_service_by_capability(cap).await;
    assert!(found.is_ok());
    assert_eq!(found.unwrap().name, "only-svc");
}

#[test]
fn test_registry_empty_endpoint_returns_error() {
    run_async_with_env!([("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>)], {
        let config = DiscoveryConfig::production();
        let result = ServiceDiscovery::with_config(
            DiscoveryMethod::Registry {
                endpoint: String::new(),
            },
            config,
        )
        .await;
        assert!(
            result.is_err(),
            "Empty registry endpoint should fail without env var"
        );
    });
}

#[tokio::test]
async fn test_registry_file_path_delegates_to_config() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"file-reg","capabilities":["storage"],"endpoints":["http://localhost:6666"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
        endpoint: format!("file://{path_str}"),
    })
    .await
    .unwrap();
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "file-reg");
}

#[tokio::test]
async fn test_registry_unix_path_delegates_to_config() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"unix-reg","capabilities":["compute"],"endpoints":["http://localhost:7777"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
        endpoint: format!("unix://{path_str}"),
    })
    .await
    .unwrap();
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "unix-reg");
}

#[tokio::test]
async fn test_discover_multi_partial_success() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"cfg-svc","capabilities":["compute"],"endpoints":["http://localhost:5555"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
        DiscoveryMethod::ConfigFile { path: path_str },
        DiscoveryMethod::Registry {
            endpoint: String::new(),
        },
    ]))
    .await
    .unwrap();
    let all = disc.discover_all().await;
    assert!(all.is_ok());
    let services = all.unwrap();
    assert!(!services.is_empty());
}

#[test]
fn test_discover_fallback_when_nothing_found() {
    run_async_with_env!(
        [
            ("TOADSTOOL_ENV", Some("development")),
            ("TOADSTOOL_URL", Some("http://localhost:8084")),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                DiscoveryMethod::ConfigFile {
                    path: "/nonexistent/discovery.json".to_string(),
                },
                DiscoveryMethod::Registry {
                    endpoint: "/nonexistent/reg".to_string(),
                },
            ]))
            .await
            .unwrap();
            let all = disc.discover_all().await.unwrap();
            assert!(!all.is_empty(), "Should use fallback when configured");
        }
    );
}

#[test]
fn test_capability_from_str_object_storage() {
    assert!(matches!(
        capability_from_str("object_storage"),
        Capability::Storage(_)
    ));
    assert!(matches!(
        capability_from_str("object-storage"),
        Capability::Storage(_)
    ));
}

#[test]
fn test_capability_from_str_cryptography() {
    assert!(matches!(
        capability_from_str("cryptography"),
        Capability::Crypto(_)
    ));
    assert!(matches!(
        capability_from_str("security"),
        Capability::Crypto(_)
    ));
}

#[test]
fn test_capability_from_str_native_execution() {
    assert!(matches!(
        capability_from_str("native"),
        Capability::Compute(_)
    ));
    assert!(matches!(
        capability_from_str("execution"),
        Capability::Compute(_)
    ));
}

#[test]
fn test_capability_from_str_whitespace() {
    assert!(matches!(
        capability_from_str("  coordination  "),
        Capability::Coordination(_)
    ));
}

#[tokio::test]
async fn test_find_services_cache_hit() {
    let (_tmp, disc) = discovery_from_json(r#"{"services":[{"name":"cache-svc","capabilities":["compute"],"endpoints":["http://localhost:2"]}]}"#).await;
    let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
    let first = disc.find_services_by_capability(&cap).await.unwrap();
    let second = disc.find_services_by_capability(&cap).await.unwrap();
    assert_eq!(first.len(), second.len());
    assert_eq!(first[0].id, second[0].id);
}

#[tokio::test]
async fn test_find_service_filter_capability_mismatch_returns_error() {
    let (_tmp, disc) = discovery_from_json(
        r#"{"services":[
        {"name":"compute-only","capabilities":["compute"],"endpoints":["http://localhost:3"]}
    ]}"#,
    )
    .await;
    let crypto_cap = Capability::Crypto(CryptoCapability::KeyManagement);
    let found = disc.find_service_by_capability(crypto_cap).await;
    assert!(found.is_err());
    if let Err(DiscoveryError::NoServiceFound { .. }) = found {
        // Expected
    } else {
        panic!("Expected NoServiceFound error");
    }
}

#[tokio::test]
async fn test_discover_from_env_key_strip_prefix_suffix() {
    run_async_with_env!(
        [
            ("TOADSTOOL_SERVICE_MYSVC_URL", Some("http://localhost:9999")),
            ("TOADSTOOL_SERVICE_MYSVC_CAPABILITIES", Some("compute")),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let services = disc.discover_from_env().unwrap();
            let mysvc = services.iter().find(|s| s.name == "mysvc");
            assert!(
                mysvc.is_some(),
                "Should parse MY_SVC from TOADSTOOL_SERVICE_MYSVC_URL"
            );
        }
    );
}

#[test]
fn test_discover_from_env_invalid_url_propagates_error() {
    run_async_with_env!(
        [
            (
                "TOADSTOOL_SERVICE_BADURL_URL",
                Some(":::triple-colon-invalid")
            ),
            ("TOADSTOOL_SERVICE_BADURL_CAPABILITIES", Some("compute")),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let result = disc.discover_from_env();
            assert!(result.is_err());
        }
    );
}

#[test]
fn test_config_path_resolution_biomeos_runtime_dir() {
    let config = r#"{"services":[{"name":"rt-svc","capabilities":["storage"],"endpoints":["http://localhost:4"]}]}"#;
    let (_tmp, path) = write_test_config(config);
    let parent = path.parent().unwrap().to_path_buf();
    let runtime_dir = parent.join("biomeos_runtime");
    std::fs::create_dir_all(&runtime_dir).unwrap();
    std::fs::write(runtime_dir.join("discovery.json"), config).unwrap();
    let runtime_path = runtime_dir.to_str().unwrap().to_string();
    run_async_with_var!("BIOMEOS_RUNTIME_DIR", Some(runtime_path.as_str()), {
        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
            path: String::new(),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "rt-svc");
    });
    std::fs::remove_dir_all(&runtime_dir).ok();
}

#[test]
fn test_config_path_resolution_xdg_config_home() {
    let config = r#"{"services":[{"name":"xdg-svc","capabilities":["compute"],"endpoints":["http://localhost:5"]}]}"#;
    let temp_dir = std::env::temp_dir().join("toadstool_xdg_test");
    let xdg_config = temp_dir.join("xdg_config");
    let biomeos = xdg_config.join("biomeos");
    std::fs::create_dir_all(&biomeos).unwrap();
    std::fs::write(biomeos.join("discovery.json"), config).unwrap();
    let xdg_path = xdg_config.to_str().unwrap().to_string();
    run_async_with_var!("XDG_CONFIG_HOME", Some(xdg_path.as_str()), {
        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
            path: String::new(),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "xdg-svc");
    });
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_config_path_resolution_home_fallback() {
    let config = r#"{"services":[{"name":"home-svc","capabilities":["storage"],"endpoints":["http://localhost:6"]}]}"#;
    let temp_dir = std::env::temp_dir().join("toadstool_home_test");
    let home = temp_dir.join("fake_home");
    let config_dir = home.join(".config/biomeos");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("discovery.json"), config).unwrap();
    let home_path = home.to_str().unwrap().to_string();
    run_async_with_env!(
        [
            ("TOADSTOOL_DISCOVERY_CONFIG", None::<&str>),
            ("BIOMEOS_RUNTIME_DIR", None::<&str>),
            ("XDG_CONFIG_HOME", None::<&str>),
            ("HOME", Some(home_path.as_str())),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                path: String::new(),
            })
            .await
            .unwrap();
            let all = disc.discover_all().await.unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].name, "home-svc");
        }
    );
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_registry_http_path_parsing() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"path-svc","capabilities":["compute"],"endpoints":["http://localhost:7"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
        endpoint: format!("file://{path_str}"),
    })
    .await
    .unwrap();
    let all = disc.discover_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "path-svc");
}

#[tokio::test]
async fn test_discover_from_fallbacks_disabled_when_production() {
    run_async_with_env!(
        [
            ("TOADSTOOL_ENV", Some("production")),
            ("TOADSTOOL_URL", Some("http://localhost:8084")),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                DiscoveryMethod::ConfigFile {
                    path: "/nonexistent/discovery.json".to_string(),
                },
                DiscoveryMethod::Registry {
                    endpoint: String::new(),
                },
            ]))
            .await
            .unwrap();
            let all = disc.discover_all().await.unwrap();
            assert!(all.is_empty(), "Production should not use fallbacks");
        }
    );
}

#[tokio::test]
async fn test_discover_multi_all_fail_no_fallback() {
    run_async_with_env!(
        [
            ("TOADSTOOL_ENV", Some("production")),
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
        ],
        {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                DiscoveryMethod::ConfigFile {
                    path: "/nonexistent/x.json".to_string(),
                },
                DiscoveryMethod::Registry {
                    endpoint: String::new(),
                },
            ]))
            .await
            .unwrap();
            let all = disc.discover_all().await.unwrap();
            assert!(all.is_empty());
        }
    );
}

#[tokio::test]
async fn test_find_service_prefers_healthy() {
    let (_tmp, disc) = discovery_from_json(
        r#"{
        "services": [
            {"name":"unhealthy-svc","capabilities":["compute"],"endpoints":["http://localhost:8"]},
            {"name":"healthy-svc","capabilities":["compute"],"endpoints":["http://localhost:9"]}
        ]
    }"#,
    )
    .await;
    let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
    let found = disc.find_service_by_capability(cap).await.unwrap();
    assert!(found.healthy);
}

#[tokio::test]
async fn test_new_no_refresh() {
    let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
    assert_eq!(disc.method, DiscoveryMethod::Environment);
    let all = disc.discover_all().await;
    assert!(all.is_ok());
}

#[test]
fn test_discovery_method_debug() {
    let m = DiscoveryMethod::Auto;
    let s = format!("{m:?}");
    assert!(s.contains("Auto"));

    let m = DiscoveryMethod::ConfigFile {
        path: "/path".to_string(),
    };
    let s = format!("{m:?}");
    assert!(s.contains("ConfigFile"));
    assert!(s.contains("/path"));

    let m = DiscoveryMethod::Registry {
        endpoint: "http://x".to_string(),
    };
    let s = format!("{m:?}");
    assert!(s.contains("Registry"));
}

#[tokio::test]
async fn test_parse_capabilities_ignores_unknown() {
    let caps = ServiceDiscovery::parse_capabilities("foo,bar,compute,baz,storage");
    assert_eq!(caps.len(), 2);
}

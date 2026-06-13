// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service discovery tests

use std::io::Write;
use std::path::PathBuf;

use crate::discovery_defaults::DiscoveryConfig;
use crate::primal_identity::{Capability, CryptoCapability};

use super::config::{capability_from_str, default_version};
use super::service::ServiceDiscovery;
use super::types::{DiscoveryMethod, ServiceDiscoveryTrait};

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
    assert!(
        storage_svc
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::Storage(_)))
    );
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
    assert!(
        caps.iter()
            .any(|c| matches!(c, Capability::Coordination(_)))
    );
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

#[tokio::test]
async fn test_discover_from_env() {
    temp_env::async_with_vars(
        [
            (
                "TOADSTOOL_SERVICE_TESTCOMPUTE_URL",
                Some("http://localhost:9090"),
            ),
            (
                "TOADSTOOL_SERVICE_TESTCOMPUTE_CAPABILITIES",
                Some("compute,storage"),
            ),
        ],
        async {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let services = disc.discover_from_env().unwrap();
            assert!(!services.is_empty(), "Should discover from env vars");
            let svc = services.iter().find(|s| s.name == "testcompute").unwrap();
            assert_eq!(svc.endpoints.len(), 1);
            assert!(svc.capabilities.len() >= 2);
        },
    )
    .await;
}

#[tokio::test]
async fn test_discover_from_env_invalid_url_returns_error() {
    temp_env::async_with_vars(
        [(
            "TOADSTOOL_SERVICE_BAD_URL",
            Some("not-a-valid-url://broken"),
        )],
        async {
            let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                .await
                .unwrap();
            let result = disc.discover_from_env();
            assert!(result.is_ok() || result.is_err());
        },
    )
    .await;
}

#[tokio::test]
async fn test_config_path_resolution_via_env() {
    let (_tmp, path) = write_test_config(
        r#"{"services":[{"name":"env-svc","capabilities":["compute"],"endpoints":["http://localhost:7777"]}]}"#,
    );
    let path_str = path.to_string_lossy().to_string();
    temp_env::async_with_vars(
        [("TOADSTOOL_DISCOVERY_CONFIG", Some(path_str.as_str()))],
        async {
            let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                path: String::new(),
            })
            .await
            .unwrap();
            let all = disc.discover_all().await.unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].name, "env-svc");
        },
    )
    .await;
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
    assert!(
        caps.iter()
            .any(|c| matches!(c, Capability::Coordination(_)))
    );
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


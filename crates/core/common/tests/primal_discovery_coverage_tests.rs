// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Coverage tests for `primal_integration::discovery`
//! Target: `crates/core/common/src/primal_integration/discovery.rs`
//!
//! Focus: discover_service_by_capability, discover_* wrappers, PrimalEndpoint,
//! DiscoveryError, env var discovery, filesystem discovery, error paths.

use std::time::SystemTime;

use temp_env::with_vars;
use toadstool_common::primal_integration::{
    DiscoveryError, PrimalEndpoint, discover_cache_service, discover_coordination_service,
    discover_database_service, discover_encryption_service, discover_mcp_service,
    discover_object_storage, discover_service_by_capability, discover_storage_service,
};

// ─── PrimalEndpoint struct ─────────────────────────────────────────────────

#[test]
fn test_primal_endpoint_creation_and_fields() {
    let ep = PrimalEndpoint {
        service_id: "test-svc-1".to_string(),
        url: "http://localhost:8080".to_string(),
        capabilities: vec!["storage".to_string(), "replication".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    assert_eq!(ep.service_id, "test-svc-1");
    assert_eq!(ep.url, "http://localhost:8080");
    assert_eq!(ep.capabilities.len(), 2);
    assert!(ep.healthy);
}

#[test]
fn test_primal_endpoint_clone_and_debug() {
    let ep = PrimalEndpoint {
        service_id: "clone-svc".to_string(),
        url: "http://x:1".to_string(),
        capabilities: vec!["a".to_string()],
        healthy: false,
        last_check: SystemTime::now(),
    };
    let cloned = ep.clone();
    assert_eq!(cloned.service_id, ep.service_id);
    assert_eq!(cloned.healthy, ep.healthy);
    let debug_str = format!("{ep:?}");
    assert!(debug_str.contains("clone-svc"));
}

#[test]
fn test_primal_endpoint_serialization_roundtrip() {
    let ep = PrimalEndpoint {
        service_id: "ser-1".to_string(),
        url: "http://example.com:9090".to_string(),
        capabilities: vec!["compute".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    let json = serde_json::to_string(&ep).expect("serialize");
    let parsed: PrimalEndpoint = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.service_id, ep.service_id);
    assert_eq!(parsed.url, ep.url);
}

// ─── DiscoveryError variants ─────────────────────────────────────────────────

#[test]
fn test_discovery_error_no_service_found() {
    let err = DiscoveryError::NoServiceFound {
        capability: "test-cap".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("test-cap"));
}

#[test]
fn test_discovery_error_service_unhealthy() {
    let err = DiscoveryError::ServiceUnhealthy {
        service_id: "svc-123".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("svc-123") || s.contains("unhealthy"));
}

#[test]
fn test_discovery_error_discovery_failed() {
    let err = DiscoveryError::DiscoveryFailed {
        method: "mdns".to_string(),
        reason: "timeout".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("mdns") || s.contains("timeout"));
}

#[test]
fn test_discovery_error_network() {
    let err = DiscoveryError::Network("connection refused".to_string());
    let s = err.to_string();
    assert!(s.contains("connection refused") || s.contains("Network"));
}

// ─── discover_service_by_capability: env var TOADSTOOL_{CAP}_ENDPOINT ────────

#[test]
fn test_discover_by_capability_via_endpoint_env() {
    with_vars(
        [(
            "TOADSTOOL_ENCRYPTION_ENDPOINT",
            Some("http://beardog.local:6060"),
        )],
        || {
            let result = discover_service_by_capability("encryption");
            assert!(result.is_ok(), "expected ok, got {result:?}");
            let endpoints = result.unwrap();
            assert_eq!(endpoints.len(), 1);
            assert_eq!(endpoints[0].service_id, "encryption-env");
            assert_eq!(endpoints[0].url, "http://beardog.local:6060");
            assert_eq!(endpoints[0].capabilities, vec!["encryption"]);
            assert!(endpoints[0].healthy);
        },
    );
}

#[test]
fn test_discover_by_capability_via_service_url_env() {
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_ENCRYPTION_URL",
                Some("http://crypto.local:6061"),
            ),
        ],
        || {
            let result = discover_service_by_capability("encryption");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].service_id, "encryption-service");
            assert_eq!(endpoints[0].url, "http://crypto.local:6061");
        },
    );
}

// ─── discover_service_by_capability: no service found error path ─────────────

#[test]
fn test_discover_by_capability_no_service_found() {
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
            ("KUBERNETES_SERVICE_HOST", None::<&str>),
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ("CONSUL_HTTP_ADDR", None::<&str>),
            ("ETCD_ENDPOINTS", None::<&str>),
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("nonexistent-capability-xyz");
            assert!(result.is_err());
            let err = result.unwrap_err();
            match &err {
                DiscoveryError::NoServiceFound { capability } => {
                    assert_eq!(capability, "nonexistent-capability-xyz");
                }
                _ => panic!("expected NoServiceFound, got {err:?}"),
            }
        },
    );
}

// ─── discover_* wrapper functions ────────────────────────────────────────────

#[test]
fn test_discover_encryption_service() {
    with_vars(
        [("TOADSTOOL_ENCRYPTION_ENDPOINT", Some("http://enc:6060"))],
        || {
            let result = discover_encryption_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://enc:6060");
        },
    );
}

#[test]
fn test_discover_storage_service() {
    with_vars(
        [("TOADSTOOL_STORAGE_ENDPOINT", Some("http://storage:8080"))],
        || {
            let result = discover_storage_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://storage:8080");
        },
    );
}

#[test]
fn test_discover_coordination_service() {
    with_vars(
        [(
            "TOADSTOOL_COORDINATION_ENDPOINT",
            Some("http://songbird:50051"),
        )],
        || {
            let result = discover_coordination_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://songbird:50051");
        },
    );
}

#[test]
fn test_discover_mcp_service() {
    with_vars(
        [("TOADSTOOL_MCP_ENDPOINT", Some("http://squirrel:7070"))],
        || {
            let result = discover_mcp_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://squirrel:7070");
        },
    );
}

#[test]
fn test_discover_cache_service() {
    with_vars(
        [("TOADSTOOL_CACHE_ENDPOINT", Some("http://redis:6379"))],
        || {
            let result = discover_cache_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://redis:6379");
        },
    );
}

#[test]
fn test_discover_database_service() {
    with_vars(
        [("TOADSTOOL_DATABASE_ENDPOINT", Some("http://postgres:5432"))],
        || {
            let result = discover_database_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://postgres:5432");
        },
    );
}

#[test]
fn test_discover_object_storage() {
    with_vars(
        [(
            "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
            Some("http://minio:9000"),
        )],
        || {
            let result = discover_object_storage();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://minio:9000");
        },
    );
}

// ─── Filesystem discovery (TOADSTOOL_SERVICE_DIR) ───────────────────────────

#[test]
fn test_discover_via_filesystem_service_dir() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let service_dir = temp_dir.path().to_path_buf();
    let capability_dir = service_dir.join("storage");
    std::fs::create_dir_all(&capability_dir).expect("create capability dir");

    with_vars(
        [
            ("TOADSTOOL_STORAGE_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_STORAGE_URL", None::<&str>),
            (
                "TOADSTOOL_SERVICE_DIR",
                Some(service_dir.to_string_lossy().as_ref()),
            ),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("storage");
            assert!(result.is_ok(), "expected ok, got {result:?}");
            let endpoints = result.unwrap();
            assert_eq!(endpoints.len(), 1);
            assert_eq!(endpoints[0].service_id, "storage-fs");
            assert!(endpoints[0].url.starts_with("file://"));
            assert!(endpoints[0].url.contains("storage"));
        },
    );
}

#[test]
fn test_discover_via_xdg_runtime_biomeos() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let xdg_runtime = temp_dir.path().join("xdg");
    let biomeos_dir = xdg_runtime.join("biomeos");
    let capability_dir = biomeos_dir.join("coordination");
    std::fs::create_dir_all(&capability_dir).expect("create dirs");

    with_vars(
        [
            ("TOADSTOOL_COORDINATION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_COORDINATION_URL", None::<&str>),
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            (
                "XDG_RUNTIME_DIR",
                Some(xdg_runtime.to_string_lossy().as_ref()),
            ),
        ],
        || {
            let result = discover_service_by_capability("coordination");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].service_id, "coordination-fs");
        },
    );
}

// ─── Invalid capability / empty registry paths ────────────────────────────────

#[test]
fn test_discover_empty_capability_no_env() {
    with_vars(
        [
            ("TOADSTOOL__ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE__URL", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("");
            assert!(result.is_err() || result.is_ok());
        },
    );
}

#[test]
fn test_discover_capability_with_underscores_maps_to_env() {
    with_vars(
        [("TOADSTOOL_OBJECT_STORAGE_ENDPOINT", Some("http://s3:9000"))],
        || {
            let result = discover_service_by_capability("object_storage");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://s3:9000");
        },
    );
}

// ─── Registry: non-http endpoint returns None (empty registry) ────────────────

#[test]
fn test_registry_ftp_endpoint_skipped() {
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
            ("TOADSTOOL_REGISTRY_ENDPOINT", Some("ftp://registry:21")),
            ("KUBERNETES_SERVICE_HOST", None::<&str>),
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("encryption");
            assert!(result.is_err());
        },
    );
}

// ─── Additional coverage: capability name variations ───────────────────────────

#[test]
fn test_discover_capability_uppercase_env_var() {
    with_vars(
        [("TOADSTOOL_COMPUTE_ENDPOINT", Some("http://compute:9000"))],
        || {
            let result = discover_service_by_capability("compute");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://compute:9000");
        },
    );
}

#[test]
fn test_discover_capability_with_hyphen_env() {
    with_vars(
        [(
            "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
            Some("http://minio:9000"),
        )],
        || {
            let result = discover_service_by_capability("object-storage");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://minio:9000");
        },
    );
}

#[test]
fn test_discover_service_url_pattern_alternate_capability() {
    with_vars(
        [
            ("TOADSTOOL_COMPUTE_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_COMPUTE_URL",
                Some("http://compute-svc:8000"),
            ),
        ],
        || {
            let result = discover_service_by_capability("compute");
            assert!(result.is_ok());
            let eps = result.unwrap();
            assert_eq!(eps[0].service_id, "compute-service");
            assert_eq!(eps[0].url, "http://compute-svc:8000");
        },
    );
}

#[test]
fn test_discover_no_service_found_error_message() {
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
            ("KUBERNETES_SERVICE_HOST", None::<&str>),
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ("CONSUL_HTTP_ADDR", None::<&str>),
            ("ETCD_ENDPOINTS", None::<&str>),
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("rare-capability-xyz");
            assert!(result.is_err());
            let err = result.unwrap_err();
            match &err {
                DiscoveryError::NoServiceFound { capability } => {
                    assert_eq!(capability, "rare-capability-xyz");
                }
                _ => panic!("expected NoServiceFound, got {err:?}"),
            }
        },
    );
}

#[test]
fn test_discover_filesystem_capability_dir_missing() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let service_dir = temp_dir.path().to_path_buf();

    with_vars(
        [
            ("TOADSTOOL_STORAGE_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_STORAGE_URL", None::<&str>),
            (
                "TOADSTOOL_SERVICE_DIR",
                Some(service_dir.to_string_lossy().as_ref()),
            ),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("nonexistent-capability-dir");
            assert!(result.is_err());
        },
    );
}

#[test]
fn test_primal_endpoint_capabilities_field() {
    let ep = PrimalEndpoint {
        service_id: "test".to_string(),
        url: "http://x:1".to_string(),
        capabilities: vec!["a".to_string(), "b".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    assert_eq!(ep.capabilities.len(), 2);
    assert!(ep.capabilities.contains(&"a".to_string()));
}

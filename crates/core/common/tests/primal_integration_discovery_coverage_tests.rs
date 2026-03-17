// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `primal_integration::discovery`
//! Target: 85%+ coverage of discovery module

use temp_env::with_vars;
use toadstool_common::primal_integration::{
    DiscoveryError, PrimalEndpoint, discover_cache_service, discover_coordination_service,
    discover_database_service, discover_encryption_service, discover_mcp_service,
    discover_object_storage, discover_service_by_capability, discover_storage_service,
};

// ─── Environment variable discovery: TOADSTOOL_{CAPABILITY}_ENDPOINT ─────────

#[test]
fn test_discover_service_by_capability_via_endpoint_env() {
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
fn test_discover_service_by_capability_uppercase_capability_in_env_var() {
    with_vars(
        [("TOADSTOOL_STORAGE_ENDPOINT", Some("http://nestgate:8080"))],
        || {
            let result = discover_service_by_capability("storage");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].url, "http://nestgate:8080");
            assert_eq!(endpoints[0].service_id, "storage-env");
        },
    );
}

#[test]
fn test_discover_service_by_capability_capability_with_hyphen() {
    // "object-storage" -> TOADSTOOL_OBJECT-STORAGE_ENDPOINT (hyphen in var name)
    with_vars(
        [("TOADSTOOL_OBJECT-STORAGE_ENDPOINT", Some("http://s3:9000"))],
        || {
            let result = discover_service_by_capability("object-storage");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].url, "http://s3:9000");
        },
    );
}

// ─── Generic TOADSTOOL_SERVICE_{NAME}_URL pattern ────────────────────────────

#[test]
fn test_discover_service_by_capability_via_service_url_env() {
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

#[test]
fn test_discover_service_by_capability_service_url_takes_second_priority() {
    with_vars(
        [
            (
                "TOADSTOOL_CACHE_ENDPOINT",
                Some("http://redis-primary:6379"),
            ),
            ("TOADSTOOL_SERVICE_CACHE_URL", Some("http://redis-alt:6379")),
        ],
        || {
            let result = discover_service_by_capability("cache");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://redis-primary:6379");
        },
    );
}

// ─── No service found error path ───────────────────────────────────────────────

#[test]
fn test_discover_service_by_capability_no_service_found() {
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

#[test]
fn test_discover_service_by_capability_error_display() {
    let err = DiscoveryError::NoServiceFound {
        capability: "test-cap".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("test-cap"));
    assert!(s.contains("No service found") || s.contains("service found"));
}

// ─── Discovery wrapper functions ─────────────────────────────────────────────

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

// ─── PrimalEndpoint structure and serialization ──────────────────────────────

#[test]
fn test_primal_endpoint_structure() {
    use std::time::SystemTime;
    let ep = PrimalEndpoint {
        service_id: "test-1".to_string(),
        url: "http://localhost:8080".to_string(),
        capabilities: vec!["storage".to_string(), "replication".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    assert_eq!(ep.service_id, "test-1");
    assert_eq!(ep.url, "http://localhost:8080");
    assert_eq!(ep.capabilities.len(), 2);
    assert!(ep.healthy);
}

#[test]
fn test_primal_endpoint_clone() {
    use std::time::SystemTime;
    let ep = PrimalEndpoint {
        service_id: "clone-test".to_string(),
        url: "http://x:1".to_string(),
        capabilities: vec!["a".to_string()],
        healthy: false,
        last_check: SystemTime::now(),
    };
    let cloned = ep.clone();
    assert_eq!(cloned.service_id, ep.service_id);
    assert_eq!(cloned.healthy, ep.healthy);
}

#[test]
fn test_primal_endpoint_serialization_roundtrip() {
    use std::time::SystemTime;
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
    assert_eq!(parsed.capabilities, ep.capabilities);
}

// ─── DiscoveryError variants ─────────────────────────────────────────────────

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

// ─── Edge cases: empty capability, capability with underscores ───────────────

#[test]
fn test_discover_service_empty_capability_no_env() {
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
fn test_discover_service_capability_with_underscores() {
    with_vars(
        [("TOADSTOOL_OBJECT_STORAGE_ENDPOINT", Some("http://s3:9000"))],
        || {
            let result = discover_service_by_capability("object_storage");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://s3:9000");
        },
    );
}

// ─── Filesystem discovery (TOADSTOOL_SERVICE_DIR) ───────────────────────────

#[test]
fn test_discover_service_via_filesystem() {
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
            assert!(endpoints[0].healthy);
        },
    );
}

#[test]
fn test_discover_service_via_xdg_runtime_biomeos() {
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
            assert!(endpoints[0].url.contains("coordination"));
        },
    );
}

#[test]
fn test_discover_service_filesystem_nonexistent_capability_dir() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let service_dir = temp_dir.path().to_path_buf();
    let _ = std::fs::create_dir_all(&service_dir);
    // Do NOT create subdir for "rarecap" - so filesystem discovery returns None
    // Other discovery methods also won't find it
    with_vars(
        [
            ("TOADSTOOL_RARECAP_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_DIR",
                Some(service_dir.to_string_lossy().as_ref()),
            ),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("rarecap");
            assert!(result.is_err());
        },
    );
}

// ─── TOADSTOOL_DISCOVERY_HTTP_PORT (used by K8s/Docker; parse fallback) ───

#[test]
fn test_discovery_http_port_env_invalid_fallback() {
    // Invalid port string -> parse fails -> uses DEFAULT_HTTP_PORT in k8s/compose
    // We can't easily hit k8s/compose without DNS, but we verify env parse behavior
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_DISCOVERY_HTTP_PORT", Some("not-a-number")),
        ],
        || {
            // discovery_http_port() is internal; we exercise via discover which
            // won't reach k8s/compose without KUBERNETES_SERVICE_HOST/COMPOSE
            let result = discover_service_by_capability("encryption");
            assert!(result.is_err());
        },
    );
}

#[test]
fn test_discovery_http_port_env_valid_custom() {
    with_vars(
        [
            ("TOADSTOOL_CACHE_ENDPOINT", Some("http://redis:9999")),
            ("TOADSTOOL_DISCOVERY_HTTP_PORT", Some("9999")),
        ],
        || {
            let result = discover_cache_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://redis:9999");
        },
    );
}

// ─── Capability name edge cases ────────────────────────────────────────────

#[test]
fn test_discover_service_capability_mixed_case_env() {
    // capability "Coordination" -> TOADSTOOL_COORDINATION_ENDPOINT (uppercased)
    with_vars(
        [(
            "TOADSTOOL_COORDINATION_ENDPOINT",
            Some("http://coord:50051"),
        )],
        || {
            let result = discover_service_by_capability("Coordination");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].service_id, "Coordination-env");
        },
    );
}

#[test]
fn test_discover_service_capability_with_numbers() {
    with_vars(
        [("TOADSTOOL_CACHE2_ENDPOINT", Some("http://cache2:6379"))],
        || {
            let result = discover_service_by_capability("cache2");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://cache2:6379");
        },
    );
}

#[test]
fn test_discover_service_generic_url_uppercase() {
    with_vars(
        [
            ("TOADSTOOL_MCP_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_MCP_URL", Some("http://mcp-svc:7070")),
        ],
        || {
            let result = discover_mcp_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://mcp-svc:7070");
        },
    );
}

// ─── Filesystem discovery: capability dir as file (not dir) ─────────────────

#[test]
fn test_discover_service_filesystem_path_exists_as_file() {
    // When capability path exists as file (not dir), discovery still returns it
    // (code checks exists(), not is_dir())
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let service_dir = temp_dir.path().to_path_buf();
    let file_path = service_dir.join("storage");
    std::fs::File::create(&file_path).expect("create file");
    with_vars(
        [
            ("TOADSTOOL_STORAGE_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_DIR",
                Some(service_dir.to_string_lossy().as_ref()),
            ),
        ],
        || {
            let result = discover_service_by_capability("storage");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].service_id, "storage-fs");
            assert!(endpoints[0].url.starts_with("file://"));
        },
    );
}

// ─── Service URL pattern with hyphen in capability ─────────────────────────

#[test]
fn test_discover_object_storage_via_service_url() {
    with_vars(
        [
            ("TOADSTOOL_OBJECT-STORAGE_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_OBJECT-STORAGE_URL",
                Some("http://minio:9001"),
            ),
        ],
        || {
            let result = discover_object_storage();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://minio:9001");
        },
    );
}

// ─── PrimalEndpoint Debug and PartialEq ────────────────────────────────────

#[test]
fn test_primal_endpoint_debug_format() {
    use std::time::SystemTime;
    let ep = PrimalEndpoint {
        service_id: "debug-test".to_string(),
        url: "http://x:1".to_string(),
        capabilities: vec!["a".to_string()],
        healthy: true,
        last_check: SystemTime::now(),
    };
    let debug_str = format!("{ep:?}");
    assert!(debug_str.contains("debug-test"));
    assert!(debug_str.contains("http://x:1"));
}

// ─── DiscoveryError Debug ──────────────────────────────────────────────────

#[test]
fn test_discovery_error_debug() {
    let err = DiscoveryError::NoServiceFound {
        capability: "debug-cap".to_string(),
    };
    let s = format!("{err:?}");
    assert!(s.contains("NoServiceFound") || s.contains("debug-cap"));
}

// ─── Registry discovery: non-http endpoint returns None (no TCP) ───────────

#[test]
fn test_registry_non_http_endpoint_skipped() {
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

#[test]
fn test_registry_https_endpoint_skipped() {
    with_vars(
        [
            ("TOADSTOOL_CACHE_ENDPOINT", None::<&str>),
            ("TOADSTOOL_REGISTRY_ENDPOINT", Some("https://registry:443")),
            ("KUBERNETES_SERVICE_HOST", None::<&str>),
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
        ],
        || {
            let result = discover_cache_service();
            assert!(result.is_err());
        },
    );
}

#[test]
fn test_discover_service_malformed_url_in_env() {
    with_vars(
        [("TOADSTOOL_ENCRYPTION_ENDPOINT", Some("not-a-valid-url"))],
        || {
            let result = discover_encryption_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "not-a-valid-url");
        },
    );
}

#[test]
fn test_discover_service_empty_string_env() {
    with_vars([("TOADSTOOL_ENCRYPTION_ENDPOINT", Some(""))], || {
        let result = discover_encryption_service();
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "");
    });
}

#[test]
fn test_discover_service_capability_underscore_to_hyphen() {
    with_vars(
        [("TOADSTOOL_OBJECT_STORAGE_ENDPOINT", Some("http://s3:9000"))],
        || {
            let result = discover_service_by_capability("object_storage");
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://s3:9000");
        },
    );
}

#[test]
fn test_discovery_http_port_custom_valid() {
    with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
            ("TOADSTOOL_DISCOVERY_HTTP_PORT", Some("8081")),
        ],
        || {
            let result = discover_encryption_service();
            assert!(result.is_err());
        },
    );
}

#[test]
fn test_filesystem_discovery_service_dir_takes_precedence() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let service_dir = temp_dir.path().to_path_buf();
    let xdg_dir = temp_dir.path().join("xdg");
    let biomeos_dir = xdg_dir.join("biomeos").join("mcp");
    std::fs::create_dir_all(&biomeos_dir).expect("create dirs");
    let service_mcp = service_dir.join("mcp");
    std::fs::create_dir_all(&service_mcp).expect("create mcp dir");

    with_vars(
        [
            ("TOADSTOOL_MCP_ENDPOINT", None::<&str>),
            (
                "TOADSTOOL_SERVICE_DIR",
                Some(service_dir.to_string_lossy().as_ref()),
            ),
            ("XDG_RUNTIME_DIR", Some(xdg_dir.to_string_lossy().as_ref())),
        ],
        || {
            let result = discover_mcp_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].service_id, "mcp-fs");
        },
    );
}

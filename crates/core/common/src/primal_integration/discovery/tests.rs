// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[test]
fn discover_service_by_capability_env_uppercase_conversion() {
    // capability "object-storage" -> TOADSTOOL_OBJECT-STORAGE_ENDPOINT
    temp_env::with_var(
        "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
        Some("https://s3.example.com/bucket"),
        || {
            let result = discover_service_by_capability("object-storage");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints.len(), 1);
            assert_eq!(endpoints[0].url, "https://s3.example.com/bucket");
            assert_eq!(endpoints[0].service_id, "object-storage-env");
        },
    );
}

#[test]
fn discover_service_by_capability_generic_url_pattern() {
    // TOADSTOOL_SERVICE_{CAPABILITY}_URL - capability uppercased, hyphens stay
    temp_env::with_var(
        "TOADSTOOL_SERVICE_OBJECT-STORAGE_URL",
        Some("http://minio:9000"),
        || {
            let result = discover_service_by_capability("object-storage");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].service_id, "object-storage-service");
            assert_eq!(endpoints[0].url, "http://minio:9000");
        },
    );
}

#[test]
fn discover_via_filesystem_xdg_runtime_dir_fallback() {
    let dir = tempfile::tempdir().expect("tempdir");
    let biomeos_dir = dir.path().join("biomeos");
    let capability_dir = biomeos_dir.join("coordination");
    std::fs::create_dir_all(&capability_dir).expect("create dirs");

    temp_env::with_vars(
        [
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            ("XDG_RUNTIME_DIR", Some(dir.path().to_str().unwrap())),
        ],
        || {
            let result = discover_service_by_capability("coordination");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints.len(), 1);
            assert_eq!(endpoints[0].service_id, "coordination-fs");
            assert!(endpoints[0].url.starts_with("file://"));
            assert!(endpoints[0].url.contains("coordination"));
        },
    );
}

#[test]
fn discover_service_capability_generic_url_underscore_capability() {
    // TOADSTOOL_SERVICE_{CAP}_URL - "custom_cap" -> CUSTOM_CAP
    temp_env::with_var(
        "TOADSTOOL_SERVICE_CUSTOM_CAP_URL",
        Some("http://custom:9999"),
        || {
            let result = discover_service_by_capability("custom_cap");
            assert!(result.is_ok());
            let endpoints = result.unwrap();
            assert_eq!(endpoints[0].url, "http://custom:9999");
        },
    );
}

#[test]
fn discover_encryption_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_ENCRYPTION_ENDPOINT",
        Some("http://crypto:6060"),
        || {
            let result = discover_encryption_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://crypto:6060");
        },
    );
}

#[test]
fn discover_storage_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_STORAGE_ENDPOINT",
        Some("http://storage:8080"),
        || {
            let result = discover_storage_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://storage:8080");
        },
    );
}

#[test]
fn discover_coordination_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_COORDINATION_ENDPOINT",
        Some("http://coord:6061"),
        || {
            let result = discover_coordination_service();
            assert!(result.is_ok());
            assert_eq!(result.unwrap()[0].url, "http://coord:6061");
        },
    );
}

#[test]
fn discover_mcp_delegates_to_capability() {
    temp_env::with_var("TOADSTOOL_MCP_ENDPOINT", Some("http://mcp:6062"), || {
        let result = discover_mcp_service();
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "http://mcp:6062");
    });
}

#[test]
fn discover_cache_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_CACHE_ENDPOINT",
        Some("redis://localhost:6379"),
        || {
            let result = discover_cache_service();
            assert!(result.is_ok());
        },
    );
}

#[test]
fn discover_database_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_DATABASE_ENDPOINT",
        Some("postgres://localhost:5432"),
        || {
            let result = discover_database_service();
            assert!(result.is_ok());
        },
    );
}

#[test]
fn discover_object_storage_delegates_to_capability() {
    temp_env::with_var(
        "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
        Some("https://s3.local"),
        || {
            let result = discover_object_storage();
            assert!(result.is_ok());
        },
    );
}

#[test]
fn no_service_found_error_format() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_UNKNOWN_CAP_XYZ_ENDPOINT", None::<&str>),
            ("TOADSTOOL_SERVICE_UNKNOWN_CAP_XYZ_URL", None::<&str>),
        ],
        || {
            let result = discover_service_by_capability("unknown-cap-xyz");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("unknown-cap-xyz"));
            assert!(err.to_string().contains("No service found"));
        },
    );
}

#[test]
fn primal_endpoint_healthy_and_last_check() {
    let endpoint = super::super::PrimalEndpoint {
        service_id: "test".to_string(),
        url: "http://test:80".to_string(),
        capabilities: vec!["test".to_string()],
        healthy: true,
        last_check: std::time::SystemTime::now(),
    };
    assert!(endpoint.healthy);
}

// --- discovery_http_port ---

#[test]
fn discovery_http_port_defaults_when_unset() {
    temp_env::with_var("TOADSTOOL_DISCOVERY_HTTP_PORT", None::<&str>, || {
        assert_eq!(
            super::discovery_http_port(),
            crate::constants::network::DEFAULT_HTTP_PORT
        );
    });
}

#[test]
fn discovery_http_port_invalid_env_falls_back_to_default() {
    temp_env::with_var("TOADSTOOL_DISCOVERY_HTTP_PORT", Some("not-a-port"), || {
        assert_eq!(
            super::discovery_http_port(),
            crate::constants::network::DEFAULT_HTTP_PORT
        );
    });
}

#[test]
fn discovery_http_port_valid_env_override() {
    temp_env::with_var("TOADSTOOL_DISCOVERY_HTTP_PORT", Some("9443"), || {
        assert_eq!(super::discovery_http_port(), 9443);
    });
}

// --- try_discover_via_filesystem (direct) ---

#[test]
fn try_discover_via_filesystem_no_base_env_returns_none() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SERVICE_DIR", None::<&str>),
            ("XDG_RUNTIME_DIR", None::<&str>),
        ],
        || {
            assert!(super::try_discover_via_filesystem("any").is_none());
        },
    );
}

#[test]
fn try_discover_via_filesystem_base_but_missing_subdir_returns_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    temp_env::with_var(
        "TOADSTOOL_SERVICE_DIR",
        Some(dir.path().to_str().expect("utf8 path")),
        || {
            assert!(super::try_discover_via_filesystem("missing-cap-dir").is_none());
        },
    );
}

// --- try_discover_via_kubernetes (direct) ---

#[test]
fn try_discover_via_kubernetes_without_cluster_returns_none() {
    temp_env::with_var("KUBERNETES_SERVICE_HOST", None::<&str>, || {
        assert!(super::try_discover_via_kubernetes("storage").is_none());
    });
}

#[test]
fn try_discover_via_kubernetes_unresolvable_service_returns_none() {
    temp_env::with_vars(
        [
            ("KUBERNETES_SERVICE_HOST", Some("10.96.0.1")),
            ("POD_NAMESPACE", Some("default")),
        ],
        || {
            assert!(super::try_discover_via_kubernetes("zz-unresolvable-cap-xyz-999").is_none());
        },
    );
}

// --- try_discover_via_docker_compose (direct, no cwd mutation) ---

#[test]
fn try_discover_via_docker_compose_without_signals_returns_none() {
    temp_env::with_vars(
        [
            ("COMPOSE_PROJECT_NAME", None::<&str>),
            ("TOADSTOOL_DISCOVERY_HTTP_PORT", None::<&str>),
        ],
        || {
            // When no compose project and no compose files in CWD, returns None immediately.
            if !std::path::Path::new("docker-compose.yml").exists()
                && !std::path::Path::new("compose.yaml").exists()
                && !std::path::Path::new("compose.yml").exists()
            {
                assert!(super::try_discover_via_docker_compose("storage").is_none());
            }
        },
    );
}

#[test]
fn try_discover_via_docker_compose_with_project_but_unresolvable_returns_none() {
    temp_env::with_vars(
        [
            ("COMPOSE_PROJECT_NAME", Some("testproj")),
            ("TOADSTOOL_DISCOVERY_HTTP_PORT", None::<&str>),
        ],
        || {
            assert!(super::try_discover_via_docker_compose("zz-no-such-compose-svc").is_none());
        },
    );
}

// --- try_discover_via_registry (mock HTTP server) ---

fn spawn_registry_response(body: String) -> (std::thread::JoinHandle<()>, String) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind registry mock");
    let addr = listener.local_addr().expect("local addr");
    let endpoint = format!("http://127.0.0.1:{}", addr.port());

    let handle = std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });

    (handle, endpoint)
}

#[test]
fn try_discover_via_registry_success_via_toadstool_endpoint() {
    let json = r#"{"services":[{"name":"svc-a","capabilities":["security"],"endpoints":["http://127.0.0.1:65001"]}]}"#
        .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            let got = super::try_discover_via_registry("security").expect("some");
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].url, "http://127.0.0.1:65001");
            assert_eq!(got[0].service_id, "svc-a-registry");
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_consul_http_addr() {
    let json = r#"{"services":[{"name":"c1","capabilities":["cache"],"endpoints":["http://127.0.0.1:65002"]}]}"#
        .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    let host_port = endpoint.trim_start_matches("http://");
    temp_env::with_vars(
        [
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ("CONSUL_HTTP_ADDR", Some(host_port)),
            ("ETCD_ENDPOINTS", None::<&str>),
        ],
        || {
            let got = super::try_discover_via_registry("cache").expect("some");
            assert_eq!(got[0].url, "http://127.0.0.1:65002");
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_etcd_endpoints_first_segment() {
    let json = r#"{"services":[{"name":"e1","capabilities":["database"],"endpoints":["http://127.0.0.1:65003"]}]}"#
        .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    let host_port = endpoint.trim_start_matches("http://");
    temp_env::with_vars(
        [
            ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ("CONSUL_HTTP_ADDR", None::<&str>),
            (
                "ETCD_ENDPOINTS",
                Some(&format!("{host_port},http://127.0.0.1:9")),
            ),
        ],
        || {
            let got = super::try_discover_via_registry("database").expect("some");
            assert_eq!(got[0].url, "http://127.0.0.1:65003");
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_non_http_registry_url_returns_none() {
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some("ftp://127.0.0.1:8080"),
        || {
            assert!(super::try_discover_via_registry("security").is_none());
        },
    );
}

#[test]
fn try_discover_via_registry_connect_fails_returns_none() {
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some("http://127.0.0.1:1"),
        || {
            assert!(super::try_discover_via_registry("security").is_none());
        },
    );
}

#[test]
fn try_discover_via_registry_invalid_json_returns_none() {
    let (server, endpoint) = spawn_registry_response("not json {".to_string());
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            assert!(super::try_discover_via_registry("security").is_none());
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_no_matching_capability_returns_none() {
    let json = r#"{"services":[{"name":"x","capabilities":["other"],"endpoints":["http://127.0.0.1:1"]}]}"#
        .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            assert!(super::try_discover_via_registry("security").is_none());
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_matching_but_non_http_endpoints_filtered_to_empty() {
    let json =
        r#"{"services":[{"name":"x","capabilities":["ai"],"endpoints":["grpc://127.0.0.1:99"]}]}"#
            .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            assert!(super::try_discover_via_registry("ai").is_none());
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_capability_underscore_matches_hyphen() {
    let json = r#"{"services":[{"name":"u","capabilities":["custom-cap"],"endpoints":["http://127.0.0.1:65004"]}]}"#
        .to_string();
    let (server, endpoint) = spawn_registry_response(json);
    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            let got = super::try_discover_via_registry("custom_cap").expect("some");
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].url, "http://127.0.0.1:65004");
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_registry_url_with_custom_path_segment() {
    let json = r#"{"services":[{"name":"p","capabilities":["mcp"],"endpoints":["http://127.0.0.1:65005"]}]}"#
        .to_string();
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let endpoint = format!("http://127.0.0.1:{port}/api/v1/services/list");

    let body = json.clone();
    let server = std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });

    temp_env::with_var(
        "TOADSTOOL_REGISTRY_ENDPOINT",
        Some(endpoint.as_str()),
        || {
            let got = super::try_discover_via_registry("mcp").expect("some");
            assert_eq!(got[0].url, "http://127.0.0.1:65005");
        },
    );
    server.join().expect("registry mock");
}

#[test]
fn try_discover_via_mdns_returns_none_or_some_without_panicking() {
    let _ = super::try_discover_via_mdns("unlikely-mdns-cap-xyz");
}

// --- builtin_default_endpoint (const) ---

#[test]
fn builtin_default_endpoint_is_none() {
    assert!(super::builtin_default_endpoint("anything").is_none());
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! DiscoveryClient parse_node_data tests

use std::sync::Arc;

use crate::songbird_integration::types::{DiscoveryClient, NodeType};
use toadstool_common::constants::ecosystem::node_type;

use super::make_songbird_connection;

#[test]
fn test_discovery_client_parse_node_data_success() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "parsed-node",
        "type": node_type::TOADSTOOL,
        "capabilities": {
            "cpu_cores": 8.0,
            "memory_gb": 16.0,
            "storage_gb": 256.0,
            "gpu_count": 1,
            "specialized_hardware": ["nvidia"],
            "software_capabilities": ["cuda"]
        },
        "endpoints": ["http://10.0.0.1:8080"],
        "protocols": ["http"],
        "version": "2.0",
        "build_info": "test-build"
    });

    let parsed = client.parse_node_data(&node_json).unwrap();
    assert_eq!(parsed.node_id, "parsed-node");
    assert!(matches!(parsed.node_type, NodeType::ToadStool));
    assert_eq!(parsed.capabilities.cpu_cores, 8.0);
    assert_eq!(parsed.capabilities.gpu_count, 1);
    assert_eq!(parsed.endpoints.len(), 1);
}

#[test]
fn test_discovery_client_parse_node_data_missing_node_id() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "type": node_type::TOADSTOOL,
        "capabilities": {},
        "endpoints": ["http://x"]
    });

    let result = client.parse_node_data(&node_json);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .to_lowercase()
        .contains("node_id"));
}

#[test]
fn test_discovery_client_parse_node_data_custom_type() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "custom-node",
        "type": "custom-type",
        "capabilities": {
            "cpu_cores": 1.0,
            "memory_gb": 2.0,
            "storage_gb": 10.0,
            "gpu_count": 0,
            "specialized_hardware": [],
            "software_capabilities": []
        },
        "endpoints": ["http://x"],
        "protocols": ["http"]
    });

    let parsed = client.parse_node_data(&node_json).unwrap();
    assert!(matches!(parsed.node_type, NodeType::Custom(s) if s == "custom-type"));
}

#[test]
fn test_discovery_client_parse_node_data_nestgate_type() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "nest-node",
        "type": node_type::NESTGATE,
        "capabilities": {
            "cpu_cores": 4.0,
            "memory_gb": 8.0,
            "storage_gb": 100.0,
            "gpu_count": 0,
            "specialized_hardware": [],
            "software_capabilities": []
        },
        "endpoints": ["http://nest:8080"],
        "protocols": ["http"]
    });

    let parsed = client.parse_node_data(&node_json).unwrap();
    assert!(matches!(parsed.node_type, NodeType::NestGate));
}

#[test]
fn test_discovery_client_parse_node_data_beardog_type() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "bd-node",
        "type": node_type::BEARDOG,
        "capabilities": {"cpu_cores": 1.0, "memory_gb": 2.0, "storage_gb": 10.0, "gpu_count": 0, "specialized_hardware": [], "software_capabilities": []},
        "endpoints": ["http://bd"],
        "protocols": ["http"]
    });

    let parsed = client.parse_node_data(&node_json).unwrap();
    assert!(matches!(parsed.node_type, NodeType::BearDog));
}

#[test]
fn test_discovery_client_parse_node_data_songbird_type() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "sb-node",
        "type": node_type::SONGBIRD,
        "capabilities": {"cpu_cores": 1.0, "memory_gb": 2.0, "storage_gb": 10.0, "gpu_count": 0, "specialized_hardware": [], "software_capabilities": []},
        "endpoints": ["http://sb"],
        "protocols": ["http"]
    });

    let parsed = client.parse_node_data(&node_json).unwrap();
    assert!(matches!(parsed.node_type, NodeType::Songbird));
}

#[test]
fn test_discovery_client_parse_node_data_minimal() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "minimal-node",
        "capabilities": {},
        "endpoints": ["http://localhost:8080"]
    });
    let result = client.parse_node_data(&node_json);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.node_id, "minimal-node");
}

#[test]
fn test_discovery_client_clone() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);
    let cloned = client.clone();
    let _ = cloned;
}

#[test]
fn test_discovery_client_parse_node_data_missing_endpoints_uses_unknown() {
    let conn = Arc::new(make_songbird_connection());
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("songbird.sock");
    let client = DiscoveryClient::for_test(conn, socket_path);

    let node_json = serde_json::json!({
        "node_id": "defaults-node",
        "capabilities": {}
    });
    let result = client.parse_node_data(&node_json);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.node_id, "defaults-node");
    assert_eq!(parsed.endpoints, vec!["unknown"]);
    assert_eq!(parsed.protocols, vec!["http"]);
    assert_eq!(parsed.metadata.version, "unknown");
}

//! Comprehensive tests for IPC helpers
//!
//! Tests the service-based IPC architecture components.

use toadstool::ipc_helpers::{connect_to_primal, PrimalCapabilities};

#[tokio::test]
async fn test_primal_capabilities_creation() {
    let caps = PrimalCapabilities {
        primal_id: "test-primal".to_string(),
        primal_type: "compute".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["compute".to_string(), "test".to_string()],
        socket_path: "/tmp/test.sock".to_string(),
        metadata: serde_json::json!({
            "test": true,
            "features": ["async", "concurrent"]
        }),
    };

    assert_eq!(caps.primal_id, "test-primal");
    assert_eq!(caps.primal_type, "compute");
    assert_eq!(caps.capabilities.len(), 2);
    assert!(caps.capabilities.contains(&"compute".to_string()));
}

#[tokio::test]
async fn test_primal_capabilities_serialization() {
    let caps = PrimalCapabilities {
        primal_id: "test".to_string(),
        primal_type: "compute".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["cap1".to_string()],
        socket_path: "/tmp/test.sock".to_string(),
        metadata: serde_json::json!({"key": "value"}),
    };

    // Should serialize to JSON
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("compute"));

    // Should round-trip
    let caps2: PrimalCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(caps2.primal_id, caps.primal_id);
    assert_eq!(caps2.primal_type, caps.primal_type);
}

#[tokio::test]
async fn test_primal_capabilities_discover_self() {
    // Should discover self
    let caps = PrimalCapabilities::discover_self("toadstool-test").await;

    assert_eq!(caps.primal_id, "toadstool-test");
    assert_eq!(caps.primal_type, "compute");
    assert!(!caps.version.is_empty());
    assert!(!caps.capabilities.is_empty());
    assert!(caps.capabilities.contains(&"compute".to_string()));
    assert!(!caps.socket_path.is_empty());
}

#[tokio::test]
async fn test_connect_to_primal_nonexistent() {
    // Should fail gracefully for non-existent socket
    let result = connect_to_primal("/tmp/nonexistent-primal-12345.sock").await;
    assert!(result.is_err());
}

#[test]
fn test_primal_capabilities_defaults() {
    // Test various capability combinations
    let caps1 = PrimalCapabilities {
        primal_id: "minimal".to_string(),
        primal_type: "compute".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec![],
        socket_path: "/tmp/minimal.sock".to_string(),
        metadata: serde_json::json!({}),
    };
    assert!(caps1.capabilities.is_empty());
    assert_eq!(caps1.metadata, serde_json::json!({}));

    let caps2 = PrimalCapabilities {
        primal_id: "full".to_string(),
        primal_type: "ui".to_string(),
        version: "2.0.0".to_string(),
        capabilities: vec![
            "ui".to_string(),
            "graphics".to_string(),
            "input".to_string(),
        ],
        socket_path: "/run/user/1000/full.sock".to_string(),
        metadata: serde_json::json!({
            "features": ["async", "gpu"],
            "priority": "high"
        }),
    };
    assert_eq!(caps2.capabilities.len(), 3);
    assert!(caps2.metadata["features"].is_array());
}

#[test]
fn test_socket_path_patterns() {
    // Test various socket path formats
    let patterns = vec![
        "/tmp/primal.sock",
        "/run/user/1000/toadstool/primal.sock",
        "/var/run/ecoPrimals/primal.sock",
    ];

    for path in patterns {
        let caps = PrimalCapabilities {
            primal_id: "test".to_string(),
            primal_type: "compute".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            socket_path: path.to_string(),
            metadata: serde_json::json!({}),
        };
        assert_eq!(caps.socket_path, path);
    }
}

// ✅ IPC Helpers: Comprehensive test coverage
// Tests discovery, serialization, connection handling, and various scenarios

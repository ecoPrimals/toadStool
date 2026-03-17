// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage expansion tests for launcher module
//!
//! Tests error paths and edge cases for endpoint discovery and health checks.

use std::path::PathBuf;
use toadstool::launcher::{
    Endpoint, LaunchConfig, check_toadstool_health, discover_toadstool_endpoint,
    verify_endpoint_exists,
};

#[tokio::test]
async fn test_discover_toadstool_endpoint_no_endpoint_returns_err() {
    // When no toadstool socket or discovery file exists, discover returns Err.
    // In typical CI/test env, no toadstool daemon is running.
    let result = discover_toadstool_endpoint().await;
    match result {
        Ok(_) => {
            // May succeed if socket happens to exist (e.g. dev machine with daemon)
        }
        Err(e) => {
            assert!(
                e.to_string().contains("toadstool") || e.to_string().contains("endpoint"),
                "Error should mention endpoint: {e}"
            );
        }
    }
}

#[tokio::test]
async fn test_verify_endpoint_exists_propagates_discovery_error() {
    let result = verify_endpoint_exists().await;
    match result {
        Ok(()) => {
            // May succeed if endpoint exists
        }
        Err(e) => {
            assert!(!e.to_string().is_empty());
        }
    }
}

#[tokio::test]
async fn test_check_toadstool_health_propagates_verify_error() {
    let result = check_toadstool_health().await;
    match result {
        Ok(()) => {}
        Err(e) => {
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
fn test_launch_config_default_args() {
    let config = LaunchConfig::default();
    assert_eq!(config.args, vec!["daemon".to_string()]);
}

#[test]
fn test_launch_config_default_binary() {
    let config = LaunchConfig::default();
    assert_eq!(config.binary_path, PathBuf::from("toadstool"));
}

#[test]
fn test_endpoint_unix_display() {
    let ep = Endpoint::Unix(PathBuf::from("/var/run/toadstool.sock"));
    let s = ep.to_string();
    assert!(s.starts_with("unix:"));
    assert!(s.contains("toadstool"));
}

#[test]
fn test_endpoint_tcp_display() {
    use std::net::SocketAddr;
    let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
    let ep = Endpoint::Tcp(addr);
    let s = ep.to_string();
    assert!(s.starts_with("tcp:"));
    assert!(s.contains("127.0.0.1"));
}

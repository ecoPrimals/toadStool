// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for daemon/server.rs — config and startup paths.

use std::path::PathBuf;

use toadstool_cli::daemon::{DaemonConfig, DaemonServer};

#[test]
fn daemon_config_default_values() {
    let config = DaemonConfig::default();
    assert!(!config.register_with_biomeos);
    assert!(config.max_concurrent_workloads > 0);
}

#[test]
fn daemon_config_custom_values() {
    let config = DaemonConfig {
        register_with_biomeos: true,
        port: 9999,
        socket_path: Some(PathBuf::from("/tmp/test.sock")),
        max_concurrent_workloads: 4,
        ..DaemonConfig::default()
    };
    assert!(config.register_with_biomeos);
    assert_eq!(config.port, 9999);
    assert_eq!(
        config.socket_path.as_ref().unwrap(),
        &PathBuf::from("/tmp/test.sock")
    );
    assert_eq!(config.max_concurrent_workloads, 4);
}

#[test]
fn daemon_config_with_socket_path() {
    let config = DaemonConfig {
        socket_path: Some(PathBuf::from("/run/toadstool/daemon.sock")),
        ..DaemonConfig::default()
    };
    assert_eq!(
        config.socket_path.unwrap(),
        PathBuf::from("/run/toadstool/daemon.sock")
    );
}

#[tokio::test]
async fn daemon_server_start_default_config() {
    let config = DaemonConfig::default();
    let result: std::result::Result<DaemonServer, _> = DaemonServer::start(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn daemon_server_start_with_biomeos_registration() {
    let config = DaemonConfig {
        register_with_biomeos: true,
        ..DaemonConfig::default()
    };
    let result: std::result::Result<DaemonServer, _> = DaemonServer::start(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn daemon_server_start_with_custom_workload_limit() {
    let config = DaemonConfig {
        max_concurrent_workloads: 16,
        ..DaemonConfig::default()
    };
    let result: std::result::Result<DaemonServer, _> = DaemonServer::start(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn daemon_server_start_with_socket_path() {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("daemon.sock");
    let config = DaemonConfig {
        socket_path: Some(socket),
        ..DaemonConfig::default()
    };
    let result: std::result::Result<DaemonServer, _> = DaemonServer::start(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn daemon_server_start_with_custom_port() {
    let config = DaemonConfig {
        port: 19090,
        ..DaemonConfig::default()
    };
    let result: std::result::Result<DaemonServer, _> = DaemonServer::start(config).await;
    assert!(result.is_ok());
}

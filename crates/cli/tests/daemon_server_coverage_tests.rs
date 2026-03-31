// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Coverage tests for daemon/server.rs — config and startup paths.

use std::path::PathBuf;
use std::time::Duration;

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

/// Verify socket path resolution when socket_path is None uses PlatformPaths
#[test]
fn daemon_socket_path_resolution_uses_platform_paths_when_none() {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let config = DaemonConfig::default();
    assert!(config.socket_path.is_none());
    let env = PathEnv::from_env();
    let paths = PlatformPaths::new(&env);
    let socket = paths.toadstool_socket();
    assert!(!socket.as_os_str().is_empty());
}

/// Run daemon briefly and verify it shuts down on SIGINT (Unix only)
#[cfg(unix)]
#[tokio::test]
async fn daemon_server_run_shuts_down_on_sigint() {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("daemon_run.sock");
    let sock_path = socket.clone();
    let config = DaemonConfig {
        socket_path: Some(socket),
        ..DaemonConfig::default()
    };
    let server = DaemonServer::start(config).await.expect("start");
    let handle = tokio::spawn(async move { server.run().await });
    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if sock_path.exists() {
                break;
            }
            tokio::task::yield_now().await;
        }
        let pid = rustix::process::getpid();
        let _ = rustix::process::kill_process(pid, rustix::process::Signal::Int);
    });
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok(), "run should complete within timeout");
    let run_result = result.unwrap();
    assert!(run_result.is_ok(), "run should succeed");
}

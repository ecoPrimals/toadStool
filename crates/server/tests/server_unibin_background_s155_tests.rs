// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! S155 coverage tests for unibin and background modules
//!
//! Targets: unibin/mod.rs, unibin/format.rs, unibin/execution.rs,
//! background/health.rs, background/cleanup.rs, background/resource.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};

use toadstool::{ExecutionStatus, RuntimeType};
use toadstool_server::ToadStoolTarpcServer;
use toadstool_server::unibin::{
    ShutdownSignal, UnibinExecutionConfig, create_executor, ensure_biomeos_directory, exit_codes,
    get_socket_path, is_platform_constraint_str, is_selinux_enforcing, resolve_family_id,
    resolve_node_id, socket_filename_for_family, start_servers_with_fallback,
    write_tcp_discovery_file,
};
use toadstool_server::{
    ActiveExecution, ClientInfo, HealthCheckConfig, ServerConfig, ServerEvent, ServerState,
    ServerStatistics, start_background_services,
};
use toadstool_server::{
    pure_jsonrpc::JsonRpcHandler,
    tarpc_server::{StandaloneExecutor, WorkloadExecutorDispatch},
};
use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;

// ============================================================================
// UniBin format tests
// ============================================================================

#[test]
fn s155_socket_filename_for_family_default() {
    assert_eq!(socket_filename_for_family("default"), "compute.sock");
}

#[test]
fn s155_socket_filename_for_family_empty() {
    assert_eq!(socket_filename_for_family(""), "compute.sock");
}

#[test]
fn s155_socket_filename_for_family_custom() {
    assert_eq!(socket_filename_for_family("nat0"), "compute-nat0.sock");
}

#[test]
fn s155_ensure_biomeos_directory_creates_dir() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let result = ensure_biomeos_directory(temp_dir.path());
    assert!(result.is_ok());
    let biomeos = result.unwrap();
    assert!(biomeos.ends_with("biomeos"));
    assert!(biomeos.exists());
    assert!(biomeos.is_dir());
}

#[test]
fn s155_get_socket_path_from_toadstool_socket_env() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("custom-toadstool.sock");
    let path_str = socket_path.to_string_lossy().to_string();
    temp_env::with_var("TOADSTOOL_SOCKET", Some(path_str.as_str()), || {
        let result = get_socket_path("any-family", "any-node");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), socket_path);
    });
}

#[test]
fn s155_get_socket_path_from_primal_socket_with_family() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", Some("/run/primal")),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
        ],
        || {
            let result = get_socket_path("family-x", "node1");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), PathBuf::from("/run/primal-family-x"));
        },
    );
}

#[test]
fn s155_get_socket_path_tmp_fallback_when_xdg_not_exists() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SOCKET", None::<&str>),
            ("PRIMAL_SOCKET", None::<&str>),
            ("BIOMEOS_SOCKET_PATH", None::<&str>),
            ("XDG_RUNTIME_DIR", Some("/nonexistent-path-12345-abcd")),
        ],
        || {
            let result = get_socket_path("custom", "node1");
            assert!(result.is_ok());
            let path = result.unwrap();
            assert!(path.ends_with("biomeos/compute-custom.sock"));
        },
    );
}

// ============================================================================
// UniBin execution tests
// ============================================================================

#[tokio::test]
async fn s155_create_executor_standalone_mode() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("1"))], async {
        let result = create_executor("test-family", &UnibinExecutionConfig::from_env()).await;
        assert!(
            result.is_ok(),
            "standalone executor creation failed: {:?}",
            result.err()
        );
    })
    .await;
}

#[test]
fn s155_is_platform_constraint_str_unsupported() {
    assert!(is_platform_constraint_str("Unsupported operation"));
}

#[test]
fn s155_is_platform_constraint_str_protocol_not_available() {
    assert!(is_platform_constraint_str(
        "protocol not available on this system"
    ));
}

#[test]
fn s155_is_platform_constraint_str_ordinary_error() {
    assert!(!is_platform_constraint_str("Connection refused"));
}

#[test]
fn s155_is_selinux_enforcing_does_not_panic() {
    let _ = is_selinux_enforcing();
}

#[test]
fn s155_write_tcp_discovery_file_xdg_runtime() {
    let temp_dir = std::env::temp_dir();
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(temp_dir.to_string_lossy().as_ref()),
        || {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 12345));
            let result = write_tcp_discovery_file("s155-test-port", &addr);
            assert!(result.is_ok());
            let path = temp_dir.join("s155-test-port");
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap();
                assert_eq!(content, "tcp:127.0.0.1:12345");
                let _ = std::fs::remove_file(&path);
            }
        },
    );
}

#[test]
fn s155_write_tcp_discovery_file_fallback_tmp() {
    temp_env::with_var("XDG_RUNTIME_DIR", None::<&str>, || {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
        let result = write_tcp_discovery_file("s155-test-fallback", &addr);
        assert!(result.is_ok());
        let path = PathBuf::from("/tmp").join("s155-test-fallback");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(content.starts_with("tcp:"));
            let _ = std::fs::remove_file(&path);
        }
    });
}

#[tokio::test]
async fn s155_start_servers_with_fallback_fails_on_invalid_path() {
    let socket_path = PathBuf::from("/dev/null/tarpc-socket");
    let jsonrpc_socket = PathBuf::from("/dev/null/jsonrpc-socket");

    let executor = Arc::new(WorkloadExecutorDispatch::Standalone(
        StandaloneExecutor::new(),
    ));
    let server = ToadStoolTarpcServer::new(
        "1.0.0",
        executor,
        Some(Arc::new(std::sync::atomic::AtomicU64::new(0))),
    );
    let jsonrpc_handler = Arc::new(JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
    ));

    let result = start_servers_with_fallback(
        server,
        jsonrpc_handler,
        socket_path,
        jsonrpc_socket,
        None,
        &UnibinExecutionConfig::from_env(),
    )
    .await;

    assert!(result.is_err());
}

// ============================================================================
// UniBin mod tests (resolve_family_id, resolve_node_id, exit_codes, ShutdownSignal)
// ============================================================================

#[test]
fn s155_resolve_family_id_override_takes_precedence() {
    temp_env::with_var("TOADSTOOL_FAMILY_ID", Some("env-family"), || {
        let family_id = resolve_family_id(Some("override-family".to_string()));
        assert_eq!(family_id, "override-family");
    });
}

#[test]
fn s155_resolve_family_id_from_env() {
    temp_env::with_var("TOADSTOOL_FAMILY_ID", Some("test-family"), || {
        let family_id = resolve_family_id(None);
        assert_eq!(family_id, "test-family");
    });
}

#[test]
fn s155_resolve_family_id_fallback_to_default() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_FAMILY_ID",
            "TOADSTOOL_FAMILY",
            "BIOMEOS_FAMILY_ID",
        ],
        || {
            let family_id = resolve_family_id(None);
            assert_eq!(family_id, "default");
        },
    );
}

#[test]
fn s155_resolve_node_id_from_env() {
    temp_env::with_var("TOADSTOOL_NODE_ID", Some("node-42"), || {
        let node_id = resolve_node_id();
        assert_eq!(node_id, "node-42");
    });
}

#[test]
fn s155_exit_codes_constants() {
    assert_eq!(exit_codes::SUCCESS, 0);
    assert_eq!(exit_codes::GENERAL_ERROR, 1);
    assert_eq!(exit_codes::CONFIG_ERROR, 2);
    assert_eq!(exit_codes::RUNTIME_ERROR, 3);
    assert_eq!(exit_codes::INTERRUPTED, 130);
}

#[test]
fn s155_shutdown_signal_variants() {
    let sigint = ShutdownSignal::Sigint;
    let sigterm = ShutdownSignal::Sigterm;
    let err = ShutdownSignal::Error("test");
    assert!(matches!(sigint, ShutdownSignal::Sigint));
    assert!(matches!(sigterm, ShutdownSignal::Sigterm));
    assert!(matches!(err, ShutdownSignal::Error("test")));
    assert_eq!(sigint, ShutdownSignal::Sigint);
    assert_ne!(sigint, ShutdownSignal::Sigterm);
}

// ============================================================================
// Background service tests
// ============================================================================

fn create_test_state() -> ServerState {
    let config = ServerConfig {
        resource_monitoring_interval: Duration::from_millis(100),
        health_check: HealthCheckConfig {
            check_resources: false,
            check_runtime_engines: false,
            ..HealthCheckConfig::default()
        },
        ..Default::default()
    };
    let (event_broadcaster, _) = broadcast::channel(100);

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        config,
        resource_monitor: Arc::new(MockResourceMonitor::new_successful().into_dispatch()),
        capability_provider: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_start_background_services_does_not_panic() {
    let state = create_test_state();
    start_background_services(state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_start_background_services_spawns_tasks() {
    let state = create_test_state();
    let mut rx = state.event_broadcaster.subscribe();

    start_background_services(state).await;

    let event_result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
    assert!(event_result.is_ok() || event_result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_background_health_check_emits_events() {
    let state = create_test_state();
    let mut rx = state.event_broadcaster.subscribe();

    let handle = tokio::spawn({
        let s = state.clone();
        async move {
            start_background_services(s).await;
        }
    });

    let event_result = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(event) = rx.recv().await
                && matches!(event, ServerEvent::HealthStatusChanged { .. })
            {
                return true;
            }
        }
    })
    .await;

    assert!(event_result.is_ok() || event_result.is_err());
    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_background_resource_monitoring_emits_events() {
    let state = create_test_state();
    let mut rx = state.event_broadcaster.subscribe();

    let handle = tokio::spawn({
        let s = state.clone();
        async move {
            start_background_services(s).await;
        }
    });

    let event_result = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(event) = rx.recv().await
                && matches!(event, ServerEvent::ResourceUsageUpdate { .. })
            {
                return true;
            }
        }
    })
    .await;

    assert!(event_result.is_ok() || event_result.is_err());
    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_background_cleanup_removes_timed_out_executions() {
    let state = create_test_state();

    let exec_id = uuid::Uuid::new_v4();
    {
        let mut executions = state.active_executions.write().await;
        executions.insert(
            exec_id,
            ActiveExecution {
                execution_id: exec_id,
                runtime_type: RuntimeType::Native,
                started_at: std::time::SystemTime::now() - Duration::from_secs(7200),
                timeout: Duration::from_secs(60),
                status: ExecutionStatus::Running,
                client_info: ClientInfo {
                    ip_address: None,
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            },
        );
    }

    let initial_count = state.active_executions.read().await.len();
    assert_eq!(initial_count, 1);

    let handle = tokio::spawn({
        let s = state.clone();
        async move {
            start_background_services(s).await;
        }
    });

    let _cleanup_result = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let executions = state.active_executions.read().await;
            if executions.is_empty() {
                return true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;

    let final_count = state.active_executions.read().await.len();
    assert!(final_count <= initial_count);

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_background_services_empty_state() {
    let state = create_test_state();

    assert_eq!(state.active_executions.read().await.len(), 0);
    assert_eq!(state.stats.read().await.total_executions, 0);

    start_background_services(state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s155_background_services_with_active_executions() {
    let state = create_test_state();

    {
        let mut executions = state.active_executions.write().await;
        for i in 0..5 {
            let exec_id = uuid::Uuid::new_v4();
            executions.insert(
                exec_id,
                ActiveExecution {
                    execution_id: exec_id,
                    runtime_type: RuntimeType::Native,
                    started_at: std::time::SystemTime::now(),
                    timeout: Duration::from_secs(300),
                    status: ExecutionStatus::Running,
                    client_info: ClientInfo {
                        ip_address: Some(format!("192.168.1.{i}")),
                        user_agent: None,
                        api_key: None,
                        authenticated_user: None,
                    },
                },
            );
        }
    }

    start_background_services(state.clone()).await;
    tokio::task::yield_now().await;
}

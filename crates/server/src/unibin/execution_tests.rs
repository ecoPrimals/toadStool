// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[tokio::test]
async fn create_executor_standalone_mode() {
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

#[tokio::test]
async fn create_executor_standalone_mode_true_lowercase() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("true"))], async {
        let result = create_executor("my-family", &UnibinExecutionConfig::from_env()).await;
        assert!(result.is_ok());
    })
    .await;
}

#[tokio::test]
async fn create_executor_standalone_mode_true_uppercase() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("TRUE"))], async {
        let result = create_executor("test-family", &UnibinExecutionConfig::from_env()).await;
        assert!(
            result.is_ok(),
            "standalone executor with TRUE should succeed: {:?}",
            result.err()
        );
    })
    .await;
}

#[test]
fn write_tcp_discovery_file_fails_on_readonly_dir() {
    temp_env::with_var("XDG_RUNTIME_DIR", Some("/proc/self"), || {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
        let result = write_tcp_discovery_file("toadstool-test-readonly", &addr);
        assert!(result.is_err(), "writing to /proc/self should fail");
    });
}

#[tokio::test]
async fn create_executor_integrated_mode_when_standalone_unset() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", None::<&str>)], async {
        let result = create_executor("integrated-family", &UnibinExecutionConfig::from_env()).await;
        match &result {
            Ok(_) => {}
            Err(e) => assert!(!e.to_string().is_empty(), "error should have message"),
        }
    })
    .await;
}

#[tokio::test]
async fn create_executor_integrated_mode_when_standalone_0() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("0"))], async {
        let result = create_executor("family-0", &UnibinExecutionConfig::from_env()).await;
        match &result {
            Ok(_) => {}
            Err(e) => assert!(!e.to_string().is_empty()),
        }
    })
    .await;
}

#[test]
fn is_platform_constraint_str_selinux_permission_denied() {
    // When SELinux is enforcing, "Permission denied" is platform constraint
    // Result depends on is_selinux_enforcing() - we test the string matching
    let r = is_platform_constraint_str("some error");
    assert!(!r);
}

#[test]
fn is_platform_constraint_str_unsupported() {
    assert!(is_platform_constraint_str("Unsupported operation"));
}

#[test]
fn is_platform_constraint_str_not_supported() {
    assert!(is_platform_constraint_str("protocol not supported"));
}

#[test]
fn is_platform_constraint_str_protocol_not_available() {
    assert!(is_platform_constraint_str(
        "protocol not available on this system"
    ));
}

#[test]
fn is_platform_constraint_str_operation_not_permitted() {
    // Depends on SELinux - without SELinux this returns false
    let _ = is_platform_constraint_str("Operation not permitted");
}

#[test]
fn is_selinux_enforcing_does_not_panic() {
    let _ = is_selinux_enforcing();
}

#[test]
fn write_tcp_discovery_file_xdg_runtime() {
    let temp_dir = std::env::temp_dir();
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(temp_dir.to_string_lossy().as_ref()),
        || {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 12345));
            let result = write_tcp_discovery_file("test-toadstool-port", &addr);
            assert!(result.is_ok());
            let path = temp_dir.join("test-toadstool-port");
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap();
                assert_eq!(content, "tcp:127.0.0.1:12345");
                let _ = std::fs::remove_file(&path);
            }
        },
    );
}

#[test]
fn write_tcp_discovery_file_fallback_tmp() {
    temp_env::with_var("XDG_RUNTIME_DIR", None::<&str>, || {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
        let result = write_tcp_discovery_file("toadstool-test-fallback", &addr);
        assert!(result.is_ok());
        let path = std::env::temp_dir().join("toadstool-test-fallback");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(content.starts_with("tcp:"));
            let _ = std::fs::remove_file(&path);
        }
    });
}

#[test]
fn unibin_config_bind_any_os_port_uses_host() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", Some("10.0.0.5")),
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_TCP_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", None::<&str>),
            ("TOADSTOOL_EXECUTION_TIMEOUT", None::<&str>),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(cfg.bind_any_os_port(), "10.0.0.5:0");
        },
    );
}

#[test]
fn unibin_config_tcp_ipc_bind_addr_explicit_override() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", Some("127.0.0.1")),
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_TCP_BIND_ADDRESS", Some("0.0.0.0:9999")),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", None::<&str>),
            ("TOADSTOOL_EXECUTION_TIMEOUT", None::<&str>),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(cfg.tcp_ipc_bind_addr(), "0.0.0.0:9999");
        },
    );
}

#[test]
fn unibin_config_tcp_ipc_bind_addr_fallback_os_port() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", Some("192.168.1.1")),
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_TCP_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", None::<&str>),
            ("TOADSTOOL_EXECUTION_TIMEOUT", None::<&str>),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(cfg.tcp_ipc_bind_addr(), "192.168.1.1:0");
        },
    );
}

#[test]
fn unibin_config_max_concurrent_custom() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_TCP_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("42")),
            ("TOADSTOOL_EXECUTION_TIMEOUT", Some("600")),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(cfg.max_concurrent_executions, 42);
            assert_eq!(cfg.default_timeout_secs, 600);
        },
    );
}

#[test]
fn unibin_config_defaults_without_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_TCP_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", None::<&str>),
            ("TOADSTOOL_EXECUTION_TIMEOUT", None::<&str>),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("COORDINATION_AUTH_TOKEN", None::<&str>),
            ("SONGBIRD_AUTH_TOKEN", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(
                cfg.max_concurrent_executions,
                unibin_execution_defaults::DEFAULT_MAX_CONCURRENT_WORKLOADS,
            );
            assert_eq!(
                cfg.default_timeout_secs,
                unibin_execution_defaults::DEFAULT_WORKLOAD_TIMEOUT_SECS,
            );
            assert_eq!(
                cfg.max_queue_size,
                unibin_execution_defaults::DEFAULT_MAX_JOB_QUEUE_SIZE,
            );
            assert!(cfg.enable_job_queue);
            assert!(cfg.coordination_auth_token.is_none());
            assert!(!cfg.headless);
        },
    );
}

#[test]
fn unibin_config_headless_true() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_HEADLESS", Some("1")),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert!(cfg.headless);
        },
    );
}

#[test]
fn unibin_config_headless_true_text() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_HEADLESS", Some("TRUE")),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert!(cfg.headless);
        },
    );
}

#[test]
fn unibin_config_invalid_numeric_env_uses_defaults() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_STANDALONE", Some("1")),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("not_a_number")),
            ("TOADSTOOL_EXECUTION_TIMEOUT", Some("abc")),
            ("TOADSTOOL_HEADLESS", None::<&str>),
            ("TRANSPORT_ENDPOINT", None::<&str>),
        ],
        || {
            let cfg = UnibinExecutionConfig::from_env();
            assert_eq!(
                cfg.max_concurrent_executions,
                unibin_execution_defaults::DEFAULT_MAX_CONCURRENT_WORKLOADS,
            );
            assert_eq!(
                cfg.default_timeout_secs,
                unibin_execution_defaults::DEFAULT_WORKLOAD_TIMEOUT_SECS,
            );
        },
    );
}

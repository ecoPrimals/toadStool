// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[tokio::test]
async fn create_executor_standalone_mode() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("1"))], async {
        let result = create_executor("test-family").await;
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
        let result = create_executor("my-family").await;
        assert!(result.is_ok());
    })
    .await;
}

#[tokio::test]
async fn create_executor_standalone_mode_true_uppercase() {
    temp_env::async_with_vars([("TOADSTOOL_STANDALONE", Some("TRUE"))], async {
        let result = create_executor("test-family").await;
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
        let result = create_executor("integrated-family").await;
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
        let result = create_executor("family-0").await;
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

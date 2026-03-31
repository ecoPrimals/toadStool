// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for configuration constants and network utilities

use super::*;

// ===== Network constants =====
#[test]
#[allow(clippy::assertions_on_constants)]
fn test_network_constants() {
    assert_eq!(network::DEFAULT_REQUEST_TIMEOUT_SECS, 30);
    assert_eq!(network::DEFAULT_CONNECTION_TIMEOUT_SECS, 10);
    assert_eq!(network::DEFAULT_MAX_RETRIES, 3);
    assert_eq!(network::DEFAULT_KEEPALIVE_INTERVAL_SECS, 30);
    assert_eq!(network::DEFAULT_MAX_CONNECTIONS_PER_HOST, 100);
}

// ===== Network deprecated endpoint functions (with #[allow(deprecated)]) =====
#[test]
#[allow(deprecated)]
fn test_default_songbird_endpoint() {
    let ep = network::default_songbird_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(':'));
}

#[test]
#[allow(deprecated)]
fn test_default_beardog_endpoint() {
    let ep = network::default_beardog_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains("8081"));
}

#[test]
#[allow(deprecated)]
fn test_default_nestgate_endpoint() {
    let ep = network::default_nestgate_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains("8082"));
}

#[test]
#[allow(deprecated)]
fn test_default_squirrel_endpoint() {
    let ep = network::default_squirrel_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains("8083"));
}

#[test]
#[allow(deprecated)]
fn test_default_toadstool_endpoint() {
    let ep = network::default_toadstool_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(':'));
}

#[test]
fn test_default_federation_address() {
    let _addr = network::default_federation_address();
}

// ===== Port getters (with temp_env for thread-safe scoping) =====
#[test]
#[allow(deprecated)]
fn test_get_songbird_port_default() {
    temp_env::with_var("SONGBIRD_PORT", None::<&str>, || {
        let port = network::get_songbird_port();
        assert!(port > 0);
    });
}

#[test]
#[allow(deprecated)]
fn test_get_beardog_port_default() {
    temp_env::with_var("BEARDOG_PORT", None::<&str>, || {
        let port = network::get_beardog_port();
        assert_eq!(port, 8081);
    });
}

#[test]
#[allow(deprecated)]
fn test_get_nestgate_port_default() {
    temp_env::with_var("NESTGATE_PORT", None::<&str>, || {
        let port = network::get_nestgate_port();
        assert_eq!(port, 8082);
    });
}

#[test]
#[allow(deprecated)]
fn test_get_squirrel_port_default() {
    temp_env::with_var("TOADSTOOL_SQUIRREL_PORT", None::<&str>, || {
        let port = network::get_squirrel_port();
        assert_eq!(port, 8083);
    });
}

#[test]
#[allow(deprecated)]
fn test_get_toadstool_port_default() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PORT", None::<&str>),
            ("TOADSTOOL_API_PORT", None::<&str>),
        ],
        || {
            let _port = network::get_toadstool_port();
        },
    );
}

#[test]
fn test_get_bind_host_default() {
    temp_env::with_var("BIND_ADDRESS", None::<&str>, || {
        let host = network::get_bind_host();
        assert_eq!(host, "127.0.0.1");
    });
}

// ===== Endpoint getters =====
#[test]
#[allow(deprecated)]
fn test_get_songbird_endpoint() {
    let ep = network::get_songbird_endpoint();
    assert!(ep.starts_with("http://"));
}

#[test]
#[allow(deprecated)]
fn test_get_beardog_endpoint() {
    let ep = network::get_beardog_endpoint();
    assert!(ep.starts_with("http://"));
}

#[test]
#[allow(deprecated)]
fn test_get_nestgate_endpoint() {
    let ep = network::get_nestgate_endpoint();
    assert!(ep.starts_with("http://"));
}

#[test]
#[allow(deprecated)]
fn test_get_squirrel_endpoint() {
    let ep = network::get_squirrel_endpoint();
    assert!(ep.starts_with("http://"));
}

#[test]
#[allow(deprecated)]
fn test_get_toadstool_endpoint() {
    let ep = network::get_toadstool_endpoint();
    assert!(ep.starts_with("http://"));
}

// ===== App constants =====
#[test]
#[allow(clippy::assertions_on_constants)]
fn test_app_constants() {
    assert_eq!(app::DEFAULT_APP_NAME, "toadstool");
    assert_eq!(app::DEFAULT_ENVIRONMENT, "development");
    assert_eq!(app::DEFAULT_LOG_LEVEL, "info");
    assert_eq!(app::DEFAULT_CONFIG_FILE, "toadstool.toml");
    assert_eq!(app::DEFAULT_DATA_DIR, "./data");
    assert_eq!(app::DEFAULT_CACHE_DIR, "./cache");
    assert_eq!(app::DEFAULT_LOGS_DIR, "./logs");
    assert_eq!(app::default_temp_dir(), std::env::temp_dir());
}

// ===== Testing constants =====
#[test]
#[allow(clippy::assertions_on_constants)]
fn test_testing_constants() {
    assert_eq!(testing::DEFAULT_TEST_TIMEOUT_SECS, 30);
    assert_eq!(testing::DEFAULT_TEST_PORT, 9999);
    assert_eq!(testing::DEFAULT_TEST_ENVIRONMENT, "test");
}

// ===== Development constants =====
#[test]
#[allow(clippy::assertions_on_constants)]
fn test_development_constants() {
    assert_eq!(development::DEFAULT_DEV_ENVIRONMENT, "development");
    assert_eq!(development::DEFAULT_DEV_LOG_LEVEL, "debug");
    assert!(development::DEFAULT_DEV_HOT_RELOAD);
    assert!(development::DEFAULT_DEV_DEBUG_MODE);
}

// ===== Production constants =====
#[test]
#[allow(clippy::assertions_on_constants)]
fn test_production_constants() {
    assert_eq!(production::DEFAULT_PROD_ENVIRONMENT, "production");
    assert_eq!(production::DEFAULT_PROD_LOG_LEVEL, "info");
    assert!(!production::DEFAULT_PROD_HOT_RELOAD);
    assert!(!production::DEFAULT_PROD_DEBUG_MODE);
}

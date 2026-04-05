// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for configuration constants and network utilities

use super::*;

// ===== Network constants =====
#[test]
fn test_network_constants() {
    assert_eq!(network::DEFAULT_REQUEST_TIMEOUT_SECS, 30);
    assert_eq!(network::DEFAULT_CONNECTION_TIMEOUT_SECS, 10);
    assert_eq!(network::DEFAULT_MAX_RETRIES, 3);
    assert_eq!(network::DEFAULT_KEEPALIVE_INTERVAL_SECS, 30);
    assert_eq!(network::DEFAULT_MAX_CONNECTIONS_PER_HOST, 100);
}

#[test]
fn test_default_federation_address() {
    let _addr = network::default_federation_address();
}

#[test]
fn test_get_bind_host_default() {
    temp_env::with_var("BIND_ADDRESS", None::<&str>, || {
        let host = network::get_bind_host();
        assert_eq!(host, "127.0.0.1");
    });
}

// ===== App constants =====
#[test]
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
fn test_testing_constants() {
    assert_eq!(testing::DEFAULT_TEST_TIMEOUT_SECS, 30);
    assert_eq!(testing::DEFAULT_TEST_PORT, 9999);
    assert_eq!(testing::DEFAULT_TEST_ENVIRONMENT, "test");
}

// ===== Development constants =====
#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "compile-time assertion by design"
)]
fn test_development_constants() {
    assert_eq!(development::DEFAULT_DEV_ENVIRONMENT, "development");
    assert_eq!(development::DEFAULT_DEV_LOG_LEVEL, "debug");
    assert!(development::DEFAULT_DEV_HOT_RELOAD);
    assert!(development::DEFAULT_DEV_DEBUG_MODE);
}

// ===== Production constants =====
#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "compile-time assertion by design"
)]
fn test_production_constants() {
    assert_eq!(production::DEFAULT_PROD_ENVIRONMENT, "production");
    assert_eq!(production::DEFAULT_PROD_LOG_LEVEL, "info");
    assert!(!production::DEFAULT_PROD_HOT_RELOAD);
    assert!(!production::DEFAULT_PROD_DEBUG_MODE);
}

//! Extended validation tests for configuration system
//!
//! These tests cover edge cases and additional scenarios for config validation.

use std::time::Duration;
use toadstool_config::validation::*;

#[test]
fn test_validate_port_edge_cases() {
    // Test boundary values
    assert!(
        validate_port(1024, "port").is_ok(),
        "Port 1024 should be valid"
    );
    assert!(
        validate_port(65535, "port").is_ok(),
        "Port 65535 should be valid"
    );

    // Test just below valid range
    assert!(
        validate_port(1023, "port").is_err(),
        "Port 1023 should be invalid"
    );

    // Test privileged ports
    assert!(
        validate_port(1, "port").is_err(),
        "Port 1 should be invalid"
    );
    assert!(
        validate_port(80, "port").is_err(),
        "Port 80 should be invalid"
    );
    assert!(
        validate_port(443, "port").is_err(),
        "Port 443 should be invalid"
    );

    // Test common development ports
    assert!(
        validate_port(3000, "port").is_ok(),
        "Port 3000 should be valid"
    );
    assert!(
        validate_port(8080, "port").is_ok(),
        "Port 8080 should be valid"
    );
    assert!(
        validate_port(9000, "port").is_ok(),
        "Port 9000 should be valid"
    );
}

#[test]
fn test_validate_port_ecosystem_ports() {
    // Test all ecosystem primal default ports are valid
    use toadstool_config::defaults::network;

    assert!(validate_port(network::SONGBIRD_PORT, "songbird").is_ok());
    assert!(validate_port(network::BEARDOG_PORT, "beardog").is_ok());
    assert!(validate_port(network::NESTGATE_PORT, "nestgate").is_ok());
    assert!(validate_port(network::SQUIRREL_PORT, "squirrel").is_ok());
    assert!(validate_port(network::API_PORT, "api").is_ok());
    assert!(validate_port(network::METRICS_PORT, "metrics").is_ok());
}

#[test]
fn test_validate_timeout_boundaries() {
    // Test minimum valid timeout (100ms)
    assert!(validate_timeout(Duration::from_millis(100), "timeout").is_ok());
    assert!(validate_timeout(Duration::from_millis(101), "timeout").is_ok());

    // Test just below minimum
    assert!(validate_timeout(Duration::from_millis(99), "timeout").is_err());
    assert!(validate_timeout(Duration::from_millis(50), "timeout").is_err());
    assert!(validate_timeout(Duration::from_millis(0), "timeout").is_err());

    // Test maximum valid timeout (3600s = 1 hour)
    assert!(validate_timeout(Duration::from_secs(3600), "timeout").is_ok());
    assert!(validate_timeout(Duration::from_secs(3599), "timeout").is_ok());

    // Test just above maximum
    assert!(validate_timeout(Duration::from_secs(3601), "timeout").is_err());
    assert!(validate_timeout(Duration::from_secs(7200), "timeout").is_err());

    // Test common timeout values
    assert!(validate_timeout(Duration::from_secs(30), "timeout").is_ok());
    assert!(validate_timeout(Duration::from_secs(60), "timeout").is_ok());
    assert!(validate_timeout(Duration::from_secs(300), "timeout").is_ok());
}

#[test]
fn test_validate_worker_threads_boundaries() {
    // Test minimum
    assert!(validate_worker_threads(1, "workers").is_ok());
    assert!(validate_worker_threads(0, "workers").is_err());

    // Test maximum
    assert!(validate_worker_threads(128, "workers").is_ok());
    assert!(validate_worker_threads(256, "workers").is_err());
    assert!(validate_worker_threads(512, "workers").is_err());

    // Test common values
    assert!(validate_worker_threads(2, "workers").is_ok());
    assert!(validate_worker_threads(4, "workers").is_ok());
    assert!(validate_worker_threads(8, "workers").is_ok());
    assert!(validate_worker_threads(16, "workers").is_ok());
    assert!(validate_worker_threads(32, "workers").is_ok());
    assert!(validate_worker_threads(64, "workers").is_ok());
}

#[test]
fn test_validate_non_empty_strings() {
    // Valid strings
    assert!(validate_non_empty("hello", "field").is_ok());
    assert!(validate_non_empty("a", "field").is_ok());
    assert!(validate_non_empty("test123", "field").is_ok());
    assert!(validate_non_empty("with spaces", "field").is_ok());

    // Invalid strings
    assert!(validate_non_empty("", "field").is_err());
    assert!(validate_non_empty("   ", "field").is_err());
    assert!(validate_non_empty("\t", "field").is_err());
    assert!(validate_non_empty("\n", "field").is_err());
    assert!(validate_non_empty("  \t  \n  ", "field").is_err());
}

#[test]
fn test_validate_url_formats() {
    // Valid URLs
    assert!(validate_url("http://localhost:8080", "url").is_ok());
    assert!(validate_url("https://example.com", "url").is_ok());
    assert!(validate_url("http://192.168.1.1", "url").is_ok());
    assert!(validate_url("https://example.com:443", "url").is_ok());
    assert!(validate_url("http://example.com/path", "url").is_ok());
    assert!(validate_url("https://example.com/path?query=value", "url").is_ok());

    // Invalid URLs
    assert!(validate_url("ftp://example.com", "url").is_err());
    assert!(validate_url("", "url").is_err());
    assert!(validate_url("not-a-url", "url").is_err());
    assert!(validate_url("://example.com", "url").is_err());
}

#[test]
fn test_validate_pool_size() {
    // Valid pool sizes
    assert!(validate_pool_size(1, "pool").is_ok());
    assert!(validate_pool_size(10, "pool").is_ok());
    assert!(validate_pool_size(100, "pool").is_ok());

    // Invalid pool sizes (if too small or too large)
    assert!(validate_pool_size(0, "pool").is_err());
}

#[test]
fn test_validate_url_http_schemes_only() {
    // Test that validation ensures http/https only
    assert!(validate_url("http://example.com", "url").is_ok());
    assert!(validate_url("https://example.com", "url").is_ok());

    // Other schemes should be rejected
    assert!(validate_url("ftp://example.com", "url").is_err());
    assert!(validate_url("file:///path", "url").is_err());
}

#[test]
fn test_error_messages_contain_field_names() {
    // Verify error messages include the field name for better debugging
    match validate_port(80, "custom_port") {
        Err(e) => assert!(e.to_string().contains("custom_port")),
        Ok(_) => panic!("Expected error for privileged port"),
    }

    match validate_timeout(Duration::from_millis(50), "custom_timeout") {
        Err(e) => assert!(e.to_string().contains("custom_timeout")),
        Ok(_) => panic!("Expected error for short timeout"),
    }

    match validate_non_empty("", "custom_field") {
        Err(e) => assert!(e.to_string().contains("custom_field")),
        Ok(_) => panic!("Expected error for empty string"),
    }
}

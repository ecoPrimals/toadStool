// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(feature = "background-monitors")]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Tests for `ToadStoolServer` core functionality (lib.rs)

use toadstool_server::*;

// ============================================================================
// Server Configuration Tests
// ============================================================================

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();

    // Default bind address comes from env config
    assert!(config.bind_address.contains(':'));
    assert!(config.enable_api);
    assert!(config.enable_cors);
}

#[test]
fn test_server_config_custom_bind_address() {
    let config = ServerConfig::default().bind_address("0.0.0.0:9000");

    assert_eq!(config.bind_address, "0.0.0.0:9000");
}

#[test]
fn test_server_config_enable_flags() {
    let config = ServerConfig::default().enable_api(false);

    assert!(!config.enable_api);
    assert!(config.enable_cors); // Default is true
}

#[test]
fn test_server_config_builder_pattern() {
    let config = ServerConfig::default()
        .bind_address("localhost:8888")
        .enable_api(true)
        .max_concurrent_executions(50);

    assert_eq!(config.bind_address, "localhost:8888");
    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 50);
}

#[test]
fn test_server_config_clone() {
    let config = ServerConfig::default().bind_address("0.0.0.0:7000");
    let cloned = config.clone();

    assert_eq!(config.bind_address, cloned.bind_address);
    assert_eq!(config.enable_api, cloned.enable_api);
}

#[test]
fn test_server_config_debug() {
    let config = ServerConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("ServerConfig"));
    assert!(debug_str.contains("bind_address"));
}

// ============================================================================
// Server Statistics Tests
// ============================================================================

#[test]
fn test_server_statistics_default() {
    let stats = ServerStatistics::default();

    assert_eq!(stats.total_executions, 0);
    assert_eq!(stats.successful_executions, 0);
    assert_eq!(stats.failed_executions, 0);
    assert_eq!(stats.total_requests, 0);
}

#[test]
fn test_server_statistics_increment() {
    let mut stats = ServerStatistics::default();

    stats.total_executions += 1;
    stats.successful_executions += 1;
    stats.total_requests += 1;

    assert_eq!(stats.total_executions, 1);
    assert_eq!(stats.successful_executions, 1);
    assert_eq!(stats.total_requests, 1);
}

#[test]
fn test_server_statistics_success_rate() {
    let stats = ServerStatistics {
        total_executions: 100,
        successful_executions: 95,
        failed_executions: 5,
        ..Default::default()
    };

    assert_eq!(stats.total_executions, 100);
    let success_rate = (stats.successful_executions as f64 / stats.total_executions as f64) * 100.0;
    assert!((success_rate - 95.0).abs() < 0.01);
}

#[test]
fn test_server_statistics_clone() {
    let stats = ServerStatistics {
        total_executions: 10,
        successful_executions: 8,
        failed_executions: 2,
        average_execution_time_ms: 150.0,
        peak_concurrent_executions: 5,
        uptime_seconds: 3600,
        total_requests: 100,
        errors_count: 2,
    };

    let cloned = stats.clone();
    assert_eq!(stats.total_executions, cloned.total_executions);
    assert_eq!(stats.successful_executions, cloned.successful_executions);
}

// ToadStoolServer tests removed — axum HTTP server is songBird's domain.
// Server functionality is now via pure_jsonrpc (see pure_jsonrpc/tests.rs).

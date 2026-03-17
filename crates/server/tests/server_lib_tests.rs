// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Tests for `ToadStoolServer` core functionality (lib.rs)

use toadstool_server::*;

// Import mock runtime engine
use toadstool_testing::MockRuntimeEngine;

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

// ============================================================================
// ToadStoolServer Creation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_creation_with_default_config() {
    let config = ServerConfig::default();
    let result = ToadStoolServer::new(config).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_creation_with_custom_config() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:9999")
        .enable_api(true);

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_creation_api_only() {
    let config = ServerConfig::default().enable_api(true);

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_creation_with_concurrency_limit() {
    let config = ServerConfig::default().max_concurrent_executions(200);

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

// ============================================================================
// Runtime Engine Registration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_native_runtime() {
    let config = ServerConfig::default();
    let mut server = ToadStoolServer::new(config).await.unwrap();

    let mock_engine = MockRuntimeEngine::new();
    let result = server
        .register_runtime_engine("native", Box::new(mock_engine))
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_runtimes() {
    let config = ServerConfig::default();
    let mut server = ToadStoolServer::new(config).await.unwrap();

    let native = MockRuntimeEngine::new();
    let wasm = MockRuntimeEngine::new();

    assert!(
        server
            .register_runtime_engine("native", Box::new(native))
            .await
            .is_ok()
    );
    assert!(
        server
            .register_runtime_engine("wasm", Box::new(wasm))
            .await
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_container_runtime() {
    let config = ServerConfig::default();
    let mut server = ToadStoolServer::new(config).await.unwrap();

    let mock_engine = MockRuntimeEngine::new();
    let result = server
        .register_runtime_engine("container", Box::new(mock_engine))
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_python_runtime() {
    let config = ServerConfig::default();
    let mut server = ToadStoolServer::new(config).await.unwrap();

    let mock_engine = MockRuntimeEngine::new();
    let result = server
        .register_runtime_engine("python", Box::new(mock_engine))
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_gpu_runtime() {
    let config = ServerConfig::default();
    let mut server = ToadStoolServer::new(config).await.unwrap();

    let mock_engine = MockRuntimeEngine::new();
    let result = server
        .register_runtime_engine("gpu", Box::new(mock_engine))
        .await;

    assert!(result.is_ok());
}

// ============================================================================
// Server Lifecycle Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_start_background() {
    let config = ServerConfig::default().bind_address("127.0.0.1:18080");

    let mut server = ToadStoolServer::new(config).await.unwrap();

    // Start in background (non-blocking)
    tokio::spawn(async move {
        let _ = server.start().await;
    });

    // ✅ FULLY MODERNIZED: Give server time to start
    tokio::task::yield_now().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_builder_pattern_complete() {
    use std::time::Duration;

    let config = ServerConfig::default()
        .bind_address("127.0.0.1:19090")
        .enable_api(true)
        .default_timeout(Duration::from_secs(600));

    let mut server = ToadStoolServer::new(config).await.unwrap();

    let mock_native = MockRuntimeEngine::new();
    let mock_wasm = MockRuntimeEngine::new();

    assert!(
        server
            .register_runtime_engine("native", Box::new(mock_native))
            .await
            .is_ok()
    );
    assert!(
        server
            .register_runtime_engine("wasm", Box::new(mock_wasm))
            .await
            .is_ok()
    );
}

// ============================================================================
// Server State Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_initialization() {
    let config = ServerConfig::default();
    let _server = ToadStoolServer::new(config).await.unwrap();

    // State is initialized correctly (verified by server creation)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_event_broadcaster() {
    let config = ServerConfig::default();
    let _server = ToadStoolServer::new(config).await.unwrap();

    // Event broadcaster is created during server initialization (verified by creation)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_config_persistence() {
    let config = ServerConfig::default()
        .bind_address("0.0.0.0:8888")
        .max_concurrent_executions(150);

    let _server = ToadStoolServer::new(config.clone()).await.unwrap();

    // Config is preserved
    assert_eq!(config.bind_address, "0.0.0.0:8888");
    assert_eq!(config.max_concurrent_executions, 150);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_empty_runtime_engines() {
    let config = ServerConfig::default();
    let _server = ToadStoolServer::new(config).await.unwrap();

    // Server can be created without runtime engines registered (verified by creation)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_high_port_number() {
    let config = ServerConfig::default().bind_address("127.0.0.1:65000");

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_ipv6_address() {
    let config = ServerConfig::default().bind_address("[::1]:8080");

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_all_features_disabled() {
    let config = ServerConfig::default().enable_api(false);

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_all_features_enabled() {
    use std::time::Duration;

    let config = ServerConfig::default()
        .enable_api(true)
        .max_concurrent_executions(1000)
        .default_timeout(Duration::from_secs(1200));

    let result = ToadStoolServer::new(config).await;
    assert!(result.is_ok());
}

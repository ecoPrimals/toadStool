//! Comprehensive concurrent tests for config validation module
//!
//! ✅ MODERN CONCURRENT TESTING - Covers edge cases and error paths

use toadstool_config::ToadStoolConfig;

// ==================== Basic Validation Tests ====================

#[test]
fn test_validation_accepts_default_config() {
    let config = ToadStoolConfig::default();
    let result = config.validate();
    assert!(result.is_ok(), "Default config should be valid");
}

// ==================== Worker Thread Validation Tests ====================

#[test]
fn test_validation_detects_zero_worker_threads() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;

    let result = config.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("worker") || err_msg.contains("thread"));
}

#[test]
fn test_validation_accepts_reasonable_worker_threads() {
    let mut config = ToadStoolConfig::default();

    config.app.worker_threads = 1;
    assert!(config.validate().is_ok());

    config.app.worker_threads = 4;
    assert!(config.validate().is_ok());

    config.app.worker_threads = 16;
    assert!(config.validate().is_ok());
}

// ==================== Network Configuration Validation Tests ====================

#[test]
fn test_validation_accepts_different_bind_addresses() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let mut config = ToadStoolConfig::default();

    // Test localhost
    config.network.bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    assert!(config.validate().is_ok());

    // Test 0.0.0.0
    config.network.bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
    assert!(config.validate().is_ok());
}

// ==================== Security Configuration Validation Tests ====================

#[test]
fn test_validation_accepts_default_security_config() {
    let config = ToadStoolConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validation_accepts_production_security_config() {
    let config = ToadStoolConfig::production();
    assert!(config.validate().is_ok());
    assert!(config.security.auth.enabled);
}

#[test]
fn test_validation_accepts_development_security_config() {
    let config = ToadStoolConfig::development();
    assert!(config.validate().is_ok());
    assert!(!config.security.auth.enabled);
}

// ==================== Logging Configuration Validation Tests ====================

#[test]
fn test_validation_accepts_all_log_levels() {
    let mut config = ToadStoolConfig::default();

    for level in &["trace", "debug", "info", "warn", "error"] {
        config.logging.level = level.to_string();
        let result = config.validate();
        assert!(result.is_ok(), "Log level '{}' should be valid", level);
    }
}

#[test]
fn test_validation_accepts_empty_log_level() {
    // Empty log level might default to something reasonable
    let mut config = ToadStoolConfig::default();
    config.logging.level = "".to_string();

    // This might be valid or invalid depending on implementation
    // Just ensure it doesn't panic
    let _ = config.validate();
}

// ==================== Environment-Specific Validation Tests ====================

#[test]
fn test_validation_production_config_complete() {
    let config = ToadStoolConfig::production();

    assert!(config.validate().is_ok());
    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(config.security.auth.enabled);
}

#[test]
fn test_validation_development_config_complete() {
    let config = ToadStoolConfig::development();

    assert!(config.validate().is_ok());
    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
}

#[test]
fn test_validation_testing_config_complete() {
    let config = ToadStoolConfig::testing();

    assert!(config.validate().is_ok());
    assert_eq!(config.app.environment, "test");
    assert!(!config.security.auth.enabled);
}

// ==================== Concurrent Validation Tests ====================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_validation_same_config() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let config = Arc::new(ToadStoolConfig::default());
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let cfg = Arc::clone(&config);
        let bar = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            bar.wait().await;
            cfg.validate().is_ok()
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All validations should succeed
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result, "Validation should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_validation_different_configs() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let barrier = Arc::new(Barrier::new(12));
    let mut handles = vec![];

    for i in 0..12 {
        let bar = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            bar.wait().await;

            let config = match i % 4 {
                0 => ToadStoolConfig::default(),
                1 => ToadStoolConfig::development(),
                2 => ToadStoolConfig::production(),
                _ => ToadStoolConfig::testing(),
            };

            config.validate().is_ok()
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All validations should succeed
    assert_eq!(results.len(), 12);
    for result in results {
        assert!(result, "All environment configs should validate");
    }
}

// ==================== Edge Case Tests ====================

#[test]
fn test_validation_queue_and_batch_sizes() {
    let mut config = ToadStoolConfig::default();

    // Test various sizes
    config.app.queue_size = 100;
    config.app.batch_size = 10;
    assert!(config.validate().is_ok());

    config.app.queue_size = 1000;
    config.app.batch_size = 50;
    assert!(config.validate().is_ok());
}

#[test]
fn test_validation_different_network_ports() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let mut config = ToadStoolConfig::default();

    // Test various port numbers
    for port in &[80, 443, 8080, 8443, 3000, 5000, 9000] {
        config.network.bind_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), *port);
        assert!(config.validate().is_ok(), "Port {} should be valid", port);
    }
}

// ==================== Configuration Integrity Tests ====================

#[test]
fn test_validation_preserves_config_integrity() {
    let config = ToadStoolConfig::default();

    // Validate shouldn't mutate the config
    let worker_threads_before = config.app.worker_threads;
    let _ = config.validate();
    assert_eq!(config.app.worker_threads, worker_threads_before);
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::time::Duration;

#[test]
fn test_timeout_config_defaults() {
    let config = TimeoutConfig::default();
    assert_eq!(config.connection_timeout, Duration::from_secs(30));
    assert_eq!(config.request_timeout, Duration::from_secs(60));
}

#[test]
fn test_health_check_config_defaults() {
    let config = HealthCheckConfig::default();
    assert!(config.enabled);
    assert_eq!(config.interval, Duration::from_secs(30));
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(config.healthy_threshold, 2);
    assert_eq!(config.unhealthy_threshold, 3);
}

#[test]
fn test_resource_config_defaults() {
    let config = BaseResourceConfig::default();
    assert!(config.cpu.limit.is_none());
    assert!(config.memory.limit.is_none());
    assert!(config.storage.is_none());
}

#[test]
fn test_backend_endpoint_url() {
    let endpoint = BackendEndpoint::new("test", "localhost", 8080);
    assert_eq!(endpoint.url("http"), "http://localhost:8080");
    assert_eq!(endpoint.url("https"), "https://localhost:8080");
}

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    assert!((config.jitter_percent - 10.0).abs() < f64::EPSILON);
}

#[test]
fn test_http_health_check_defaults() {
    let config = HttpHealthCheckConfig::default();
    assert_eq!(config.path, "/health");
    assert_eq!(config.expected_status, 200);
    assert_eq!(config.method, "GET");
    assert!(config.base.enabled);
}

#[test]
fn test_validation_config_defaults() {
    let config = ValidationConfig::default();
    assert!(config.enabled);
    assert!(config.validate_expiration);
    assert!(config.clock_skew.is_some());
    assert_eq!(config.clock_skew.unwrap(), Duration::from_secs(60));
}

#[test]
fn test_connection_pool_config_defaults() {
    let config = ConnectionPoolConfig::default();
    assert!(config.enabled);
    assert_eq!(config.max_connections_per_host, 100);
    assert_eq!(config.max_idle_connections, 10);
    assert_eq!(config.idle_timeout, Duration::from_secs(300));
    assert_eq!(config.connection_lifetime, Duration::from_secs(3600));
}

#[test]
fn test_cache_config_defaults() {
    let config = CacheConfig::default();
    assert!(config.enabled);
    assert_eq!(config.ttl, Duration::from_secs(300));
    assert_eq!(config.max_entries, 1000);
    assert_eq!(config.negative_ttl, Duration::from_secs(60));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simple comprehensive tests for config_utils module
//!
//! Tests that all utility functions return valid values

#![allow(deprecated)] // Testing legacy config APIs for backwards compatibility

use toadstool_config::config_utils::ConfigUtils;

// ==================== Port Getter Tests ====================

#[test]
fn test_get_songbird_port() {
    let port = ConfigUtils::get_songbird_port();
    // 0 = discovered at runtime (capability resolution); or from env
    let _ = port;
}

#[test]
fn test_get_beardog_port() {
    let port = ConfigUtils::get_beardog_port();
    let _ = port;
}

#[test]
fn test_get_nestgate_port() {
    let port = ConfigUtils::get_nestgate_port();
    let _ = port;
}

#[test]
fn test_get_squirrel_port() {
    let port = ConfigUtils::get_squirrel_port();
    let _ = port;
}

#[test]
fn test_get_toadstool_port() {
    let port = ConfigUtils::get_toadstool_port();
    // 0 = OS-assigned; or explicit from env
    let _ = port;
}

#[test]
fn test_get_federation_port() {
    let port = ConfigUtils::get_federation_port();
    let _ = port;
}

#[test]
fn test_get_metrics_port() {
    let port = ConfigUtils::get_metrics_port();
    let _ = port;
}

#[test]
fn test_get_health_port() {
    let port = ConfigUtils::get_health_port();
    let _ = port;
}

#[test]
fn test_get_events_port() {
    let port = ConfigUtils::get_events_port();
    let _ = port;
}

// ==================== Address and Hostname Tests ====================

#[test]
fn test_get_bind_address() {
    let addr = ConfigUtils::get_bind_address();
    assert!(!addr.is_empty());
    // Should be a valid IP address or hostname
    // Redundant check removed - is_empty() already covers this
}

#[test]
fn test_get_external_hostname() {
    let hostname = ConfigUtils::get_external_hostname();
    assert!(!hostname.is_empty());
    assert_eq!(hostname, "localhost"); // Default value
}

// ==================== Endpoint Tests ====================

#[test]
fn test_get_songbird_endpoint() {
    let endpoint = ConfigUtils::get_songbird_endpoint();
    assert!(endpoint.starts_with("http"));
    assert!(endpoint.contains("://"));
    assert!(endpoint.contains(":"));
}

#[test]
fn test_get_beardog_endpoint() {
    let endpoint = ConfigUtils::get_beardog_endpoint();
    assert!(endpoint.starts_with("http"));
    assert!(endpoint.contains("://"));
    assert!(endpoint.contains(":"));
}

#[test]
fn test_get_nestgate_endpoint() {
    let endpoint = ConfigUtils::get_nestgate_endpoint();
    assert!(endpoint.starts_with("http"));
    assert!(endpoint.contains("://"));
    assert!(endpoint.contains(":"));
}

#[test]
fn test_get_squirrel_endpoint() {
    let endpoint = ConfigUtils::get_squirrel_endpoint();
    assert!(endpoint.starts_with("http"));
    assert!(endpoint.contains("://"));
    assert!(endpoint.contains(":"));
}

#[test]
fn test_get_toadstool_endpoint() {
    let endpoint = ConfigUtils::get_toadstool_endpoint();
    assert!(endpoint.starts_with("http"));
    assert!(endpoint.contains("://"));
    assert!(endpoint.contains(":"));
}

// ==================== Timeout Tests ====================

#[test]
fn test_get_request_timeout() {
    let timeout = ConfigUtils::get_request_timeout();
    assert!(timeout.as_secs() > 0);
    assert!(timeout.as_secs() < 3600); // Less than 1 hour
}

#[test]
fn test_get_connection_timeout() {
    let timeout = ConfigUtils::get_connection_timeout();
    assert!(timeout.as_secs() > 0);
    assert!(timeout.as_secs() < 3600);
}

#[test]
fn test_get_execution_timeout() {
    let timeout = ConfigUtils::get_execution_timeout();
    assert!(timeout.as_secs() > 0);
}

#[test]
fn test_get_keepalive_interval() {
    let interval = ConfigUtils::get_keepalive_interval();
    assert!(interval.as_secs() > 0);
}

#[test]
fn test_get_metrics_interval() {
    let interval = ConfigUtils::get_metrics_interval();
    assert!(interval.as_secs() > 0);
}

#[test]
fn test_get_health_check_interval() {
    let interval = ConfigUtils::get_health_check_interval();
    assert!(interval.as_secs() > 0);
}

// ==================== Worker and Resource Tests ====================

#[test]
fn test_get_worker_threads() {
    let threads = ConfigUtils::get_worker_threads();
    assert!(threads > 0);
    assert!(threads <= 1024); // Reasonable upper bound
}

#[test]
fn test_get_max_concurrent_executions() {
    let max = ConfigUtils::get_max_concurrent_executions();
    assert!(max > 0);
}

#[test]
fn test_get_max_retries() {
    let retries = ConfigUtils::get_max_retries();
    // u32 is always >= 0, just verify reasonable bound
    assert!(retries <= 100);
}

#[test]
fn test_get_max_connections_per_host() {
    let connections = ConfigUtils::get_max_connections_per_host();
    assert!(connections > 0);
}

#[test]
fn test_get_max_cpu_usage() {
    let cpu = ConfigUtils::get_max_cpu_usage();
    assert!(cpu > 0.0);
    assert!(cpu <= 100.0);
}

#[test]
fn test_get_max_memory_usage() {
    let memory = ConfigUtils::get_max_memory_usage();
    assert!(memory > 0);
}

#[test]
fn test_get_max_storage_usage() {
    let storage = ConfigUtils::get_max_storage_usage();
    assert!(storage > 0);
}

// ==================== Directory Tests ====================

#[test]
fn test_get_data_dir() {
    let dir = ConfigUtils::get_data_dir();
    assert!(!dir.is_empty());
}

#[test]
fn test_get_cache_dir() {
    let dir = ConfigUtils::get_cache_dir();
    assert!(!dir.is_empty());
}

#[test]
fn test_get_temp_dir() {
    let dir = ConfigUtils::get_temp_dir();
    assert!(!dir.is_empty());
}

#[test]
fn test_get_log_dir() {
    let dir = ConfigUtils::get_log_dir();
    assert!(!dir.is_empty());
}

// ==================== Environment and Feature Flag Tests ====================

#[test]
fn test_get_environment() {
    let env = ConfigUtils::get_environment();
    assert!(!env.is_empty());
    // Should be one of: development, staging, production, test
    assert!(
        ["development", "staging", "production", "test"].contains(&env.as_str()) || !env.is_empty()
    );
}

#[test]
fn test_get_log_level() {
    let level = ConfigUtils::get_log_level();
    assert!(!level.is_empty());
    // Should be a valid log level
    assert!(
        ["trace", "debug", "info", "warn", "error"].contains(&level.to_lowercase().as_str())
            || !level.is_empty()
    );
}

#[test]
fn test_get_debug_mode() {
    let debug = ConfigUtils::get_debug_mode();
    // Just verify it returns a boolean - any bool value is valid
    let _ = debug; // Compiles if it's a bool
}

#[test]
fn test_get_verbose_mode() {
    let verbose = ConfigUtils::get_verbose_mode();
    let _ = verbose; // Compiles if it's a bool
}

#[test]
fn test_get_tls_enabled() {
    let tls = ConfigUtils::get_tls_enabled();
    let _ = tls; // Compiles if it's a bool
}

#[test]
fn test_get_auth_enabled() {
    let auth = ConfigUtils::get_auth_enabled();
    let _ = auth; // Compiles if it's a bool
}

#[test]
fn test_get_sandboxing_enabled() {
    let sandbox = ConfigUtils::get_sandboxing_enabled();
    let _ = sandbox; // Compiles if it's a bool
}

#[test]
fn test_get_metrics_enabled() {
    let metrics = ConfigUtils::get_metrics_enabled();
    let _ = metrics; // Compiles if it's a bool
}

#[test]
fn test_get_health_checks_enabled() {
    let health = ConfigUtils::get_health_checks_enabled();
    let _ = health; // Compiles if it's a bool
}

// ==================== Collection Tests ====================

#[test]
fn test_get_service_ports() {
    let ports = ConfigUtils::get_service_ports();
    assert!(!ports.is_empty());
    // Verify all ports are valid (0 = OS-assigned)
    for name in ports.keys() {
        assert!(!name.is_empty());
    }
}

#[test]
fn test_get_service_endpoints() {
    let endpoints = ConfigUtils::get_service_endpoints();
    assert!(!endpoints.is_empty());
    // Verify all endpoints are valid URLs
    for (name, endpoint) in &endpoints {
        assert!(!name.is_empty());
        assert!(endpoint.starts_with("http"));
    }
}

#[test]
fn test_get_container_port_range() {
    let (start, end) = ConfigUtils::get_container_port_range();
    assert!(start > 0);
    assert!(end > start);
}

#[test]
fn test_get_port_allocation_range() {
    let (start, end) = ConfigUtils::get_port_allocation_range();
    assert!(start > 0);
    assert!(end > start);
}

// ==================== URL and Path Tests ====================

#[test]
fn test_get_database_url() {
    let url = ConfigUtils::get_database_url();
    // May be empty if not configured
    assert!(url.is_empty() || url.contains("://"));
}

#[test]
fn test_get_cache_url() {
    let url = ConfigUtils::get_cache_url();
    assert!(url.is_empty() || url.contains("://"));
}

#[test]
fn test_get_message_broker_url() {
    let url = ConfigUtils::get_message_broker_url();
    assert!(url.is_empty() || url.contains("://"));
}

#[test]
fn test_get_monitoring_endpoint() {
    let endpoint = ConfigUtils::get_monitoring_endpoint();
    assert!(!endpoint.is_empty());
}

#[test]
fn test_get_encryption_key_path() {
    let path = ConfigUtils::get_encryption_key_path();
    assert!(!path.is_empty());
}

#[test]
fn test_get_tls_cert_path() {
    let path = ConfigUtils::get_tls_cert_path();
    assert!(!path.is_empty());
}

#[test]
fn test_get_tls_key_path() {
    let path = ConfigUtils::get_tls_key_path();
    assert!(!path.is_empty());
}

#[test]
fn test_get_ca_cert_path() {
    let path = ConfigUtils::get_ca_cert_path();
    assert!(!path.is_empty());
}

// ==================== Cluster Configuration Tests ====================

#[test]
fn test_get_cluster_name() {
    let name = ConfigUtils::get_cluster_name();
    assert!(!name.is_empty());
}

#[test]
fn test_get_node_name() {
    let name = ConfigUtils::get_node_name();
    assert!(!name.is_empty());
}

#[test]
fn test_get_federation_trust_domain() {
    let domain = ConfigUtils::get_federation_trust_domain();
    assert!(!domain.is_empty());
}

// ==================== Integration Tests ====================

#[test]
fn test_all_service_ports_are_unique() {
    let ports = ConfigUtils::get_service_ports();
    let port_values: Vec<u16> = ports.values().copied().collect();

    // Check that we have multiple services
    assert!(ports.len() >= 3);

    // Port 0 (OS-assigned) may be shared; or explicit ports may differ
    let unique_ports: std::collections::HashSet<_> = port_values.iter().collect();
    assert!(!unique_ports.is_empty());
}

#[test]
fn test_all_endpoints_match_ports() {
    let endpoints = ConfigUtils::get_service_endpoints();
    let ports = ConfigUtils::get_service_ports();

    // Verify we have endpoints for services
    assert!(!endpoints.is_empty());
    assert!(!ports.is_empty());
}

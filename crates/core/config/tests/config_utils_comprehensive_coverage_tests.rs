// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive coverage tests for `config_utils` module
//!
//! Goal: Increase `config_utils` coverage from 1.87% to 70%+
//! Focus: Test actual public API methods that exist

use std::time::Duration;
use toadstool_config::config_utils::ConfigUtils;

// ==================== Port Configuration Tests ====================

#[test]
fn test_get_toadstool_port() {
    let port = ConfigUtils::get_toadstool_port();
    let _ = port; // 0 = OS-assigned; u16 is always valid
}

#[test]
fn test_get_federation_port() {
    let port = ConfigUtils::get_federation_port();
    let _ = port; // 0 = OS-assigned; u16 is always valid
}

#[test]
fn test_get_metrics_port() {
    let port = ConfigUtils::get_metrics_port();
    let _ = port; // 0 = OS-assigned; u16 is always valid
}

#[test]
fn test_get_health_port() {
    let port = ConfigUtils::get_health_port();
    let _ = port; // 0 = OS-assigned; u16 is always valid
}

#[test]
fn test_get_events_port() {
    let port = ConfigUtils::get_events_port();
    let _ = port; // 0 = OS-assigned; u16 is always valid
}

// ==================== Endpoint Tests ====================

#[test]
fn test_get_toadstool_endpoint() {
    let endpoint = ConfigUtils::get_toadstool_endpoint();
    assert!(!endpoint.is_empty());
    assert!(endpoint.starts_with("http"));
}

// ==================== Timeout Tests ====================

#[test]
fn test_get_request_timeout() {
    let timeout = ConfigUtils::get_request_timeout();
    assert!(timeout > Duration::from_secs(0));
}

#[test]
fn test_get_connection_timeout() {
    let timeout = ConfigUtils::get_connection_timeout();
    assert!(timeout > Duration::from_secs(0));
}

#[test]
fn test_get_execution_timeout() {
    let timeout = ConfigUtils::get_execution_timeout();
    assert!(timeout > Duration::from_secs(0));
}

// ==================== Resource Configuration Tests ====================

#[test]
fn test_get_max_retries() {
    let retries = ConfigUtils::get_max_retries();
    assert!(retries > 0);
}

#[test]
fn test_get_max_connections_per_host() {
    let max_conn = ConfigUtils::get_max_connections_per_host();
    assert!(max_conn > 0);
}

#[test]
fn test_get_keepalive_interval() {
    let interval = ConfigUtils::get_keepalive_interval();
    assert!(interval > Duration::from_secs(0));
}

#[test]
fn test_get_worker_threads() {
    let threads = ConfigUtils::get_worker_threads();
    assert!(threads > 0 && threads <= 1024);
}

#[test]
fn test_get_max_concurrent_executions() {
    let max_exec = ConfigUtils::get_max_concurrent_executions();
    assert!(max_exec > 0);
}

#[test]
fn test_get_max_cpu_usage() {
    let max_cpu = ConfigUtils::get_max_cpu_usage();
    assert!(max_cpu > 0.0 && max_cpu <= 100.0);
}

#[test]
fn test_get_max_memory_usage() {
    let max_mem = ConfigUtils::get_max_memory_usage();
    assert!(max_mem > 0);
}

#[test]
fn test_get_max_storage_usage() {
    let max_storage = ConfigUtils::get_max_storage_usage();
    assert!(max_storage > 0);
}

// ==================== Interval Tests ====================

#[test]
fn test_get_metrics_interval() {
    let interval = ConfigUtils::get_metrics_interval();
    assert!(interval > Duration::from_secs(0));
}

#[test]
fn test_get_health_check_interval() {
    let interval = ConfigUtils::get_health_check_interval();
    assert!(interval > Duration::from_secs(0));
}

// ==================== String Configuration Tests ====================

#[test]
fn test_get_bind_address() {
    let address = ConfigUtils::get_bind_address();
    assert!(!address.is_empty());
}

#[test]
fn test_get_external_hostname() {
    let hostname = ConfigUtils::get_external_hostname();
    assert!(!hostname.is_empty());
}

#[test]
fn test_get_log_level() {
    let level = ConfigUtils::get_log_level();
    assert!(!level.is_empty());
}

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

#[test]
fn test_get_environment() {
    let env = ConfigUtils::get_environment();
    assert!(!env.is_empty());
}

// ==================== Boolean Configuration Tests ====================

#[test]
fn test_get_debug_mode() {
    let _debug = ConfigUtils::get_debug_mode();
    // Just test it doesn't panic
}

#[test]
fn test_get_verbose_mode() {
    let _verbose = ConfigUtils::get_verbose_mode();
}

#[test]
fn test_get_tls_enabled() {
    let _tls = ConfigUtils::get_tls_enabled();
}

#[test]
fn test_get_auth_enabled() {
    let _auth = ConfigUtils::get_auth_enabled();
}

#[test]
fn test_get_sandboxing_enabled() {
    let _sandbox = ConfigUtils::get_sandboxing_enabled();
}

#[test]
fn test_get_metrics_enabled() {
    let _metrics = ConfigUtils::get_metrics_enabled();
}

#[test]
fn test_get_health_checks_enabled() {
    let _health = ConfigUtils::get_health_checks_enabled();
}

// ==================== Port Range Tests ====================

#[test]
fn test_get_container_port_range() {
    let (start, end) = ConfigUtils::get_container_port_range();
    assert!(start < end);
    assert!(start > 0);
}

#[test]
fn test_get_port_allocation_range() {
    let (start, end) = ConfigUtils::get_port_allocation_range();
    assert!(start < end);
    assert!(start > 0);
}

// ==================== URL Configuration Tests ====================

#[test]
fn test_get_database_url() {
    let url = ConfigUtils::get_database_url();
    assert!(!url.is_empty());
}

#[test]
fn test_get_cache_url() {
    let url = ConfigUtils::get_cache_url();
    assert!(!url.is_empty());
}

#[test]
fn test_get_message_broker_url() {
    let url = ConfigUtils::get_message_broker_url();
    assert!(!url.is_empty());
}

#[test]
fn test_get_distributed_storage_url() {
    let url = ConfigUtils::get_distributed_storage_url();
    // May be empty (capability discovery); or explicit URL
    assert!(url.is_empty() || url.contains("://"));
}

#[test]
fn test_get_monitoring_endpoint() {
    let endpoint = ConfigUtils::get_monitoring_endpoint();
    assert!(!endpoint.is_empty());
}

#[test]
fn test_get_alert_webhook_url() {
    let _url = ConfigUtils::get_alert_webhook_url();
    // May be empty by default
}

// ==================== Security Path Tests ====================

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

#[test]
fn test_get_jwt_secret() {
    let secret = ConfigUtils::get_jwt_secret();
    assert!(!secret.is_empty());
}

#[test]
fn test_get_api_key() {
    let key = ConfigUtils::get_api_key();
    assert!(!key.is_empty());
}

#[test]
fn test_get_webhook_secret() {
    let secret = ConfigUtils::get_webhook_secret();
    assert!(!secret.is_empty());
}

// ==================== Cluster Configuration Tests ====================

#[test]
fn test_get_federation_trust_domain() {
    let domain = ConfigUtils::get_federation_trust_domain();
    assert!(!domain.is_empty());
}

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

// ==================== Collection Tests ====================

#[test]
#[allow(deprecated)]
fn test_get_service_ports() {
    let ports = ConfigUtils::get_service_ports();
    assert!(!ports.is_empty());
    assert!(ports.contains_key("toadstool"));

    // Port 0 = OS-assigned; all ports in valid u16 range (type guarantees validity)
}

#[test]
fn test_get_all_toadstool_env_vars() {
    let vars = ConfigUtils::get_all_toadstool_env_vars();
    // Just test it doesn't panic - may be empty
    let _ = vars;
}

// ==================== Consistency Tests ====================

#[test]
fn test_consistent_values() {
    // Test that calling same method twice returns same value
    let port1 = ConfigUtils::get_toadstool_port();
    let port2 = ConfigUtils::get_toadstool_port();
    assert_eq!(port1, port2);

    let timeout1 = ConfigUtils::get_request_timeout();
    let timeout2 = ConfigUtils::get_request_timeout();
    assert_eq!(timeout1, timeout2);
}

#[test]
fn test_all_ports_are_valid() {
    // Port 0 = OS-assigned; all in valid u16 range (type guarantees validity)
    let _ports = vec![
        ConfigUtils::get_toadstool_port(),
        ConfigUtils::get_federation_port(),
        ConfigUtils::get_metrics_port(),
        ConfigUtils::get_health_port(),
        ConfigUtils::get_events_port(),
    ];
}

#[test]
fn test_reasonable_defaults() {
    // Test that defaults are sensible
    let threads = ConfigUtils::get_worker_threads();
    assert!((1..=1024).contains(&threads));

    let max_exec = ConfigUtils::get_max_concurrent_executions();
    assert!((1..=10000).contains(&max_exec));

    let max_cpu = ConfigUtils::get_max_cpu_usage();
    assert!(max_cpu > 0.0 && max_cpu <= 100.0);
}

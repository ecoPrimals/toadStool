// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[test]
fn test_config_utils() {
    temp_env::with_vars(
        [
            ("SONGBIRD_PORT", Some("8080")),
            ("BEARDOG_PORT", Some("8081")),
            ("NESTGATE_PORT", Some("8082")),
            ("BIND_ADDRESS", Some("127.0.0.1")),
            ("TOADSTOOL_ENV", Some("development")),
            ("TOADSTOOL_DEBUG", Some("false")),
        ],
        || {
            assert_eq!(ConfigUtils::get_songbird_port(), 8080);
            assert_eq!(ConfigUtils::get_beardog_port(), 8081);
            assert_eq!(ConfigUtils::get_nestgate_port(), 8082);
            assert_eq!(ConfigUtils::get_bind_address(), "127.0.0.1");
            assert_eq!(ConfigUtils::get_environment(), "development");
            assert!(!ConfigUtils::get_debug_mode());

            temp_env::with_vars(
                [
                    ("SONGBIRD_PORT", Some("9080")),
                    ("TOADSTOOL_DEBUG", Some("true")),
                ],
                || {
                    assert_eq!(ConfigUtils::get_songbird_port(), 9080);
                    assert!(ConfigUtils::get_debug_mode());
                },
            );
        },
    );
}

#[test]
fn test_service_ports() {
    use toadstool_common::constants::primal_identity::PRIMAL_NAME;

    let ports = ConfigUtils::get_service_ports();
    // Self-knowledge only: toadstool + federation/metrics/health/events
    assert!(ports.contains_key(PRIMAL_NAME));
    assert!(ports.contains_key("federation"));
    assert!(ports.contains_key("metrics"));
    assert!(ports.contains_key("health"));
    assert!(ports.contains_key("events"));
}

#[test]
fn test_service_endpoints() {
    use toadstool_common::constants::primal_identity::PRIMAL_NAME;

    let endpoints = ConfigUtils::get_service_endpoints();
    // Self-knowledge only: ToadStool's own endpoint
    assert!(endpoints.contains_key(PRIMAL_NAME));

    // Check endpoint format
    for endpoint in endpoints.values() {
        assert!(endpoint.starts_with("http://"));
    }
}

#[test]
fn test_port_ranges() {
    let (start, end) = ConfigUtils::get_container_port_range();
    assert!(start < end);
    assert!(start >= 3000);
    assert!(end <= 3999);

    let (start, end) = ConfigUtils::get_port_allocation_range();
    assert!(start < end);
    assert!(start >= 8000); // Updated to match new default
    assert!(end <= 8999);
}

#[test]
fn test_get_federation_metrics_health_events_ports() {
    let _fed = ConfigUtils::get_federation_port();
    let _metrics = ConfigUtils::get_metrics_port();
    let _health = ConfigUtils::get_health_port();
    let _events = ConfigUtils::get_events_port();
}

#[test]
fn test_get_external_hostname_default() {
    temp_env::with_var("TOADSTOOL_EXTERNAL_HOSTNAME", None::<&str>, || {
        let host = ConfigUtils::get_external_hostname();
        assert!(!host.is_empty());
    });
}

#[test]
fn test_get_request_connection_timeout() {
    let req = ConfigUtils::get_request_timeout();
    let conn = ConfigUtils::get_connection_timeout();
    assert!(req.as_secs() > 0);
    assert!(conn.as_secs() > 0);
}

#[test]
fn test_get_max_retries_and_connections() {
    let retries = ConfigUtils::get_max_retries();
    let conn_per_host = ConfigUtils::get_max_connections_per_host();
    assert!(retries > 0);
    assert!(conn_per_host > 0);
}

#[test]
fn test_get_keepalive_interval() {
    let interval = ConfigUtils::get_keepalive_interval();
    assert!(interval.as_secs() > 0);
}

#[test]
fn test_get_worker_threads() {
    let threads = ConfigUtils::get_worker_threads();
    assert!(threads > 0);
}

#[test]
fn test_get_max_concurrent_executions() {
    let max = ConfigUtils::get_max_concurrent_executions();
    assert!(max > 0);
}

#[test]
fn test_get_execution_timeout() {
    let timeout = ConfigUtils::get_execution_timeout();
    assert!(timeout.as_secs() > 0);
}

#[test]
fn test_get_max_cpu_memory_storage_usage() {
    let cpu = ConfigUtils::get_max_cpu_usage();
    let mem = ConfigUtils::get_max_memory_usage();
    let storage = ConfigUtils::get_max_storage_usage();
    assert!(cpu > 0.0);
    assert!(mem > 0);
    assert!(storage > 0);
}

#[test]
fn test_get_metrics_health_check_intervals() {
    let metrics = ConfigUtils::get_metrics_interval();
    let health = ConfigUtils::get_health_check_interval();
    assert!(metrics.as_secs() > 0);
    assert!(health.as_secs() > 0);
}

#[test]
fn test_get_log_level_data_cache_temp_log_dirs() {
    let level = ConfigUtils::get_log_level();
    let data = ConfigUtils::get_data_dir();
    let cache = ConfigUtils::get_cache_dir();
    let temp = ConfigUtils::get_temp_dir();
    let log = ConfigUtils::get_log_dir();
    assert!(!level.is_empty());
    assert!(!data.is_empty());
    assert!(!cache.is_empty());
    assert!(!temp.is_empty());
    assert!(!log.is_empty());
}

#[test]
fn test_get_environment_debug_verbose() {
    let env_name = ConfigUtils::get_environment();
    let _debug = ConfigUtils::get_debug_mode();
    let _verbose = ConfigUtils::get_verbose_mode();
    assert!(!env_name.is_empty());
}

#[test]
fn test_get_tls_auth_sandbox_flags() {
    let _tls = ConfigUtils::get_tls_enabled();
    let _auth = ConfigUtils::get_auth_enabled();
    let _sandbox = ConfigUtils::get_sandboxing_enabled();
    let metrics = ConfigUtils::get_metrics_enabled();
    let health = ConfigUtils::get_health_checks_enabled();
    // Exercise getters; both return valid bools
    let _ = (metrics, health);
}

#[test]
fn test_get_container_port_allocation_range() {
    let (c_start, c_end) = ConfigUtils::get_container_port_range();
    assert!(c_start < c_end);
    let (p_start, p_end) = ConfigUtils::get_port_allocation_range();
    assert!(p_start < p_end);
}

#[test]
fn test_get_database_cache_message_broker_urls() {
    let db = ConfigUtils::get_database_url();
    let cache = ConfigUtils::get_cache_url();
    let broker = ConfigUtils::get_message_broker_url();
    assert!(db.contains("sqlite") || db.contains("postgres") || db.contains("mysql"));
    assert!(!cache.is_empty());
    assert!(!broker.is_empty());
}

#[test]
fn test_get_distributed_storage_monitoring_urls() {
    let _storage = ConfigUtils::get_distributed_storage_url();
    let monitoring = ConfigUtils::get_monitoring_endpoint();
    // Storage may be empty (capability discovery); monitoring has URL format
    assert!(monitoring.starts_with("http://") || !monitoring.is_empty());
}

#[test]
fn test_get_alert_webhook_encryption_paths() {
    let _webhook = ConfigUtils::get_alert_webhook_url();
    let enc_path = ConfigUtils::get_encryption_key_path();
    let tls_cert = ConfigUtils::get_tls_cert_path();
    let tls_key = ConfigUtils::get_tls_key_path();
    let ca_cert = ConfigUtils::get_ca_cert_path();
    assert!(!enc_path.is_empty());
    assert!(!tls_cert.is_empty());
    assert!(!tls_key.is_empty());
    assert!(!ca_cert.is_empty());
}

#[test]
fn test_get_jwt_api_webhook_secrets() {
    let jwt = ConfigUtils::get_jwt_secret();
    let api = ConfigUtils::get_api_key();
    let webhook = ConfigUtils::get_webhook_secret();
    assert!(!jwt.is_empty());
    assert!(!api.is_empty());
    assert!(!webhook.is_empty());
}

#[test]
fn test_get_federation_trust_domain_cluster_node_name() {
    let trust = ConfigUtils::get_federation_trust_domain();
    let cluster = ConfigUtils::get_cluster_name();
    let node = ConfigUtils::get_node_name();
    assert!(!trust.is_empty());
    assert!(!cluster.is_empty());
    assert!(!node.is_empty());
}

#[test]
fn test_get_all_toadstool_env_vars() {
    let vars = ConfigUtils::get_all_toadstool_env_vars();
    for k in vars.keys() {
        assert!(
            k.starts_with("TOADSTOOL_"),
            "key {k} should have TOADSTOOL_ prefix"
        );
    }
}

#[test]
fn test_get_toadstool_endpoint() {
    let endpoint = ConfigUtils::get_toadstool_endpoint();
    assert!(endpoint.starts_with("http://"));
    assert!(endpoint.contains(':'));
}

#[test]
fn test_get_squirrel_port_default() {
    temp_env::with_var("SQUIRREL_PORT", None::<&str>, || {
        let port = ConfigUtils::get_squirrel_port();
        assert_eq!(
            port, 8083,
            "DEFAULT_SQUIRREL_DISCOVERY_PORT = 8083 for cold-start bootstrap"
        );
    });
}

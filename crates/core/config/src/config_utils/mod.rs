// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration Utilities
//!
//! This module provides utility functions to replace hardcoded values with
//! environment-aware configuration throughout the `ToadStool` codebase.
//!
//! ## Module structure
//!
//! - `paths` — Config file paths, directory resolution, XDG compliance
//! - `network` — Port constants, network configuration helpers
//! - `environment` — Environment variable parsing, overrides
//! - `defaults` — Default values, fallback configuration

mod defaults;
mod environment;
mod network;
mod paths;

/// Global configuration utilities for replacing hardcoded values
pub struct ConfigUtils;

impl ConfigUtils {
    // ========== Network (delegate to network) ==========

    /// Get ToadStool port from environment or default
    #[must_use]
    pub fn get_toadstool_port() -> u16 {
        network::get_toadstool_port()
    }

    /// Get federation port from environment or default
    #[must_use]
    pub fn get_federation_port() -> u16 {
        network::get_federation_port()
    }

    /// Get metrics port from environment or default
    #[must_use]
    pub fn get_metrics_port() -> u16 {
        network::get_metrics_port()
    }

    /// Get health check port from environment or default
    #[must_use]
    pub fn get_health_port() -> u16 {
        network::get_health_port()
    }

    /// Get events port from environment or default
    #[must_use]
    pub fn get_events_port() -> u16 {
        network::get_events_port()
    }

    /// Get bind address from environment or default
    #[must_use]
    pub fn get_bind_address() -> String {
        network::get_bind_address()
    }

    /// Get external hostname from environment or default
    #[must_use]
    pub fn get_external_hostname() -> String {
        network::get_external_hostname()
    }

    /// Get ToadStool endpoint from environment or default
    #[must_use]
    pub fn get_toadstool_endpoint() -> String {
        network::get_toadstool_endpoint()
    }

    /// Get request timeout from environment or default
    #[must_use]
    pub fn get_request_timeout() -> std::time::Duration {
        network::get_request_timeout()
    }

    /// Get connection timeout from environment or default
    #[must_use]
    pub fn get_connection_timeout() -> std::time::Duration {
        network::get_connection_timeout()
    }

    /// Get max retries from environment or default
    #[must_use]
    pub fn get_max_retries() -> u32 {
        network::get_max_retries()
    }

    /// Get max connections per host from environment or default
    #[must_use]
    pub fn get_max_connections_per_host() -> u32 {
        network::get_max_connections_per_host()
    }

    /// Get keepalive interval from environment or default
    #[must_use]
    pub fn get_keepalive_interval() -> std::time::Duration {
        network::get_keepalive_interval()
    }

    /// Get all service ports as a map
    #[must_use]
    pub fn get_service_ports() -> std::collections::HashMap<String, u16> {
        network::get_service_ports()
    }

    /// Get all service endpoints as a map
    #[must_use]
    pub fn get_service_endpoints() -> std::collections::HashMap<String, String> {
        network::get_service_endpoints()
    }

    /// Get container port range from environment or default
    #[must_use]
    pub fn get_container_port_range() -> (u16, u16) {
        network::get_container_port_range()
    }

    /// Get port allocation range from environment or default
    #[must_use]
    pub fn get_port_allocation_range() -> (u16, u16) {
        network::get_port_allocation_range()
    }

    // ========== Paths (delegate to paths) ==========

    /// Get data directory from environment or default
    #[must_use]
    pub fn get_data_dir() -> String {
        paths::get_data_dir()
    }

    /// Get cache directory from environment or default
    #[must_use]
    pub fn get_cache_dir() -> String {
        paths::get_cache_dir()
    }

    /// Get temp directory from environment or default
    #[must_use]
    pub fn get_temp_dir() -> String {
        paths::get_temp_dir()
    }

    /// Get log directory from environment or default
    #[must_use]
    pub fn get_log_dir() -> String {
        paths::get_log_dir()
    }

    /// Get encryption key path from environment or default
    #[must_use]
    pub fn get_encryption_key_path() -> String {
        paths::get_encryption_key_path()
    }

    /// Get TLS cert path from environment or default
    #[must_use]
    pub fn get_tls_cert_path() -> String {
        paths::get_tls_cert_path()
    }

    /// Get TLS key path from environment or default
    #[must_use]
    pub fn get_tls_key_path() -> String {
        paths::get_tls_key_path()
    }

    /// Get CA cert path from environment or default
    #[must_use]
    pub fn get_ca_cert_path() -> String {
        paths::get_ca_cert_path()
    }

    // ========== Environment (delegate to environment) ==========

    /// Get environment name from environment or default
    #[must_use]
    pub fn get_environment() -> String {
        environment::get_environment()
    }

    /// Get debug mode from environment or default
    #[must_use]
    pub fn get_debug_mode() -> bool {
        environment::get_debug_mode()
    }

    /// Get verbose mode from environment or default
    #[must_use]
    pub fn get_verbose_mode() -> bool {
        environment::get_verbose_mode()
    }

    /// Get all environment variables with TOADSTOOL prefix
    #[must_use]
    pub fn get_all_toadstool_env_vars() -> std::collections::HashMap<String, String> {
        environment::get_all_toadstool_env_vars()
    }

    // ========== Defaults (delegate to defaults) ==========

    /// Get worker threads from environment or default
    #[must_use]
    pub fn get_worker_threads() -> u32 {
        defaults::get_worker_threads()
    }

    /// Get max concurrent executions from environment or default
    #[must_use]
    pub fn get_max_concurrent_executions() -> u32 {
        defaults::get_max_concurrent_executions()
    }

    /// Get execution timeout from environment or default
    #[must_use]
    pub fn get_execution_timeout() -> std::time::Duration {
        defaults::get_execution_timeout()
    }

    /// Get max CPU usage from environment or default
    #[must_use]
    pub fn get_max_cpu_usage() -> f64 {
        defaults::get_max_cpu_usage()
    }

    /// Get max memory usage from environment or default
    #[must_use]
    pub fn get_max_memory_usage() -> u64 {
        defaults::get_max_memory_usage()
    }

    /// Get max storage usage from environment or default
    #[must_use]
    pub fn get_max_storage_usage() -> u64 {
        defaults::get_max_storage_usage()
    }

    /// Get metrics collection interval from environment or default
    #[must_use]
    pub fn get_metrics_interval() -> std::time::Duration {
        defaults::get_metrics_interval()
    }

    /// Get health check interval from environment or default
    #[must_use]
    pub fn get_health_check_interval() -> std::time::Duration {
        defaults::get_health_check_interval()
    }

    /// Get log level from environment or default
    #[must_use]
    pub fn get_log_level() -> String {
        defaults::get_log_level()
    }

    /// Get TLS enabled from environment or default
    #[must_use]
    pub fn get_tls_enabled() -> bool {
        defaults::get_tls_enabled()
    }

    /// Get auth enabled from environment or default
    #[must_use]
    pub fn get_auth_enabled() -> bool {
        defaults::get_auth_enabled()
    }

    /// Get sandboxing enabled from environment or default
    #[must_use]
    pub fn get_sandboxing_enabled() -> bool {
        defaults::get_sandboxing_enabled()
    }

    /// Get metrics enabled from environment or default
    #[must_use]
    pub fn get_metrics_enabled() -> bool {
        defaults::get_metrics_enabled()
    }

    /// Get health checks enabled from environment or default
    #[must_use]
    pub fn get_health_checks_enabled() -> bool {
        defaults::get_health_checks_enabled()
    }

    /// Get database URL from environment or default
    #[must_use]
    pub fn get_database_url() -> String {
        defaults::get_database_url()
    }

    /// Get cache URL from environment or default
    #[must_use]
    pub fn get_cache_url() -> String {
        defaults::get_cache_url()
    }

    /// Get message broker URL from environment or default
    #[must_use]
    pub fn get_message_broker_url() -> String {
        defaults::get_message_broker_url()
    }

    /// Get distributed storage URL from environment or default
    #[must_use]
    pub fn get_distributed_storage_url() -> String {
        defaults::get_distributed_storage_url()
    }

    /// Get monitoring endpoint from environment or default
    #[must_use]
    pub fn get_monitoring_endpoint() -> String {
        defaults::get_monitoring_endpoint()
    }

    /// Get alert webhook URL from environment or default
    #[must_use]
    pub fn get_alert_webhook_url() -> String {
        defaults::get_alert_webhook_url()
    }

    /// Get JWT secret from environment or default
    #[must_use]
    pub fn get_jwt_secret() -> String {
        defaults::get_jwt_secret()
    }

    /// Get API key from environment or default
    #[must_use]
    pub fn get_api_key() -> String {
        defaults::get_api_key()
    }

    /// Get webhook secret from environment or default
    #[must_use]
    pub fn get_webhook_secret() -> String {
        defaults::get_webhook_secret()
    }

    /// Get federation trust domain from environment or default
    #[must_use]
    pub fn get_federation_trust_domain() -> String {
        defaults::get_federation_trust_domain()
    }

    /// Get cluster name from environment or default
    #[must_use]
    pub fn get_cluster_name() -> String {
        defaults::get_cluster_name()
    }

    /// Get node name from environment or default
    #[must_use]
    pub fn get_node_name() -> String {
        defaults::get_node_name()
    }

    /// Print all current configuration values (for debugging)
    #[cfg(debug_assertions)]
    pub fn print_current_config() {
        defaults::print_current_config();
    }
}

#[cfg(test)]
mod tests;

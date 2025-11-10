// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Environment-based configuration system
//!
//! Replaces hardcoded constants with environment variable configuration.
//! This enables true infant discovery and dynamic configuration.

use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

/// Environment-based network configuration
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    // Network
    pub api_port: u16,
    pub metrics_port: u16,
    pub discovery_port: u16,
    pub monitoring_port: u16,

    // Discovery
    pub discovery_enabled: bool,
    pub discovery_timeout: Duration,
    pub discovery_interval: Duration,
    pub discovery_strategy: DiscoveryStrategy,

    // Performance
    pub max_concurrent_connections: usize,
    pub request_timeout: Duration,
    pub connection_timeout: Duration,
    pub keepalive_timeout: Duration,
    pub network_buffer_size: usize,

    // Rate Limiting
    pub rate_limit_requests_per_minute: u32,
    pub rate_limit_window_secs: u64,

    // Security
    pub tls_enabled: bool,
    pub auth_required: bool,
    pub auth_token_lifetime: Duration,

    // Logging
    pub log_level: LogLevel,
    pub log_format: LogFormat,
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,

    // Deployment
    pub deployment_env: DeploymentEnv,
    pub dev_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryStrategy {
    Dns,
    Consul,
    Kubernetes,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentEnv {
    Development,
    Staging,
    Production,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            // Network - use env vars or fallback to defaults
            api_port: get_env_u16("TOADSTOOL_API_PORT")
                .unwrap_or(crate::defaults::network::API_PORT),
            metrics_port: get_env_u16("TOADSTOOL_METRICS_PORT")
                .unwrap_or(crate::defaults::network::METRICS_PORT),
            discovery_port: get_env_u16("TOADSTOOL_DISCOVERY_PORT")
                .unwrap_or(crate::defaults::network::DISCOVERY_PORT),
            monitoring_port: get_env_u16("TOADSTOOL_MONITORING_PORT").unwrap_or(8082),

            // Discovery
            discovery_enabled: get_env_bool("TOADSTOOL_DISCOVERY_ENABLED").unwrap_or(true),
            discovery_timeout: Duration::from_millis(
                get_env_u64("TOADSTOOL_DISCOVERY_TIMEOUT_MS")
                    .unwrap_or(crate::defaults::timeouts::DISCOVERY_MS),
            ),
            discovery_interval: Duration::from_millis(
                get_env_u64("TOADSTOOL_DISCOVERY_INTERVAL_MS")
                    .unwrap_or(crate::defaults::timeouts::DISCOVERY_INTERVAL_MS),
            ),
            discovery_strategy: get_env_string("TOADSTOOL_DISCOVERY_STRATEGY")
                .and_then(|s| match s.as_str() {
                    "dns" => Some(DiscoveryStrategy::Dns),
                    "consul" => Some(DiscoveryStrategy::Consul),
                    "kubernetes" | "k8s" => Some(DiscoveryStrategy::Kubernetes),
                    "manual" => Some(DiscoveryStrategy::Manual),
                    _ => None,
                })
                .unwrap_or(DiscoveryStrategy::Dns),

            // Performance
            max_concurrent_connections: get_env_usize("TOADSTOOL_MAX_CONCURRENT_CONNECTIONS")
                .unwrap_or(crate::defaults::resources::MAX_CONNECTIONS),
            request_timeout: Duration::from_millis(
                get_env_u64("TOADSTOOL_REQUEST_TIMEOUT_MS")
                    .unwrap_or(crate::defaults::timeouts::REQUEST_MS),
            ),
            connection_timeout: Duration::from_millis(
                get_env_u64("TOADSTOOL_CONNECTION_TIMEOUT_MS")
                    .unwrap_or(crate::defaults::timeouts::CONNECTION_MS),
            ),
            keepalive_timeout: Duration::from_secs(
                get_env_u64("TOADSTOOL_KEEPALIVE_TIMEOUT_SECS").unwrap_or(30),
            ),
            network_buffer_size: get_env_usize("TOADSTOOL_NETWORK_BUFFER_SIZE").unwrap_or(8192),

            // Rate Limiting
            rate_limit_requests_per_minute: get_env_u32("TOADSTOOL_RATE_LIMIT_REQUESTS_PER_MINUTE")
                .unwrap_or(1000),
            rate_limit_window_secs: get_env_u64("TOADSTOOL_RATE_LIMIT_WINDOW_SECS").unwrap_or(60),

            // Security
            tls_enabled: get_env_bool("TOADSTOOL_TLS_ENABLED").unwrap_or(true),
            auth_required: get_env_bool("TOADSTOOL_AUTH_REQUIRED").unwrap_or(true),
            auth_token_lifetime: Duration::from_secs(
                get_env_u64("TOADSTOOL_AUTH_TOKEN_LIFETIME_SECS").unwrap_or(3600),
            ),

            // Logging
            log_level: get_env_string("TOADSTOOL_LOG_LEVEL")
                .and_then(|s| match s.as_str() {
                    "debug" => Some(LogLevel::Debug),
                    "info" => Some(LogLevel::Info),
                    "warn" => Some(LogLevel::Warn),
                    "error" => Some(LogLevel::Error),
                    _ => None,
                })
                .unwrap_or(LogLevel::Info),
            log_format: get_env_string("TOADSTOOL_LOG_FORMAT")
                .and_then(|s| match s.as_str() {
                    "json" => Some(LogFormat::Json),
                    "pretty" => Some(LogFormat::Pretty),
                    _ => None,
                })
                .unwrap_or(LogFormat::Json),
            metrics_enabled: get_env_bool("TOADSTOOL_METRICS_ENABLED").unwrap_or(true),
            tracing_enabled: get_env_bool("TOADSTOOL_TRACING_ENABLED").unwrap_or(true),

            // Deployment
            deployment_env: get_env_string("TOADSTOOL_DEPLOYMENT_ENV")
                .and_then(|s| match s.as_str() {
                    "development" | "dev" => Some(DeploymentEnv::Development),
                    "staging" | "stage" => Some(DeploymentEnv::Staging),
                    "production" | "prod" => Some(DeploymentEnv::Production),
                    _ => None,
                })
                .unwrap_or(DeploymentEnv::Production),
            dev_mode: get_env_bool("TOADSTOOL_DEV_MODE").unwrap_or(false),
        }
    }
}

impl EnvironmentConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Create development configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.deployment_env = DeploymentEnv::Development;
        config.dev_mode = true;
        config.tls_enabled = false;
        config.auth_required = false;
        config.log_level = LogLevel::Debug;
        config.log_format = LogFormat::Pretty;
        config
    }

    /// Create production configuration (strict defaults)
    pub fn production() -> Self {
        let mut config = Self::default();
        config.deployment_env = DeploymentEnv::Production;
        config.dev_mode = false;
        config.tls_enabled = true;
        config.auth_required = true;
        config.log_level = LogLevel::Info;
        config.log_format = LogFormat::Json;
        config
    }
}

// Helper functions for environment variable parsing
fn get_env_string(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn get_env_bool(key: &str) -> Option<bool> {
    env::var(key).ok().and_then(|v| match v.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    })
}

fn get_env_u16(key: &str) -> Option<u16> {
    env::var(key).ok().and_then(|v| v.parse().ok())
}

fn get_env_u32(key: &str) -> Option<u32> {
    env::var(key).ok().and_then(|v| v.parse().ok())
}

fn get_env_u64(key: &str) -> Option<u64> {
    env::var(key).ok().and_then(|v| v.parse().ok())
}

fn get_env_usize(key: &str) -> Option<usize> {
    env::var(key).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EnvironmentConfig::default();
        assert_eq!(config.api_port, 8080);
        assert!(config.discovery_enabled);
        assert_eq!(config.deployment_env, DeploymentEnv::Production);
    }

    #[test]
    fn test_development_config() {
        let config = EnvironmentConfig::development();
        assert_eq!(config.deployment_env, DeploymentEnv::Development);
        assert!(config.dev_mode);
        assert!(!config.tls_enabled);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_production_config() {
        let config = EnvironmentConfig::production();
        assert_eq!(config.deployment_env, DeploymentEnv::Production);
        assert!(!config.dev_mode);
        assert!(config.tls_enabled);
        assert!(config.auth_required);
    }
}

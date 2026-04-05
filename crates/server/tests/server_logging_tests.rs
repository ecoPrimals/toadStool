// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! LoggingConfig tests

use toadstool_server::LoggingConfig;

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();
    assert_eq!(config.level, "info");
    assert!(config.log_requests);
    assert!(config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_debug_level() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert_eq!(config.level, "debug");
}

#[test]
fn test_logging_config_warn_level() {
    let config = LoggingConfig {
        level: "warn".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert_eq!(config.level, "warn");
}

#[test]
fn test_logging_config_error_level() {
    let config = LoggingConfig {
        level: "error".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert_eq!(config.level, "error");
}

#[test]
fn test_logging_config_no_request_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: false,
        log_executions: true,
        log_metrics: true,
    };
    assert!(!config.log_requests);
}

#[test]
fn test_logging_config_no_execution_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: true,
        log_executions: false,
        log_metrics: true,
    };
    assert!(!config.log_executions);
}

#[test]
fn test_logging_config_no_metrics_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: false,
    };
    assert!(!config.log_metrics);
}

#[test]
fn test_logging_config_minimal() {
    let config = LoggingConfig {
        level: "error".to_string(),
        log_requests: false,
        log_executions: false,
        log_metrics: false,
    };
    assert_eq!(config.level, "error");
    assert!(!config.log_requests);
    assert!(!config.log_executions);
    assert!(!config.log_metrics);
}

#[test]
fn test_logging_config_trace_level() {
    let config = LoggingConfig {
        level: "trace".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert_eq!(config.level, "trace");
}

#[test]
fn test_logging_config_all_logging_enabled() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert!(config.log_requests);
    assert!(config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_all_logging_disabled() {
    let config = LoggingConfig {
        level: "off".to_string(),
        log_requests: false,
        log_executions: false,
        log_metrics: false,
    };
    assert!(!config.log_requests);
    assert!(!config.log_executions);
    assert!(!config.log_metrics);
}

#[test]
fn test_logging_config_custom_level() {
    let config = LoggingConfig {
        level: "custom".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };
    assert_eq!(config.level, "custom");
}

#[test]
fn test_logging_config_clone() {
    let config1 = LoggingConfig::default();
    let config2 = config1.clone();
    assert_eq!(config1.level, config2.level);
    assert_eq!(config1.log_requests, config2.log_requests);
}

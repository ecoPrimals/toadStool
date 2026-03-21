// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! HealthCheckConfig tests

use std::time::Duration;
use toadstool_server::HealthCheckConfig;

#[test]
fn test_health_check_config_default() {
    let config = HealthCheckConfig::default();
    assert_eq!(config.interval, Duration::from_secs(30));
    assert!(config.check_runtime_engines);
    assert!(config.check_resources);
    assert_eq!(config.memory_threshold_percent, 90.0);
    assert_eq!(config.cpu_threshold_percent, 95.0);
}

#[test]
fn test_health_check_config_custom_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(60),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert_eq!(config.interval, Duration::from_secs(60));
}

#[test]
fn test_health_check_config_no_runtime_checks() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: false,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert!(!config.check_runtime_engines);
}

#[test]
fn test_health_check_config_no_resource_checks() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: false,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert!(!config.check_resources);
}

#[test]
fn test_health_check_config_custom_memory_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 80.0,
        cpu_threshold_percent: 95.0,
    };
    assert_eq!(config.memory_threshold_percent, 80.0);
}

#[test]
fn test_health_check_config_custom_cpu_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 85.0,
    };
    assert_eq!(config.cpu_threshold_percent, 85.0);
}

#[test]
fn test_health_check_config_strict_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(15),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 70.0,
        cpu_threshold_percent: 75.0,
    };
    assert_eq!(config.memory_threshold_percent, 70.0);
    assert_eq!(config.cpu_threshold_percent, 75.0);
}

#[test]
fn test_health_check_config_lenient_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(60),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 98.0,
        cpu_threshold_percent: 99.0,
    };
    assert_eq!(config.memory_threshold_percent, 98.0);
    assert_eq!(config.cpu_threshold_percent, 99.0);
}

#[test]
fn test_health_check_config_very_short_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(1),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert_eq!(config.interval, Duration::from_secs(1));
}

#[test]
fn test_health_check_config_very_long_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(3600),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert_eq!(config.interval, Duration::from_secs(3600));
}

#[test]
fn test_health_check_config_no_checks_enabled() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: false,
        check_resources: false,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };
    assert!(!config.check_runtime_engines);
    assert!(!config.check_resources);
}

#[test]
fn test_health_check_config_low_memory_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 50.0,
        cpu_threshold_percent: 95.0,
    };
    assert_eq!(config.memory_threshold_percent, 50.0);
}

#[test]
fn test_health_check_config_low_cpu_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 60.0,
    };
    assert_eq!(config.cpu_threshold_percent, 60.0);
}

#[test]
fn test_health_check_config_100_percent_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 100.0,
        cpu_threshold_percent: 100.0,
    };
    assert_eq!(config.memory_threshold_percent, 100.0);
    assert_eq!(config.cpu_threshold_percent, 100.0);
}

#[test]
fn test_health_check_config_clone() {
    let config1 = HealthCheckConfig::default();
    let config2 = config1.clone();
    assert_eq!(config1.interval, config2.interval);
    assert_eq!(
        config1.memory_threshold_percent,
        config2.memory_threshold_percent
    );
}

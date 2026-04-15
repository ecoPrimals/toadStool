// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;

#[test]
fn test_sandbox_config_disabled_all_features() {
    let config = SandboxConfig {
        advanced_features_enabled: false,
        enable_seccomp: false,
        enable_capability_dropping: false,
        enable_namespace_isolation: false,
        enable_resource_limits: false,
        enable_monitoring: false,
        ..SandboxConfig::default()
    };

    assert!(!config.advanced_features_enabled);
    assert!(!config.enable_seccomp);
    assert!(!config.enable_capability_dropping);
}

#[test]
fn test_sandbox_config_very_high_concurrency() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1000);
}

#[test]
fn test_sandbox_config_very_low_concurrency() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1);
}

#[test]
fn test_sandbox_config_long_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 300, // 5 minutes
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 300);
}

#[test]
fn test_sandbox_config_short_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 5, // 5 seconds
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 5);
}

#[test]
fn test_sandbox_config_fast_monitoring() {
    let config = SandboxConfig {
        monitoring_interval_ms: 100, // 100ms
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 100);
}

#[test]
fn test_sandbox_config_slow_monitoring() {
    let config = SandboxConfig {
        monitoring_interval_ms: 10000, // 10 seconds
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 10000);
}

#[test]
fn test_sandbox_config_isolation_level_minimum() {
    let config = SandboxConfig {
        default_isolation_level: toadstool::security::IsolationLevel::None,
        ..SandboxConfig::default()
    };

    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::None
    ));
}

#[test]
fn test_sandbox_config_isolation_level_enhanced() {
    let config = SandboxConfig {
        default_isolation_level: toadstool::security::IsolationLevel::Enhanced,
        ..SandboxConfig::default()
    };

    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::Enhanced
    ));
}

#[test]
fn test_sandbox_config_clone() {
    let config1 = SandboxConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.max_concurrent_sandboxes,
        config2.max_concurrent_sandboxes
    );
    assert_eq!(config1.cleanup_timeout_secs, config2.cleanup_timeout_secs);
}

#[test]
fn test_sandbox_config_serialization() {
    let config = SandboxConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("advanced_features_enabled"));
}

#[test]
fn test_sandbox_config_deserialization() {
    let config = SandboxConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SandboxConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        config.max_concurrent_sandboxes,
        deserialized.max_concurrent_sandboxes
    );
}

#[test]
fn test_sandbox_config_minimal_concurrent() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1);
}

#[test]
fn test_sandbox_config_high_concurrent() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1000);
}

#[test]
fn test_sandbox_config_very_short_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 5,
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 5);
}

#[test]
fn test_sandbox_config_very_long_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 300,
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 300);
}

#[test]
fn test_sandbox_config_very_fast_monitoring() {
    let config = SandboxConfig {
        enable_monitoring: true,
        monitoring_interval_ms: 100,
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 100);
}

#[test]
fn test_sandbox_config_very_slow_monitoring() {
    let config = SandboxConfig {
        enable_monitoring: true,
        monitoring_interval_ms: 10000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 10000);
}

#[test]
fn test_sandbox_config_all_disabled() {
    let config = SandboxConfig {
        advanced_features_enabled: false,
        enable_seccomp: false,
        enable_capability_dropping: false,
        enable_namespace_isolation: false,
        enable_resource_limits: false,
        enable_monitoring: false,
        ..SandboxConfig::default()
    };

    assert!(!config.advanced_features_enabled);
    assert!(!config.enable_seccomp);
    assert!(!config.enable_monitoring);
}

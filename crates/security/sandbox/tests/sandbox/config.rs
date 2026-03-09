// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive sandbox security tests
//!
//! This test suite provides extensive coverage for sandbox configuration,
//! resource limits, filesystem isolation, and security enforcement.

use toadstool_security_sandbox::*;

use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Sandbox Configuration Tests
// ============================================================================

#[test]
fn test_sandbox_config_default() {
    let config = SandboxConfig::default();

    assert!(config.advanced_features_enabled);
    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::Standard
    ));
    assert!(config.enable_capability_dropping);
    assert!(config.enable_resource_limits);
    assert_eq!(config.max_concurrent_sandboxes, 100);
}

#[test]
fn test_sandbox_config_custom() {
    let config = SandboxConfig {
        advanced_features_enabled: false,
        default_isolation_level: toadstool::security::IsolationLevel::Maximum,
        enable_seccomp: false,
        enable_capability_dropping: false,
        enable_namespace_isolation: false,
        enable_resource_limits: false,
        sandbox_root: PathBuf::from("/custom/sandbox"),
        temp_dir: PathBuf::from("/custom/tmp"),
        max_concurrent_sandboxes: 50,
        cleanup_timeout_secs: 60,
        enable_monitoring: false,
        monitoring_interval_ms: 500,
    };

    assert!(!config.advanced_features_enabled);
    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::Maximum
    ));
    assert!(!config.enable_monitoring);
}

#[test]
fn test_sandbox_config_paths() {
    let config = SandboxConfig {
        sandbox_root: PathBuf::from("/var/lib/sandbox"),
        temp_dir: PathBuf::from("/tmp/sandbox"),
        ..SandboxConfig::default()
    };

    assert_eq!(config.sandbox_root, PathBuf::from("/var/lib/sandbox"));
    assert_eq!(config.temp_dir, PathBuf::from("/tmp/sandbox"));
}

#[test]
fn test_sandbox_config_monitoring() {
    let mut config = SandboxConfig::default();

    assert!(config.enable_monitoring);
    assert_eq!(config.monitoring_interval_ms, 1000);

    config.enable_monitoring = false;
    config.monitoring_interval_ms = 5000;

    assert!(!config.enable_monitoring);
    assert_eq!(config.monitoring_interval_ms, 5000);
}


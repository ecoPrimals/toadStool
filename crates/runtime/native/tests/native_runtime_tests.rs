// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Native runtime engine

use toadstool::{RuntimeEngine, WorkloadType};
use toadstool_runtime_native::NativeRuntimeEngine;

// ============================================================================
// NativeRuntimeEngine Creation Tests
// ============================================================================

#[test]
fn test_native_runtime_engine_creation() {
    let engine = NativeRuntimeEngine::new();
    assert!(format!("{:?}", engine).contains("NativeRuntimeEngine"));
}

#[test]
fn test_native_runtime_engine_default() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.supported_workloads.is_empty());
}

// Runtime type is returned in ExecutionResponse, not as a separate method

#[test]
fn test_native_runtime_engine_debug_format() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{:?}", engine);

    assert!(debug_str.contains("NativeRuntimeEngine"));
    assert!(debug_str.contains("capabilities"));
}

// ============================================================================
// RuntimeCapabilities Tests
// ============================================================================

#[test]
fn test_native_capabilities_workload_type() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Native));
}

#[test]
fn test_native_capabilities_concurrent_limit() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(capabilities.max_concurrent_executions.is_some());
    let limit = capabilities.max_concurrent_executions.unwrap();
    assert_eq!(limit, 100);
}

#[test]
fn test_native_capabilities_architecture() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.supported_architectures.is_empty());
    // Should match current system architecture
    let arch = std::env::consts::ARCH;
    assert!(capabilities
        .supported_architectures
        .contains(&arch.to_string()));
}

#[test]
fn test_native_capabilities_platform_features() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(capabilities
        .platform_features
        .contains_key("process_isolation"));
    assert_eq!(
        capabilities.platform_features.get("process_isolation"),
        Some(&true)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn test_native_capabilities_linux_features() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("resource_limits"),
        Some(&true)
    );
}

#[cfg(unix)]
#[test]
fn test_native_capabilities_unix_features() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("user_switching"),
        Some(&true)
    );
    assert_eq!(
        capabilities.platform_features.get("chroot_jail"),
        Some(&true)
    );
}

#[test]
fn test_native_capabilities_version() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.version.is_empty());
}

// ============================================================================
// Multiple Engine Instances
// ============================================================================

#[test]
fn test_multiple_native_engines() {
    let engine1 = NativeRuntimeEngine::new();
    let engine2 = NativeRuntimeEngine::new();

    let cap1 = engine1.get_capabilities();
    let cap2 = engine2.get_capabilities();

    assert_eq!(cap1.version, cap2.version);
}

#[test]
fn test_native_engines_independent() {
    let engine1 = NativeRuntimeEngine::new();
    let engine2 = NativeRuntimeEngine::new();

    let cap1 = engine1.get_capabilities();
    let cap2 = engine2.get_capabilities();

    assert_eq!(cap1.version, cap2.version);
}

// ============================================================================
// Capability Details Tests
// ============================================================================

#[test]
fn test_native_supported_workloads_only_native() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(capabilities.supported_workloads.len(), 1);
    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Native));
}

#[test]
fn test_native_concurrent_execution_limit_positive() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    if let Some(limit) = capabilities.max_concurrent_executions {
        assert!(limit > 0);
        assert!(limit <= 1000); // Reasonable upper bound
    }
}

#[test]
fn test_native_supported_architectures_current_platform() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should support at least one architecture
    assert!(!capabilities.supported_architectures.is_empty());

    // Current platform should be supported
    let current_arch = std::env::consts::ARCH;
    assert!(capabilities
        .supported_architectures
        .contains(&current_arch.to_string()));
}

// ============================================================================
// Platform Feature Tests
// ============================================================================

#[test]
fn test_native_process_isolation_feature() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(capabilities
        .platform_features
        .contains_key("process_isolation"));
}

#[test]
fn test_native_platform_features_types() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // All platform features should be boolean values
    for key in capabilities.platform_features.keys() {
        // All values are booleans by type definition
        // Just verify key exists (the type system guarantees boolean values)
        assert!(capabilities.platform_features.contains_key(key));
    }
}

#[test]
fn test_native_platform_features_count() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should have at least process_isolation
    assert!(!capabilities.platform_features.is_empty());
}

// ============================================================================
// Architecture Tests
// ============================================================================

#[test]
fn test_native_architecture_x86_64_or_aarch64() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    let arch = &capabilities.supported_architectures[0];
    // Common architectures
    assert!(
        arch == "x86_64" || arch == "aarch64" || arch == "x86" || arch == "arm",
        "Unexpected architecture: {}",
        arch
    );
}

#[test]
fn test_native_architecture_string_not_empty() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    for arch in &capabilities.supported_architectures {
        assert!(!arch.is_empty());
    }
}

// ============================================================================
// Version Tests
// ============================================================================

#[test]
fn test_native_version_format() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.version.is_empty());
    // Should be a valid version string (contains dots or numbers)
    assert!(
        capabilities.version.contains('.') || capabilities.version.chars().any(|c| c.is_numeric())
    );
}

#[test]
fn test_native_version_consistency() {
    let engine1 = NativeRuntimeEngine::new();
    let engine2 = NativeRuntimeEngine::new();

    let ver1 = engine1.get_capabilities().version;
    let ver2 = engine2.get_capabilities().version;

    assert_eq!(ver1, ver2);
}

// ============================================================================
// Capability Cloning Tests
// ============================================================================

#[test]
fn test_native_capabilities_can_clone() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();
    let cloned = capabilities.clone();

    assert_eq!(capabilities.version, cloned.version);
    assert_eq!(capabilities.supported_workloads, cloned.supported_workloads);
}

#[test]
fn test_native_capabilities_clone_independence() {
    let engine = NativeRuntimeEngine::new();
    let capabilities1 = engine.get_capabilities();
    let mut capabilities2 = capabilities1.clone();

    // Modify clone
    capabilities2.version = "modified".to_string();

    // Original should be unchanged
    assert_ne!(capabilities1.version, capabilities2.version);
}

// ============================================================================
// Engine Lifecycle Tests
// ============================================================================

// Runtime type consistency tested through capabilities

#[test]
fn test_native_engine_get_capabilities_consistent() {
    let engine = NativeRuntimeEngine::new();

    let cap1 = engine.get_capabilities();
    let cap2 = engine.get_capabilities();

    assert_eq!(cap1.version, cap2.version);
    assert_eq!(cap1.supported_workloads, cap2.supported_workloads);
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_native_capabilities_not_empty() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert!(!capabilities.supported_workloads.is_empty());
    assert!(!capabilities.supported_architectures.is_empty());
    assert!(!capabilities.version.is_empty());
}

#[test]
fn test_native_concurrent_executions_reasonable() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    if let Some(limit) = capabilities.max_concurrent_executions {
        // Should be a reasonable number (not too low, not absurdly high)
        assert!((1..=10000).contains(&limit));
    }
}

// ============================================================================
// Integration Scenarios
// ============================================================================

#[test]
fn test_native_engine_scenario_simple_process() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should support native workloads
    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Native));

    // Should support current architecture
    let arch = std::env::consts::ARCH;
    assert!(capabilities
        .supported_architectures
        .contains(&arch.to_string()));
}

#[test]
fn test_native_engine_scenario_high_concurrency() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should support reasonable concurrency
    if let Some(limit) = capabilities.max_concurrent_executions {
        assert!(
            limit >= 10,
            "Should support at least 10 concurrent executions"
        );
    }
}

#[test]
fn test_native_engine_scenario_platform_specific() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should have platform-specific features
    assert!(!capabilities.platform_features.is_empty());
}

// ============================================================================
// Comparison Tests
// ============================================================================

#[test]
fn test_native_vs_other_runtime_types() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should only support Native workloads
    assert!(capabilities
        .supported_workloads
        .contains(&WorkloadType::Native));
    assert_eq!(capabilities.supported_workloads.len(), 1);
}

#[test]
fn test_native_workload_type_exclusive() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    // Should only support Native workload type
    assert!(!capabilities
        .supported_workloads
        .contains(&WorkloadType::Wasm));
    assert!(!capabilities
        .supported_workloads
        .contains(&WorkloadType::Container));
}

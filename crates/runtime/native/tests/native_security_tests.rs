// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Native runtime security and isolation

use toadstool::RuntimeEngine;
use toadstool::WorkloadType;
use toadstool::security::{IsolationLevel, SecurityContext};
use toadstool_runtime_native::NativeRuntimeEngine;

// ============================================================================
// IsolationLevel Tests (5 variants)
// ============================================================================

#[test]
fn test_isolation_level_none() {
    let level = IsolationLevel::None;
    assert!(matches!(level, IsolationLevel::None));
}

#[test]
fn test_isolation_level_basic() {
    let level = IsolationLevel::Basic;
    assert!(matches!(level, IsolationLevel::Basic));
}

#[test]
fn test_isolation_level_standard() {
    let level = IsolationLevel::Standard;
    assert!(matches!(level, IsolationLevel::Standard));
}

#[test]
fn test_isolation_level_enhanced() {
    let level = IsolationLevel::Enhanced;
    assert!(matches!(level, IsolationLevel::Enhanced));
}

#[test]
fn test_isolation_level_maximum() {
    let level = IsolationLevel::Maximum;
    assert!(matches!(level, IsolationLevel::Maximum));
}

#[test]
fn test_all_isolation_levels() {
    let levels = [
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
        IsolationLevel::Maximum,
    ];

    assert_eq!(levels.len(), 5);
}

// ============================================================================
// SecurityContext Tests
// ============================================================================

#[test]
fn test_security_context_default() {
    let _context = SecurityContext::default();
    // Default context should be created successfully
}

#[test]
fn test_security_context_with_none_isolation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::None,
        ..Default::default()
    };

    assert!(matches!(context.isolation_level, IsolationLevel::None));
}

#[test]
fn test_security_context_with_basic_isolation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Basic,
        ..Default::default()
    };

    assert!(matches!(context.isolation_level, IsolationLevel::Basic));
}

#[test]
fn test_security_context_with_standard_isolation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        ..Default::default()
    };

    assert!(matches!(context.isolation_level, IsolationLevel::Standard));
}

#[test]
fn test_security_context_with_enhanced_isolation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Enhanced,
        ..Default::default()
    };

    assert!(matches!(context.isolation_level, IsolationLevel::Enhanced));
}

#[test]
fn test_security_context_with_maximum_isolation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        ..Default::default()
    };

    assert!(matches!(context.isolation_level, IsolationLevel::Maximum));
}

// ============================================================================
// Workload Support Tests
// ============================================================================

#[test]
fn test_native_supports_native_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(engine.supports_workload(&WorkloadType::Native));
}

#[test]
fn test_native_not_supports_wasm_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Wasm));
}

#[test]
fn test_native_not_supports_container_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Container));
}

#[test]
fn test_native_not_supports_python_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Python));
}

#[test]
fn test_native_not_supports_gpu_workload() {
    let engine = NativeRuntimeEngine::new();
    assert!(!engine.supports_workload(&WorkloadType::Gpu));
}

// ============================================================================
// Platform Features Tests
// ============================================================================

#[test]
fn test_process_isolation_always_available() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("process_isolation"),
        Some(&true)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn test_resource_limits_on_linux() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("resource_limits"),
        Some(&true)
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn test_resource_limits_not_on_non_linux() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("resource_limits"),
        Some(&false)
    );
}

#[cfg(unix)]
#[test]
fn test_user_switching_on_unix() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("user_switching"),
        Some(&true)
    );
}

#[cfg(unix)]
#[test]
fn test_chroot_jail_on_unix() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("chroot_jail"),
        Some(&true)
    );
}

#[cfg(not(unix))]
#[test]
fn test_user_switching_not_on_windows() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(
        capabilities.platform_features.get("user_switching"),
        Some(&false)
    );
}

// ============================================================================
// Engine Configuration Tests
// ============================================================================

#[test]
fn test_native_engine_max_concurrent_100() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(capabilities.max_concurrent_executions, Some(100));
}

#[test]
fn test_native_engine_single_workload_type() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    assert_eq!(capabilities.supported_workloads.len(), 1);
    assert_eq!(capabilities.supported_workloads[0], WorkloadType::Native);
}

#[test]
fn test_native_engine_current_arch_supported() {
    let engine = NativeRuntimeEngine::new();
    let capabilities = engine.get_capabilities();

    let current_arch = std::env::consts::ARCH;
    assert!(
        capabilities
            .supported_architectures
            .contains(&current_arch.to_string())
    );
}

// ============================================================================
// Security Context Scenarios
// ============================================================================

#[test]
fn test_security_context_minimal() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::None,
        ..Default::default()
    };

    // Minimal security should work
    assert!(matches!(context.isolation_level, IsolationLevel::None));
}

#[test]
fn test_security_context_high_security() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        ..Default::default()
    };

    // High security should be configurable
    assert!(matches!(context.isolation_level, IsolationLevel::Maximum));
}

// ============================================================================
// Engine Debug Tests
// ============================================================================

#[test]
fn test_native_engine_debug_contains_type() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{engine:?}");

    assert!(debug_str.contains("NativeRuntimeEngine"));
}

#[test]
fn test_native_engine_debug_contains_config() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{engine:?}");

    assert!(debug_str.contains("config"));
}

#[test]
fn test_native_engine_debug_contains_capabilities() {
    let engine = NativeRuntimeEngine::new();
    let debug_str = format!("{engine:?}");

    assert!(debug_str.contains("capabilities"));
}

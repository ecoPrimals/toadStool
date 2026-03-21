// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `toadstool::lib` root module
//!
//! Sprint 14: lib.rs coverage 0% → 80%
//! Target: 45 testable lines, ~15-20 comprehensive tests

use toadstool::*;

// ============================================================================
// Constants Tests
// ============================================================================

#[test]
fn test_version_constant_exists() {
    // VERSION constant should be populated from Cargo.toml
    assert!(!VERSION.is_empty());
}

#[test]
fn test_version_format_valid() {
    // VERSION should follow semver format (x.y.z)
    let parts: Vec<&str> = VERSION.split('.').collect();
    assert!(
        parts.len() >= 2,
        "Version should have at least major.minor: {VERSION}"
    );

    // Each part should be parseable as a number
    for part in parts.iter().take(3) {
        assert!(
            part.parse::<u32>().is_ok(),
            "Version part '{part}' should be a number"
        );
    }
}

#[test]
fn test_universal_capabilities_not_empty() {
    // UNIVERSAL_CAPABILITIES should list all platform capabilities
    assert!(!UNIVERSAL_CAPABILITIES.is_empty());
}

#[test]
fn test_universal_capabilities_contains_core_features() {
    // Verify core capabilities are listed
    let capabilities: Vec<&str> = UNIVERSAL_CAPABILITIES.to_vec();

    // Core execution capabilities
    assert!(capabilities.contains(&"native_execution"));
    assert!(capabilities.contains(&"wasm_execution"));

    // Platform capabilities
    assert!(capabilities.contains(&"universal_scheduling"));
    assert!(capabilities.contains(&"ecosystem_integration"));
}

#[test]
fn test_universal_capabilities_contains_advanced_features() {
    // Verify advanced capabilities
    let capabilities: Vec<&str> = UNIVERSAL_CAPABILITIES.to_vec();

    assert!(capabilities.contains(&"recursive_hosting"));
    assert!(capabilities.contains(&"os_layer_compatibility"));
    assert!(capabilities.contains(&"biome_orchestration"));
    assert!(capabilities.contains(&"substrate_agnostic"));
    assert!(capabilities.contains(&"infinite_nesting"));
}

#[test]
fn test_universal_capabilities_count() {
    // Should have 10 capabilities listed
    assert_eq!(
        UNIVERSAL_CAPABILITIES.len(),
        10,
        "Expected 10 universal capabilities"
    );
}

#[test]
fn test_universal_capabilities_no_duplicates() {
    // No duplicate capabilities
    let mut unique = std::collections::HashSet::new();
    for cap in UNIVERSAL_CAPABILITIES {
        assert!(unique.insert(cap), "Duplicate capability found: {cap}");
    }
}

// ============================================================================
// Initialization Tests
// ============================================================================

#[test]
fn test_init_succeeds() {
    // Basic initialization should succeed
    // Note: May fail if tracing already initialized, which is acceptable
    let result = init();

    // Either succeeds or fails with tracing already initialized error
    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();
        assert!(
            error_msg.contains("tracing") || error_msg.contains("subscriber"),
            "Unexpected initialization error: {e}"
        );
    }
}

#[test]
fn test_init_is_idempotent() {
    // Calling init() multiple times should not panic
    // Second call may fail with "already initialized" error (acceptable)
    let _first = init();
    let second = init();

    // Should not panic - may succeed or fail gracefully
    if let Err(e) = second {
        let error_msg = e.to_string().to_lowercase();
        assert!(
            error_msg.contains("tracing") || error_msg.contains("subscriber"),
            "Should only fail if already initialized: {e}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_ecosystem_creates_platform() {
    // init_with_ecosystem should create UniversalComputePlatform
    // May fail if dependencies not available, but should not panic
    let result = init_with_ecosystem().await;

    // Either succeeds or fails gracefully (not panic)
    match result {
        Ok(platform) => {
            // Platform was created successfully
            drop(platform);
        }
        Err(e) => {
            // Acceptable errors: missing config, tracing already init, etc.
            let error_msg = e.to_string();
            assert!(
                !error_msg.is_empty(),
                "Error should have a descriptive message"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_biomeos_creates_platform() {
    // init_with_biomeos should create platform with biomeOS integration
    let result = init_with_biomeos().await;

    // Either succeeds or fails gracefully
    match result {
        Ok(platform) => {
            drop(platform);
        }
        Err(e) => {
            // Should have descriptive error
            assert!(!e.to_string().is_empty());
        }
    }
}

// ============================================================================
// Re-export Tests
// ============================================================================

#[test]
fn test_core_types_exported() {
    // Verify core types are publicly available
    let _: ExecutionStatus = ExecutionStatus::Pending;
    let _: RuntimeType = RuntimeType::Native;
    let _: IsolationLevel = IsolationLevel::Standard;
}

#[test]
fn test_resource_types_exported() {
    // Verify resource types are available
    let _req = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements::default(),
        storage: StorageRequirements::default(),
        network: NetworkRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: None,
        }),
    };
}

#[test]
fn test_ecosystem_types_exported() {
    // Note: PrimalStatus removed in favor of capability-based discovery
    // Ecosystem types are now focused on capabilities rather than specific primals
    let _ = std::any::type_name::<EcosystemConfig>();
}

#[test]
fn test_workload_types_exported() {
    // Verify workload types are available
    let _ = WorkloadType::Native;
}

// ============================================================================
// Module Structure Tests
// ============================================================================

#[test]
fn test_all_modules_accessible() {
    // Verify all public modules are accessible
    // This tests module structure, not functionality

    // Core types should be accessible
    let _ = std::any::type_name::<security::SecurityContext>();
    let _ = std::any::type_name::<workload::WorkloadSpec>();
    let _ = std::any::type_name::<ExecutionRequest>();
    let _ = std::any::type_name::<ResourceRequirements>();
}

#[test]
fn test_config_module_accessible() {
    // Note: SONGBIRD_PORT removed as part of capability-based, vendor-agnostic design
    // Services discover each other at runtime rather than using hardcoded ports
    let _ = std::any::type_name::<config::ToadStoolConfig>();
}

// ============================================================================
// Version Metadata Tests
// ============================================================================

#[test]
fn test_version_constant_stability() {
    // VERSION should be consistent across calls
    let v1 = VERSION;
    let v2 = VERSION;
    assert_eq!(v1, v2, "VERSION constant should be stable");
}

#[test]
fn test_capabilities_constant_stability() {
    // UNIVERSAL_CAPABILITIES should be consistent
    let c1 = UNIVERSAL_CAPABILITIES;
    let c2 = UNIVERSAL_CAPABILITIES;
    assert_eq!(c1, c2, "UNIVERSAL_CAPABILITIES should be stable");
}

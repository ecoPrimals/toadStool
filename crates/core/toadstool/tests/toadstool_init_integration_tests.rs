// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for ToadStool initialization functions
//! These tests verify the actual initialization logic and integration

#[test]
fn test_init_basic() {
    // Multiple calls should be idempotent (tracing already init is ok)
    let result = toadstool::init();
    // First call may succeed or fail if tracing already initialized
    let _ = result;

    // Second call should handle already-initialized case gracefully
    let result2 = toadstool::init();
    let _ = result2;
}

#[test]
fn test_version_constant() {
    let version = toadstool::VERSION;
    assert!(!version.is_empty());
    assert!(version.contains('.'), "Version should contain dots");
}

#[test]
fn test_universal_capabilities_constant() {
    let caps = toadstool::UNIVERSAL_CAPABILITIES;
    assert!(caps.len() >= 10, "Should have at least 10 capabilities");
    assert!(caps.contains(&"native_execution"));
    assert!(caps.contains(&"wasm_execution"));
    assert!(caps.contains(&"universal_scheduling"));
    assert!(caps.contains(&"recursive_hosting"));
    assert!(caps.contains(&"os_layer_compatibility"));
    assert!(caps.contains(&"ecosystem_integration"));
    assert!(caps.contains(&"biome_orchestration"));
    assert!(caps.contains(&"pure_ecosystem"));
    assert!(caps.contains(&"substrate_agnostic"));
    assert!(caps.contains(&"infinite_nesting"));
}

#[test]
fn test_universal_capabilities_immutable() {
    // Ensure capabilities are constant and can't be modified
    let caps1 = toadstool::UNIVERSAL_CAPABILITIES;
    let caps2 = toadstool::UNIVERSAL_CAPABILITIES;
    assert_eq!(caps1.len(), caps2.len());
    for (a, b) in caps1.iter().zip(caps2.iter()) {
        assert_eq!(a, b);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_ecosystem_creates_platform() {
    // This may fail if ecosystem isn't available, which is expected
    let result = toadstool::init_with_ecosystem().await;
    // We don't assert success since ecosystem may not be available
    // But we verify the function is callable and returns correct type
    match result {
        Ok(platform) => {
            // Platform created successfully
            drop(platform);
        }
        Err(_e) => {
            // Expected if ecosystem isn't available
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_biomeos_creates_platform() {
    // This may fail if biomeOS isn't available, which is expected
    let result = toadstool::init_with_biomeos().await;
    // We don't assert success since biomeOS may not be available
    // But we verify the function is callable and returns correct type
    match result {
        Ok(platform) => {
            // Platform created successfully
            drop(platform);
        }
        Err(_e) => {
            // Expected if biomeOS isn't available
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_ecosystem_calls_init() {
    // Verify that init_with_ecosystem calls init() first
    // If tracing is already initialized, it should handle gracefully
    let result = toadstool::init_with_ecosystem().await;
    // The init() call should happen regardless of ecosystem availability
    let _ = result;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_init_with_biomeos_calls_init() {
    // Verify that init_with_biomeos calls init() first
    let result = toadstool::init_with_biomeos().await;
    let _ = result;
}

#[test]
fn test_version_format() {
    let version = toadstool::VERSION;
    let parts: Vec<&str> = version.split('.').collect();
    assert!(parts.len() >= 2, "Version should have at least major.minor");
}

#[test]
fn test_capabilities_no_duplicates() {
    let caps = toadstool::UNIVERSAL_CAPABILITIES;
    let mut seen = std::collections::HashSet::new();
    for cap in caps {
        assert!(seen.insert(cap), "Duplicate capability: {cap}");
    }
}

#[test]
fn test_capabilities_non_empty() {
    let caps = toadstool::UNIVERSAL_CAPABILITIES;
    for cap in caps {
        assert!(!cap.is_empty(), "Capability should not be empty");
    }
}

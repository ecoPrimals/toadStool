// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for substrate detectors
//!
//! ## Evolution (Feb 15, 2026)
//!
//! Tests updated to focus on `BareMetalDetector` only.
//! Vendor-specific detectors (Kubernetes, Docker, Consul, Cloud) removed.
//! ToadStool only cares about hardware capabilities - service discovery
//! is delegated to Songbird (comms primal).

use toadstool_common::infant_discovery::capabilities::*;
use toadstool_common::infant_discovery::detectors::*;

// ============================================================================
// BareMetalDetector Tests
// ============================================================================

#[test]
fn test_bare_metal_detector_new() {
    let detector = BareMetalDetector::new();
    assert_eq!(detector.name(), "bare_metal");
}

#[test]
fn test_bare_metal_detector_default() {
    let detector = BareMetalDetector;
    assert_eq!(detector.name(), "bare_metal");
}

#[test]
fn test_bare_metal_detector_name() {
    let detector = BareMetalDetector::new();
    assert_eq!(detector.name(), "bare_metal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_detect() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await;

    assert!(result.is_ok());
    let substrate = result.unwrap();
    assert!(substrate.is_some());
    let substrate = substrate.unwrap();
    assert_eq!(substrate.substrate_type, SubstrateType::Bare);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_capabilities() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await.unwrap();
    assert!(result.is_some());
    let substrate = result.unwrap();
    assert!(!substrate.capabilities.is_empty());
    assert!(substrate
        .capabilities
        .contains(&SubstrateCapability::BareMetal));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_metadata() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await.unwrap();
    assert!(result.is_some());
    let substrate = result.unwrap();
    assert!(!substrate.metadata.is_empty());
    assert!(substrate.metadata.contains_key("deployment"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_bare_metal_detector_cpu_threads() {
    let detector = BareMetalDetector::new();
    let result = detector.detect().await.unwrap();
    assert!(result.is_some());
    let substrate = result.unwrap();
    // Should have cpu_threads metadata
    assert!(substrate.metadata.contains_key("cpu_threads"));
    let threads: usize = substrate
        .metadata
        .get("cpu_threads")
        .unwrap()
        .parse()
        .unwrap();
    assert!(threads >= 1);
}

// ============================================================================
// Standard Detectors Chain Tests
// ============================================================================

#[test]
fn test_standard_detectors_returns_only_bare_metal() {
    let detectors = standard_detectors();
    assert_eq!(detectors.len(), 1, "Only BareMetalDetector should remain");
    assert_eq!(detectors[0].name(), "bare_metal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_standard_detectors_all_dont_panic() {
    let detectors = standard_detectors();
    for detector in detectors {
        let result = detector.detect().await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_standard_detectors_at_least_one_succeeds() {
    let detectors = standard_detectors();
    let mut found = false;
    for detector in detectors {
        if let Ok(Some(_)) = detector.detect().await {
            found = true;
            break;
        }
    }
    assert!(found, "At least one detector should succeed");
}

// ============================================================================
// HardwareEnvironment Tests
// ============================================================================

#[test]
fn test_hardware_environment_default() {
    let env = HardwareEnvironment::default();
    assert!(env.hostname.is_none());
}

#[test]
fn test_hardware_environment_from_env() {
    let env = HardwareEnvironment::from_env();
    // May or may not have hostname depending on test environment
    let _ = env.hostname;
}

#[test]
fn test_hardware_environment_clone() {
    let env1 = HardwareEnvironment::default();
    let env2 = env1.clone();
    assert_eq!(env1.hostname, env2.hostname);
}

#[test]
fn test_hardware_environment_debug() {
    let env = HardwareEnvironment::default();
    let debug = format!("{env:?}");
    assert!(debug.contains("HardwareEnvironment"));
}

// ============================================================================
// Substrate Type Tests
// ============================================================================

#[test]
fn test_substrate_type_bare() {
    let substrate_type = SubstrateType::Bare;
    assert_eq!(substrate_type, SubstrateType::Bare);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detector_multiple_sequential_calls() {
    let detector = BareMetalDetector::new();
    // Multiple calls should all succeed
    for _ in 0..5 {
        let result = detector.detect().await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_detector_concurrent_calls() {
    let detector = BareMetalDetector::new();

    // Run multiple concurrent detections
    let (r1, r2, r3, r4) = tokio::join!(
        detector.detect(),
        detector.detect(),
        detector.detect(),
        detector.detect()
    );

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
    assert!(r4.is_ok());
}

// ============================================================================
// Legacy CloudEnvironment Tests (Deprecated)
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_cloud_environment_alias_deprecated() {
    // CloudEnvironment is now an alias to HardwareEnvironment
    let _env: CloudEnvironment = HardwareEnvironment::default();
}

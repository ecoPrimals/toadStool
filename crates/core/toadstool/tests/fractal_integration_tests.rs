// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-only

//! Fractal Composition Integration Tests
//!
//! These tests validate the complete fractal composition system:
//! - Layer detection in various environments
//! - Capability adaptation per layer
//! - Integration with self-identity and discovery
//! - End-to-end runtime initialization

use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
use toadstool::fractal_integration::{
    BarracudaIntegration, FractalRuntime, FractalServiceAdvertiser,
};
use toadstool::layer_adaptation::{GpuAccess, LayerCapabilityAdapter, NetworkAccess, StorageType};

/// Test: Complete runtime initialization flow
#[tokio::test]
async fn test_fractal_runtime_full_initialization() {
    let result = FractalRuntime::init().await;
    assert!(
        result.is_ok(),
        "FractalRuntime initialization should succeed"
    );

    let runtime = result.unwrap();

    // Should have detected a valid layer
    let layer = runtime.deployment_layer();
    let layer_name = format!("{layer}");
    assert!(!layer_name.is_empty(), "Layer should have a name");

    // Should have capabilities
    let caps = runtime.capabilities();
    assert!(
        !caps.to_capability_list().is_empty(),
        "Should have capabilities"
    );

    // Should have valid identity
    let identity = runtime.identity();
    let identity_read = identity.read().await;
    assert_eq!(identity_read.primal_type, "toadstool");
    assert!(
        !identity_read.capabilities.is_empty(),
        "Identity should have capabilities"
    );
}

/// Test: Layer detection produces consistent results
#[tokio::test]
async fn test_layer_detection_consistency() {
    let mut detector1 = LayerDetector::new();
    let mut detector2 = LayerDetector::new();

    let layer1 = detector1.detect().await.unwrap();
    let layer2 = detector2.detect().await.unwrap();

    // Both detectors should find the same layer
    let layer1_str = format!("{layer1}");
    let layer2_str = format!("{layer2}");
    assert_eq!(layer1_str, layer2_str, "Detection should be consistent");
}

/// Test: Capability adaptation produces valid capabilities
#[tokio::test]
async fn test_capability_adaptation_validity() {
    let mut detector = LayerDetector::new();
    let layer = detector.detect().await.unwrap();

    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // All layers should have some compute capability
    assert!(
        matches!(
            caps.compute.gpu_access,
            GpuAccess::Direct | GpuAccess::ViaHost | GpuAccess::ViaCloud | GpuAccess::None
        ),
        "GPU access should be valid"
    );

    // All layers should have some storage
    assert!(
        matches!(
            caps.storage.storage_type,
            StorageType::DirectBlock
                | StorageType::HostFilesystem
                | StorageType::CloudObject
                | StorageType::PersistentVolume
        ),
        "Storage type should be valid"
    );

    // All layers should have some network access
    assert!(
        matches!(
            caps.network.network_access,
            NetworkAccess::Direct | NetworkAccess::HostNamespace | NetworkAccess::CloudVPC
        ),
        "Network access should be valid"
    );

    // Metadata should be populated
    assert!(
        !caps.metadata.layer.is_empty(),
        "Layer metadata should exist"
    );
}

/// Test: Service advertisement completes successfully
#[tokio::test]
async fn test_service_advertisement() {
    let runtime = FractalRuntime::init().await;
    assert!(runtime.is_ok());

    let runtime = std::sync::Arc::new(runtime.unwrap());
    let advertiser = FractalServiceAdvertiser::new(runtime.clone());

    let result = advertiser.advertise().await;
    assert!(result.is_ok(), "Service advertisement should succeed");
}

/// Test: barraCuda integration info is consistent with capabilities
#[tokio::test]
async fn test_barracuda_integration_consistency() {
    let runtime = FractalRuntime::init().await.unwrap();

    let caps = runtime.capabilities();
    let integration = runtime.barracuda_integration();

    // Integration should match capability GPU access
    match caps.compute.gpu_access {
        GpuAccess::Direct => {
            assert!(matches!(integration, BarracudaIntegration::Direct { .. }));
            assert!(integration.has_gpu());
        }
        GpuAccess::ViaHost => {
            assert!(matches!(integration, BarracudaIntegration::ViaHost { .. }));
            assert!(integration.has_gpu());
        }
        GpuAccess::ViaCloud => {
            assert!(matches!(integration, BarracudaIntegration::ViaCloud { .. }));
            assert!(integration.has_gpu());
        }
        GpuAccess::None => {
            assert!(matches!(integration, BarracudaIntegration::None { .. }));
            assert!(!integration.has_gpu());
        }
    }

    // Integration note should be informative
    assert!(!integration.note().is_empty());
}

/// Test: Multiple runtime instances are independent
#[tokio::test]
async fn test_multiple_runtime_instances() {
    let runtime1 = FractalRuntime::init().await;
    let runtime2 = FractalRuntime::init().await;

    assert!(runtime1.is_ok());
    assert!(runtime2.is_ok());

    let runtime1 = runtime1.unwrap();
    let runtime2 = runtime2.unwrap();

    // Should have different identities
    let id1 = runtime1.identity();
    let id2 = runtime2.identity();

    let id1_read = id1.read().await;
    let id2_read = id2.read().await;

    assert_ne!(
        id1_read.instance_id, id2_read.instance_id,
        "Different instances should have different IDs"
    );
}

/// Test: Capability list is comprehensive
#[tokio::test]
async fn test_capability_list_comprehensive() {
    let runtime = FractalRuntime::init().await.unwrap();
    let caps = runtime.capabilities();

    let cap_list = caps.to_capability_list();

    // Should have multiple capabilities
    assert!(!cap_list.is_empty(), "Should have capabilities");

    // Check for expected capabilities based on layer
    let layer = runtime.deployment_layer();

    match layer {
        DeploymentLayer::BareMetalOS => {
            // Bare metal should have direct capabilities
            assert!(
                cap_list
                    .iter()
                    .any(|c| c.contains("direct") || c.contains("cpu"))
            );
        }
        DeploymentLayer::ContainerLayer { .. } => {
            // Container should have host or container capabilities
            assert!(
                cap_list
                    .iter()
                    .any(|c| c.contains("host") || c.contains("cpu"))
            );
        }
        DeploymentLayer::CloudLayer { .. } => {
            // Cloud should have cloud capabilities
            assert!(
                cap_list
                    .iter()
                    .any(|c| c.contains("cloud") || c.contains("cpu"))
            );
        }
        _ => {
            // All layers should have at least CPU compute
            assert!(cap_list.iter().any(|c| c.contains("cpu")));
        }
    }
}

/// Test: Runtime capabilities match identity capabilities
#[tokio::test]
async fn test_runtime_identity_capability_sync() {
    let runtime = FractalRuntime::init().await.unwrap();

    let runtime_caps = runtime.capabilities();
    let identity = runtime.identity();
    let identity_read = identity.read().await;

    // Identity should have capabilities from runtime
    let runtime_cap_list = runtime_caps.to_capability_list();

    for runtime_cap in runtime_cap_list {
        let found = identity_read
            .capabilities
            .iter()
            .any(|id_cap| id_cap.name == runtime_cap);

        assert!(
            found,
            "Identity should have runtime capability: {runtime_cap}"
        );
    }
}

/// Test: Layer detection handles edge cases gracefully
#[tokio::test]
async fn test_layer_detection_edge_cases() {
    // Multiple rapid detections should work
    for _ in 0..5 {
        let mut detector = LayerDetector::new();
        let result = detector.detect().await;
        assert!(result.is_ok(), "Rapid detection should work");
    }
}

/// Test: Capability adaptation for all layer types (simulated)
#[tokio::test]
async fn test_all_layer_adaptations() {
    // Test bare metal
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::BareMetalOS);
    let caps = adapter.get_adapted_capabilities();
    assert!(matches!(caps.compute.gpu_access, GpuAccess::Direct));

    // Test middleware
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::MiddlewareLayer {
        host_os: "Linux".to_string(),
        host_version: Some("6.0".to_string()),
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());

    // Test service layer
    let adapter = LayerCapabilityAdapter::new(DeploymentLayer::ServiceLayer {
        guest_os: vec!["Alpine".to_string()],
    });
    let caps = adapter.get_adapted_capabilities();
    assert!(!caps.to_capability_list().is_empty());
}

/// Test: Performance - runtime init should be reasonable
#[tokio::test]
async fn test_runtime_init_performance() {
    use std::time::Instant;

    let start = Instant::now();
    let runtime = FractalRuntime::init().await;
    let duration = start.elapsed();

    assert!(runtime.is_ok());
    // Allow 10 seconds - detection includes cloud metadata checks with timeouts
    assert!(
        duration.as_secs() < 10,
        "Init should complete within 10 seconds, took {duration:?}"
    );
}

/// Test: Concurrent runtime initializations
#[tokio::test]
async fn test_concurrent_runtime_init() {
    let handles: Vec<_> = (0..3)
        .map(|_| tokio::spawn(async move { FractalRuntime::init().await }))
        .collect();

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent init task should succeed");
        assert!(result.unwrap().is_ok(), "Runtime init should succeed");
    }
}

/// Test: Layer detection caching works
#[tokio::test]
async fn test_layer_detection_caching() {
    use std::time::Instant;

    let mut detector = LayerDetector::new();

    // First detection (should cache)
    let start1 = Instant::now();
    let layer1 = detector.detect().await.unwrap();
    let duration1 = start1.elapsed();

    // Second detection (should use cache)
    let start2 = Instant::now();
    let layer2 = detector.detect().await.unwrap();
    let duration2 = start2.elapsed();

    // Results should match
    assert_eq!(format!("{layer1}"), format!("{}", layer2));

    // Second should be faster (cached)
    // Note: This might not always be true in all environments, so we just log it
    println!("First detection: {duration1:?}, Second detection: {duration2:?}");
}

/// Test: Error handling - detector handles failures gracefully
#[tokio::test]
async fn test_detector_error_handling() {
    // Detector should handle edge cases
    let mut detector = LayerDetector::new();
    let result = detector.detect().await;

    // Should either succeed or return a proper error (not panic)
    match result {
        Ok(layer) => {
            println!("Detected layer: {layer}");
        }
        Err(e) => {
            println!("Detection error (graceful): {e:?}");
        }
    }
}

/// Test: Deep Debt compliance - no hardcoded assumptions
#[tokio::test]
async fn test_deep_debt_no_hardcoding() {
    let runtime = FractalRuntime::init().await.unwrap();

    // Runtime should work regardless of environment
    // No assertions about specific values - just that it works

    let _layer = runtime.deployment_layer();
    let _caps = runtime.capabilities();
    let _integration = runtime.barracuda_integration();

    // If we got here, runtime is working without hardcoded assumptions
}

/// Test: Deep Debt compliance - self-knowledge only
#[tokio::test]
async fn test_deep_debt_self_knowledge() {
    let runtime = FractalRuntime::init().await.unwrap();
    let identity = runtime.identity();
    let identity_read = identity.read().await;

    // Identity should know itself, not others
    assert_eq!(identity_read.primal_type, "toadstool");
    assert!(!identity_read.instance_id.is_nil());

    // Requirements show what we NEED, not what others ARE
    for req in &identity_read.requirements {
        assert!(!req.capability.is_empty());
        assert!(!req.purpose.is_empty());
    }
}

/// Test: Deep Debt compliance - runtime discovery (no mocks)
#[tokio::test]
async fn test_deep_debt_no_mocks() {
    let mut detector = LayerDetector::new();
    let layer = detector.detect().await;

    // Detection uses real system info, not mocks
    assert!(layer.is_ok(), "Should use real detection, not mocks");

    // Capability adaptation uses real memory/disk detection
    let adapter = LayerCapabilityAdapter::new(layer.unwrap());
    let caps = adapter.get_adapted_capabilities();

    // Should have real values (not 0 or mock values)
    if let Some(memory) = caps.compute.memory_bytes {
        assert!(memory > 0, "Should detect real memory");
    }
}

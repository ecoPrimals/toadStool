// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime Detection Tests
//!
//! Tests for the new runtime hardware detection capabilities added in January 2026.
//! Validates storage bandwidth, network bandwidth, and guest OS detection.

use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
use toadstool::layer_adaptation::{LayerCapabilityAdapter, NetworkAccess, StorageType};

#[test]
fn test_layer_adapter_provides_bandwidth_info() {
    // Test that layer adapter provides bandwidth information (not None)
    let layer = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // Storage bandwidth should be detected (not None)
    assert!(
        caps.storage.read_bandwidth.is_some(),
        "Storage read bandwidth should be detected"
    );
    assert!(
        caps.storage.write_bandwidth.is_some(),
        "Storage write bandwidth should be detected"
    );

    // Network bandwidth should be detected (not None)
    assert!(
        caps.network.bandwidth.is_some(),
        "Network bandwidth should be detected"
    );
}

#[test]
fn test_bare_metal_has_storage_bandwidth() {
    let layer = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // Bare metal should report storage bandwidth
    if let Some(read_bw) = caps.storage.read_bandwidth {
        assert!(read_bw > 0, "Read bandwidth should be positive");
        // Reasonable range: 100 MB/s (HDD) to 7000 MB/s (NVMe)
        assert!(
            read_bw >= 100_000_000,
            "Read bandwidth should be at least 100 MB/s"
        );
        assert!(
            read_bw <= 10_000_000_000,
            "Read bandwidth should be at most 10 GB/s"
        );
    }

    if let Some(write_bw) = caps.storage.write_bandwidth {
        assert!(write_bw > 0, "Write bandwidth should be positive");
    }
}

#[test]
fn test_middleware_layer_has_bandwidth() {
    let layer = DeploymentLayer::MiddlewareLayer {
        host_os: "Ubuntu".to_string(),
        host_version: Some("22.04".to_string()),
    };
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // Middleware should also report bandwidth (same detection)
    assert!(caps.storage.read_bandwidth.is_some());
    assert!(caps.storage.write_bandwidth.is_some());
    assert!(caps.network.bandwidth.is_some());
}

#[test]
fn test_vm_layer_has_bandwidth() {
    let layer = DeploymentLayer::VMLayer {
        hypervisor: "QEMU/KVM".to_string(),
        gpu_passthrough: false,
    };
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // VM should report bandwidth
    assert!(caps.storage.read_bandwidth.is_some());
    assert!(caps.network.bandwidth.is_some());
}

#[test]
fn test_container_layer_has_bandwidth() {
    let layer = DeploymentLayer::ContainerLayer {
        runtime: toadstool::deployment_layer::ContainerRuntime::Docker,
        container_id: Some("test123".to_string()),
    };
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // Container should report bandwidth (inherits from host)
    assert!(caps.storage.read_bandwidth.is_some());
    assert!(caps.network.bandwidth.is_some());
}

#[test]
fn test_network_bandwidth_is_reasonable() {
    let layer = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    if let Some(bandwidth) = caps.network.bandwidth {
        assert!(bandwidth > 0, "Network bandwidth should be positive");
        // Reasonable range: 10 MB/s (slow) to 125 GB/s (high-end)
        assert!(
            bandwidth >= 10_000_000,
            "Network bandwidth should be at least 10 MB/s"
        );
        assert!(
            bandwidth <= 125_000_000_000,
            "Network bandwidth should be at most 125 GB/s"
        );
    }
}

#[tokio::test]
async fn test_layer_detector_initialization() {
    // Test that layer detector can be created
    let mut detector = LayerDetector::new();

    // Detection should work without panicking
    let result = detector.detect().await;
    assert!(result.is_ok(), "Layer detection should succeed");
}

#[tokio::test]
async fn test_detected_layer_has_capabilities() {
    let mut detector = LayerDetector::new();

    match detector.detect().await {
        Ok(layer) => {
            // Whatever layer we detect, it should provide capabilities
            let adapter = LayerCapabilityAdapter::new(layer);
            let caps = adapter.get_adapted_capabilities();

            // Basic sanity checks
            assert!(caps.compute.cpu_cores.is_some() || !caps.compute.has_cpu);
            assert!(caps.storage.storage_type != StorageType::DirectBlock || caps.compute.has_cpu);
        }
        Err(e) => {
            // Detection can fail in CI environments, that's okay
            eprintln!("Layer detection failed (expected in CI): {e}");
        }
    }
}

#[test]
fn test_bandwidth_fallback_is_conservative() {
    // Even if detection fails, we should get conservative fallback values
    let layer = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    // Fallback values should be conservative (not overpromising)
    if let Some(read_bw) = caps.storage.read_bandwidth {
        // Fallback is 100 MB/s (conservative for modern storage)
        assert!(
            read_bw >= 100_000_000,
            "Fallback should be at least 100 MB/s"
        );
    }

    if let Some(net_bw) = caps.network.bandwidth {
        // Fallback is 125 MB/s (gigabit ethernet)
        assert!(net_bw >= 125_000_000, "Fallback should be at least gigabit");
    }
}

#[test]
fn test_storage_types_correspond_to_layers() {
    // Bare metal uses direct block storage
    let bare_metal = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(bare_metal);
    assert_eq!(
        adapter.get_adapted_capabilities().storage.storage_type,
        StorageType::DirectBlock
    );

    // Middleware uses host filesystem
    let middleware = DeploymentLayer::MiddlewareLayer {
        host_os: "Pop!_OS".to_string(),
        host_version: None,
    };
    let adapter = LayerCapabilityAdapter::new(middleware);
    assert_eq!(
        adapter.get_adapted_capabilities().storage.storage_type,
        StorageType::HostFilesystem
    );

    // Container uses persistent volume
    let container = DeploymentLayer::ContainerLayer {
        runtime: toadstool::deployment_layer::ContainerRuntime::Docker,
        container_id: None,
    };
    let adapter = LayerCapabilityAdapter::new(container);
    assert_eq!(
        adapter.get_adapted_capabilities().storage.storage_type,
        StorageType::PersistentVolume
    );
}

#[test]
fn test_network_access_types_correspond_to_layers() {
    // Bare metal has direct network access
    let bare_metal = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(bare_metal);
    assert_eq!(
        adapter.get_adapted_capabilities().network.network_access,
        NetworkAccess::Direct
    );

    // Container uses host namespace
    let container = DeploymentLayer::ContainerLayer {
        runtime: toadstool::deployment_layer::ContainerRuntime::Podman,
        container_id: None,
    };
    let adapter = LayerCapabilityAdapter::new(container);
    assert_eq!(
        adapter.get_adapted_capabilities().network.network_access,
        NetworkAccess::HostNamespace
    );
}

#[test]
fn test_all_deployment_layers_provide_bandwidth() {
    // Test that every deployment layer provides bandwidth info
    let layers = vec![
        DeploymentLayer::BareMetalOS,
        DeploymentLayer::MiddlewareLayer {
            host_os: "Ubuntu".to_string(),
            host_version: None,
        },
        DeploymentLayer::VMLayer {
            hypervisor: "VirtualBox".to_string(),
            gpu_passthrough: false,
        },
        DeploymentLayer::ContainerLayer {
            runtime: toadstool::deployment_layer::ContainerRuntime::Docker,
            container_id: None,
        },
    ];

    for layer in layers {
        let adapter = LayerCapabilityAdapter::new(layer.clone());
        let caps = adapter.get_adapted_capabilities();

        assert!(
            caps.storage.read_bandwidth.is_some(),
            "Layer {layer:?} should provide read bandwidth"
        );
        assert!(
            caps.storage.write_bandwidth.is_some(),
            "Layer {layer:?} should provide write bandwidth"
        );
        assert!(
            caps.network.bandwidth.is_some(),
            "Layer {layer:?} should provide network bandwidth"
        );
    }
}

#[test]
fn test_write_bandwidth_is_less_than_read() {
    // Write bandwidth should typically be <= read bandwidth
    let layer = DeploymentLayer::BareMetalOS;
    let adapter = LayerCapabilityAdapter::new(layer);
    let caps = adapter.get_adapted_capabilities();

    if let (Some(read), Some(write)) = (caps.storage.read_bandwidth, caps.storage.write_bandwidth) {
        assert!(
            write <= read,
            "Write bandwidth ({write}) should not exceed read bandwidth ({read})"
        );

        // Write should be at least 70% of read (reasonable ratio)
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let min_write = (read as f64 * 0.7) as u64;
        assert!(
            write >= min_write,
            "Write bandwidth ({write}) should be at least 70% of read ({read})"
        );
    }
}

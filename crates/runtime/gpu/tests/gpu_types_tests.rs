// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for GPU runtime types

use toadstool_runtime_gpu::types::*;

// ============================================================================
// GpuFramework Tests
// ============================================================================

#[test]
fn test_gpu_framework_webgpu() {
    let framework = GpuFramework::WebGpu;
    assert_eq!(framework.name(), "WebGPU");
    assert!(framework.is_universal());
}

#[test]
fn test_gpu_framework_vulkan() {
    let framework = GpuFramework::Vulkan;
    assert_eq!(framework.name(), "Vulkan");
    assert!(framework.is_universal());
}

#[test]
fn test_gpu_framework_opencl() {
    let framework = GpuFramework::OpenCl;
    assert_eq!(framework.name(), "OpenCL");
    assert!(framework.is_universal());
}

#[test]
fn test_gpu_framework_cuda() {
    let framework = GpuFramework::Cuda;
    assert_eq!(framework.name(), "CUDA");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_metal() {
    let framework = GpuFramework::Metal;
    assert_eq!(framework.name(), "Metal");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_rocm() {
    let framework = GpuFramework::Rocm;
    assert_eq!(framework.name(), "ROCm");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_directcompute() {
    let framework = GpuFramework::DirectCompute;
    assert_eq!(framework.name(), "DirectCompute");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_custom() {
    let framework = GpuFramework::Custom("MyFramework".to_string());
    assert_eq!(framework.name(), "MyFramework");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_webgpu_compatibility() {
    let framework = GpuFramework::WebGpu;
    let compat = framework.platform_compatibility();

    assert!(compat.contains(&"Windows"));
    assert!(compat.contains(&"macOS"));
    assert!(compat.contains(&"Linux"));
    assert!(compat.contains(&"Web"));
}

#[test]
fn test_gpu_framework_vulkan_compatibility() {
    let framework = GpuFramework::Vulkan;
    let compat = framework.platform_compatibility();

    assert!(compat.contains(&"Windows"));
    assert!(compat.contains(&"macOS"));
    assert!(compat.contains(&"Linux"));
    assert!(compat.contains(&"Android"));
}

#[test]
fn test_gpu_framework_cuda_compatibility() {
    let framework = GpuFramework::Cuda;
    let compat = framework.platform_compatibility();

    assert!(compat.contains(&"Windows"));
    assert!(compat.contains(&"Linux"));
    assert!(!compat.contains(&"macOS"));
}

#[test]
fn test_gpu_framework_metal_compatibility() {
    let framework = GpuFramework::Metal;
    let compat = framework.platform_compatibility();

    assert!(compat.contains(&"macOS"));
    assert!(compat.contains(&"iOS"));
    assert!(!compat.contains(&"Windows"));
}

#[test]
fn test_gpu_framework_rocm_compatibility() {
    let framework = GpuFramework::Rocm;
    let compat = framework.platform_compatibility();

    assert!(compat.contains(&"Linux"));
    assert!(!compat.contains(&"Windows"));
    assert!(!compat.contains(&"macOS"));
}

#[test]
fn test_gpu_framework_clone() {
    let framework1 = GpuFramework::WebGpu;
    let framework2 = framework1.clone();

    assert_eq!(framework1, framework2);
}

#[test]
fn test_gpu_framework_equality() {
    assert_eq!(GpuFramework::WebGpu, GpuFramework::WebGpu);
    assert_eq!(GpuFramework::Vulkan, GpuFramework::Vulkan);
    assert_ne!(GpuFramework::WebGpu, GpuFramework::Vulkan);
}

#[test]
fn test_gpu_framework_serialization() {
    let framework = GpuFramework::Cuda;
    let serialized = serde_json::to_string(&framework).unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_gpu_framework_custom_serialization() {
    let framework = GpuFramework::Custom("Test".to_string());
    let serialized = serde_json::to_string(&framework).unwrap();
    let deserialized: GpuFramework = serde_json::from_str(&serialized).unwrap();

    assert_eq!(framework, deserialized);
}

// ============================================================================
// DeviceId Tests
// ============================================================================

#[test]
fn test_device_id_creation() {
    let device_id = DeviceId::new(GpuFramework::Vulkan, 0, "device-001".to_string());

    assert_eq!(device_id.framework, GpuFramework::Vulkan);
    assert_eq!(device_id.device_index, 0);
    assert_eq!(device_id.uuid, "device-001");
}

#[test]
fn test_device_id_with_different_indices() {
    let indices = vec![0, 1, 5, 10, 100];

    for index in indices {
        let device_id = DeviceId::new(GpuFramework::Cuda, index, format!("device-{index}"));

        assert_eq!(device_id.device_index, index);
    }
}

#[test]
fn test_device_id_clone() {
    let device_id1 = DeviceId::new(GpuFramework::WebGpu, 0, "test-device".to_string());

    let device_id2 = device_id1.clone();

    assert_eq!(device_id1, device_id2);
}

#[test]
fn test_device_id_equality() {
    let device_id1 = DeviceId::new(GpuFramework::Metal, 1, "dev-123".to_string());

    let device_id2 = DeviceId::new(GpuFramework::Metal, 1, "dev-123".to_string());

    assert_eq!(device_id1, device_id2);
}

#[test]
fn test_device_id_serialization() {
    let device_id = DeviceId::new(GpuFramework::OpenCl, 2, "opencl-device".to_string());

    let serialized = serde_json::to_string(&device_id).unwrap();
    let deserialized: DeviceId = serde_json::from_str(&serialized).unwrap();

    assert_eq!(device_id, deserialized);
}

// ============================================================================
// DeviceInfo Tests
// ============================================================================

#[test]
fn test_device_info_creation() {
    let info = DeviceInfo {
        name: "NVIDIA RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "535.104".to_string(),
        architecture: "Ada Lovelace".to_string(),
        physical_location: Some("PCIe Slot 1".to_string()),
    };

    assert_eq!(info.name, "NVIDIA RTX 4090");
    assert_eq!(info.vendor, "NVIDIA");
    assert!(info.physical_location.is_some());
}

#[test]
fn test_device_info_without_location() {
    let info = DeviceInfo {
        name: "Intel UHD Graphics".to_string(),
        vendor: "Intel".to_string(),
        device_type: DeviceType::IntegratedGpu,
        driver_version: "30.0.101.1960".to_string(),
        architecture: "Gen12".to_string(),
        physical_location: None,
    };

    assert!(info.physical_location.is_none());
}

#[test]
fn test_device_info_clone() {
    let info1 = DeviceInfo {
        name: "AMD RX 7900 XTX".to_string(),
        vendor: "AMD".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "23.20.01.01".to_string(),
        architecture: "RDNA3".to_string(),
        physical_location: Some("PCIe Slot 2".to_string()),
    };

    let info2 = info1.clone();

    assert_eq!(info1.name, info2.name);
    assert_eq!(info1.vendor, info2.vendor);
}

#[test]
fn test_device_info_serialization() {
    let info = DeviceInfo {
        name: "Apple M2 Pro".to_string(),
        vendor: "Apple".to_string(),
        device_type: DeviceType::Apu,
        driver_version: "14.2.1".to_string(),
        architecture: "M2 Pro".to_string(),
        physical_location: None,
    };

    let serialized = serde_json::to_string(&info).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// DeviceType Tests
// ============================================================================

#[test]
fn test_device_type_discrete_gpu() {
    let device_type = DeviceType::DiscreteGpu;
    assert!(matches!(device_type, DeviceType::DiscreteGpu));
}

#[test]
fn test_device_type_integrated_gpu() {
    let device_type = DeviceType::IntegratedGpu;
    assert!(matches!(device_type, DeviceType::IntegratedGpu));
}

#[test]
fn test_device_type_apu() {
    let device_type = DeviceType::Apu;
    assert!(matches!(device_type, DeviceType::Apu));
}

#[test]
fn test_device_type_compute_only() {
    let device_type = DeviceType::ComputeOnly;
    assert!(matches!(device_type, DeviceType::ComputeOnly));
}

#[test]
fn test_device_type_virtual_gpu() {
    let device_type = DeviceType::VirtualGpu;
    assert!(matches!(device_type, DeviceType::VirtualGpu));
}

#[test]
fn test_device_type_clone() {
    let device_type1 = DeviceType::DiscreteGpu;
    let device_type2 = device_type1.clone();

    match (device_type1, device_type2) {
        (DeviceType::DiscreteGpu, DeviceType::DiscreteGpu) => {} // Clone successful
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_device_type_serialization() {
    let device_type = DeviceType::Apu;
    let serialized = serde_json::to_string(&device_type).unwrap();
    assert!(!serialized.is_empty());
}

#[test]
fn test_all_device_types() {
    let types = [
        DeviceType::DiscreteGpu,
        DeviceType::IntegratedGpu,
        DeviceType::Apu,
        DeviceType::ComputeOnly,
        DeviceType::VirtualGpu,
    ];

    assert_eq!(types.len(), 5);
}

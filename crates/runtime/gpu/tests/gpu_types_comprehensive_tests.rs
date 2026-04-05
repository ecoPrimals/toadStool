// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for GPU runtime types

use toadstool_runtime_gpu::types::{DeviceId, DeviceInfo, DeviceType, GpuFramework};

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
    let framework = GpuFramework::Custom("MyCustomGPU".to_string());
    assert_eq!(framework.name(), "MyCustomGPU");
    assert!(!framework.is_universal());
}

#[test]
fn test_gpu_framework_webgpu_platform_compatibility() {
    let framework = GpuFramework::WebGpu;
    let platforms = framework.platform_compatibility();
    assert!(platforms.contains(&"Windows"));
    assert!(platforms.contains(&"macOS"));
    assert!(platforms.contains(&"Linux"));
    assert!(platforms.contains(&"Web"));
}

#[test]
fn test_gpu_framework_vulkan_platform_compatibility() {
    let framework = GpuFramework::Vulkan;
    let platforms = framework.platform_compatibility();
    assert!(platforms.contains(&"Windows"));
    assert!(platforms.contains(&"macOS"));
    assert!(platforms.contains(&"Linux"));
    assert!(platforms.contains(&"Android"));
}

#[test]
fn test_gpu_framework_cuda_platform_compatibility() {
    let framework = GpuFramework::Cuda;
    let platforms = framework.platform_compatibility();
    assert!(platforms.contains(&"Windows"));
    assert!(platforms.contains(&"Linux"));
    assert!(!platforms.contains(&"macOS"));
}

#[test]
fn test_gpu_framework_metal_platform_compatibility() {
    let framework = GpuFramework::Metal;
    let platforms = framework.platform_compatibility();
    assert!(platforms.contains(&"macOS"));
    assert!(platforms.contains(&"iOS"));
    assert!(!platforms.contains(&"Windows"));
}

#[test]
fn test_gpu_framework_rocm_platform_compatibility() {
    let framework = GpuFramework::Rocm;
    let platforms = framework.platform_compatibility();
    assert!(platforms.contains(&"Linux"));
    assert_eq!(platforms.len(), 1);
}

#[test]
fn test_gpu_framework_serialization() {
    let framework = GpuFramework::Cuda;
    let json = serde_json::to_string(&framework).unwrap();
    assert!(!json.is_empty());

    let deserialized: GpuFramework = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, framework);
}

#[test]
fn test_gpu_framework_equality() {
    let fw1 = GpuFramework::Vulkan;
    let fw2 = GpuFramework::Vulkan;
    let fw3 = GpuFramework::Cuda;

    assert_eq!(fw1, fw2);
    assert_ne!(fw1, fw3);
}

#[test]
fn test_gpu_framework_clone() {
    let framework = GpuFramework::WebGpu;
    let cloned = framework.clone();
    assert_eq!(framework, cloned);
}

// ============================================================================
// DeviceId Tests
// ============================================================================

#[test]
fn test_device_id_creation() {
    let device_id = DeviceId::new(GpuFramework::Cuda, 0, "gpu-uuid-123".to_string());

    assert_eq!(device_id.device_index, 0);
    assert_eq!(device_id.uuid, "gpu-uuid-123");
    assert_eq!(device_id.framework, GpuFramework::Cuda);
}

#[test]
fn test_device_id_multiple_devices() {
    let device1 = DeviceId::new(GpuFramework::Cuda, 0, "uuid-1".to_string());
    let device2 = DeviceId::new(GpuFramework::Cuda, 1, "uuid-2".to_string());

    assert_ne!(device1.device_index, device2.device_index);
    assert_ne!(device1.uuid, device2.uuid);
}

#[test]
fn test_device_id_serialization() {
    let device_id = DeviceId::new(GpuFramework::Vulkan, 2, "vulkan-device".to_string());

    let json = serde_json::to_string(&device_id).unwrap();
    assert!(!json.is_empty());

    let deserialized: DeviceId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.device_index, 2);
}

#[test]
fn test_device_id_equality() {
    let id1 = DeviceId::new(GpuFramework::WebGpu, 0, "id1".to_string());
    let id2 = DeviceId::new(GpuFramework::WebGpu, 0, "id1".to_string());
    let id3 = DeviceId::new(GpuFramework::WebGpu, 1, "id2".to_string());

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn test_device_id_clone() {
    let device_id = DeviceId::new(GpuFramework::Metal, 0, "metal-0".to_string());
    let cloned = device_id.clone();
    assert_eq!(device_id, cloned);
}

// ============================================================================
// DeviceInfo Tests
// ============================================================================

#[test]
fn test_device_info_creation() {
    let info = DeviceInfo {
        name: "NVIDIA GeForce RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "535.104.05".to_string(),
        architecture: "Ada Lovelace".to_string(),
        physical_location: Some("PCIe 4.0 x16".to_string()),
    };

    assert_eq!(info.name, "NVIDIA GeForce RTX 4090");
    assert_eq!(info.vendor, "NVIDIA");
}

#[test]
fn test_device_info_no_physical_location() {
    let info = DeviceInfo {
        name: "Intel UHD Graphics".to_string(),
        vendor: "Intel".to_string(),
        device_type: DeviceType::IntegratedGpu,
        driver_version: "30.0.101.1191".to_string(),
        architecture: "Gen12".to_string(),
        physical_location: None,
    };

    assert!(info.physical_location.is_none());
}

#[test]
fn test_device_info_serialization() {
    let info = DeviceInfo {
        name: "AMD Radeon RX 7900 XTX".to_string(),
        vendor: "AMD".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "23.11.1".to_string(),
        architecture: "RDNA 3".to_string(),
        physical_location: Some("PCIe 5.0 x16".to_string()),
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(!json.is_empty());

    let deserialized: DeviceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "AMD Radeon RX 7900 XTX");
}

#[test]
fn test_device_info_clone() {
    let info = DeviceInfo {
        name: "Test GPU".to_string(),
        vendor: "Test Vendor".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "1.0.0".to_string(),
        architecture: "Test Arch".to_string(),
        physical_location: None,
    };

    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
}

// ============================================================================
// DeviceType Tests
// ============================================================================

#[test]
fn test_device_type_discrete_gpu() {
    let device_type = DeviceType::DiscreteGpu;
    let json = serde_json::to_string(&device_type).unwrap();
    let deserialized: DeviceType = serde_json::from_str(&json).unwrap();

    match deserialized {
        DeviceType::DiscreteGpu => (),
        _ => panic!("Expected DiscreteGpu"),
    }
}

#[test]
fn test_device_type_integrated_gpu() {
    let device_type = DeviceType::IntegratedGpu;
    let json = serde_json::to_string(&device_type).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_device_type_apu() {
    let device_type = DeviceType::Apu;
    let json = serde_json::to_string(&device_type).unwrap();
    let deserialized: DeviceType = serde_json::from_str(&json).unwrap();

    match deserialized {
        DeviceType::Apu => (),
        _ => panic!("Expected Apu"),
    }
}

#[test]
fn test_device_type_compute_only() {
    let device_type = DeviceType::ComputeOnly;
    let json = serde_json::to_string(&device_type).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_device_type_virtual_gpu() {
    let device_type = DeviceType::VirtualGpu;
    let json = serde_json::to_string(&device_type).unwrap();
    let deserialized: DeviceType = serde_json::from_str(&json).unwrap();

    match deserialized {
        DeviceType::VirtualGpu => (),
        _ => panic!("Expected VirtualGpu"),
    }
}

#[test]
fn test_device_type_clone() {
    let device_type = DeviceType::DiscreteGpu;
    let cloned = device_type;

    match cloned {
        DeviceType::DiscreteGpu => (),
        _ => panic!("Expected DiscreteGpu"),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_device_configuration() {
    let framework = GpuFramework::Cuda;
    let device_id = DeviceId::new(framework.clone(), 0, "cuda-device-0".to_string());
    let device_info = DeviceInfo {
        name: "NVIDIA RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "535.104.05".to_string(),
        architecture: "Ada Lovelace".to_string(),
        physical_location: Some("PCIe 4.0".to_string()),
    };

    assert_eq!(device_id.framework, framework);
    assert!(framework.platform_compatibility().contains(&"Linux"));
    assert_eq!(device_info.vendor, "NVIDIA");
}

#[test]
fn test_multi_framework_support() {
    let frameworks = [
        GpuFramework::WebGpu,
        GpuFramework::Vulkan,
        GpuFramework::OpenCl,
        GpuFramework::Cuda,
        GpuFramework::Metal,
    ];

    let universal_count = frameworks.iter().filter(|f| f.is_universal()).count();
    assert_eq!(universal_count, 3);
}

#[test]
fn test_device_identification_workflow() {
    let device_id = DeviceId::new(GpuFramework::Vulkan, 1, "vulkan-gpu-1".to_string());

    let device_info = DeviceInfo {
        name: "AMD Radeon RX 7900 XT".to_string(),
        vendor: "AMD".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "23.11.1".to_string(),
        architecture: "RDNA 3".to_string(),
        physical_location: Some("PCIe 5.0 x16".to_string()),
    };

    assert_eq!(device_id.framework, GpuFramework::Vulkan);
    assert_eq!(device_info.vendor, "AMD");
}

#[test]
fn test_framework_platform_coverage() {
    let webgpu = GpuFramework::WebGpu;
    let cuda = GpuFramework::Cuda;
    let metal = GpuFramework::Metal;

    assert!(webgpu.platform_compatibility().len() > cuda.platform_compatibility().len());
    assert!(metal.platform_compatibility().contains(&"macOS"));
}

#[test]
fn test_device_info_amd_gpu() {
    let info = DeviceInfo {
        name: "AMD Radeon Pro W7900".to_string(),
        vendor: "AMD".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "23.Q4".to_string(),
        architecture: "RDNA 3".to_string(),
        physical_location: Some("PCIe 4.0 x16".to_string()),
    };

    assert_eq!(info.vendor, "AMD");
    assert!(info.architecture.contains("RDNA"));
}

#[test]
fn test_device_info_intel_igpu() {
    let info = DeviceInfo {
        name: "Intel Iris Xe Graphics".to_string(),
        vendor: "Intel".to_string(),
        device_type: DeviceType::IntegratedGpu,
        driver_version: "31.0.101.4502".to_string(),
        architecture: "Gen12.7".to_string(),
        physical_location: None,
    };

    assert!(matches!(info.device_type, DeviceType::IntegratedGpu));
    assert!(info.physical_location.is_none());
}

#[test]
fn test_device_id_hash_equality() {
    use std::collections::HashMap;

    let id1 = DeviceId::new(GpuFramework::Cuda, 0, "id1".to_string());
    let id2 = DeviceId::new(GpuFramework::Cuda, 0, "id1".to_string());
    let mut map = HashMap::new();
    map.insert(id1, "device1");

    assert_eq!(map.get(&id2), Some(&"device1"));
}

#[test]
fn test_gpu_framework_directcompute_platform_compatibility() {
    let framework = GpuFramework::DirectCompute;
    let platforms = framework.platform_compatibility();

    assert_eq!(platforms.len(), 1);
    assert!(platforms.contains(&"Windows"));
}

#[test]
fn test_gpu_framework_custom_name() {
    let custom1 = GpuFramework::Custom("CustomGPU1".to_string());
    let custom2 = GpuFramework::Custom("CustomGPU2".to_string());

    assert_ne!(custom1.name(), custom2.name());
    assert_eq!(custom1.name(), "CustomGPU1");
}

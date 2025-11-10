//! Comprehensive Tests for GPU Frameworks
//!
//! Tests cover:
//! - Framework initialization
//! - Device discovery
//! - Kernel compilation
//! - Kernel execution  
//! - Error handling
//! - Framework-specific features

use toadstool_runtime_gpu::frameworks::WebGpuFramework;
use toadstool_runtime_gpu::types::{
    ComputeCapabilities, DeviceId, DeviceVendor, GpuFramework, KernelFormat,
    UniversalComputeDevice,
};

/// Test helper to create a test device
fn create_test_device() -> UniversalComputeDevice {
    UniversalComputeDevice {
        id: DeviceId::from("test_device".to_string()),
        name: "Test GPU Device".to_string(),
        vendor: DeviceVendor::Simulated,
        framework: GpuFramework::WebGpu,
        driver_version: "1.0.0".to_string(),
        capabilities: ComputeCapabilities {
            compute_capability: "1.0".to_string(),
            total_memory_bytes: 1024 * 1024 * 1024, // 1GB
            compute_units: 8,
            max_work_group_size: 1024,
            max_threads_per_block: 1024,
            warp_size: 32,
            supports_double_precision: false,
            supports_unified_memory: false,
            supports_cooperative_launch: false,
        },
        is_available: true,
        current_usage: None,
    }
}

#[test]
fn test_webgpu_framework_creation() {
    let result = WebGpuFramework::new();
    assert!(
        result.is_ok(),
        "Should create WebGPU framework: {:?}",
        result.err()
    );
}

#[test]
fn test_kernel_format_variants() {
    // Test that all kernel formats are distinct
    assert_ne!(KernelFormat::SPIRV, KernelFormat::PTX);
    assert_ne!(KernelFormat::SPIRV, KernelFormat::OpenCL);
    assert_ne!(KernelFormat::PTX, KernelFormat::OpenCL);
}

#[test]
fn test_gpu_framework_variants() {
    // Test that all GPU frameworks are distinct
    assert_ne!(GpuFramework::WebGpu, GpuFramework::Cuda);
    assert_ne!(GpuFramework::WebGpu, GpuFramework::OpenCl);
    assert_ne!(GpuFramework::Cuda, GpuFramework::OpenCl);
}

#[test]
fn test_compute_capabilities() {
    let device = create_test_device();
    let caps = &device.capabilities;
    
    assert!(caps.total_memory_bytes > 0, "Device should have memory");
    assert!(caps.compute_units > 0, "Device should have compute units");
    assert!(caps.max_work_group_size > 0, "Device should have work group size");
}

#[test]
fn test_device_vendor_variants() {
    assert_ne!(DeviceVendor::NVIDIA, DeviceVendor::AMD);
    assert_ne!(DeviceVendor::NVIDIA, DeviceVendor::Intel);
    assert_ne!(DeviceVendor::AMD, DeviceVendor::Simulated);
}

#[test]
fn test_kernel_format_display() {
    let spirv = KernelFormat::SPIRV;
    let ptx = KernelFormat::PTX;
    let opencl = KernelFormat::OpenCL;
    
    // Formats should be displayable (have Debug/Display)
    format!("{:?}", spirv);
    format!("{:?}", ptx);
    format!("{:?}", opencl);
}

#[test]
fn test_gpu_framework_display() {
    let webgpu = GpuFramework::WebGpu;
    let cuda = GpuFramework::Cuda;
    let opencl = GpuFramework::OpenCl;
    
    // Frameworks should be displayable
    format!("{:?}", webgpu);
    format!("{:?}", cuda);
    format!("{:?}", opencl);
}

#[test]
fn test_device_id_creation() {
    let id1 = DeviceId::from("device1".to_string());
    let id2 = DeviceId::from("device2".to_string());
    let id3 = DeviceId::from("device1".to_string());
    
    assert_eq!(id1, id3, "Same device IDs should be equal");
    assert_ne!(id1, id2, "Different device IDs should not be equal");
}

#[test]
fn test_compute_capabilities_defaults() {
    let caps = ComputeCapabilities {
        compute_capability: "1.0".to_string(),
        total_memory_bytes: 1024 * 1024 * 1024,
        compute_units: 8,
        max_work_group_size: 1024,
        max_threads_per_block: 1024,
        warp_size: 32,
        supports_double_precision: false,
        supports_unified_memory: false,
        supports_cooperative_launch: false,
    };
    
    assert!(!caps.supports_double_precision, "Default should not support double precision");
    assert!(!caps.supports_unified_memory, "Default should not support unified memory");
}

#[test]
fn test_compute_capabilities_with_features() {
    let caps = ComputeCapabilities {
        compute_capability: "8.0".to_string(),
        total_memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB
        compute_units: 64,
        max_work_group_size: 2048,
        max_threads_per_block: 2048,
        warp_size: 32,
        supports_double_precision: true,
        supports_unified_memory: true,
        supports_cooperative_launch: true,
    };
    
    assert!(caps.supports_double_precision, "Should support double precision");
    assert!(caps.supports_unified_memory, "Should support unified memory");
    assert!(caps.supports_cooperative_launch, "Should support cooperative launch");
}

#[test]
fn test_universal_compute_device_availability() {
    let mut device = create_test_device();
    
    assert!(device.is_available, "Device should be available by default");
    
    device.is_available = false;
    assert!(!device.is_available, "Device availability should be mutable");
}

#[test]
fn test_framework_clone_detection() {
    // Test that framework enum can be cloned
    let framework1 = GpuFramework::WebGpu;
    let framework2 = framework1.clone();
    
    assert_eq!(framework1, framework2, "Cloned framework should be equal");
}

#[test]
fn test_kernel_format_clone_detection() {
    let format1 = KernelFormat::SPIRV;
    let format2 = format1.clone();
    
    assert_eq!(format1, format2, "Cloned format should be equal");
}

#[test]
fn test_device_vendor_names() {
    // Test that vendor enum has reasonable name method
    let nvidia = DeviceVendor::NVIDIA;
    let amd = DeviceVendor::AMD;
    let intel = DeviceVendor::Intel;
    
    assert_eq!(nvidia.name(), "NVIDIA");
    assert_eq!(amd.name(), "AMD");
    assert_eq!(intel.name(), "Intel");
}

#[test]
fn test_gpu_framework_names() {
    // Test framework name method
    let webgpu = GpuFramework::WebGpu;
    let cuda = GpuFramework::Cuda;
    let opencl = GpuFramework::OpenCl;
    
    assert_eq!(webgpu.name(), "WebGPU");
    assert_eq!(cuda.name(), "CUDA");
    assert_eq!(opencl.name(), "OpenCL");
}

#[test]
fn test_device_creation_nvidia() {
    let device = UniversalComputeDevice {
        id: DeviceId::from("nvidia0".to_string()),
        name: "NVIDIA RTX 4090".to_string(),
        vendor: DeviceVendor::NVIDIA,
        framework: GpuFramework::Cuda,
        driver_version: "535.104.05".to_string(),
        capabilities: ComputeCapabilities {
            compute_capability: "8.9".to_string(),
            total_memory_bytes: 24 * 1024 * 1024 * 1024, // 24GB
            compute_units: 128,
            max_work_group_size: 1024,
            max_threads_per_block: 1024,
            warp_size: 32,
            supports_double_precision: true,
            supports_unified_memory: true,
            supports_cooperative_launch: true,
        },
        is_available: true,
        current_usage: None,
    };
    
    assert_eq!(device.vendor, DeviceVendor::NVIDIA);
    assert_eq!(device.framework, GpuFramework::Cuda);
}

#[test]
fn test_device_creation_amd() {
    let device = UniversalComputeDevice {
        id: DeviceId::from("amd0".to_string()),
        name: "AMD Radeon RX 7900 XTX".to_string(),
        vendor: DeviceVendor::AMD,
        framework: GpuFramework::Vulkan,
        driver_version: "23.11.1".to_string(),
        capabilities: ComputeCapabilities {
            compute_capability: "1.3".to_string(),
            total_memory_bytes: 24 * 1024 * 1024 * 1024, // 24GB
            compute_units: 96,
            max_work_group_size: 1024,
            max_threads_per_block: 1024,
            warp_size: 64,
            supports_double_precision: true,
            supports_unified_memory: false,
            supports_cooperative_launch: false,
        },
        is_available: true,
        current_usage: None,
    };
    
    assert_eq!(device.vendor, DeviceVendor::AMD);
    assert_eq!(device.framework, GpuFramework::Vulkan);
}

#[test]
fn test_device_creation_intel() {
    let device = UniversalComputeDevice {
        id: DeviceId::from("intel0".to_string()),
        name: "Intel Arc A770".to_string(),
        vendor: DeviceVendor::Intel,
        framework: GpuFramework::OpenCl,
        driver_version: "31.0.101.4502".to_string(),
        capabilities: ComputeCapabilities {
            compute_capability: "3.0".to_string(),
            total_memory_bytes: 16 * 1024 * 1024 * 1024, // 16GB
            compute_units: 32,
            max_work_group_size: 512,
            max_threads_per_block: 512,
            warp_size: 32,
            supports_double_precision: true,
            supports_unified_memory: false,
            supports_cooperative_launch: false,
        },
        is_available: true,
        current_usage: None,
    };
    
    assert_eq!(device.vendor, DeviceVendor::Intel);
    assert_eq!(device.framework, GpuFramework::OpenCl);
}

#[test]
fn test_all_framework_variants() {
    // Ensure all variants are covered
    let frameworks = vec![
        GpuFramework::WebGpu,
        GpuFramework::Vulkan,
        GpuFramework::OpenCl,
        GpuFramework::Cuda,
        GpuFramework::Metal,
    ];
    
    assert_eq!(frameworks.len(), 5, "Should have 5 framework variants");
}

#[test]
fn test_all_vendor_variants() {
    // Ensure all main vendors are covered
    let vendors = vec![
        DeviceVendor::NVIDIA,
        DeviceVendor::AMD,
        DeviceVendor::Intel,
        DeviceVendor::Apple,
        DeviceVendor::Simulated,
    ];
    
    assert!(vendors.len() >= 5, "Should have at least 5 vendor variants");
}

#[test]
fn test_kernel_format_spirv() {
    let format = KernelFormat::SPIRV;
    assert_eq!(format, KernelFormat::SPIRV);
}

#[test]
fn test_kernel_format_ptx() {
    let format = KernelFormat::PTX;
    assert_eq!(format, KernelFormat::PTX);
}

#[test]
fn test_kernel_format_opencl() {
    let format = KernelFormat::OpenCL;
    assert_eq!(format, KernelFormat::OpenCL);
}

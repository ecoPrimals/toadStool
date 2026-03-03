// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simplified Tests for GPU Frameworks
//!
//! Tests cover basic framework functionality that matches the actual API

use std::collections::HashMap;
use std::sync::Arc;
use toadstool_runtime_gpu::frameworks::WebGpuFramework;
use toadstool_runtime_gpu::types::{
    DataType, DeviceCapabilities, DeviceId, DeviceInfo, DeviceType, DeviceUsage, GpuFramework,
    KernelFormat, PerformanceCharacteristics, UniversalComputeDevice,
};
use tokio::sync::RwLock;

/// Test helper to create a test device
fn create_test_device() -> UniversalComputeDevice {
    let device_info = DeviceInfo {
        name: "Test GPU Device".to_string(),
        vendor: "Test Vendor".to_string(),
        device_type: DeviceType::DiscreteGpu,
        driver_version: "1.0.0".to_string(),
        architecture: "Test Arch".to_string(),
        physical_location: None,
    };

    let capabilities = DeviceCapabilities {
        compute_capability: "1.0".to_string(),
        total_memory_bytes: 1024 * 1024 * 1024, // 1GB
        memory_bandwidth_gbps: 100.0,
        compute_units: 8,
        max_work_group_size: (1024, 1024, 64),
        supported_data_types: vec![DataType::Float32, DataType::Int32],
        extensions: HashMap::new(),
        performance: PerformanceCharacteristics {
            peak_gflops_fp32: 1000.0,
            peak_gflops_fp64: Some(500.0),
            peak_gflops_fp16: Some(2000.0),
            peak_memory_bandwidth_utilization: 0.8,
            typical_power_watts: 150.0,
            max_power_watts: 200.0,
        },
    };

    let usage = DeviceUsage::default();

    UniversalComputeDevice {
        id: DeviceId::new(GpuFramework::WebGpu, 0, "test_device".to_string()),
        info: device_info,
        capabilities,
        usage: Arc::new(RwLock::new(usage)),
        framework_handle: None,
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
fn test_compute_capabilities() {
    let device = create_test_device();
    let caps = &device.capabilities;

    assert!(caps.total_memory_bytes > 0, "Device should have memory");
    assert!(caps.compute_units > 0, "Device should have compute units");
    assert!(
        caps.max_work_group_size.0 > 0,
        "Device should have work group size"
    );
}

#[test]
fn test_gpu_framework_name() {
    let webgpu = GpuFramework::WebGpu;
    let cuda = GpuFramework::Cuda;
    let opencl = GpuFramework::OpenCl;

    assert_eq!(webgpu.name(), "WebGPU");
    assert_eq!(cuda.name(), "CUDA");
    assert_eq!(opencl.name(), "OpenCL");
}

#[test]
fn test_gpu_framework_is_universal() {
    let webgpu = GpuFramework::WebGpu;
    let cuda = GpuFramework::Cuda;
    let vulkan = GpuFramework::Vulkan;

    assert!(webgpu.is_universal());
    assert!(!cuda.is_universal());
    assert!(vulkan.is_universal());
}

#[test]
fn test_gpu_framework_platform_compatibility() {
    let webgpu = GpuFramework::WebGpu;
    let platforms = webgpu.platform_compatibility();

    assert!(!platforms.is_empty());
    assert!(platforms.contains(&"Windows"));
    assert!(platforms.contains(&"Linux"));
}

#[test]
fn test_device_id_creation() {
    let id = DeviceId::new(GpuFramework::WebGpu, 0, "test".to_string());
    assert_eq!(id.device_index, 0);
    assert_eq!(id.uuid, "test");
}

#[test]
fn test_device_info_structure() {
    let device = create_test_device();
    assert_eq!(device.info.name, "Test GPU Device");
    assert_eq!(device.info.vendor, "Test Vendor");
    assert_eq!(device.info.driver_version, "1.0.0");
}

#[test]
fn test_device_capabilities_memory() {
    let device = create_test_device();
    assert_eq!(device.capabilities.total_memory_bytes, 1024 * 1024 * 1024);
    assert_eq!(device.capabilities.memory_bandwidth_gbps, 100.0);
}

#[test]
fn test_device_capabilities_compute_units() {
    let device = create_test_device();
    assert_eq!(device.capabilities.compute_units, 8);
}

#[test]
fn test_device_capabilities_work_group_size() {
    let device = create_test_device();
    let (x, y, z) = device.capabilities.max_work_group_size;
    assert_eq!(x, 1024);
    assert_eq!(y, 1024);
    assert_eq!(z, 64);
}

#[test]
fn test_performance_characteristics() {
    let device = create_test_device();
    let perf = &device.capabilities.performance;

    assert_eq!(perf.peak_gflops_fp32, 1000.0);
    assert_eq!(perf.peak_gflops_fp64, Some(500.0));
    assert_eq!(perf.typical_power_watts, 150.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_device_usage_default() {
    let device = create_test_device();
    let usage = device.usage.read().await;

    assert_eq!(usage.gpu_utilization_percent, 0.0);
    assert_eq!(usage.memory_utilization_percent, 0.0);
    assert_eq!(usage.memory_used_bytes, 0);
}

#[test]
fn test_device_clone() {
    let device1 = create_test_device();
    let device2 = device1.clone();

    assert_eq!(device1.info.name, device2.info.name);
    assert_eq!(
        device1.capabilities.compute_units,
        device2.capabilities.compute_units
    );
}

#[test]
fn test_data_type_variants() {
    // Just verify all variants exist
    let _f32 = DataType::Float32;
    let _f64 = DataType::Float64;
    let _i32 = DataType::Int32;
    let _u32 = DataType::UInt32;
}

#[test]
fn test_kernel_format_debug() {
    // Verify Debug trait works
    let _ = format!("{:?}", KernelFormat::Spirv);
    let _ = format!("{:?}", KernelFormat::CudaC);
    let _ = format!("{:?}", KernelFormat::OpenClC);
}

#[test]
fn test_device_type_debug() {
    // Verify Debug trait works
    let _ = format!("{:?}", DeviceType::DiscreteGpu);
    let _ = format!("{:?}", DeviceType::IntegratedGpu);
    let _ = format!("{:?}", DeviceType::ComputeOnly);
}

#[test]
fn test_gpu_framework_custom() {
    let custom = GpuFramework::Custom("MyFramework".to_string());
    assert_eq!(custom.name(), "MyFramework");
}

#[test]
fn test_device_capabilities_extensions() {
    let device = create_test_device();
    assert!(device.capabilities.extensions.is_empty());
}

#[test]
fn test_device_capabilities_supported_data_types() {
    let device = create_test_device();
    assert_eq!(device.capabilities.supported_data_types.len(), 2);
}

#[test]
fn test_device_info_optional_fields() {
    let device = create_test_device();
    assert!(device.info.physical_location.is_none());
}

#[test]
fn test_performance_optional_precision() {
    let device = create_test_device();
    let perf = &device.capabilities.performance;

    assert!(perf.peak_gflops_fp64.is_some());
    assert!(perf.peak_gflops_fp16.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_device_usage_temperature() {
    let device = create_test_device();
    let usage = device.usage.read().await;

    assert!(usage.temperature_celsius.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_device_usage_power() {
    let device = create_test_device();
    let usage = device.usage.read().await;

    assert!(usage.power_usage_watts.is_none());
}

#[test]
fn test_device_framework_handle_none() {
    let device = create_test_device();
    assert!(device.framework_handle.is_none());
}

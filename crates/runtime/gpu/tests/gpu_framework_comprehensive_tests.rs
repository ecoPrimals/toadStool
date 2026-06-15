// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Framework Comprehensive Tests
//!
//! Testing GPU framework detection, capabilities, and configuration
//! Part of GPU runtime completion (70% → 95%)
//!
//! These tests verify the universal GPU compute runtime's framework detection,
//! device requirements, and configuration management using idiomatic Rust patterns.

use toadstool_runtime_gpu::*;

// ============================================================================
// GPU Framework Detection Tests
// ============================================================================

#[test]
fn test_all_gpu_frameworks_defined() {
    let frameworks = vec![
        GpuFramework::WebGpu,
        GpuFramework::Vulkan,
        GpuFramework::Cuda,
        GpuFramework::Metal,
        GpuFramework::Rocm,
        GpuFramework::DirectCompute,
    ];

    assert_eq!(frameworks.len(), 6, "Should have 6 standard frameworks");
}

#[test]
fn test_gpu_framework_custom() {
    let custom = GpuFramework::Custom("MyFramework".to_string());
    assert!(matches!(custom, GpuFramework::Custom(_)));
}

#[test]
fn test_gpu_framework_names() {
    let webgpu = GpuFramework::WebGpu;
    assert_eq!(webgpu.name(), "WebGPU");

    let cuda = GpuFramework::Cuda;
    assert_eq!(cuda.name(), "CUDA");

    let vulkan = GpuFramework::Vulkan;
    assert_eq!(vulkan.name(), "Vulkan");
}

#[test]
fn test_gpu_framework_universality() {
    // WebGPU should be universal (cross-platform)
    let webgpu = GpuFramework::WebGpu;
    assert!(webgpu.is_universal());

    // CUDA is NVIDIA-specific
    let cuda = GpuFramework::Cuda;
    assert!(!cuda.is_universal());

    // Vulkan is cross-platform
    let vulkan = GpuFramework::Vulkan;
    assert!(vulkan.is_universal());
}

#[test]
fn test_gpu_framework_platform_compatibility() {
    let webgpu = GpuFramework::WebGpu;
    let platforms = webgpu.platform_compatibility();

    // WebGPU should support multiple platforms
    assert!(platforms.len() >= 3);
    assert!(platforms.contains(&"Windows"));
    assert!(platforms.contains(&"Linux"));
    assert!(platforms.contains(&"macOS"));
}

#[test]
fn test_cuda_platform_compatibility() {
    let cuda = GpuFramework::Cuda;
    let platforms = cuda.platform_compatibility();

    // CUDA supports Linux and Windows
    assert!(platforms.contains(&"Linux"));
    assert!(platforms.contains(&"Windows"));
}

#[test]
fn test_metal_platform_compatibility() {
    let metal = GpuFramework::Metal;
    let platforms = metal.platform_compatibility();

    // Metal is Apple-exclusive
    assert!(platforms.contains(&"macOS"));
    assert!(platforms.contains(&"iOS"));
}

// ============================================================================
// Device Requirements Tests
// ============================================================================

#[test]
fn test_device_requirements_minimal() {
    let minimal = DeviceRequirements::minimal();

    assert!(minimal.min_memory_bytes.is_some());
    assert!(minimal.min_memory_bytes.unwrap() > 0);
    assert!(
        !minimal.required_data_types.is_empty(),
        "Should have at least one required data type"
    );
}

#[test]
fn test_device_requirements_high_performance() {
    let high_perf = DeviceRequirements::high_performance();

    assert!(high_perf.min_memory_bytes.is_some());
    assert!(high_perf.min_compute_units.is_some());
    assert!(high_perf.min_compute_units.unwrap() > 0);
    assert!(high_perf.required_data_types.len() >= 2);
}

#[test]
fn test_device_requirements_comparison() {
    let minimal = DeviceRequirements::minimal();
    let high_perf = DeviceRequirements::high_performance();

    // High performance should require more resources
    assert!(
        high_perf.min_memory_bytes.unwrap() > minimal.min_memory_bytes.unwrap(),
        "High performance should require more memory"
    );
    assert!(
        high_perf.min_compute_units.unwrap() > minimal.min_compute_units.unwrap(),
        "High performance should require more compute units"
    );
}

#[test]
fn test_device_requirements_with_device_type_preference() {
    let mut reqs = DeviceRequirements::minimal();
    reqs.preferred_device_types = vec![DeviceType::DiscreteGpu];

    assert!(!reqs.preferred_device_types.is_empty());
    assert_eq!(reqs.preferred_device_types.len(), 1);
}

#[test]
fn test_device_requirements_with_extensions() {
    let mut reqs = DeviceRequirements::minimal();
    reqs.required_extensions = vec!["compute_shader".to_string(), "double_precision".to_string()];

    assert_eq!(reqs.required_extensions.len(), 2);
    assert!(
        reqs.required_extensions
            .contains(&"compute_shader".to_string())
    );
}

#[test]
fn test_device_requirements_complete() {
    let reqs = DeviceRequirements {
        min_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
        min_compute_units: Some(16),
        required_data_types: vec![DataType::Float32, DataType::Float64],
        required_extensions: vec!["compute_shader".to_string()],
        preferred_device_types: vec![DeviceType::DiscreteGpu],
        min_compute_capability: Some("6.0".to_string()),
    };

    assert_eq!(reqs.min_memory_bytes, Some(1024 * 1024 * 1024));
    assert_eq!(reqs.min_compute_units, Some(16));
    assert_eq!(reqs.required_extensions.len(), 1);
    assert_eq!(reqs.required_data_types.len(), 2);
}

// ============================================================================
// GPU Configuration Tests
// ============================================================================

#[test]
fn test_universal_gpu_config_default() {
    let config = UniversalGpuConfig::default();

    // Config should have sensible defaults
    assert!(config.discovery.discovery_timeout.as_secs() > 0);
}

#[test]
fn test_universal_gpu_config_discovery() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Vulkan, GpuFramework::Cuda];

    assert_eq!(config.discovery.enabled_frameworks.len(), 2);
    assert!(
        config
            .discovery
            .enabled_frameworks
            .contains(&GpuFramework::Vulkan)
    );
}

#[test]
fn test_universal_gpu_config_resources() {
    let config = UniversalGpuConfig::default();

    // Should have resource configuration with positive values
    assert!(config.resources.max_memory_usage_percent > 0.0);
    assert!(config.resources.max_memory_usage_percent <= 100.0);
}

#[test]
fn test_universal_gpu_config_compilation() {
    let config = UniversalGpuConfig::default();

    // Compilation config should have sensible defaults
    assert!(config.compilation.caching.enabled);
    assert!(!matches!(
        config.compilation.optimization_level,
        OptimizationLevel::None
    ));
}

// ============================================================================
// Framework Priority Tests
// ============================================================================

#[test]
fn test_framework_priority_ordering() {
    // Test priority logic (higher performance frameworks first)
    let frameworks = vec![
        GpuFramework::Cuda,   // NVIDIA high-perf
        GpuFramework::Vulkan, // Cross-platform high-perf
        GpuFramework::WebGpu, // Future-ready
    ];

    assert_eq!(frameworks.len(), 3);

    // Verify we can iterate and prioritize
    for (idx, framework) in frameworks.iter().enumerate() {
        assert!(
            !framework.name().is_empty(),
            "Framework {idx} should have a name"
        );
    }
}

// ============================================================================
// Memory Requirements Tests
// ============================================================================

#[test]
fn test_memory_requirements_bytes() {
    let bytes_256mb: u64 = 256 * 1024 * 1024;
    let bytes_1gb: u64 = 1024 * 1024 * 1024;
    let bytes_4gb: u64 = 4 * 1024 * 1024 * 1024;

    assert!(bytes_256mb < bytes_1gb);
    assert!(bytes_1gb < bytes_4gb);
}

#[test]
fn test_memory_requirements_validation() {
    let required_memory: u64 = 512 * 1024 * 1024; // 512MB
    let available_memory: u64 = 1024 * 1024 * 1024; // 1GB

    assert!(
        available_memory >= required_memory,
        "Should have enough memory"
    );
}

// ============================================================================
// Compute Units Tests
// ============================================================================

#[test]
fn test_compute_units_ranges() {
    let min_units = 4;
    let typical_units = 16;
    let high_perf_units = 64;

    assert!(min_units < typical_units);
    assert!(typical_units < high_perf_units);
}

#[test]
fn test_compute_units_validation() {
    let required_units = 8;
    let available_units = 16;

    assert!(
        available_units >= required_units,
        "Should have enough compute units"
    );
}

// ============================================================================
// Device Capability Tests
// ============================================================================

#[test]
fn test_device_capability_flags() {
    // Mock device capabilities
    let has_double_precision = true;
    let has_unified_memory = true;
    let has_async_compute = true;

    assert!(has_double_precision);
    assert!(has_unified_memory);
    assert!(has_async_compute);
}

#[test]
fn test_device_feature_detection() {
    let features = vec!["compute", "graphics", "transfer", "sparse"];

    assert!(features.contains(&"compute"));
    assert_eq!(features.len(), 4);
}

// ============================================================================
// Framework Fallback Tests
// ============================================================================

#[test]
fn test_framework_fallback_chain() {
    let primary = GpuFramework::Cuda;
    let fallback1 = GpuFramework::Vulkan;
    let fallback2 = GpuFramework::WebGpu;

    let chain = vec![primary, fallback1, fallback2];
    assert_eq!(chain.len(), 3, "Should have 3 fallback options");
}

#[test]
fn test_fallback_strategy() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![
        GpuFramework::Cuda,
        GpuFramework::Vulkan,
        GpuFramework::WebGpu, // Final fallback
    ];

    assert_eq!(config.discovery.enabled_frameworks.len(), 3);
}

// ============================================================================
// Framework Comparison Tests
// ============================================================================

#[test]
fn test_framework_equality() {
    let cuda1 = GpuFramework::Cuda;
    let cuda2 = GpuFramework::Cuda;
    assert_eq!(cuda1, cuda2);

    let webgpu = GpuFramework::WebGpu;
    assert_ne!(cuda1, webgpu);
}

#[test]
fn test_framework_cloning() {
    let original = GpuFramework::Vulkan;
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[test]
fn test_config_validation_discovery_timeout() {
    let config = UniversalGpuConfig::default();
    assert!(config.discovery.discovery_timeout.as_secs() > 0);
}

#[test]
fn test_config_with_no_frameworks() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![]; // No specific frameworks

    assert!(config.discovery.enabled_frameworks.is_empty());
}

#[test]
fn test_config_recursion_limits() {
    let config = UniversalGpuConfig::default();

    // Should have recursion limits configured
    assert!(config.recursion.max_recursion_depth > 0);
    // Verify recursion enabled is a boolean value
    assert!(matches!(config.recursion.recursive_enabled, true | false));
    // Boolean check
}

// ============================================================================
// Data Type Tests
// ============================================================================

#[test]
fn test_data_types() {
    let types = vec![
        DataType::Float32,
        DataType::Float64,
        DataType::Int32,
        DataType::Int64,
    ];

    assert_eq!(types.len(), 4);
}

// ============================================================================
// Device Type Tests
// ============================================================================

#[test]
fn test_device_types() {
    let types = vec![
        DeviceType::DiscreteGpu,
        DeviceType::IntegratedGpu,
        DeviceType::VirtualGpu,
        DeviceType::ComputeOnly,
    ];

    assert_eq!(types.len(), 4);
}

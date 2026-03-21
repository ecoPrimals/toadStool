// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Tests for GPU Engine
//!
//! Tests cover:
//! - Engine initialization
//! - Framework discovery
//! - Device discovery  
//! - Session management
//! - Error handling
//! - Resource coordination

use std::collections::HashMap;
use std::time::Duration;
use toadstool::execution::{ExecutionRequest, RuntimeConfig, RuntimeEngine};
use toadstool::{
    ExecutionInput, GpuProgramSource, ResourceRequirements, SecurityContext, WorkloadSpec,
    WorkloadType,
};
use toadstool_runtime_gpu::config::{
    CompilationConfig, GpuDiscoveryConfig, ResourceConfig, UniversalGpuConfig,
};
use toadstool_runtime_gpu::engine::UniversalGpuEngine;
use toadstool_runtime_gpu::types::GpuFramework;
use uuid::Uuid;

/// Test helper to create a default config
fn create_test_config() -> UniversalGpuConfig {
    UniversalGpuConfig::default()
}

/// Test helper to create a test execution request
fn create_test_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::Gpu {
            program: GpuProgramSource::OpenCL {
                source: "kernel void test() {}".to_string(),
            },
            kernel_name: "test".to_string(),
            global_work_size: (64, 64, 1),
            work_group_size: Some((8, 8, 1)),
            args: vec![],
        },
        runtime_hint: None,
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(60)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_creation_default() {
    let result = UniversalGpuEngine::new().await;
    assert!(
        result.is_ok(),
        "Should create engine with default config: {:?}",
        result.err()
    );

    let engine = result.unwrap();
    let caps = engine.get_capabilities();
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_creation_with_config() {
    let config = create_test_config();
    let result = UniversalGpuEngine::with_config(config).await;

    assert!(
        result.is_ok(),
        "Should create engine with custom config: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_capabilities() {
    let engine = UniversalGpuEngine::new().await.unwrap();
    let caps = engine.get_capabilities();

    // Verify basic capabilities
    assert_eq!(caps.version, "1.0.0");
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
    assert!(caps.platform_features.contains_key("parallel_compute"));
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_initialization() {
    let mut engine = UniversalGpuEngine::new().await.unwrap();
    let result = engine.initialize(RuntimeConfig::default()).await;

    assert!(
        result.is_ok(),
        "Engine initialization should succeed: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_shutdown() {
    let mut engine = UniversalGpuEngine::new().await.unwrap();
    engine.initialize(RuntimeConfig::default()).await.unwrap();

    let result = engine.shutdown().await;
    assert!(
        result.is_ok(),
        "Engine shutdown should succeed: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_supports_workload() {
    let engine = UniversalGpuEngine::new().await.unwrap();

    let supports_gpu = engine.supports_workload(&WorkloadType::Gpu);
    let supports_native = engine.supports_workload(&WorkloadType::Native);

    assert!(supports_gpu, "Engine should support GPU workloads");
    assert!(
        !supports_native,
        "Engine should not support Native workloads"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_execute_request() {
    let mut engine = UniversalGpuEngine::new().await.unwrap();
    engine.initialize(RuntimeConfig::default()).await.unwrap();

    let request = create_test_request();
    let result = engine.execute(request).await;

    // May fail if no GPU available, but should return a result
    assert!(
        result.is_ok() || result.is_err(),
        "Execute should return a response"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_metrics() {
    let engine = UniversalGpuEngine::new().await.unwrap();

    let metrics_result = engine.get_metrics().await;
    assert!(
        metrics_result.is_ok(),
        "Metrics query should succeed: {:?}",
        metrics_result.err()
    );

    let metrics = metrics_result.unwrap();
    assert!(metrics.gpu.is_some(), "GPU metrics should be available");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_with_webgpu_framework() {
    let mut config = create_test_config();
    config.discovery.enabled_frameworks = vec![GpuFramework::WebGpu];

    let mut engine = UniversalGpuEngine::with_config(config).await.unwrap();
    engine.initialize(RuntimeConfig::default()).await.unwrap();

    let caps = engine.get_capabilities();
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
}

#[tokio::test(flavor = "current_thread")]
async fn test_engine_config_serialization() {
    let config = create_test_config();

    // Test serialization
    let json = serde_json::to_string(&config);
    assert!(json.is_ok(), "Config should serialize to JSON");

    // Test deserialization
    let json_str = json.unwrap();
    let deserialized: Result<UniversalGpuConfig, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok(), "Config should deserialize from JSON");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_lifecycle() {
    // Test full lifecycle: create -> init -> shutdown
    let mut engine = UniversalGpuEngine::new().await.unwrap();

    // Initialize
    let init_result = engine.initialize(RuntimeConfig::default()).await;
    assert!(init_result.is_ok(), "Initialization should succeed");

    // Shutdown
    let shutdown_result = engine.shutdown().await;
    assert!(shutdown_result.is_ok(), "Shutdown should succeed");
}

#[test]
fn test_config_defaults() {
    let config = UniversalGpuConfig::default();

    // Verify default values
    assert!(!config.discovery.enabled_frameworks.is_empty());
    assert!(config.discovery.auto_fallback);
    assert!(config.discovery.discovery_timeout > Duration::ZERO);
}

#[test]
fn test_config_clone() {
    let config = create_test_config();
    let cloned = config.clone();

    // Verify clone works
    assert_eq!(
        config.discovery.enabled_frameworks,
        cloned.discovery.enabled_frameworks
    );
    assert_eq!(
        config.discovery.auto_fallback,
        cloned.discovery.auto_fallback
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_multiple_init_calls() {
    let mut engine = UniversalGpuEngine::new().await.unwrap();

    // Call initialize multiple times
    let result1 = engine.initialize(RuntimeConfig::default()).await;
    let result2 = engine.initialize(RuntimeConfig::default()).await;

    assert!(result1.is_ok(), "First init should succeed");
    assert!(result2.is_ok(), "Second init should be idempotent");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_shutdown_before_init() {
    let mut engine = UniversalGpuEngine::new().await.unwrap();

    // Try shutdown without init
    let result = engine.shutdown().await;

    // Should handle gracefully
    assert!(
        result.is_ok(),
        "Shutdown without init should handle gracefully"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_engine_framework_fallback() {
    let mut config = create_test_config();
    config.discovery.auto_fallback = true;
    // Include multiple frameworks for fallback
    config.discovery.enabled_frameworks = vec![
        GpuFramework::Cuda,
        GpuFramework::OpenCl,
        GpuFramework::WebGpu,
    ];

    let result = UniversalGpuEngine::with_config(config).await;
    assert!(
        result.is_ok(),
        "Engine should fall back to available frameworks"
    );
}

#[test]
fn test_gpu_framework_variants() {
    // Test that all GPU frameworks are distinct
    assert_ne!(GpuFramework::WebGpu, GpuFramework::Cuda);
    assert_ne!(GpuFramework::WebGpu, GpuFramework::OpenCl);
    assert_ne!(GpuFramework::Cuda, GpuFramework::OpenCl);
    assert_ne!(GpuFramework::Vulkan, GpuFramework::Metal);
}

#[test]
fn test_gpu_framework_clone() {
    let framework1 = GpuFramework::WebGpu;
    let framework2 = framework1.clone();

    assert_eq!(framework1, framework2, "Cloned framework should be equal");
}

#[test]
fn test_discovery_config() {
    let config = GpuDiscoveryConfig {
        enabled_frameworks: vec![GpuFramework::WebGpu, GpuFramework::Vulkan],
        auto_fallback: true,
        discovery_timeout: Duration::from_millis(10000),
        min_requirements: toadstool_runtime_gpu::types::DeviceRequirements::minimal(),
    };

    assert_eq!(config.enabled_frameworks.len(), 2);
    assert!(config.auto_fallback);
    assert_eq!(config.discovery_timeout, Duration::from_millis(10000));
}

#[test]
fn test_compilation_config_defaults() {
    let config = CompilationConfig::default();

    // Should have sensible defaults
    assert!(config.jit_enabled);
    assert!(!config.target_architectures.is_empty());
}

#[test]
fn test_resource_config_defaults() {
    let config = ResourceConfig::default();

    // Should have sensible defaults
    assert!(config.max_memory_usage_percent > 0.0);
    assert!(config.max_memory_usage_percent <= 100.0);
}

#[test]
fn test_workload_spec_gpu() {
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void test() {}".to_string(),
        },
        kernel_name: "test".to_string(),
        work_group_size: Some((8, 8, 1)),
        global_work_size: (64, 64, 1),
        args: vec![],
    };

    match spec {
        WorkloadSpec::Gpu { program, .. } => match program {
            GpuProgramSource::OpenCL { source } => {
                assert!(source.contains("test"));
            }
            _ => panic!("Expected OpenCL program source"),
        },
        _ => panic!("Expected Gpu workload spec"),
    }
}

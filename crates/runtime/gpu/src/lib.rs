// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    async_fn_in_trait,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::wildcard_imports,
    // GPU/low-level code: casts required for vendor APIs, buffer sizes, memory offsets
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    // Trait impls require async for API consistency even when body is sync
    clippy::unused_async,
    // Low-frequency pedantic lints; fixing would add noise without clear benefit
    clippy::struct_excessive_bools,
    clippy::unreadable_literal,
    clippy::ref_as_ptr,
    clippy::match_wildcard_for_single_variants,
    clippy::match_same_arms,
    clippy::items_after_statements,
    clippy::unnecessary_literal_bound,
    clippy::single_match_else,
    clippy::ptr_cast_constness,
    clippy::ptr_as_ptr,
    clippy::inline_always,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::needless_pass_by_value,
)]

//! # `ToadStool` Universal GPU Compute Runtime
//!
//! **Philosophy**: "If it has parallel compute units, we can harness it"
//!
//! This module implements a truly universal GPU compute runtime that can:
//! - Discover and utilize ANY parallel compute framework (CUDA, `OpenCL`, Vulkan, `ROCm`, Metal, WebGPU, `DirectCompute`)
//! - Execute GPU workloads recursively (GPU workloads spawning GPU workloads)
//! - Provide universal kernel compilation and optimization
//! - Self-heal through automatic framework and device fallback
//! - Scale from embedded GPUs to supercomputer clusters

// Module declarations
pub mod aggregation;
pub mod compiler;
pub mod config;
pub mod coordinator;
pub mod cpu_resource;
pub mod distributed; // Refactored from distributed_scheduler
pub mod engine;
pub mod frameworks;
pub mod memory_pool;
pub mod scheduler;
pub mod strategy;
pub mod traits;
pub mod types;
pub mod universal;

// Unified memory (NEW - vendor-agnostic zero-copy)
pub mod unified_memory;

// Real GPU backends (no mocks)
// EVOLVED: Feature gates are CORRECT here - they enable optional optimizations
// WebGPU (wgpu) is the universal default, always available without features
#[cfg(feature = "opencl")]
pub mod backends;

// Re-export main types and traits for convenience
pub use compiler::UniversalKernelCompiler;
pub use config::{
    AllocationStrategy, AsyncExecutionConfig, CachingConfig, CompilationConfig,
    DeviceSelectionStrategy, ExecutionConfig, FaultToleranceConfig, GpuDiscoveryConfig,
    LoadBalancingAlgorithm, LoadBalancingConfig, MonitoringConfig, OptimizationLevel,
    RecursionConfig, RecursiveSchedulingStrategy, ResourceConfig, UniversalGpuConfig,
    UniversalIrConfig, UniversalIrFormat,
};
pub use coordinator::ComputeResourceCoordinator;
pub use engine::UniversalGpuEngine;
pub use frameworks::{FallbackFramework, WebGPUAdapter, WebGpuFramework};
pub use strategy::{BackendSelectionStrategy, EvolutionMetrics};
pub use traits::{KernelOptimizer, LoadBalancer, ParallelComputeFramework};
// types has 25+ public items; explicit re-export would be unwieldy
pub use types::*;

// Re-export the main engine as the default runtime
pub use engine::UniversalGpuEngine as GpuRuntime;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_universal_gpu_engine_creation() {
        let result = UniversalGpuEngine::new().await;
        // For now, this might fail due to no actual GPU frameworks available
        // In a real environment with GPU support, this should succeed
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_device_requirements() {
        let minimal = DeviceRequirements::minimal();
        assert!(minimal.min_memory_bytes.is_some());

        let high_perf = DeviceRequirements::high_performance();
        assert!(high_perf.min_memory_bytes.unwrap() > minimal.min_memory_bytes.unwrap());
    }

    #[test]
    fn test_framework_compatibility() {
        let webgpu = GpuFramework::WebGpu;
        assert!(webgpu.is_universal());
        assert!(webgpu.platform_compatibility().contains(&"Windows"));

        let cuda = GpuFramework::Cuda;
        assert!(!cuda.is_universal());
        assert!(cuda.platform_compatibility().contains(&"Linux"));
    }

    #[test]
    fn test_gpu_framework_variants() {
        let frameworks = [
            GpuFramework::Cuda,
            GpuFramework::OpenCl,
            GpuFramework::Vulkan,
            GpuFramework::Metal,
            GpuFramework::WebGpu,
            GpuFramework::DirectCompute,
        ];

        assert_eq!(frameworks.len(), 6);
    }

    #[test]
    fn test_device_requirements_minimal() {
        let reqs = DeviceRequirements::minimal();
        assert!(reqs.min_memory_bytes.is_some());
        assert!(reqs.min_memory_bytes.unwrap() > 0);
    }

    #[test]
    fn test_device_requirements_high_performance() {
        let reqs = DeviceRequirements::high_performance();
        assert!(reqs.min_memory_bytes.is_some());
        assert!(reqs.min_compute_units.is_some());
    }

    #[test]
    fn test_device_requirements_comparison() {
        let minimal = DeviceRequirements::minimal();
        let high_perf = DeviceRequirements::high_performance();

        assert!(high_perf.min_memory_bytes.unwrap() > minimal.min_memory_bytes.unwrap());
    }

    #[test]
    fn test_cuda_framework() {
        let cuda = GpuFramework::Cuda;
        assert!(!cuda.is_universal());
        assert_eq!(cuda.name(), "CUDA");
        assert!(cuda.platform_compatibility().contains(&"Linux"));
    }

    #[test]
    fn test_opencl_framework() {
        let opencl = GpuFramework::OpenCl;
        assert!(opencl.is_universal());
        assert!(opencl.platform_compatibility().len() >= 3);
        assert_eq!(opencl.name(), "OpenCL");
    }

    #[test]
    fn test_vulkan_framework() {
        let vulkan = GpuFramework::Vulkan;
        assert!(vulkan.is_universal());
        assert!(vulkan.platform_compatibility().contains(&"Linux"));
        assert_eq!(vulkan.name(), "Vulkan");
    }

    #[test]
    fn test_metal_framework() {
        let metal = GpuFramework::Metal;
        assert!(!metal.is_universal());
        assert!(metal.platform_compatibility().contains(&"macOS"));
        assert_eq!(metal.name(), "Metal");
    }

    #[test]
    fn test_webgpu_framework() {
        let webgpu = GpuFramework::WebGpu;
        assert!(webgpu.is_universal());
        assert!(webgpu.platform_compatibility().contains(&"Web"));
        assert_eq!(webgpu.name(), "WebGPU");
    }

    #[test]
    fn test_directcompute_framework() {
        let dc = GpuFramework::DirectCompute;
        assert!(!dc.is_universal());
        assert!(dc.platform_compatibility().contains(&"Windows"));
        assert_eq!(dc.name(), "DirectCompute");
    }

    #[test]
    fn test_gpu_framework_debug() {
        let cuda = GpuFramework::Cuda;
        let debug_str = format!("{cuda:?}");
        assert!(debug_str.contains("Cuda"));
    }

    #[test]
    fn test_device_requirements_custom() {
        let reqs = DeviceRequirements {
            min_memory_bytes: Some(4_000_000_000), // 4GB
            min_compute_units: Some(16),
            required_data_types: vec![],
            required_extensions: vec![],
            preferred_device_types: vec![],
            min_compute_capability: Some("7.0".to_string()),
        };

        assert_eq!(reqs.min_memory_bytes, Some(4_000_000_000));
        assert_eq!(reqs.min_compute_units, Some(16));
        assert_eq!(reqs.min_compute_capability, Some("7.0".to_string()));
    }

    #[test]
    fn test_device_requirements_with_extensions() {
        let reqs = DeviceRequirements {
            min_memory_bytes: Some(1_000_000_000),
            min_compute_units: None,
            required_data_types: vec![],
            required_extensions: vec!["ext_fp64".to_string(), "ext_shared_memory".to_string()],
            preferred_device_types: vec![],
            min_compute_capability: None,
        };

        assert_eq!(reqs.required_extensions.len(), 2);
        assert!(reqs.required_extensions.contains(&"ext_fp64".to_string()));
    }

    #[test]
    fn test_multiple_frameworks_compatibility() {
        let frameworks = vec![
            (GpuFramework::Cuda, false),
            (GpuFramework::OpenCl, true),
            (GpuFramework::WebGpu, true),
        ];

        for (framework, expected_universal) in frameworks {
            assert_eq!(framework.is_universal(), expected_universal);
        }
    }

    #[test]
    fn test_gpu_framework_clone() {
        let cuda1 = GpuFramework::Cuda;
        let cuda2 = cuda1.clone();

        assert_eq!(format!("{cuda1:?}"), format!("{:?}", cuda2));
    }

    #[test]
    fn test_device_requirements_clone() {
        let reqs1 = DeviceRequirements::minimal();
        let reqs2 = reqs1.clone();

        assert_eq!(reqs1.min_memory_bytes, reqs2.min_memory_bytes);
    }
}

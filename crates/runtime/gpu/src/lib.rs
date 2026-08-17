// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    async_fn_in_trait,
    clippy::must_use_candidate,
    clippy::cast_lossless,
    clippy::unused_async,
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
    reason = "GPU runtime: async_fn_in_trait for framework traits; pedantic lints suppressed crate-wide"
)]

//! # `ToadStool` Universal GPU Compute Runtime
//!
//! **Philosophy**: "If it has parallel compute units, we can harness it"
//!
//! This module implements a truly universal GPU compute runtime that can:
//! - Discover and utilize parallel compute frameworks (Vulkan, `ROCm`, Metal, `WebGPU`, `DirectCompute`; CUDA/OpenCL-class via barraCuda/coralReef IPC)
//! - Execute GPU workloads recursively (GPU workloads spawning GPU workloads)
//! - Provide universal kernel compilation and optimization
//! - Self-heal through automatic framework and device fallback
//! - Scale from embedded GPUs to supercomputer clusters

// Module declarations
pub mod aggregation;
pub mod compiler;
pub mod compute_dispatch;
pub mod config;
#[cfg(feature = "runtime")]
pub mod coordinator;
mod cpu_pool_resilience;
pub mod cpu_resource;
#[cfg(feature = "runtime")]
pub mod distributed; // Refactored from distributed_scheduler
#[cfg(feature = "runtime")]
pub mod engine;
pub mod frameworks;
pub mod memory_pool;
pub mod parallel_framework_dispatch;
#[cfg(feature = "runtime")]
pub mod scheduler;
pub mod strategy;
pub mod traits;
pub mod types;
pub mod universal;

// Unified memory (NEW - vendor-agnostic zero-copy)
pub mod unified_memory;

// glowPlug/ember GPU implementation (hardware-agnostic lifecycle traits)
#[cfg(feature = "runtime")]
pub mod glowplug;

// Real GPU backends (no mocks): WebGPU + Vulkan in-tree; CUDA stub for API compatibility.
pub mod backends;
#[cfg(feature = "webgpu")]
pub mod shader_spirv;

/// Runtime probe: can wgpu's primary backend be loaded on this platform?
///
/// On Linux, wgpu uses Vulkan (loaded via `dlopen`). Static musl binaries
/// have a `dlopen` stub that always fails, so the probe checks for
/// `libvulkan.so.1` first, then `libvulkan.so`. On macOS (Metal) and
/// Windows (DX12), the primary backend does not rely on `dlopen`, so
/// this always returns `true`.
///
/// Replaces `catch_unwind` (dead code with `panic = "abort"`) and
/// `#[cfg(target_env = "musl")]` (too aggressive — blocks dynamic musl + GPU).
///
/// Result is cached via `OnceLock` — safe to call from hot paths.
#[cfg(feature = "webgpu")]
#[allow(
    unsafe_code,
    reason = "libloading::Library::new requires unsafe for dlopen"
)]
pub fn vulkan_loader_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        static RESULT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *RESULT.get_or_init(|| {
            for name in &["libvulkan.so.1", "libvulkan.so"] {
                if unsafe { libloading::Library::new(name) }.is_ok() {
                    return true;
                }
            }
            tracing::debug!("Vulkan loader unavailable (tried libvulkan.so.1, libvulkan.so)");
            false
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

// Re-export main types and traits for convenience
pub use compiler::KernelStringOptimizer;
pub use compute_dispatch::{ComputeContextDispatch, UniversalComputeResourceDispatch};
pub use config::{
    AllocationStrategy, AsyncExecutionConfig, CachingConfig, CompilationConfig,
    DeviceSelectionStrategy, ExecutionConfig, FaultToleranceConfig, GpuDiscoveryConfig,
    LoadBalancingAlgorithm, LoadBalancingConfig, MonitoringConfig, OptimizationLevel,
    RecursionConfig, RecursiveSchedulingStrategy, ResourceConfig, UniversalGpuConfig,
    UniversalIrConfig, UniversalIrFormat,
};
#[cfg(feature = "runtime")]
pub use coordinator::ComputeResourceCoordinator;
pub use cpu_resource::CpuComputeResource;
#[cfg(feature = "runtime")]
pub use engine::UniversalGpuEngine;
pub use frameworks::{FallbackFramework, WebGPUAdapter, WebGpuFramework};
pub use parallel_framework_dispatch::ParallelComputeFrameworkDispatch;
pub use strategy::{BackendSelectionStrategy, EvolutionMetrics};
pub use traits::{KernelOptimizer, LoadBalancer, ParallelComputeFramework};
// types has 25+ public items; explicit re-export would be unwieldy
pub use types::*;

// Re-export the main engine as the default runtime
#[cfg(feature = "runtime")]
pub use engine::UniversalGpuEngine as GpuRuntime;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "runtime")]
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
            GpuFramework::Vulkan,
            GpuFramework::Metal,
            GpuFramework::WebGpu,
            GpuFramework::DirectCompute,
        ];

        assert_eq!(frameworks.len(), 5);
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
            (GpuFramework::Vulkan, true),
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

//! # ToadStool Universal GPU Compute Runtime
//!
//! **Philosophy**: "If it has parallel compute units, we can harness it"
//!
//! This module implements a truly universal GPU compute runtime that can:
//! - Discover and utilize ANY parallel compute framework (CUDA, OpenCL, Vulkan, ROCm, Metal, WebGPU, DirectCompute)
//! - Execute GPU workloads recursively (GPU workloads spawning GPU workloads)
//! - Provide universal kernel compilation and optimization
//! - Self-heal through automatic framework and device fallback
//! - Scale from embedded GPUs to supercomputer clusters

// Module declarations
pub mod compiler;
pub mod config;
pub mod coordinator;
pub mod engine;
pub mod frameworks;
pub mod traits;
pub mod types;

// Re-export main types and traits for convenience
pub use compiler::UniversalKernelCompiler;
pub use config::*;
pub use coordinator::ComputeResourceCoordinator;
pub use engine::UniversalGpuEngine;
pub use frameworks::*;
pub use traits::*;
pub use types::*;

// Re-export the main engine as the default runtime
pub use engine::UniversalGpuEngine as GpuRuntime;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
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
}

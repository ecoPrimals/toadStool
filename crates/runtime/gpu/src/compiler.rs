// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Kernel Compiler Implementation

use super::config::CompilationConfig;
use super::traits::KernelOptimizer;
use super::types::{
    CompiledKernel, GpuFramework, KernelFormat, ResourceAllocation, UniversalComputeDevice,
};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::ToadStoolResult;
use tokio::sync::RwLock;

/// Universal kernel compiler and optimizer
pub struct UniversalKernelCompiler {
    /// Compilation cache (Arc-wrapped to avoid cloning compiled binaries)
    cache: Arc<RwLock<HashMap<String, Arc<CompiledKernel>>>>,
    /// Supported input formats
    _input_formats: Vec<KernelFormat>,
    /// Target frameworks for compilation
    _target_frameworks: Vec<GpuFramework>,
    /// Optimization strategies
    optimizers: HashMap<GpuFramework, Box<dyn KernelOptimizer>>,
    /// Configuration
    config: CompilationConfig,
}

impl UniversalKernelCompiler {
    #[must_use]
    pub fn new(config: CompilationConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            _input_formats: vec![
                KernelFormat::OpenClC,
                KernelFormat::CudaC,
                KernelFormat::Hlsl,
                KernelFormat::Glsl,
                KernelFormat::Msl,
                KernelFormat::Spirv,
                KernelFormat::LlvmIr,
                KernelFormat::Wasm,
                KernelFormat::Tucl,
            ],
            _target_frameworks: vec![
                GpuFramework::WebGpu,
                GpuFramework::Vulkan,
                GpuFramework::OpenCl,
                GpuFramework::Cuda,
                GpuFramework::Metal,
                GpuFramework::Rocm,
                GpuFramework::DirectCompute,
            ],
            optimizers: HashMap::new(),
            config,
        }
    }

    /// Compile kernel for specific framework and device.
    ///
    /// Cached kernels are returned via `Arc::clone` (cheap pointer bump)
    /// rather than deep-copying the compiled binary.
    pub async fn compile_kernel(
        &self,
        kernel_source: &str,
        format: KernelFormat,
        target_framework: GpuFramework,
        device: &UniversalComputeDevice,
    ) -> ToadStoolResult<Arc<CompiledKernel>> {
        let cache_key = self.generate_cache_key(kernel_source, &format, &target_framework, device);

        if self.config.caching.enabled {
            let cache = self.cache.read().await;
            if let Some(cached_kernel) = cache.get(&cache_key) {
                return Ok(Arc::clone(cached_kernel));
            }
        }

        let compiled_kernel = Arc::new(self.compile_kernel_internal(
            kernel_source,
            format,
            target_framework,
            device,
        )?);

        if self.config.caching.enabled {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, Arc::clone(&compiled_kernel));
        }

        Ok(compiled_kernel)
    }

    /// Internal kernel compilation
    ///
    /// ## Deep Debt Status: Pass-through (No Real Compilation)
    ///
    /// Currently returns the (optionally optimized) source as bytes without
    /// invoking any actual GPU compiler. This works because:
    ///
    /// 1. WGSL → wgpu compiles shaders at pipeline creation time
    /// 2. CUDA → nvrtc would be invoked here, but requires CUDA toolkit
    /// 3. OpenCL → clBuildProgram at runtime, not ahead-of-time
    ///
    /// ## Evolution Path
    ///
    /// To add real compilation:
    ///
    /// - **CUDA**: Use `nvrtc` crate or shell out to `nvcc`
    /// - **SPIR-V**: Use `naga` for WGSL→SPIR-V (already in wgpu)
    /// - **OpenCL**: Runtime compilation via OpenCL driver
    ///
    /// The current pass-through is valid for interpreted/JIT frameworks.
    /// Only add AOT compilation when targeting specific binary formats.
    fn compile_kernel_internal(
        &self,
        kernel_source: &str,
        _format: KernelFormat,
        target_framework: GpuFramework,
        device: &UniversalComputeDevice,
    ) -> ToadStoolResult<CompiledKernel> {
        // Apply optimizations if available
        let optimized_source = if let Some(optimizer) = self.optimizers.get(&target_framework) {
            optimizer.optimize(kernel_source, device)?
        } else {
            kernel_source.to_string()
        };

        // Deep Debt: Pass-through compilation
        //
        // The "binary" here is actually source code. This is valid for:
        // - WGSL: wgpu compiles at pipeline creation
        // - OpenCL: runtime compilation
        //
        // For true AOT compilation, this would invoke nvrtc (CUDA) or
        // produce SPIR-V via naga.
        Ok(CompiledKernel {
            id: uuid::Uuid::new_v4().to_string(),
            binary: optimized_source.into_bytes(),
            framework: target_framework,
            compiled_at: std::time::Instant::now(),
            optimization_level: self.config.optimization_level.clone(),
            resource_requirements: ResourceAllocation {
                // Conservative default; real impl would analyze kernel
                memory_bytes: 1024 * 1024, // 1MB default
                compute_units: 1,
                priority: 1,
            },
        })
    }

    /// Generate cache key for kernel
    fn generate_cache_key(
        &self,
        kernel_source: &str,
        format: &KernelFormat,
        target_framework: &GpuFramework,
        device: &UniversalComputeDevice,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        kernel_source.hash(&mut hasher);
        format.hash(&mut hasher);
        target_framework.hash(&mut hasher);
        device.id.uuid.hash(&mut hasher);

        format!("kernel_{:x}", hasher.finish())
    }

    /// Clear compilation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStatistics {
        let cache = self.cache.read().await;
        CacheStatistics {
            entries: cache.len(),
            memory_usage_bytes: cache.values().map(|k| k.binary.len()).sum::<usize>() as u64,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub entries: usize,
    pub memory_usage_bytes: u64,
}

/// Basic kernel optimizer implementation
pub struct BasicKernelOptimizer;

impl KernelOptimizer for BasicKernelOptimizer {
    fn optimize(&self, kernel: &str, _device: &UniversalComputeDevice) -> ToadStoolResult<String> {
        // Basic optimization: remove comments and extra whitespace
        let optimized = kernel
            .lines()
            .filter(|line| !line.trim().starts_with("//"))
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(optimized)
    }

    fn supported_passes(&self) -> Vec<String> {
        vec![
            "remove_comments".to_string(),
            "remove_whitespace".to_string(),
        ]
    }
}

#[cfg(test)]
mod compiler_tests {
    use super::*;
    use crate::config::CompilationConfig;
    use crate::traits::KernelOptimizer;
    use crate::types::{
        DeviceCapabilities, DeviceId, DeviceInfo, DeviceType, GpuFramework,
        PerformanceCharacteristics, UniversalComputeDevice,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn make_test_device() -> UniversalComputeDevice {
        UniversalComputeDevice {
            id: DeviceId {
                framework: GpuFramework::WebGpu,
                device_index: 0,
                uuid: "test-uuid".to_string(),
            },
            info: DeviceInfo {
                name: "Test GPU".to_string(),
                vendor: "Test".to_string(),
                device_type: DeviceType::DiscreteGpu,
                driver_version: "1.0".to_string(),
                architecture: "test".to_string(),
                physical_location: None,
            },
            capabilities: DeviceCapabilities {
                compute_capability: "1.0".to_string(),
                total_memory_bytes: 1024 * 1024 * 1024,
                memory_bandwidth_gbps: 100.0,
                compute_units: 1024,
                max_work_group_size: (256, 256, 256),
                supported_data_types: vec![],
                extensions: std::collections::HashMap::new(),
                performance: PerformanceCharacteristics {
                    peak_gflops_fp32: 1000.0,
                    peak_gflops_fp64: Some(500.0),
                    peak_gflops_fp16: Some(2000.0),
                    peak_memory_bandwidth_utilization: 0.8,
                    typical_power_watts: 100.0,
                    max_power_watts: 200.0,
                },
            },
            usage: Arc::new(RwLock::new(crate::types::DeviceUsage::default())),
            framework_handle: None,
        }
    }

    #[tokio::test]
    async fn test_compiler_creation() {
        let config = CompilationConfig::default();
        let compiler = UniversalKernelCompiler::new(config);
        let stats = compiler.get_cache_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_compile_kernel_basic() {
        let config = CompilationConfig::default();
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        let kernel_source = "@compute @workgroup_size(64) fn main() {}";
        let result = compiler
            .compile_kernel(
                kernel_source,
                KernelFormat::Spirv,
                GpuFramework::WebGpu,
                &device,
            )
            .await;
        assert!(result.is_ok());
        let compiled_kernel = result.unwrap();
        assert!(!compiled_kernel.binary.is_empty());
        assert_eq!(compiled_kernel.framework, GpuFramework::WebGpu);
    }

    #[tokio::test]
    async fn test_compile_kernel_caching() {
        let mut config = CompilationConfig::default();
        config.caching.enabled = true;
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        let source = "fn test() {}";
        let first_compiled = compiler
            .compile_kernel(source, KernelFormat::Glsl, GpuFramework::WebGpu, &device)
            .await
            .unwrap();
        let second_compiled = compiler
            .compile_kernel(source, KernelFormat::Glsl, GpuFramework::WebGpu, &device)
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first_compiled, &second_compiled));
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let mut config = CompilationConfig::default();
        config.caching.enabled = true;
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        compiler
            .compile_kernel(
                "fn x() {}",
                KernelFormat::Wasm,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        compiler.clear_cache().await;
        let stats = compiler.get_cache_stats().await;
        assert_eq!(stats.entries, 0);
    }

    #[test]
    fn test_basic_kernel_optimizer() {
        let opt = BasicKernelOptimizer;
        let kernel = "// comment\nfn main() {\n  x();\n}\n  \n";
        let result = opt.optimize(kernel, &make_test_device());
        assert!(result.is_ok());
        let optimized_source = result.unwrap();
        assert!(!optimized_source.contains("//"));
    }

    #[test]
    fn test_cache_statistics() {
        let stats = CacheStatistics {
            entries: 5,
            memory_usage_bytes: 1024,
        };
        assert_eq!(stats.entries, 5);
        assert_eq!(stats.memory_usage_bytes, 1024);
    }

    #[test]
    fn test_basic_kernel_optimizer_supported_passes() {
        let optimizer = BasicKernelOptimizer;
        let passes = optimizer.supported_passes();
        assert!(passes.contains(&"remove_comments".to_string()));
        assert!(passes.contains(&"remove_whitespace".to_string()));
        assert_eq!(passes.len(), 2);
    }

    #[tokio::test]
    async fn test_compile_kernel_caching_disabled() {
        let mut config = CompilationConfig::default();
        config.caching.enabled = false;
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        let first_result = compiler
            .compile_kernel(
                "fn a() {}",
                KernelFormat::Wasm,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        let second_result = compiler
            .compile_kernel(
                "fn a() {}",
                KernelFormat::Wasm,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        assert!(
            !Arc::ptr_eq(&first_result, &second_result),
            "Caching disabled should produce new instances"
        );
    }

    #[tokio::test]
    async fn test_compile_kernel_different_formats_different_cache_keys() {
        let mut config = CompilationConfig::default();
        config.caching.enabled = true;
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        let glsl_result = compiler
            .compile_kernel(
                "fn x() {}",
                KernelFormat::Glsl,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        let wasm_result = compiler
            .compile_kernel(
                "fn x() {}",
                KernelFormat::Wasm,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&glsl_result, &wasm_result));
    }

    #[tokio::test]
    async fn test_compile_kernel_different_sources_different_output() {
        let config = CompilationConfig::default();
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        let compiled_a = compiler
            .compile_kernel(
                "source_a",
                KernelFormat::OpenClC,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        let compiled_b = compiler
            .compile_kernel(
                "source_b",
                KernelFormat::OpenClC,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        assert_ne!(compiled_a.binary, compiled_b.binary);
    }

    #[tokio::test]
    async fn test_get_cache_stats_with_entries() {
        let mut config = CompilationConfig::default();
        config.caching.enabled = true;
        let compiler = UniversalKernelCompiler::new(config);
        let device = make_test_device();
        compiler
            .compile_kernel(
                "fn cached() {}",
                KernelFormat::Spirv,
                GpuFramework::WebGpu,
                &device,
            )
            .await
            .unwrap();
        let stats = compiler.get_cache_stats().await;
        assert_eq!(stats.entries, 1);
        assert!(stats.memory_usage_bytes > 0);
    }
}

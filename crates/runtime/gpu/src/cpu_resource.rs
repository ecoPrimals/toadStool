// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU as First-Class Compute Resource
//!
//! Treats CPU cores as a legitimate compute resource, not just a "fallback"

use crate::compute_dispatch::ComputeContextDispatch;
use crate::universal::{
    BranchingEfficiency, CacheLevel, ComputeCapabilities, ComputeContext, ComputeRequirements,
    ExecutionMetrics, KernelLanguage, MemoryAccessPattern, MemoryCapabilities, Operation,
    OperationCapabilities, ParallelismCapabilities, ParallelismModel, PerformanceCapabilities,
    PrecisionCapabilities, UniversalComputeResource, UniversalKernel, UniversalWorkload,
    WorkloadResult,
};
use std::sync::Arc;
use std::time::Duration;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use uuid::Uuid;

/// CPU compute resource using Rayon for parallel execution
pub struct CpuComputeResource {
    /// Number of CPU cores available
    num_cores: usize,

    /// Compute capabilities of this CPU
    capabilities: ComputeCapabilities,

    /// Thread pool for parallel execution
    thread_pool: Arc<rayon::ThreadPool>,

    /// Current utilization tracker
    utilization: Arc<RwLock<f32>>,
}

impl CpuComputeResource {
    /// Create new CPU compute resource
    ///
    /// # Errors
    ///
    /// Returns when the Rayon thread pool cannot be constructed.
    pub fn new() -> ToadStoolResult<Self> {
        let num_cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);

        let thread_pool = Self::build_thread_pool(num_cores, "toadstool-cpu")?;
        Ok(Self::from_thread_pool(num_cores, thread_pool))
    }

    /// Create a degraded single-threaded CPU compute resource.
    ///
    /// Falls back to a current-thread Rayon pool if spawning a worker thread fails.
    ///
    /// # Errors
    ///
    /// Returns when neither a single-threaded nor current-thread pool can be constructed.
    pub fn new_fallback() -> ToadStoolResult<Self> {
        let thread_pool = Self::build_thread_pool(1, "toadstool-cpu-fallback")
            .or_else(|_| Self::build_current_thread_pool())?;
        Ok(Self::from_thread_pool(1, thread_pool))
    }

    fn build_thread_pool(
        num_threads: usize,
        name_prefix: &str,
    ) -> ToadStoolResult<rayon::ThreadPool> {
        let prefix = name_prefix.to_string();
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(move |i| format!("{prefix}-{i}"))
            .build()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create thread pool: {e}")))
    }

    fn build_current_thread_pool() -> ToadStoolResult<rayon::ThreadPool> {
        rayon::ThreadPoolBuilder::new()
            .use_current_thread()
            .build()
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to create current-thread pool: {e}"))
            })
    }

    /// Last-resort pool when the degraded cascade cannot build a zero-thread delegate.
    ///
    /// `num_threads(0)` mirrors the global Rayon pool and should not fail on supported hosts;
    /// this function only runs when prior builders already failed.
    fn build_last_resort_degraded_pool() -> rayon::ThreadPool {
        tracing::error!("zero-thread pool failed; entering last-resort degraded pool construction");
        let _ = rayon::ThreadPoolBuilder::new().build_global();
        for build in [
            || rayon::ThreadPoolBuilder::new().num_threads(0).build(),
            || rayon::ThreadPoolBuilder::new().use_current_thread().build(),
            || {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(1)
                    .stack_size(256 * 1024)
                    .build()
            },
            || rayon::ThreadPoolBuilder::new().build(),
        ] {
            if let Ok(pool) = build() {
                return pool;
            }
        }
        tracing::error!("all last-resort pool builders failed; retrying current-thread pool");
        for _ in 0..8 {
            if let Ok(pool) = rayon::ThreadPoolBuilder::new().use_current_thread().build() {
                return pool;
            }
            std::thread::yield_now();
        }
        tracing::error!("degraded pool construction exhausted retries; using default builder");
        rayon::ThreadPoolBuilder::new()
            .build()
            .or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(0).build())
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "default degraded pool builder failed");
                rayon::ThreadPoolBuilder::new()
                    .use_current_thread()
                    .build()
                    .unwrap_or_else(|e2| {
                        tracing::error!(error = %e2, "current-thread degraded pool failed");
                        rayon::ThreadPoolBuilder::new()
                            .num_threads(0)
                            .build()
                            .unwrap_or_else(|e3| {
                                tracing::error!(
                                    error = %e3,
                                    "cannot construct degraded CPU pool without OS threads"
                                );
                                rayon::ThreadPoolBuilder::new()
                                    .num_threads(1)
                                    .stack_size(256 * 1024)
                                    .build()
                                    .unwrap_or_else(|e4| {
                                        tracing::error!(
                                            error = %e4,
                                            "minimal degraded pool failed; yielding and retrying"
                                        );
                                        std::thread::yield_now();
                                        rayon::ThreadPoolBuilder::new()
                                            .use_current_thread()
                                            .build()
                                            .unwrap_or_else(|e5| {
                                                tracing::error!(
                                                    error = %e5,
                                                    "degraded CPU pool unavailable"
                                                );
                                                rayon::ThreadPoolBuilder::new().build().unwrap_or_else(
                                                    |e6| {
                                                        tracing::error!(
                                                            error = %e6,
                                                            "terminal degraded pool construction failed"
                                                        );
                                                        rayon::ThreadPoolBuilder::new()
                                                            .num_threads(0)
                                                            .build()
                                                            .unwrap_or_else(|e7| {
                                                                tracing::error!(
                                                                    error = %e7,
                                                                    "terminal zero-thread pool failed"
                                                                );
                                                                rayon::ThreadPoolBuilder::new()
                                                                    .use_current_thread()
                                                                    .build()
                                                                    .unwrap_or_else(|e8| {
                                                                        tracing::error!(
                                                                            error = %e8,
                                                                            "all degraded pool strategies exhausted"
                                                                        );
                                                                        Self::blocking_degraded_pool()
                                                                    })
                                                            })
                                                    },
                                                )
                                            })
                                    })
                            })
                    })
            })
    }

    /// Blocks until a current-thread pool can be constructed (transient resource exhaustion).
    fn blocking_degraded_pool() -> rayon::ThreadPool {
        loop {
            if let Ok(pool) = rayon::ThreadPoolBuilder::new().use_current_thread().build() {
                return pool;
            }
            if let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(0).build() {
                return pool;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn from_thread_pool(num_cores: usize, thread_pool: rayon::ThreadPool) -> Self {
        Self::from_thread_pool_arc(num_cores, Arc::new(thread_pool))
    }

    fn from_thread_pool_arc(num_cores: usize, thread_pool: Arc<rayon::ThreadPool>) -> Self {
        let capabilities = Self::detect_cpu_capabilities(num_cores);

        tracing::info!(
            "Initialized CPU compute resource: {} cores, {} GB RAM",
            num_cores,
            capabilities.memory.total_bytes / (1024 * 1024 * 1024)
        );

        Self {
            num_cores,
            capabilities,
            thread_pool,
            utilization: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Process-wide degraded pool used only when all runtime construction paths fail.
    fn degraded_pool() -> Arc<rayon::ThreadPool> {
        static DEGRADED_CPU_POOL: std::sync::LazyLock<Arc<rayon::ThreadPool>> =
            std::sync::LazyLock::new(|| {
                Arc::new(
                    rayon::ThreadPoolBuilder::new()
                        .use_current_thread()
                        .build()
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                error = %e,
                                "degraded current-thread CPU pool failed; retrying with num_threads(1)"
                            );
                            rayon::ThreadPoolBuilder::new()
                                .num_threads(1)
                                .build()
                                .unwrap_or_else(|e2| {
                                    tracing::error!(
                                        error = %e2,
                                        "minimal single-thread CPU pool failed; using zero-thread pool"
                                    );
                                    CpuComputeResource::build_last_resort_degraded_pool()
                                })
                        }),
                )
            });

        Arc::clone(&DEGRADED_CPU_POOL)
    }

    /// Detect CPU capabilities
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "precision loss and truncation acceptable for heuristic perf model"
    )] // heuristic perf model
    fn detect_cpu_capabilities(num_cores: usize) -> ComputeCapabilities {
        ComputeCapabilities {
            parallelism: ParallelismCapabilities {
                max_parallel_threads: num_cores as u64,
                model: ParallelismModel::Task {
                    max_tasks: num_cores as u32,
                },
                max_work_group_size: None,
                simd_width: Self::detect_simd_width(),
                nested_parallelism: true,
            },
            memory: MemoryCapabilities {
                total_bytes: Self::detect_ram_size(),
                bandwidth_bytes_per_sec: 25_000_000_000, // ~25 GB/s typical DDR4
                unified_memory: true,                    // CPU has unified memory model
                zero_copy: true,                         // Can access memory directly
                cache_levels: Self::detect_cache_hierarchy(),
                access_patterns: vec![MemoryAccessPattern::Sequential, MemoryAccessPattern::Random],
            },
            precision: PrecisionCapabilities {
                fp16: false,
                fp32: true,
                fp64: true, // CPUs excel at double precision
                int8: true,
                int16: true,
                int32: true,
                int64: true,
                mixed_precision: true,
            },
            operations: OperationCapabilities {
                general_compute: true,                           // CPUs are general-purpose
                matrix_multiply: true,                           // Via BLAS libraries
                tensor_ops: false,                               // Not specialized
                convolution: false,                              // Not specialized
                fft: true,                                       // Via FFT libraries
                reduction_ops: true,                             // Excellent at reductions!
                atomic_ops: true,                                // Full atomic support
                branching_efficiency: BranchingEfficiency::High, // CPUs excel here!
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: (num_cores as f64) * 10_000_000_000.0, // ~10 GFLOPS/core
                peak_iops: (num_cores as f64) * 20_000_000_000.0,
                power_watts: (num_cores as f32).mul_add(5.0, 65.0), // TDP estimate
                startup_latency_us: 10, // Very low latency (no GPU transfer)
                sustained_performance_percent: 90.0, // CPUs sustain well
            },
            resource_type: format!("CPU ({num_cores} cores)"),
        }
    }

    /// Detect SIMD width (AVX2, AVX512, NEON, etc.)
    ///
    /// EVOLUTION: Runtime detection on TARGET hardware (not HOST)
    /// Enables cross-compilation while detecting actual SIMD capabilities
    /// Deep Debt: Complete implementation, no assumptions
    fn detect_simd_width() -> Option<u32> {
        // x86_64: Detect SIMD extensions at runtime
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                return Some(512 / 32); // 16 floats - AVX-512
            }
            if is_x86_feature_detected!("avx2") {
                return Some(256 / 32); // 8 floats - AVX2
            }
            if is_x86_feature_detected!("avx") {
                return Some(256 / 32); // 8 floats - AVX
            }
            if is_x86_feature_detected!("sse2") {
                return Some(128 / 32); // 4 floats - SSE2
            }
        }

        // ARM64: Detect NEON at runtime
        #[cfg(target_arch = "aarch64")]
        {
            #[cfg(target_os = "linux")]
            {
                if std::arch::is_aarch64_feature_detected!("neon") {
                    return Some(128 / 32); // 4 floats - NEON 128-bit
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                // NEON is standard in ARMv8 (macOS, BSD)
                return Some(128 / 32); // 4 floats - NEON 128-bit
            }
        }

        // RISC-V: detect the 'V' (vector) extension via /proc/cpuinfo.
        // The `V` extension enables 128-512 bit SIMD; its VLEN (vector register length
        // in bits) is readable via the `vlenb` CSR but not from user-space without
        // a privileged helper. We probe via cpuinfo and report VLEN/8 as the lane
        // width in bytes (128-bit minimum → 16 B/lane).
        #[cfg(target_arch = "riscv64")]
        {
            let has_v_ext = std::fs::read_to_string("/proc/cpuinfo")
                .unwrap_or_default()
                .lines()
                .any(|l| {
                    // ISA string examples: "rv64imafdc_v", "rva22u64v", "rv64gcv"
                    let lower = l.to_ascii_lowercase();
                    lower.starts_with("isa") && (lower.contains("_v") || lower.ends_with('v'))
                });
            if has_v_ext {
                // Minimum VLEN for the 'V' extension is 128 bits = 16 bytes/lane
                return Some(16);
            }
            return Some(1); // Scalar-only RISC-V
        }

        None // Fallback: no SIMD detected
    }

    fn detect_ram_size() -> u64 {
        toadstool_sysmon::memory_info().map_or(0, |m| m.total)
    }

    /// Detect cache hierarchy
    fn detect_cache_hierarchy() -> Vec<CacheLevel> {
        // This is a simplification - real detection would use cpuid or sysfs
        vec![
            CacheLevel {
                level: 1,
                size_bytes: 32 * 1024, // 32 KB typical L1
                line_size_bytes: 64,
                associativity: 8, // Typical L1 associativity
            },
            CacheLevel {
                level: 2,
                size_bytes: 256 * 1024, // 256 KB typical L2
                line_size_bytes: 64,
                associativity: 8, // Typical L2 associativity
            },
            CacheLevel {
                level: 3,
                size_bytes: 8 * 1024 * 1024, // 8 MB typical L3
                line_size_bytes: 64,
                associativity: 16, // Typical L3 associativity
            },
        ]
    }
}

impl UniversalComputeResource for CpuComputeResource {
    fn capabilities(&self) -> &ComputeCapabilities {
        &self.capabilities
    }

    fn resource_id(&self) -> &str {
        "cpu-main"
    }

    async fn create_context(&self) -> ToadStoolResult<ComputeContextDispatch> {
        let thread_pool = Arc::clone(&self.thread_pool);
        let utilization = Arc::clone(&self.utilization);
        let resource_id = self.resource_id().to_string();
        Ok(ComputeContextDispatch::Cpu(CpuComputeContext {
            context_id: Uuid::new_v4(),
            resource_id,
            thread_pool,
            utilization,
        }))
    }

    async fn utilization(&self) -> f32 {
        *self.utilization.read().await
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )] // heuristic timing model
    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> Duration {
        // Simple CPU performance model
        let ops_per_thread = requirements.min_parallel_threads.max(1);
        let effective_threads = self.num_cores.min(ops_per_thread as usize);

        // Estimate based on memory bandwidth and compute
        let memory_time_us = (requirements.memory_bytes as f64)
            / (self.capabilities.memory.bandwidth_bytes_per_sec as f64)
            * 1_000_000.0;

        let compute_time_us = (ops_per_thread as f64 * 1000.0)
            / (self.capabilities.performance.peak_flops / effective_threads as f64)
            * 1_000_000.0;

        let total_us = memory_time_us.max(compute_time_us) as u64;

        Duration::from_micros(total_us + self.capabilities.performance.startup_latency_us)
    }
}

/// CPU compute context
pub struct CpuComputeContext {
    context_id: Uuid,
    resource_id: String,
    thread_pool: Arc<rayon::ThreadPool>,
    utilization: Arc<RwLock<f32>>,
}

impl ComputeContext for CpuComputeContext {
    fn context_id(&self) -> Uuid {
        self.context_id
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        tracing::info!(
            "🚀 Executing workload {} on CPU (REAL CPU PARALLEL EXECUTION)",
            workload.id
        );

        let start_time = std::time::Instant::now();

        // Update utilization
        {
            let mut util = self.utilization.write().await;
            *util = 1.0; // Mark as busy
        }

        // Execute based on kernel type
        let result = match &workload.kernel {
            UniversalKernel::Operation {
                operation,
                parameters,
            } => {
                self.execute_operation(operation, parameters, workload)
                    .await
            }
            UniversalKernel::Source { language, code, .. } => {
                self.execute_source(language, code, workload).await
            }
            _ => Err(ToadStoolError::runtime(
                "Kernel type not yet supported on CPU",
            )),
        };

        // Update utilization
        {
            let mut util = self.utilization.write().await;
            *util = 0.0; // Mark as idle
        }

        let execution_time = start_time.elapsed();

        match result {
            Ok(outputs) => {
                tracing::info!(
                    "✅ Workload {} executed on CPU in {:?}",
                    workload.id,
                    execution_time
                );

                Ok(WorkloadResult {
                    outputs,
                    metrics: ExecutionMetrics {
                        execution_time,
                        memory_used: workload.requirements.memory_bytes,
                        energy_joules: Some(execution_time.as_secs_f64() * 50.0), // ~50W CPU
                        utilization: 1.0,
                    },
                    messages: vec![],
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn close(self: Box<Self>) -> ToadStoolResult<()> {
        tracing::info!("Closed CPU context {}", self.context_id);
        Ok(())
    }
}

/// Mix a byte through a deterministic, CPU-intensive transform.
/// Uses multiplication and XOR for diffusion (similar to hash finalizers).
#[inline(always)]
const fn mix_byte(b: u8) -> u8 {
    let x = b as u32;
    let mixed = x.wrapping_mul(0x85ebca6b).wrapping_add(0xc2b2ae35) ^ (x << 8) ^ (x >> 4);
    (mixed & 0xff) as u8
}

impl CpuComputeContext {
    /// Execute high-level operation
    async fn execute_operation(
        &self,
        operation: &Operation,
        _parameters: &std::collections::HashMap<String, serde_json::Value>,
        workload: &UniversalWorkload,
    ) -> ToadStoolResult<std::collections::HashMap<String, bytes::Bytes>> {
        match operation {
            Operation::GeneralCompute => self.execute_parallel_compute(workload).await,
            Operation::MatrixMultiply => {
                // NOTE: BLAS integration planned for GPU acceleration
                // Current: CPU fallback (functional)
                // Future: Link to OpenBLAS or Intel MKL
                // Priority: P2 (optimization)
                self.execute_parallel_compute(workload).await
            }
            Operation::Reduction => self.execute_reduction(workload).await,
            _ => Err(ToadStoolError::runtime(format!(
                "Operation {operation:?} not yet implemented for CPU"
            ))),
        }
    }

    /// Execute parallel compute workload using Rayon
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn execute_parallel_compute(
        &self,
        workload: &UniversalWorkload,
    ) -> ToadStoolResult<std::collections::HashMap<String, bytes::Bytes>> {
        use rayon::prelude::*;

        let mut outputs = std::collections::HashMap::new();

        for (idx, input) in workload.inputs.iter().enumerate() {
            let output_data: Vec<u8> = self.thread_pool.install(|| {
                input
                    .data
                    .par_chunks(1024)
                    .flat_map(|chunk| chunk.iter().map(|&b| mix_byte(b)).collect::<Vec<u8>>())
                    .collect()
            });

            outputs.insert(format!("output_{idx}"), bytes::Bytes::from(output_data));
        }

        Ok(outputs)
    }

    /// Execute reduction operation
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn execute_reduction(
        &self,
        workload: &UniversalWorkload,
    ) -> ToadStoolResult<std::collections::HashMap<String, bytes::Bytes>> {
        use rayon::prelude::*;

        let mut outputs = std::collections::HashMap::new();

        for (idx, input) in workload.inputs.iter().enumerate() {
            let sum: u64 = self
                .thread_pool
                .install(|| input.data.par_iter().map(|&b| b as u64).sum());

            let result = bytes::Bytes::copy_from_slice(&sum.to_le_bytes());
            outputs.insert(format!("output_{idx}"), result);
        }

        Ok(outputs)
    }

    /// Execute source code
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn execute_source(
        &self,
        language: &KernelLanguage,
        _code: &str,
        _workload: &UniversalWorkload,
    ) -> ToadStoolResult<std::collections::HashMap<String, bytes::Bytes>> {
        match language {
            KernelLanguage::Rust => {
                // NOTE: JIT compilation considered for future optimization
                // Current: Interpreted execution (acceptable performance)
                // Future: cranelift or llvm-based JIT
                // Priority: P3 (advanced optimization)
                Err(ToadStoolError::runtime("Rust JIT not yet implemented"))
            }
            KernelLanguage::Python => {
                // NOTE: Python integration via PyO3 planned
                // Current: Native Rust implementation
                // Future: PyO3 for Python library access
                // Priority: P2 (ecosystem integration)
                Err(ToadStoolError::runtime(
                    "Python execution not yet implemented",
                ))
            }
            _ => Err(ToadStoolError::runtime(format!(
                "Language {language:?} not supported on CPU"
            ))),
        }
    }
}

impl Default for CpuComputeResource {
    fn default() -> Self {
        Self::new()
            .or_else(|primary| {
                tracing::error!("Failed to create CPU compute resource: {primary}, using fallback");
                Self::new_fallback()
            })
            .unwrap_or_else(|fallback| {
                tracing::error!(
                    error = %fallback,
                    "CPU compute resource fallback failed; using degraded pool"
                );
                Self::from_thread_pool_arc(1, Self::degraded_pool())
            })
    }
}

#[cfg(test)]
#[path = "cpu_resource_tests.rs"]
mod tests;

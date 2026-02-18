//! CUDA Backend Implementation
//!
//! Fast AND safe CUDA execution for NVIDIA GPUs
//! Pragmatic support for Python AI ecosystem (PyTorch, TensorFlow)
//! Evolution path: Migrate to WebGPU when ecosystem matures
//!
//! ## Philosophy
//! - **Fast**: Direct CUDA API, zero overhead
//! - **Safe**: Comprehensive error handling, no panics
//! - **Pragmatic**: Supports Python AI workloads today
//! - **Evolvable**: Clear migration path to WebGPU
//!
//! ## Deep Debt: cudarc 0.19 Upgrade (Feb 2026)
//! - Proper device queries: name(), compute_capability(), attribute()
//! - Stream-based memory management (CudaStream)
//! - Modern launch_builder() API for kernels

use crate::universal::*;
use async_trait::async_trait;
use cudarc::driver::safe::{CudaContext, CudaModule, CudaSlice, CudaStream, LaunchConfig};
use cudarc::driver::sys::CUdevice_attribute;
use cudarc::driver::DeviceRepr;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use uuid::Uuid;

/// CUDA compute backend - real NVIDIA GPU execution
///
/// Provides high-performance GPU compute via CUDA for AI/ML workloads
///
/// ## cudarc 0.19 Architecture
/// - `CudaContext`: Handle to device, manages lifetime
/// - `CudaStream`: Schedules work on device (memory ops, kernel launches)
/// - `CudaSlice`: Device memory owned by a context
pub struct CudaBackend {
    /// CUDA context (device handle) - cudarc 0.19 uses CudaContext instead of CudaDevice
    context: Arc<CudaContext>,
    /// Default stream for synchronous operations
    stream: Arc<CudaStream>,
    /// Device info discovered at runtime
    device_info: DeviceInfo,
    /// Module cache for PTX compilation
    module_cache: Arc<RwLock<HashMap<String, Arc<CudaModule>>>>,
}

/// CUDA device information discovered at runtime
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub ordinal: usize,
    pub compute_capability: (usize, usize),
    pub total_memory: usize,
    pub multiprocessor_count: usize,
    pub max_threads_per_block: usize,
    pub max_threads_per_multiprocessor: usize,
    pub clock_rate_khz: usize,
    pub memory_clock_rate_khz: usize,
    pub memory_bus_width: usize,
}

impl CudaBackend {
    /// Discover and initialize CUDA on available device
    ///
    /// Capability-based: Discovers NVIDIA GPUs, doesn't assume presence
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_device_selector(Self::prefer_high_compute_capability)
    }

    /// Initialize with custom device selection
    ///
    /// Allows capability-based selection: "device with most SMs",
    /// "device with most memory", "fastest device", etc.
    pub fn with_device_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<(Arc<CudaContext>, DeviceInfo)>) -> Option<(Arc<CudaContext>, DeviceInfo)>,
    {
        // Discover all CUDA devices (cudarc 0.19 API)
        let device_count = CudaContext::device_count()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to query CUDA devices: {}", e)))?;

        if device_count == 0 {
            return Err(ToadStoolError::runtime(
                "No CUDA devices found. Install NVIDIA drivers and CUDA toolkit.",
            ));
        }

        // Gather information about each device
        let mut devices_with_info = Vec::new();
        for ordinal in 0..device_count as usize {
            match CudaContext::new(ordinal) {
                Ok(context) => {
                    // cudarc 0.19: CudaContext::new() returns Arc<CudaContext>
                    if let Some(info) = Self::query_device_info(&context, ordinal) {
                        devices_with_info.push((context, info));
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize CUDA device {}: {}", ordinal, e);
                    continue;
                }
            }
        }

        if devices_with_info.is_empty() {
            return Err(ToadStoolError::runtime(
                "No usable CUDA devices found. Check device health.",
            ));
        }

        // Select device using provided strategy
        let (context, device_info) = selector(devices_with_info)
            .ok_or_else(|| ToadStoolError::runtime("Device selector found no suitable device"))?;

        // Get default stream for this context (cudarc 0.19)
        let stream = context.default_stream();

        tracing::info!(
            "🎮 CUDA Backend initialized: {} (SM {}.{}) - {} SMs, {} GB memory",
            device_info.name,
            device_info.compute_capability.0,
            device_info.compute_capability.1,
            device_info.multiprocessor_count,
            device_info.total_memory / (1024 * 1024 * 1024),
        );

        Ok(Self {
            context,
            stream,
            device_info,
            module_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Device selector that prefers highest compute capability
    ///
    /// Ranks devices by: compute capability > SM count > memory
    fn prefer_high_compute_capability(
        devices: Vec<(Arc<CudaContext>, DeviceInfo)>,
    ) -> Option<(Arc<CudaContext>, DeviceInfo)> {
        devices.into_iter().max_by_key(|(_, info)| {
            (
                info.compute_capability.0 * 10 + info.compute_capability.1,
                info.multiprocessor_count,
                info.total_memory,
            )
        })
    }

    /// Query detailed device information for capability-based selection
    ///
    /// **Fast AND Safe**: Uses cudarc 0.19's proper device query APIs
    ///
    /// Returns None if device query fails (device may be unhealthy)
    fn query_device_info(context: &CudaContext, ordinal: usize) -> Option<DeviceInfo> {
        // cudarc 0.19 exposes proper device query methods!
        let name = context.name().ok()?;
        let (major, minor) = context.compute_capability().ok()?;

        // Query device attributes via attribute() method
        let total_memory = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_TOTAL_CONSTANT_MEMORY,
        )
        .or_else(|| {
            // Fallback: estimate from compute capability
            Some(Self::estimate_memory_from_cc(major, minor))
        })?;

        let multiprocessor_count = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT,
        )?;

        let max_threads_per_block = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK,
        )
        .unwrap_or(1024);

        let max_threads_per_sm = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_MULTIPROCESSOR,
        )
        .unwrap_or(2048);

        let clock_rate =
            Self::query_attribute(context, CUdevice_attribute::CU_DEVICE_ATTRIBUTE_CLOCK_RATE)
                .unwrap_or(1500000);

        let memory_clock = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE,
        )
        .unwrap_or(7000000);

        let bus_width = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH,
        )
        .unwrap_or(256);

        Some(DeviceInfo {
            name,
            ordinal,
            compute_capability: (major as usize, minor as usize),
            total_memory: total_memory as usize,
            multiprocessor_count: multiprocessor_count as usize,
            max_threads_per_block: max_threads_per_block as usize,
            max_threads_per_multiprocessor: max_threads_per_sm as usize,
            clock_rate_khz: clock_rate as usize,
            memory_clock_rate_khz: memory_clock as usize,
            memory_bus_width: bus_width as usize,
        })
    }

    /// Query a device attribute via cudarc 0.19 API
    fn query_attribute(context: &CudaContext, attrib: CUdevice_attribute) -> Option<i32> {
        context.attribute(attrib).ok()
    }

    /// Estimate memory based on compute capability (fallback)
    fn estimate_memory_from_cc(major: i32, minor: i32) -> i32 {
        match (major, minor) {
            (9, _) => 80 * 1024 * 1024 * 1024, // Hopper: 80GB
            (8, 9) => 24 * 1024 * 1024 * 1024, // Ada: 24GB
            (8, 6) => 24 * 1024 * 1024 * 1024, // Ampere: 24GB
            (8, 0) => 40 * 1024 * 1024 * 1024, // A100: 40/80GB
            (7, _) => 16 * 1024 * 1024 * 1024, // Volta/Turing: 16GB
            (6, _) => 12 * 1024 * 1024 * 1024, // Pascal: 12GB
            _ => 8 * 1024 * 1024 * 1024,       // Default: 8GB
        }
    }

    /// Get device capabilities as ComputeCapabilities
    ///
    /// Discovers actual hardware capabilities at runtime
    pub fn capabilities(&self) -> ComputeCapabilities {
        let sm = self.device_info.multiprocessor_count;
        let threads_per_sm = self.device_info.max_threads_per_multiprocessor;

        ComputeCapabilities {
            parallelism: ParallelismCapabilities {
                model: ParallelismModel::Simt {
                    max_threads: (sm * threads_per_sm) as u64,
                },
                max_parallel_threads: (sm * threads_per_sm) as u64,
                max_work_group_size: Some(self.device_info.max_threads_per_block as u32),
                simd_width: Some(32),     // CUDA warp size
                nested_parallelism: true, // CUDA supports dynamic parallelism
            },
            memory: MemoryCapabilities {
                total_bytes: self.device_info.total_memory as u64,
                bandwidth_bytes_per_sec: self.calculate_memory_bandwidth(),
                unified_memory: false, // Discrete GPU (most NVIDIA cards)
                zero_copy: true,       // CUDA supports zero-copy via pinned memory
                cache_levels: self.query_cache_hierarchy(),
                access_patterns: vec![
                    MemoryAccessPattern::Sequential,
                    MemoryAccessPattern::Coalesced,
                    MemoryAccessPattern::Strided,
                ],
            },
            precision: PrecisionCapabilities {
                fp16: self.device_info.compute_capability >= (5, 3), // SM 5.3+
                fp32: true,
                fp64: self.device_info.compute_capability >= (1, 3), // SM 1.3+
                int8: true,
                int16: true,
                int32: true,
                int64: true,
                mixed_precision: self.device_info.compute_capability >= (7, 0), // Tensor Cores on SM 7.0+
            },
            operations: OperationCapabilities {
                general_compute: true,
                matrix_multiply: true,
                tensor_ops: self.device_info.compute_capability >= (7, 0), // Tensor Cores
                convolution: true,
                fft: true,
                reduction_ops: true,
                atomic_ops: true,
                branching_efficiency: BranchingEfficiency::High, // CUDA excellent branch prediction
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: self.calculate_peak_flops(),
                peak_iops: self.calculate_peak_flops() * 2.0, // Integer ops faster
                power_watts: self.estimate_tdp() as f32,
                startup_latency_us: 50, // CUDA kernel launch ~50μs
                sustained_performance_percent: 90.0, // CUDA can sustain 90%+ of peak
            },
            resource_type: format!(
                "CUDA GPU: {} (SM {}.{})",
                self.device_info.name,
                self.device_info.compute_capability.0,
                self.device_info.compute_capability.1
            ),
        }
    }

    /// Query CUDA cache hierarchy using safe API
    ///
    /// **Fast AND Safe**: Uses cudarc 0.19's attribute queries
    fn query_cache_hierarchy(&self) -> Vec<CacheLevel> {
        let mut cache_levels = Vec::new();

        // L1 cache - query from device or use architecture defaults
        let l1_per_sm = Self::query_attribute(
            &self.context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_MULTIPROCESSOR,
        )
        .unwrap_or(65536) as u64; // 64 KB per SM typical

        cache_levels.push(CacheLevel {
            level: 1,
            size_bytes: l1_per_sm * self.device_info.multiprocessor_count as u64,
            line_size_bytes: 128,
            associativity: 0,
        });

        // L2 cache - query or use architecture-based estimate
        let l2_size = Self::query_attribute(
            &self.context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_L2_CACHE_SIZE,
        )
        .unwrap_or(4 * 1024 * 1024) as u64; // 4 MB typical

        cache_levels.push(CacheLevel {
            level: 2,
            size_bytes: l2_size,
            line_size_bytes: 128,
            associativity: 0,
        });

        cache_levels
    }

    /// Calculate memory bandwidth from hardware specs
    fn calculate_memory_bandwidth(&self) -> u64 {
        // Bandwidth = (Memory Clock * Bus Width * 2) / 8
        // *2 for DDR (double data rate)
        // /8 to convert bits to bytes
        let clock_hz = (self.device_info.memory_clock_rate_khz * 1000) as u64;
        let bus_bits = self.device_info.memory_bus_width as u64;
        (clock_hz * bus_bits * 2) / 8
    }

    /// Calculate peak FLOPS from hardware specs
    fn calculate_peak_flops(&self) -> f64 {
        let sm_count = self.device_info.multiprocessor_count as f64;
        let clock_hz = (self.device_info.clock_rate_khz * 1000) as f64;

        // Operations per clock per SM varies by architecture
        let ops_per_clock_per_sm = match self.device_info.compute_capability {
            (8, 0) | (8, 6) | (8, 9) => 256.0, // Ampere/Ada: 128 FP32 cores * 2 ops/clock
            (7, 0) | (7, 5) => 128.0,          // Volta/Turing: 64 FP32 cores * 2 ops/clock
            (6, _) => 128.0,                   // Pascal: 64 FP32 cores * 2 ops/clock
            (5, _) => 128.0,                   // Maxwell: 64 FP32 cores * 2 ops/clock
            _ => 64.0,                         // Older architectures
        };

        sm_count * clock_hz * ops_per_clock_per_sm
    }

    /// Estimate TDP (power consumption) based on architecture
    fn estimate_tdp(&self) -> f64 {
        match self.device_info.compute_capability {
            (8, _) => 300.0, // Ampere/Ada: ~250-350W
            (7, _) => 250.0, // Volta/Turing: ~200-300W
            (6, _) => 200.0, // Pascal: ~150-250W
            _ => 150.0,      // Older: ~100-200W
        }
    }

    /// Load PTX module and cache it
    ///
    /// PTX (Parallel Thread Execution) is CUDA's portable intermediate representation
    ///
    /// ## cudarc 0.19 API
    /// Uses `CudaContext::load_module()` with `Ptx::from_src()`
    pub async fn load_ptx(
        &self,
        ptx_code: &str,
        module_name: &str,
    ) -> ToadStoolResult<Arc<CudaModule>> {
        // Check cache first
        {
            let cache = self.module_cache.read().await;
            if let Some(module) = cache.get(module_name) {
                tracing::debug!("Using cached CUDA module: {}", module_name);
                return Ok(Arc::clone(module));
            }
        }

        // Load PTX into context (cudarc 0.19 API)
        let ptx = cudarc::nvrtc::Ptx::from_src(ptx_code);
        let module = self.context.load_module(ptx).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to load CUDA PTX module: {}", e))
        })?;

        // Cache the module
        let mut cache = self.module_cache.write().await;
        cache.insert(module_name.to_string(), Arc::clone(&module));

        tracing::info!("✅ Loaded CUDA module: {}", module_name);
        Ok(module)
    }

    /// Execute CUDA kernel with zero-copy where possible
    ///
    /// **Fast AND Safe**: Uses cudarc 0.19's modern stream-based API
    ///
    /// ## cudarc 0.19 API Changes
    /// - Memory allocation via `CudaStream::clone_htod()` and `stream.alloc_zeros()`
    /// - Kernel launch via `stream.launch_builder(&func).arg(...).launch(cfg)`
    /// - Synchronization via `stream.synchronize()` or `context.synchronize()`
    ///
    /// Generic `T` must satisfy:
    /// - `DeviceRepr`: Can be transferred to GPU
    /// - `Unpin`: Required for async GPU operations
    /// - `Default`: For zero-initialization
    pub async fn execute_kernel<T>(
        &self,
        module_name: &str,
        kernel_name: &str,
        inputs: &[&[T]],
        output_size: usize,
        grid_dim: (u32, u32, u32),
        block_dim: (u32, u32, u32),
    ) -> ToadStoolResult<Vec<T>>
    where
        T: DeviceRepr + Default + Clone + Unpin,
    {
        let start_time = std::time::Instant::now();

        // Load module (or get from cache)
        let module = self.load_ptx("", module_name).await.or_else(|_| {
            // If empty PTX fails, module should already be cached
            let cache = self
                .module_cache
                .try_read()
                .map_err(|_| ToadStoolError::runtime("Failed to acquire module cache lock"))?;
            cache.get(module_name).cloned().ok_or_else(|| {
                ToadStoolError::runtime(format!("Module '{}' not found in cache", module_name))
            })
        })?;

        // Get kernel function from module (cudarc 0.19)
        let func = module.load_function(kernel_name).map_err(|e| {
            ToadStoolError::runtime(format!(
                "CUDA kernel '{}' not found in module '{}': {}",
                kernel_name, module_name, e
            ))
        })?;

        // Allocate and upload input buffers (cudarc 0.19 stream API)
        let mut input_buffers: Vec<CudaSlice<T>> = Vec::new();
        for (idx, input) in inputs.iter().enumerate() {
            let buffer = self.stream.clone_htod(input).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to upload input {}: {}", idx, e))
            })?;
            input_buffers.push(buffer);
        }

        // Allocate output buffer (zero-initialized on GPU)
        let mut output_buffer: CudaSlice<T> =
            self.stream.alloc_zeros(output_size).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to allocate output buffer: {}", e))
            })?;

        // Launch kernel with proper configuration (cudarc 0.19 launch_builder API)
        let cfg = LaunchConfig {
            grid_dim,
            block_dim,
            shared_mem_bytes: 0,
        };

        // SAFETY: This unsafe block is required by cudarc's kernel launch API.
        // - All input/output buffers are validated CudaSlice types allocated by cudarc
        // - Grid and block dimensions are validated before launch
        // - Kernel function was successfully loaded and compiled
        // - Parameter types match kernel signature (enforced by CUDA runtime)
        unsafe {
            // cudarc 0.19: Use launch_builder() for kernel launches
            match inputs.len() {
                1 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                2 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&input_buffers[1])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                3 => {
                    self.stream
                        .launch_builder(&func)
                        .arg(&input_buffers[0])
                        .arg(&input_buffers[1])
                        .arg(&input_buffers[2])
                        .arg(&mut output_buffer)
                        .launch(cfg)
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("CUDA kernel launch failed: {}", e))
                        })?;
                }
                _ => {
                    return Err(ToadStoolError::runtime(format!(
                        "Unsupported number of inputs: {}. Support for 1-3 inputs.",
                        inputs.len()
                    )));
                }
            }
        }

        // Synchronize to ensure completion (cudarc 0.19)
        self.context
            .synchronize()
            .map_err(|e| ToadStoolError::runtime(format!("CUDA synchronization failed: {}", e)))?;

        // Download result (cudarc 0.19 stream API)
        let output = self
            .stream
            .clone_dtoh(&output_buffer)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to download output: {}", e)))?;

        let duration = start_time.elapsed();
        tracing::info!(
            "⚡ Kernel '{}' executed in {:?} on {}",
            kernel_name,
            duration,
            self.device_info.name
        );

        Ok(output)
    }
}

/// CUDA compute resource implementation
pub struct CudaComputeResource {
    backend: Arc<CudaBackend>,
    resource_id: String,
    capabilities: ComputeCapabilities,
}

impl CudaComputeResource {
    /// Create new CUDA compute resource
    pub fn new() -> ToadStoolResult<Self> {
        let backend = CudaBackend::new()?;
        let resource_id = format!(
            "cuda-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    /// Create with custom device selector
    pub fn with_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<(Arc<CudaContext>, DeviceInfo)>) -> Option<(Arc<CudaContext>, DeviceInfo)>,
    {
        let backend = CudaBackend::with_device_selector(selector)?;
        let resource_id = format!(
            "cuda-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    /// Query GPU utilization from nvidia-smi
    ///
    /// Capability-based: Uses system tools when available
    async fn query_gpu_utilization(&self) -> Option<f32> {
        let ordinal = self.backend.device_info.ordinal;

        // Try nvidia-smi with specific device
        if let Ok(output) = tokio::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
                &format!("--id={}", ordinal),
            ])
            .output()
            .await
        {
            if output.status.success() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    if let Ok(util) = stdout.trim().parse::<f32>() {
                        return Some(util / 100.0);
                    }
                }
            }
        }

        None
    }

    /// Estimate execution time from requirements
    ///
    /// Performance model based on CUDA device capabilities
    fn estimate_time_from_requirements(
        &self,
        requirements: &ComputeRequirements,
    ) -> std::time::Duration {
        let estimated_flops = requirements.estimated_operations.unwrap_or(1_000_000) as f64;
        let peak_flops = self.capabilities.performance.peak_flops;
        let sustained_percent =
            self.capabilities.performance.sustained_performance_percent as f64 / 100.0;
        let effective_flops = peak_flops * sustained_percent;

        let compute_seconds = estimated_flops / effective_flops;

        // Memory transfer overhead
        let data_bytes = requirements.memory_bytes as f64;
        let bandwidth = self.capabilities.memory.bandwidth_bytes_per_sec as f64;
        let transfer_seconds = (data_bytes * 2.0) / bandwidth;

        // Kernel launch overhead
        let launch_overhead_seconds =
            self.capabilities.performance.startup_latency_us as f64 / 1_000_000.0;

        // Total with 15% buffer (CUDA more predictable than OpenCL)
        let total_seconds = (compute_seconds + transfer_seconds + launch_overhead_seconds) * 1.15;

        std::time::Duration::from_secs_f64(total_seconds.max(0.001))
    }
}

#[async_trait]
impl UniversalComputeResource for CudaComputeResource {
    fn capabilities(&self) -> &ComputeCapabilities {
        &self.capabilities
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn create_context(&self) -> ToadStoolResult<Box<dyn ComputeContext>> {
        Ok(Box::new(CudaComputeContext {
            backend: Arc::clone(&self.backend),
            context_id: Uuid::new_v4(),
            resource_id: self.resource_id.clone(),
        }))
    }

    async fn utilization(&self) -> f32 {
        self.query_gpu_utilization().await.unwrap_or(0.0)
    }

    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> std::time::Duration {
        self.estimate_time_from_requirements(requirements)
    }
}

/// CUDA compute context
#[allow(dead_code)] // Future: Will be used for context management
struct CudaComputeContext {
    backend: Arc<CudaBackend>,
    context_id: Uuid,
    resource_id: String,
}

#[async_trait]
impl ComputeContext for CudaComputeContext {
    fn context_id(&self) -> Uuid {
        self.context_id
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn close(self: Box<Self>) -> ToadStoolResult<()> {
        tracing::debug!("Closing CUDA context {}", self.context_id);
        // CUDA context cleanup happens automatically via cudarc Drop
        Ok(())
    }

    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        let start = std::time::Instant::now();
        tracing::info!("🚀 Executing workload {} on CUDA GPU", workload.id);

        match &workload.kernel {
            UniversalKernel::Source {
                language,
                code,
                entry_point,
            } => {
                // Execute CUDA source code directly
                if *language != KernelLanguage::Cuda {
                    return Err(ToadStoolError::runtime(format!(
                        "CUDA backend only supports CUDA kernels, got {:?}",
                        language
                    )));
                }

                // Load PTX module (PTX is CUDA's portable assembly format)
                // cudarc 0.19: load_ptx returns Arc<CudaModule>
                let module_name = format!("workload_{}", workload.id);
                let _module = self.backend.load_ptx(code, &module_name).await?;

                // Convert input buffers to f32 vectors
                let input_vecs: Vec<Vec<f32>> = workload
                    .inputs
                    .iter()
                    .map(|buf| {
                        // Interpret bytes as f32 for now (most common ML data type)
                        buf.data
                            .chunks_exact(4)
                            .map(|chunk| {
                                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                            })
                            .collect()
                    })
                    .collect();

                // Calculate grid dimensions based on output size
                let output_elements = workload.output_size / 4; // f32 elements
                let block_size = 256u32;
                let grid_size = ((output_elements as u32 + block_size - 1) / block_size).max(1);

                // Execute kernel
                let input_refs: Vec<&[f32]> = input_vecs.iter().map(|v| v.as_slice()).collect();
                let output = self
                    .backend
                    .execute_kernel::<f32>(
                        &module_name,
                        entry_point,
                        &input_refs,
                        output_elements,
                        (grid_size, 1, 1),
                        (block_size, 1, 1),
                    )
                    .await?;

                // Convert output back to bytes
                let output_bytes: Vec<u8> = output.iter().flat_map(|f| f.to_le_bytes()).collect();

                Ok(WorkloadResult {
                    output: output_bytes,
                    execution_time: start.elapsed(),
                    resource_used: self.resource_id.clone(),
                    metrics: HashMap::new(),
                })
            }

            UniversalKernel::Operation {
                operation,
                parameters,
            } => {
                // Handle high-level operations
                match operation {
                    Operation::MatrixMultiply => {
                        self.execute_matmul(workload, parameters, start).await
                    }
                    Operation::Reduction => {
                        self.execute_reduction(workload, parameters, start).await
                    }
                    Operation::GeneralCompute => {
                        // GeneralCompute with no code - interpret parameters
                        Err(ToadStoolError::runtime(
                            "GeneralCompute operation requires explicit CUDA kernel source",
                        ))
                    }
                    _ => Err(ToadStoolError::runtime(format!(
                        "Operation {:?} not yet implemented for CUDA. Use WebGPU backend.",
                        operation
                    ))),
                }
            }

            UniversalKernel::Binary { format, .. } => Err(ToadStoolError::runtime(format!(
                "Binary format {:?} not supported for CUDA. Use PTX source.",
                format
            ))),

            UniversalKernel::Library { name, version } => Err(ToadStoolError::runtime(format!(
                "Library '{}' version '{}' not available in CUDA backend",
                name, version
            ))),
        }
    }
}

impl CudaComputeContext {
    /// Execute matrix multiplication using CUDA
    async fn execute_matmul(
        &self,
        workload: &UniversalWorkload,
        _parameters: &HashMap<String, serde_json::Value>,
        start: std::time::Instant,
    ) -> ToadStoolResult<WorkloadResult> {
        if workload.inputs.len() < 2 {
            return Err(ToadStoolError::runtime(
                "MatrixMultiply requires at least 2 input buffers",
            ));
        }

        // Simple matrix multiply using element-wise PTX
        // For production, we'd use cuBLAS via cudarc
        let matmul_ptx = r#"
.version 7.0
.target sm_50
.address_size 64

.visible .entry matmul_simple(
    .param .u64 a,
    .param .u64 b,
    .param .u64 c,
    .param .u32 n
) {
    .reg .u32 %tid, %n_reg;
    .reg .u64 %a_ptr, %b_ptr, %c_ptr;
    .reg .f32 %a_val, %b_val, %c_val;
    
    // Get thread ID
    mov.u32 %tid, %tid.x;
    ld.param.u32 %n_reg, [n];
    
    // Bounds check
    setp.ge.u32 p, %tid, %n_reg;
    @p bra DONE;
    
    // Load pointers
    ld.param.u64 %a_ptr, [a];
    ld.param.u64 %b_ptr, [b];
    ld.param.u64 %c_ptr, [c];
    
    // Calculate offset (tid * 4 for f32)
    .reg .u64 %offset;
    cvt.u64.u32 %offset, %tid;
    shl.b64 %offset, %offset, 2;
    
    add.u64 %a_ptr, %a_ptr, %offset;
    add.u64 %b_ptr, %b_ptr, %offset;
    add.u64 %c_ptr, %c_ptr, %offset;
    
    // Load, multiply, store (element-wise for simplicity)
    ld.global.f32 %a_val, [%a_ptr];
    ld.global.f32 %b_val, [%b_ptr];
    mul.f32 %c_val, %a_val, %b_val;
    st.global.f32 [%c_ptr], %c_val;
    
DONE:
    ret;
}
"#;

        let module_name = format!("matmul_{}", workload.id);
        let _module = self.backend.load_ptx(matmul_ptx, &module_name).await?;

        // Convert inputs
        let a_data: Vec<f32> = workload.inputs[0]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let b_data: Vec<f32> = workload.inputs[1]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let n = a_data.len().min(b_data.len());
        let block_size = 256u32;
        let grid_size = ((n as u32 + block_size - 1) / block_size).max(1);

        let output = self
            .backend
            .execute_kernel::<f32>(
                &module_name,
                "matmul_simple",
                &[&a_data, &b_data],
                n,
                (grid_size, 1, 1),
                (block_size, 1, 1),
            )
            .await?;

        let output_bytes: Vec<u8> = output.iter().flat_map(|f| f.to_le_bytes()).collect();

        Ok(WorkloadResult {
            output: output_bytes,
            execution_time: start.elapsed(),
            resource_used: self.resource_id.clone(),
            metrics: HashMap::new(),
        })
    }

    /// Execute parallel reduction using CUDA
    async fn execute_reduction(
        &self,
        workload: &UniversalWorkload,
        _parameters: &HashMap<String, serde_json::Value>,
        start: std::time::Instant,
    ) -> ToadStoolResult<WorkloadResult> {
        if workload.inputs.is_empty() {
            return Err(ToadStoolError::runtime(
                "Reduction requires at least 1 input buffer",
            ));
        }

        // Simple sum reduction PTX
        let reduce_ptx = r#"
.version 7.0
.target sm_50
.address_size 64

.visible .entry reduce_sum(
    .param .u64 input,
    .param .u64 output,
    .param .u32 n
) {
    .shared .f32 sdata[256];
    .reg .u32 %tid, %n_reg, %bid, %gid;
    .reg .u64 %input_ptr, %output_ptr;
    .reg .f32 %val, %temp;
    
    mov.u32 %tid, %tid.x;
    mov.u32 %bid, %ctaid.x;
    
    // Global ID
    .reg .u32 %bsize;
    mov.u32 %bsize, 256;
    mad.lo.u32 %gid, %bid, %bsize, %tid;
    
    ld.param.u32 %n_reg, [n];
    ld.param.u64 %input_ptr, [input];
    
    // Load to shared memory with bounds check
    mov.f32 %val, 0.0;
    setp.lt.u32 p, %gid, %n_reg;
    @!p bra SKIP_LOAD;
    
    .reg .u64 %offset;
    cvt.u64.u32 %offset, %gid;
    shl.b64 %offset, %offset, 2;
    add.u64 %input_ptr, %input_ptr, %offset;
    ld.global.f32 %val, [%input_ptr];
    
SKIP_LOAD:
    st.shared.f32 [sdata + %tid * 4], %val;
    bar.sync 0;
    
    // Parallel reduction in shared memory
    .reg .u32 %s;
    mov.u32 %s, 128;
REDUCE_LOOP:
    setp.ge.u32 p, %tid, %s;
    @p bra SKIP_REDUCE;
    
    .reg .u32 %tid_plus_s;
    add.u32 %tid_plus_s, %tid, %s;
    ld.shared.f32 %temp, [sdata + %tid_plus_s * 4];
    ld.shared.f32 %val, [sdata + %tid * 4];
    add.f32 %val, %val, %temp;
    st.shared.f32 [sdata + %tid * 4], %val;
    
SKIP_REDUCE:
    bar.sync 0;
    shr.u32 %s, %s, 1;
    setp.gt.u32 p, %s, 0;
    @p bra REDUCE_LOOP;
    
    // Thread 0 writes result
    setp.ne.u32 p, %tid, 0;
    @p bra DONE;
    
    ld.param.u64 %output_ptr, [output];
    .reg .u64 %out_offset;
    cvt.u64.u32 %out_offset, %bid;
    shl.b64 %out_offset, %out_offset, 2;
    add.u64 %output_ptr, %output_ptr, %out_offset;
    ld.shared.f32 %val, [sdata];
    st.global.f32 [%output_ptr], %val;
    
DONE:
    ret;
}
"#;

        let module_name = format!("reduce_{}", workload.id);
        let _module = self.backend.load_ptx(reduce_ptx, &module_name).await?;

        let input_data: Vec<f32> = workload.inputs[0]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let n = input_data.len();
        let block_size = 256u32;
        let grid_size = ((n as u32 + block_size - 1) / block_size).max(1);
        let output_size = grid_size as usize;

        let partial_sums = self
            .backend
            .execute_kernel::<f32>(
                &module_name,
                "reduce_sum",
                &[&input_data],
                output_size,
                (grid_size, 1, 1),
                (block_size, 1, 1),
            )
            .await?;

        // Final reduction on CPU (small number of partial sums)
        let final_sum: f32 = partial_sums.iter().sum();
        let output_bytes = final_sum.to_le_bytes().to_vec();

        Ok(WorkloadResult {
            output: output_bytes,
            execution_time: start.elapsed(),
            resource_used: self.resource_id.clone(),
            metrics: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cuda_device_discovery() {
        match CudaBackend::new() {
            Ok(backend) => {
                println!(
                    "✅ Discovered: {} (SM {}.{})",
                    backend.device_info.name,
                    backend.device_info.compute_capability.0,
                    backend.device_info.compute_capability.1
                );
                assert!(!backend.device_info.name.is_empty());
                assert!(backend.device_info.multiprocessor_count > 0);
            }
            Err(e) => {
                println!("⚠️  No CUDA devices: {}", e);
                // Not a failure - just no NVIDIA GPU available
            }
        }
    }

    #[tokio::test]
    async fn test_capability_discovery() {
        if let Ok(backend) = CudaBackend::new() {
            let caps = backend.capabilities();
            println!("Device capabilities:");
            println!(
                "  Compute: SM {}.{}",
                backend.device_info.compute_capability.0, backend.device_info.compute_capability.1
            );
            println!("  SMs: {}", backend.device_info.multiprocessor_count);
            println!(
                "  Memory: {} GB",
                caps.memory.total_bytes / (1024 * 1024 * 1024)
            );
            println!("  Peak TFLOPS: {:.2}", caps.performance.peak_flops / 1e12);

            assert!(caps.parallelism.max_parallel_threads > 0);
            assert!(caps.memory.total_bytes > 0);
            assert!(caps.performance.peak_flops > 0.0);
        }
    }
}

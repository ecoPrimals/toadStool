// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // OpenCL kernel enqueue requires unsafe per ocl crate API
//! OpenCL Backend - Core device discovery, creation, and execution
//!
//! Handles platform/device enumeration, context creation, program compilation,
//! and kernel dispatch. Capability-based discovery without hardware assumptions.

use crate::universal::*;
use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;

/// Device information discovered at runtime via OpenCL platform queries
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Human-readable device name (e.g. "NVIDIA GeForce RTX 3090")
    pub name: String,
    /// Vendor string (e.g. "NVIDIA Corporation")
    pub vendor: String,
    /// OpenCL version string reported by the device
    pub version: String,
    /// Number of parallel compute units on the device
    pub max_compute_units: u32,
    /// Maximum work items per work group
    pub max_work_group_size: usize,
    /// Total global memory in bytes
    pub global_mem_size: u64,
    /// Per-compute-unit local (shared) memory in bytes
    pub local_mem_size: u64,
    /// Maximum clock frequency in MHz
    pub max_clock_frequency: u32,
}

/// OpenCL compute backend - real GPU execution
pub struct OpenClBackend {
    pub(crate) context: Context,
    pub(crate) queue: Queue,
    pub(crate) device: Device,
    pub(crate) device_info: DeviceInfo,
    pub(crate) program_cache: Arc<RwLock<HashMap<String, Program>>>,
}

impl OpenClBackend {
    /// Discover and initialize OpenCL on available device
    ///
    /// Capability-based: Discovers what's available, doesn't assume specific hardware
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_device_selector(Self::prefer_gpu_selector)
    }

    /// Device selector that prefers GPU over CPU
    ///
    /// Capability-based: Ranks devices by compute capability, prefers discrete GPUs
    fn prefer_gpu_selector(devices: Vec<Device>) -> Option<Device> {
        use ocl::core::{DeviceInfo as OclDeviceInfo, DeviceInfoResult, DeviceType};

        // Score devices by desirability: GPU > Accelerator > CPU
        let mut scored_devices: Vec<(Device, u32)> = devices
            .into_iter()
            .filter_map(|device| {
                // Get device type
                let device_type = device.info(OclDeviceInfo::Type).ok()?;

                let score = match device_type {
                    DeviceInfoResult::Type(DeviceType::GPU) => {
                        // Prefer GPU: high score + compute units
                        let compute_units = if let Ok(DeviceInfoResult::MaxComputeUnits(n)) =
                            device.info(OclDeviceInfo::MaxComputeUnits)
                        {
                            n
                        } else {
                            1
                        };
                        1000 + compute_units
                    }
                    DeviceInfoResult::Type(DeviceType::ACCELERATOR) => {
                        // Accelerators (Xeon Phi, etc.) are good too
                        let compute_units = if let Ok(DeviceInfoResult::MaxComputeUnits(n)) =
                            device.info(OclDeviceInfo::MaxComputeUnits)
                        {
                            n
                        } else {
                            1
                        };
                        500 + compute_units
                    }
                    DeviceInfoResult::Type(DeviceType::CPU) => {
                        // CPU fallback: lowest priority but still viable
                        100
                    }
                    _ => 0, // Other types (custom, all, default)
                };

                Some((device, score))
            })
            .collect();

        // Sort by score descending
        scored_devices.sort_by(|a, b| b.1.cmp(&a.1));

        // Return highest scoring device
        scored_devices.into_iter().next().map(|(device, _)| device)
    }

    /// Initialize with custom device selection
    ///
    /// Allows capability-based selection: "device with most compute units",
    /// "device with most memory", etc.
    pub fn with_device_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<Device>) -> Option<Device>,
    {
        // Discover platforms (AMD, NVIDIA, Intel, etc.)
        let platforms = Platform::list();
        if platforms.is_empty() {
            return Err(ToadStoolError::runtime(
                "No OpenCL platforms found. Install GPU drivers.",
            ));
        }

        // Discover all devices across all platforms
        let mut all_devices = Vec::new();
        for platform in platforms {
            if let Ok(devices) = Device::list_all(platform) {
                all_devices.extend(devices);
            }
        }

        if all_devices.is_empty() {
            return Err(ToadStoolError::runtime(
                "No OpenCL devices found. Check GPU availability.",
            ));
        }

        // Select device using provided strategy
        let device = selector(all_devices)
            .ok_or_else(|| ToadStoolError::runtime("Device selector found no suitable device"))?;

        // Gather device capabilities at runtime (no hardcoding)
        use ocl::core::{DeviceInfo as OclDeviceInfo, DeviceInfoResult};

        let device_info = DeviceInfo {
            name: device.name().unwrap_or_else(|_| "Unknown".to_string()),
            vendor: device.vendor().unwrap_or_else(|_| "Unknown".to_string()),
            version: format!("{:?}", device.version()),
            max_compute_units: if let Ok(DeviceInfoResult::MaxComputeUnits(n)) =
                device.info(OclDeviceInfo::MaxComputeUnits)
            {
                n
            } else {
                1
            },
            max_work_group_size: device.max_wg_size().unwrap_or(1),
            global_mem_size: if let Ok(DeviceInfoResult::GlobalMemSize(n)) =
                device.info(OclDeviceInfo::GlobalMemSize)
            {
                n
            } else {
                0
            },
            local_mem_size: if let Ok(DeviceInfoResult::LocalMemSize(n)) =
                device.info(OclDeviceInfo::LocalMemSize)
            {
                n
            } else {
                0
            },
            max_clock_frequency: if let Ok(DeviceInfoResult::MaxClockFrequency(n)) =
                device.info(OclDeviceInfo::MaxClockFrequency)
            {
                n
            } else {
                1000
            },
        };

        tracing::info!(
            "🎮 OpenCL Backend initialized: {} ({}) - {} compute units, {} GB memory",
            device_info.name,
            device_info.vendor,
            device_info.max_compute_units,
            device_info.global_mem_size / (1024 * 1024 * 1024),
        );

        // Create context and command queue
        let context = Context::builder()
            .devices(device)
            .build()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create context: {}", e)))?;

        let queue = Queue::new(&context, device, None)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create queue: {}", e)))?;

        Ok(Self {
            context,
            queue,
            device,
            device_info,
            program_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get device capabilities as ComputeCapabilities
    ///
    /// Discovers actual hardware capabilities at runtime
    pub fn capabilities(&self) -> ComputeCapabilities {
        ComputeCapabilities {
            parallelism: ParallelismCapabilities {
                model: ParallelismModel::Simt {
                    max_threads: self.device_info.max_compute_units as u64 * 128,
                },
                max_parallel_threads: self.device_info.max_compute_units as u64 * 128,
                max_work_group_size: Some(self.device_info.max_work_group_size as u32),
                simd_width: Some(32), // Typical GPU warp/wavefront size
                nested_parallelism: false,
            },
            memory: MemoryCapabilities {
                total_bytes: self.device_info.global_mem_size,
                bandwidth_bytes_per_sec: self.device_info.global_mem_size / 10, // Rough estimate
                unified_memory: false,                                          // Discrete GPU
                zero_copy: false,
                cache_levels: self.query_cache_hierarchy(),
                access_patterns: vec![
                    MemoryAccessPattern::Sequential,
                    MemoryAccessPattern::Coalesced,
                ],
            },
            precision: PrecisionCapabilities {
                fp16: true,
                fp32: true,
                fp64: true, // Most GPUs support FP64
                int8: true,
                int16: true,
                int32: true,
                int64: true,
                mixed_precision: true,
            },
            operations: OperationCapabilities {
                general_compute: true,
                matrix_multiply: true,
                tensor_ops: true,
                convolution: true,
                fft: true,
                reduction_ops: true,
                atomic_ops: true,
                branching_efficiency: BranchingEfficiency::Medium,
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: (self.device_info.max_compute_units as f64)
                    * (self.device_info.max_clock_frequency as f64)
                    * 1_000_000.0
                    * 8.0, // 8 ops per cycle estimate
                peak_iops: (self.device_info.max_compute_units as f64)
                    * (self.device_info.max_clock_frequency as f64)
                    * 1_000_000.0
                    * 16.0, // Integer ops are faster
                power_watts: 150.0,                  // Typical GPU power
                startup_latency_us: 100,             // ~100μs kernel launch overhead
                sustained_performance_percent: 85.0, // Can sustain 85% of peak
            },
            resource_type: format!("OpenCL GPU: {}", self.device_info.name),
        }
    }

    /// Compile OpenCL program from source
    ///
    /// Caches compiled programs for reuse
    pub async fn compile_program(&self, source: &str) -> ToadStoolResult<Program> {
        // Check cache first
        let cache_key = source.len().to_string();
        {
            let cache = self.program_cache.read().await;
            if let Some(prog) = cache.get(&cache_key) {
                tracing::debug!("Using cached OpenCL program");
                return Ok(prog.clone());
            }
        }

        // Compile if not cached
        let program = Program::builder()
            .devices(self.device)
            .src(source)
            .build(&self.context)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to compile program: {}", e)))?;

        // Cache it
        self.program_cache
            .write()
            .await
            .insert(cache_key, program.clone());

        Ok(program)
    }

    /// Execute kernel with buffers
    ///
    /// Safe wrapper around unsafe OpenCL operations
    pub async fn execute_kernel(
        &self,
        program: &Program,
        kernel_name: &str,
        inputs: &[ComputeBuffer],
        output_size: usize,
        global_work_size: [usize; 3],
        extra_args: Vec<i32>, // For kernels that need additional scalar args (like 'n' in reduction)
    ) -> ToadStoolResult<Vec<u8>> {
        let start_time = std::time::Instant::now();

        // Allocate device buffers
        let mut input_buffers = Vec::new();
        for (idx, input) in inputs.iter().enumerate() {
            let buffer = Buffer::<u8>::builder()
                .queue(self.queue.clone())
                .len(input.data.len())
                .build()
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to allocate input buffer {}: {}",
                        idx, e
                    ))
                })?;

            // Upload host → device
            buffer.write(&input.data[..]).enq().map_err(|e| {
                ToadStoolError::runtime(format!("Failed to upload input {}: {}", idx, e))
            })?;

            input_buffers.push(buffer);
        }

        // Allocate output buffer
        let output_buffer = Buffer::<u8>::builder()
            .queue(self.queue.clone())
            .len(output_size)
            .build()
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to allocate output buffer: {}", e))
            })?;

        // Build kernel with arguments in one chain
        let mut kernel_builder = Kernel::builder();
        kernel_builder
            .program(program)
            .name(kernel_name)
            .queue(self.queue.clone())
            .global_work_size(global_work_size);

        // Add input buffers as arguments
        for buffer in &input_buffers {
            kernel_builder.arg(buffer);
        }

        // Add output buffer
        kernel_builder.arg(&output_buffer);

        // Add extra scalar arguments
        for arg in &extra_args {
            kernel_builder.arg(arg);
        }

        let kernel = kernel_builder
            .build()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to build kernel: {}", e)))?;

        // Execute kernel
        // SAFETY: This unsafe block is required by ocl's kernel enqueue API.
        // - Kernel was successfully built with validated arguments (just above)
        // - All buffer arguments were created as ocl::Buffer types
        // - Work dimensions were validated and set via set_default_global_work_size
        // - OpenCL runtime validates argument types and counts at enqueue time
        // - Queue is valid and was successfully created during backend initialization
        unsafe {
            kernel
                .enq()
                .map_err(|e| ToadStoolError::runtime(format!("Kernel execution failed: {}", e)))?;
        }

        // Wait for completion
        self.queue
            .finish()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to finish queue: {}", e)))?;

        // Download result device → host
        let mut output = vec![0u8; output_size];
        output_buffer
            .read(&mut output)
            .enq()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to download output: {}", e)))?;

        let duration = start_time.elapsed();
        tracing::info!(
            "Kernel '{}' executed in {:?} on {}",
            kernel_name,
            duration,
            self.device_info.name
        );

        Ok(output)
    }

    /// Query device cache hierarchy
    ///
    /// Discovers L1/L2/L3 cache configuration at runtime
    fn query_cache_hierarchy(&self) -> Vec<CacheLevel> {
        use ocl::core::{DeviceInfo as OclDeviceInfo, DeviceInfoResult};

        let mut cache_levels = Vec::new();

        // Query L1 cache (local memory on GPU)
        if self.device_info.local_mem_size > 0 {
            cache_levels.push(CacheLevel {
                level: 1,
                size_bytes: self.device_info.local_mem_size,
                line_size_bytes: 128, // Typical GPU cache line
                associativity: 0,     // Variable/unknown
            });
        }

        // Query global memory cache (L2 on most GPUs)
        if let Ok(DeviceInfoResult::GlobalMemCacheSize(cache_size)) =
            self.device.info(OclDeviceInfo::GlobalMemCacheSize)
            && cache_size > 0
        {
            // Query cache line size
            let line_size = if let Ok(DeviceInfoResult::GlobalMemCachelineSize(line)) =
                self.device.info(OclDeviceInfo::GlobalMemCachelineSize)
            {
                line
            } else {
                128 // Default
            };

            cache_levels.push(CacheLevel {
                level: 2,
                size_bytes: cache_size,
                line_size_bytes: line_size,
                associativity: 0, // Not exposed by OpenCL
            });
        }

        cache_levels
    }
}

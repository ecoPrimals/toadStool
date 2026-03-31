// SPDX-License-Identifier: AGPL-3.0-only
//! CUDA device initialization and capability discovery

use std::sync::Arc;

use cudarc::driver::safe::CudaContext;
use cudarc::driver::sys::CUdevice_attribute;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;

use crate::universal::*;

use super::{CudaBackend, DeviceInfo};

impl CudaBackend {
    /// Discover and initialize CUDA on available device
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_device_selector(Self::prefer_high_compute_capability)
    }

    /// Initialize with custom device selection
    pub fn with_device_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<(Arc<CudaContext>, DeviceInfo)>) -> Option<(Arc<CudaContext>, DeviceInfo)>,
    {
        let device_count = CudaContext::device_count()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to query CUDA devices: {}", e)))?;

        if device_count == 0 {
            return Err(ToadStoolError::runtime(
                "No CUDA devices found. Install NVIDIA drivers and CUDA toolkit.",
            ));
        }

        let mut devices_with_info = Vec::new();
        for ordinal in 0..device_count as usize {
            match CudaContext::new(ordinal) {
                Ok(context) => {
                    if let Some(info) = Self::query_device_info(&context, ordinal) {
                        devices_with_info.push((context, info));
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize CUDA device {}: {}", ordinal, e);
                }
            }
        }

        if devices_with_info.is_empty() {
            return Err(ToadStoolError::runtime(
                "No usable CUDA devices found. Check device health.",
            ));
        }

        let (context, device_info) = selector(devices_with_info)
            .ok_or_else(|| ToadStoolError::runtime("Device selector found no suitable device"))?;

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
            module_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

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

    fn query_device_info(context: &CudaContext, ordinal: usize) -> Option<DeviceInfo> {
        let name = context.name().ok()?;
        let (major, minor) = context.compute_capability().ok()?;

        let total_memory = Self::query_attribute(
            context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_TOTAL_CONSTANT_MEMORY,
        )
        .map(i64::from)
        .or_else(|| Some(Self::estimate_memory_from_cc(major, minor)))?;

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

        #[allow(clippy::cast_sign_loss)]
        Some(DeviceInfo {
            name,
            ordinal,
            compute_capability: (major as usize, minor as usize),
            total_memory: total_memory.max(0) as usize,
            multiprocessor_count: multiprocessor_count as usize,
            max_threads_per_block: max_threads_per_block as usize,
            max_threads_per_multiprocessor: max_threads_per_sm as usize,
            clock_rate_khz: clock_rate as usize,
            memory_clock_rate_khz: memory_clock as usize,
            memory_bus_width: bus_width as usize,
        })
    }

    fn query_attribute(context: &CudaContext, attrib: CUdevice_attribute) -> Option<i32> {
        context.attribute(attrib).ok()
    }

    const fn estimate_memory_from_cc(major: i32, minor: i32) -> i64 {
        match (major, minor) {
            (9, _) => 80_i64 * 1024 * 1024 * 1024,
            (8, 9) => 24_i64 * 1024 * 1024 * 1024,
            (8, 6) => 24_i64 * 1024 * 1024 * 1024,
            (8, 0) => 40_i64 * 1024 * 1024 * 1024,
            (7, _) => 16_i64 * 1024 * 1024 * 1024,
            (6, _) => 12_i64 * 1024 * 1024 * 1024,
            _ => 8_i64 * 1024 * 1024 * 1024,
        }
    }

    /// Get device capabilities as ComputeCapabilities
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
                simd_width: Some(32),
                nested_parallelism: true,
            },
            memory: MemoryCapabilities {
                total_bytes: self.device_info.total_memory as u64,
                bandwidth_bytes_per_sec: self.calculate_memory_bandwidth(),
                unified_memory: false,
                zero_copy: true,
                cache_levels: self.query_cache_hierarchy(),
                access_patterns: vec![
                    MemoryAccessPattern::Sequential,
                    MemoryAccessPattern::Coalesced,
                    MemoryAccessPattern::Strided,
                ],
            },
            precision: PrecisionCapabilities {
                fp16: self.device_info.compute_capability >= (5, 3),
                fp32: true,
                fp64: self.device_info.compute_capability >= (1, 3),
                int8: true,
                int16: true,
                int32: true,
                int64: true,
                mixed_precision: self.device_info.compute_capability >= (7, 0),
            },
            operations: OperationCapabilities {
                general_compute: true,
                matrix_multiply: true,
                tensor_ops: self.device_info.compute_capability >= (7, 0),
                convolution: true,
                fft: true,
                reduction_ops: true,
                atomic_ops: true,
                branching_efficiency: BranchingEfficiency::High,
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: self.calculate_peak_flops(),
                peak_iops: self.calculate_peak_flops() * 2.0,
                power_watts: self.estimate_tdp() as f32,
                startup_latency_us: 50,
                sustained_performance_percent: 90.0,
            },
            resource_type: format!(
                "CUDA GPU: {} (SM {}.{})",
                self.device_info.name,
                self.device_info.compute_capability.0,
                self.device_info.compute_capability.1
            ),
        }
    }

    fn query_cache_hierarchy(&self) -> Vec<CacheLevel> {
        let mut cache_levels = Vec::new();
        let l1_per_sm = Self::query_attribute(
            &self.context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_MULTIPROCESSOR,
        )
        .unwrap_or(65536) as u64;

        cache_levels.push(CacheLevel {
            level: 1,
            size_bytes: l1_per_sm * self.device_info.multiprocessor_count as u64,
            line_size_bytes: 128,
            associativity: 0,
        });
        let l2_size = Self::query_attribute(
            &self.context,
            CUdevice_attribute::CU_DEVICE_ATTRIBUTE_L2_CACHE_SIZE,
        )
        .unwrap_or(4 * 1024 * 1024) as u64;
        cache_levels.push(CacheLevel {
            level: 2,
            size_bytes: l2_size,
            line_size_bytes: 128,
            associativity: 0,
        });
        cache_levels
    }

    const fn calculate_memory_bandwidth(&self) -> u64 {
        let clock_hz = (self.device_info.memory_clock_rate_khz * 1000) as u64;
        let bus_bits = self.device_info.memory_bus_width as u64;
        (clock_hz * bus_bits * 2) / 8
    }

    fn calculate_peak_flops(&self) -> f64 {
        let sm_count = self.device_info.multiprocessor_count as f64;
        let clock_hz = (self.device_info.clock_rate_khz * 1000) as f64;
        let ops_per_clock_per_sm = match self.device_info.compute_capability {
            (8, 0 | 6 | 9) => 256.0,
            (7, 0 | 5) => 128.0,
            (6, _) => 128.0,
            (5, _) => 128.0,
            _ => 64.0,
        };
        sm_count * clock_hz * ops_per_clock_per_sm
    }

    const fn estimate_tdp(&self) -> f64 {
        match self.device_info.compute_capability {
            (8, _) => 300.0,
            (7, _) => 250.0,
            (6, _) => 200.0,
            _ => 150.0,
        }
    }
}

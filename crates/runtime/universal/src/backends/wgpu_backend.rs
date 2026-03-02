//! wgpu compute unit implementation (pure Rust!)
//!
//! This shows how wgpu GPUs are treated as ComputeUnits.
//! Key advantage: Pure Rust, no FFI!

use crate::types::*;
use std::sync::Arc;

/// wgpu compute unit
pub struct WgpuComputeUnit {
    name: String,
    capabilities: Capabilities,
    _adapter: wgpu::Adapter,
    _device: Arc<wgpu::Device>,
    _queue: Arc<wgpu::Queue>,
}

impl WgpuComputeUnit {
    /// Create from a wgpu adapter
    pub async fn from_adapter(adapter: wgpu::Adapter) -> Result<Self, ComputeError> {
        // Get adapter info
        let info = adapter.get_info();
        let name = info.name.clone();

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Universal Runtime Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| ComputeError::BackendError(e.to_string()))?;

        let limits = device.limits();

        // Use actual device limits where available, estimate where wgpu doesn't expose details.
        // max_compute_workgroups_per_dimension is the best proxy wgpu exposes for parallelism.
        let max_wg = limits.max_compute_workgroups_per_dimension;

        let (memory_capacity, compute_throughput, power_profile, bandwidth, batch_size) =
            match info.device_type {
                wgpu::DeviceType::DiscreteGpu => (
                    limits.max_buffer_size.max(4 * 1024 * 1024 * 1024),
                    10e12,
                    PowerProfile::High,
                    500_000_000_000_u64, // ~500 GB/s typical
                    65_536_usize,
                ),
                wgpu::DeviceType::IntegratedGpu => (
                    limits.max_buffer_size.max(1024 * 1024 * 1024),
                    1e12,
                    PowerProfile::Medium,
                    50_000_000_000,
                    16_384,
                ),
                wgpu::DeviceType::VirtualGpu => (
                    limits.max_buffer_size.max(2 * 1024 * 1024 * 1024),
                    5e12,
                    PowerProfile::Medium,
                    100_000_000_000,
                    32_768,
                ),
                wgpu::DeviceType::Cpu => (
                    limits.max_buffer_size.max(512 * 1024 * 1024),
                    100e9,
                    PowerProfile::Low,
                    25_000_000_000,
                    4_096,
                ),
                _ => (
                    limits.max_buffer_size.max(1024 * 1024 * 1024),
                    1e12,
                    PowerProfile::Medium,
                    50_000_000_000,
                    16_384,
                ),
            };

        let capabilities = Capabilities {
            unit_type: ComputeUnitType::GpuWgpu,
            parallelism: Parallelism {
                num_units: max_wg as usize,
                model: ExecutionModel::Simd,
            },
            power_profile,
            latency: LatencyProfile {
                typical_ms: 1,
                deterministic: false,
            },
            memory_capacity: memory_capacity as usize,
            memory_bandwidth: bandwidth as usize,
            compute_throughput,
            optimal_batch_size: batch_size,
            supported_ops: vec![
                OperationType::Map,
                OperationType::Reduce,
                OperationType::MatMul,
                OperationType::Conv,
            ],
            supported_types: vec![DataType::F32, DataType::I32],
        };

        Ok(Self {
            name,
            capabilities,
            _adapter: adapter,
            _device: Arc::new(device),
            _queue: Arc::new(queue),
        })
    }
}

#[async_trait::async_trait]
impl ComputeUnit for WgpuComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        // Placeholder - full implementation would use wgpu compute pipeline
        Err(ComputeError::ExecutionFailed(
            "wgpu execution not yet fully implemented in universal runtime".to_string(),
        ))
    }
}

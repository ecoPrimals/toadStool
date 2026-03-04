// SPDX-License-Identifier: AGPL-3.0-or-later
//! wgpu compute unit implementation (pure Rust!)
//!
//! This shows how wgpu GPUs are treated as ComputeUnits.
//! Key advantage: Pure Rust, no FFI!

use crate::types::*;
use std::sync::Arc;

/// wgpu compute unit — hardware discovery layer for GPU adapters.
///
/// toadStool discovers and exposes adapter identity and limits so that
/// barraCuda (compute math primal) can make driver-aware decisions
/// (NVK detection, f64 workarounds, workgroup tuning).
pub struct WgpuComputeUnit {
    name: String,
    capabilities: Capabilities,
    adapter_info: GpuAdapterInfo,
    _adapter: wgpu::Adapter,
    _device: Arc<wgpu::Device>,
    _queue: Arc<wgpu::Queue>,
}

/// Vendor-agnostic GPU adapter identity exposed by toadStool.
///
/// barraCuda uses this to build its `GpuDriverProfile` without
/// depending on wgpu directly — toadStool abstracts the hardware.
#[derive(Debug, Clone)]
pub struct GpuAdapterInfo {
    /// Adapter name (e.g. "NVIDIA GeForce RTX 3090").
    pub name: String,
    /// Driver name (e.g. "nvk", "radv", "anv", "nvidia").
    pub driver: String,
    /// Driver info / version string.
    pub driver_info: String,
    /// Vendor ID (PCI).
    pub vendor_id: u32,
    /// Device ID (PCI).
    pub device_id: u32,
    /// Backend API (Vulkan, Metal, DX12, etc.).
    pub backend: String,
    /// Device type.
    pub device_type: GpuDeviceType,
    /// Max compute workgroups per dimension.
    pub max_compute_workgroups_per_dimension: u32,
    /// Max compute workgroup size (x * y * z).
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    /// Max buffer size in bytes.
    pub max_buffer_size: u64,
    /// Whether shader-f64 feature is supported.
    pub supports_shader_f64: bool,
}

/// GPU device type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuDeviceType {
    Discrete,
    Integrated,
    Virtual,
    Cpu,
    Other,
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

        let device_type = match info.device_type {
            wgpu::DeviceType::DiscreteGpu => GpuDeviceType::Discrete,
            wgpu::DeviceType::IntegratedGpu => GpuDeviceType::Integrated,
            wgpu::DeviceType::VirtualGpu => GpuDeviceType::Virtual,
            wgpu::DeviceType::Cpu => GpuDeviceType::Cpu,
            _ => GpuDeviceType::Other,
        };

        let adapter_info = GpuAdapterInfo {
            name: name.clone(),
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
            vendor_id: info.vendor,
            device_id: info.device,
            backend: format!("{:?}", info.backend),
            device_type,
            max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
            max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
            max_buffer_size: limits.max_buffer_size,
            supports_shader_f64: adapter.features().contains(wgpu::Features::SHADER_F64),
        };

        Ok(Self {
            name,
            capabilities,
            adapter_info,
            _adapter: adapter,
            _device: Arc::new(device),
            _queue: Arc::new(queue),
        })
    }
}

impl WgpuComputeUnit {
    /// Get the adapter identity info for driver-aware decisions.
    ///
    /// barraCuda reads this to build its `GpuDriverProfile` (NVK detection,
    /// f64 workarounds, workgroup tuning) without depending on wgpu.
    #[must_use]
    pub fn adapter_info(&self) -> &GpuAdapterInfo {
        &self.adapter_info
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
        // toadStool provides hardware discovery and capability probing.
        // GPU compute dispatch (shaders, pipelines) is barraCuda's domain.
        // Use barraCuda's ComputeDispatch for actual GPU execution.
        Err(ComputeError::ExecutionFailed(
            "GPU compute dispatch is barraCuda's domain — discover via 'compute' capability IPC"
                .to_string(),
        ))
    }
}

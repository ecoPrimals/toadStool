// SPDX-License-Identifier: AGPL-3.0-only
//! wgpu adapter initialization — device request, capability probing, GpuAdapterInfo construction.

use super::types::{GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, is_nvidia_ada_lovelace};
use crate::types::*;
use std::sync::Arc;

use super::WgpuComputeUnit;

impl WgpuComputeUnit {
    /// Create from a wgpu adapter
    pub async fn from_adapter(adapter: wgpu::Adapter) -> Result<Self, ComputeError> {
        const GIB: u64 = 1024 * 1024 * 1024;
        const DISCRETE_MIN_VRAM: u64 = 4 * GIB;
        const INTEGRATED_MIN_VRAM: u64 = GIB;
        const VIRTUAL_MIN_VRAM: u64 = 2 * GIB;
        const CPU_MIN_VRAM: u64 = GIB / 2;
        const DISCRETE_BW_BPS: u64 = 500_000_000_000;
        const INTEGRATED_BW_BPS: u64 = 50_000_000_000;
        const VIRTUAL_BW_BPS: u64 = 100_000_000_000;
        const CPU_BW_BPS: u64 = 25_000_000_000;

        let info = adapter.get_info();
        let name = info.name.clone();

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
        let max_wg = limits.max_compute_workgroups_per_dimension;

        let (memory_capacity, compute_throughput, power_profile, bandwidth, batch_size) =
            match info.device_type {
                wgpu::DeviceType::DiscreteGpu => (
                    limits.max_buffer_size.max(DISCRETE_MIN_VRAM),
                    10e12,
                    PowerProfile::High,
                    DISCRETE_BW_BPS,
                    65_536_usize,
                ),
                wgpu::DeviceType::IntegratedGpu => (
                    limits.max_buffer_size.max(INTEGRATED_MIN_VRAM),
                    1e12,
                    PowerProfile::Medium,
                    INTEGRATED_BW_BPS,
                    16_384,
                ),
                wgpu::DeviceType::VirtualGpu => (
                    limits.max_buffer_size.max(VIRTUAL_MIN_VRAM),
                    5e12,
                    PowerProfile::Medium,
                    VIRTUAL_BW_BPS,
                    32_768,
                ),
                wgpu::DeviceType::Cpu => (
                    limits.max_buffer_size.max(CPU_MIN_VRAM),
                    100e9,
                    PowerProfile::Low,
                    CPU_BW_BPS,
                    4_096,
                ),
                _ => (
                    limits.max_buffer_size.max(INTEGRATED_MIN_VRAM),
                    1e12,
                    PowerProfile::Medium,
                    INTEGRATED_BW_BPS,
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

        let supports_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
        let is_nvk = info.driver.contains("nvk") || info.driver.contains("nouveau");

        let is_nvk_volta = is_nvk
            && (info.name.contains("Titan V")
                || info.name.contains("Tesla V100")
                || info.name.contains("Quadro GV100"));
        let f64_compute_unreliable = is_nvk_volta;

        let is_ada_lovelace = is_nvidia_ada_lovelace(&info.name);
        let is_proprietary_nvidia = info.driver.contains("nvidia") && !info.driver.contains("nvk");

        let min_subgroup_size = limits.min_subgroup_size;
        let max_subgroup_size = limits.max_subgroup_size;

        let safe_alloc = if is_nvk {
            // NVK PTE fault at ~1.2 GB on Nouveau — guard against it
            1_200_000_000_u64
        } else {
            limits.max_buffer_size
        };

        let fingerprint = HardwareFingerprint::from_adapter_info(
            &info,
            device_type,
            supports_f64,
            f64_compute_unreliable,
            max_wg,
        );

        // groundSpring V84-V85: naga/SPIR-V f64 shared-memory reductions return
        // zeros on ALL tested GPUs. Until coralDriver provides a native binary
        // path, this is always false via the standard wgpu/naga pipeline.
        let f64_shared_memory_reliable = false;

        // f64 zeros risk: NVK + FP64 devices, or Ada Lovelace + proprietary
        // driver (groundSpring V98 + neuralSpring V90 both report fused
        // VarianceF64/CorrelationF64 returning 0.0 on RTX 40xx).
        let f64_zeros_risk = (is_nvk && supports_f64) || (is_ada_lovelace && is_proprietary_nvidia);

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
            supports_shader_f64: supports_f64,
            f64_compute_unreliable,
            f64_shared_memory_reliable,
            f64_zeros_risk,
            min_subgroup_size,
            max_subgroup_size,
            fingerprint,
            safe_allocation_limit: safe_alloc,
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

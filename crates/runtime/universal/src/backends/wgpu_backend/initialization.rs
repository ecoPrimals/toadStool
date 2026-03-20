// SPDX-License-Identifier: AGPL-3.0-or-later
//! wgpu adapter initialization — device request, capability probing, GpuAdapterInfo construction.

use super::types::{GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, is_nvidia_ada_lovelace};
use crate::types::*;
use std::sync::Arc;
use toadstool_core::silicon::{RtCoreGen, SiliconCapabilities, SiliconUnit, TensorCoreGen};

use super::WgpuComputeUnit;

const NVIDIA_VENDOR_ID: u32 = 0x10de;
const AMD_VENDOR_ID: u32 = 0x1002;
const INTEL_VENDOR_ID: u32 = 0x8086;

/// Probe silicon capabilities from wgpu adapter info.
///
/// Uses vendor ID, device name, and feature flags to infer which
/// fixed-function hardware units are present. This doesn't require
/// VFIO — it works with the standard wgpu adapter probe.
pub(crate) fn probe_silicon_capabilities(
    info: &wgpu::AdapterInfo,
    device_type: GpuDeviceType,
) -> SiliconCapabilities {
    let name_lower = info.name.to_lowercase();
    let is_discrete = matches!(device_type, GpuDeviceType::Discrete);
    let is_nvidia = info.vendor == NVIDIA_VENDOR_ID;
    let is_amd = info.vendor == AMD_VENDOR_ID;
    let is_intel = info.vendor == INTEL_VENDOR_ID;

    let tensor_cores = if is_nvidia {
        detect_nvidia_tensor_gen(&name_lower)
    } else {
        None
    };

    let rt_cores = if is_nvidia {
        detect_nvidia_rt_gen(&name_lower)
    } else if is_amd {
        detect_amd_rt_gen(&name_lower)
    } else {
        None
    };

    let has_video_encoder = is_discrete && (is_nvidia || is_amd || is_intel);

    let (estimated_tmu_count, estimated_rop_count) =
        estimate_tmu_rop(&name_lower, is_nvidia, is_amd);

    let has_graphics = is_discrete || matches!(device_type, GpuDeviceType::Integrated);

    let mut available_units = vec![SiliconUnit::ShaderCore];
    if has_graphics {
        available_units.push(SiliconUnit::TextureUnit);
        available_units.push(SiliconUnit::Rop);
        available_units.push(SiliconUnit::Rasterizer);
        available_units.push(SiliconUnit::DepthBuffer);
        available_units.push(SiliconUnit::Tessellator);
    }
    if tensor_cores.is_some() {
        available_units.push(SiliconUnit::TensorCore);
    }
    if rt_cores.is_some() {
        available_units.push(SiliconUnit::RtCore);
    }
    if has_video_encoder {
        available_units.push(SiliconUnit::VideoEncoder);
    }

    SiliconCapabilities {
        tensor_cores,
        rt_cores,
        has_video_encoder,
        estimated_tmu_count,
        estimated_rop_count,
        rasterizer_available: has_graphics,
        tessellator_available: has_graphics,
        available_units,
    }
}

fn detect_nvidia_tensor_gen(name: &str) -> Option<TensorCoreGen> {
    if name.contains("h100") || name.contains("h200") {
        Some(TensorCoreGen::Hopper)
    } else if name.contains("rtx 40")
        || name.contains("rtx40")
        || name.contains("l40")
        || name.contains("ada")
    {
        Some(TensorCoreGen::Ada)
    } else if name.contains("rtx 30")
        || name.contains("rtx30")
        || name.contains("a100")
        || name.contains("a6000")
    {
        Some(TensorCoreGen::Ampere)
    } else if name.contains("rtx 20")
        || name.contains("rtx20")
        || name.contains("t4")
        || name.contains("quadro rtx")
    {
        Some(TensorCoreGen::Turing)
    } else if name.contains("titan v") || name.contains("v100") || name.contains("gv100") {
        Some(TensorCoreGen::Volta)
    } else {
        None
    }
}

fn detect_nvidia_rt_gen(name: &str) -> Option<RtCoreGen> {
    if name.contains("rtx 40")
        || name.contains("rtx40")
        || name.contains("l40")
        || name.contains("ada")
    {
        Some(RtCoreGen::Ada)
    } else if name.contains("rtx 30") || name.contains("rtx30") || name.contains("a6000") {
        Some(RtCoreGen::Ampere)
    } else if name.contains("rtx 20") || name.contains("rtx20") || name.contains("quadro rtx") {
        Some(RtCoreGen::Turing)
    } else {
        None
    }
}

fn detect_amd_rt_gen(name: &str) -> Option<RtCoreGen> {
    if name.contains("rx 7") || name.contains("rdna 3") {
        Some(RtCoreGen::Ampere) // RDNA 3 RT is roughly 2nd-gen equivalent
    } else if name.contains("rx 6") || name.contains("rdna 2") {
        Some(RtCoreGen::Turing) // RDNA 2 RT is roughly 1st-gen equivalent
    } else {
        None
    }
}

fn estimate_tmu_rop(name: &str, is_nvidia: bool, is_amd: bool) -> (u32, u32) {
    if is_nvidia {
        if name.contains("rtx 3090") || name.contains("rtx 4090") {
            (328, 112)
        } else if name.contains("rtx 3080") || name.contains("rtx 4080") {
            (272, 96)
        } else if name.contains("rtx 3070") || name.contains("rtx 4070") {
            (184, 96)
        } else if name.contains("rtx 3060") || name.contains("rtx 4060") {
            (112, 48)
        } else if name.contains("titan v") {
            (320, 96)
        } else {
            (128, 64) // conservative default for unknown NVIDIA
        }
    } else if is_amd {
        if name.contains("6950") || name.contains("7900") {
            (320, 128)
        } else if name.contains("6800") || name.contains("7800") {
            (240, 96)
        } else if name.contains("mi50") || name.contains("mi60") {
            (256, 64)
        } else {
            (128, 64)
        }
    } else {
        (64, 32) // Intel / other
    }
}

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

        let silicon = probe_silicon_capabilities(&info, device_type);

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
            silicon: Some(silicon),
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

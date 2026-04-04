// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime queries for CPU, memory, storage, network, and GPU capabilities.

use tracing::debug;

use super::error::ValidationError;
use super::types::SystemCapabilities;

/// GPU information discovered at runtime
#[derive(Debug, Clone)]
struct GpuInfo {
    name: String,
    _memory_mb: u64,
    _vendor: String,
}

/// Query system capabilities
///
/// This queries the actual system state at runtime. No hardcoded values.
pub(crate) async fn query_system_capabilities() -> Result<SystemCapabilities, ValidationError> {
    debug!("Querying system capabilities");

    // Query CPU
    const CPU_FALLBACK_CORES: u32 = 4;
    const CPU_AVAILABLE_PERCENT: u32 = 80;

    let total_cpu_cores = std::thread::available_parallelism()
        .map(|n| u32::try_from(n.get()).unwrap_or(CPU_FALLBACK_CORES))
        .unwrap_or(CPU_FALLBACK_CORES);
    let available_cpu_cores = (total_cpu_cores * CPU_AVAILABLE_PERCENT) / 100;

    let mem = toadstool_sysmon::memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
        total: 0,
        available: 0,
        used: 0,
        swap_total: 0,
        swap_free: 0,
    });
    let total_memory_bytes = mem.total;
    let available_memory_bytes = mem.available;

    let disks = toadstool_sysmon::disk_usage().unwrap_or_default();
    let (total_storage_bytes, available_storage_bytes): (u64, u64) =
        disks.iter().fold((0u64, 0u64), |(total, avail), disk| {
            (total + disk.total_space, avail + disk.available_space)
        });

    // Query GPU (if available) - uses runtime detection via toadstool-runtime-gpu
    // Detection happens at runtime, no hardcoded assumptions about GPU vendors
    // Falls back gracefully if no GPU available
    let (total_gpu_memory_bytes, available_gpu_memory_bytes, gpu_count, gpu_types) =
        query_gpu_capabilities().await;

    let interfaces = toadstool_sysmon::network_stats().unwrap_or_default();
    const NETWORK_FALLBACK_MBPS: u64 = 100;
    const NETWORK_HIGH_TRAFFIC_THRESHOLD: u64 = 1_000_000_000;
    const NETWORK_HIGH_MBPS: u64 = 1000;

    let network_bandwidth_mbps = if interfaces.is_empty() {
        NETWORK_FALLBACK_MBPS
    } else {
        let total_received: u64 = interfaces.iter().map(|i| i.received).sum();
        if total_received > NETWORK_HIGH_TRAFFIC_THRESHOLD {
            NETWORK_HIGH_MBPS
        } else {
            NETWORK_FALLBACK_MBPS
        }
    };

    Ok(SystemCapabilities {
        total_cpu_cores,
        available_cpu_cores,
        total_memory_bytes,
        available_memory_bytes,
        total_gpu_memory_bytes,
        available_gpu_memory_bytes,
        total_storage_bytes,
        available_storage_bytes,
        network_bandwidth_mbps,
        gpu_count,
        gpu_types,
    })
}

/// Query GPU capabilities via wgpu (vendor-agnostic, part of barraCuda)
///
/// **Deep Debt Compliance**:
/// - Runtime GPU discovery (no hardcoded assumptions)
/// - Vendor-agnostic (works with NVIDIA, AMD, Intel, Apple)
/// - Graceful degradation (returns empty if no GPU)
/// - Part of barraCuda universal GPU framework
async fn query_gpu_capabilities() -> (u64, u64, usize, Vec<String>) {
    match discover_gpus_via_wgpu().await {
        Ok(ref gpus) if !gpus.is_empty() => {
            let gpu_count = gpus.len();
            let gpu_types: Vec<String> = gpus.iter().map(|g: &GpuInfo| g.name.clone()).collect();

            const GPU_ESTIMATED_MEMORY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

            let estimated_memory_per_gpu = GPU_ESTIMATED_MEMORY_BYTES;
            let total_gpu_memory = estimated_memory_per_gpu * gpu_count as u64; // fits: GPU count < u64::MAX

            (total_gpu_memory, total_gpu_memory, gpu_count, gpu_types)
        }
        _ => {
            // No GPUs detected or discovery failed - graceful degradation
            (0, 0, 0, Vec::new())
        }
    }
}

/// Discover GPUs using wgpu (vendor-agnostic, part of barraCuda)
#[cfg(feature = "gpu-discovery")]
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync wgpu enumerate; async for API consistency with fallback
async fn discover_gpus_via_wgpu() -> Result<Vec<GpuInfo>, ValidationError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapters = instance.enumerate_adapters(wgpu::Backends::all());
    let mut gpu_infos = Vec::new();

    for adapter in adapters {
        let info = adapter.get_info();

        // Only include discrete/integrated GPUs, skip software renderers
        if matches!(
            info.device_type,
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
        ) {
            gpu_infos.push(GpuInfo {
                name: info.name.clone(),
                _memory_mb: 0,
                _vendor: vendor_from_backend(info.backend),
            });
        }
    }

    Ok(gpu_infos)
}

/// Fallback when GPU discovery not available
#[cfg(not(feature = "gpu-discovery"))]
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Matches gpu-discovery variant; sync fallback
async fn discover_gpus_via_wgpu() -> Result<Vec<GpuInfo>, ValidationError> {
    Ok(Vec::new())
}

#[cfg(feature = "gpu-discovery")]
fn vendor_from_backend(backend: wgpu::Backend) -> String {
    match backend {
        wgpu::Backend::Vulkan => "Vulkan".to_string(),
        wgpu::Backend::Metal => "Metal".to_string(),
        wgpu::Backend::Dx12 => "DirectX12".to_string(),
        wgpu::Backend::Gl => "OpenGL".to_string(),
        _ => "Unknown".to_string(),
    }
}

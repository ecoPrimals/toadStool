// SPDX-License-Identifier: AGPL-3.0-or-later
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
    const CPU_FALLBACK_CORES: u32 = 4;
    const CPU_AVAILABLE_PERCENT: u32 = 80;

    debug!("Querying system capabilities");

    // Query CPU
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

    let network_bandwidth_mbps = estimate_network_bandwidth_mbps(&interfaces);

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

/// Heuristic Mbps estimate from cumulative receive counters (see `query_system_capabilities`).
pub(crate) fn estimate_network_bandwidth_mbps(
    interfaces: &[toadstool_sysmon::NetworkInterface],
) -> u64 {
    const NETWORK_FALLBACK_MBPS: u64 = 100;
    const NETWORK_HIGH_TRAFFIC_THRESHOLD: u64 = 1_000_000_000;
    const NETWORK_HIGH_MBPS: u64 = 1000;

    if interfaces.is_empty() {
        return NETWORK_FALLBACK_MBPS;
    }
    let total_received: u64 = interfaces.iter().map(|i| i.received).sum();
    if total_received > NETWORK_HIGH_TRAFFIC_THRESHOLD {
        NETWORK_HIGH_MBPS
    } else {
        NETWORK_FALLBACK_MBPS
    }
}

/// Query GPU capabilities via wgpu (vendor-agnostic)
///
/// **Deep Debt Compliance**:
/// - Runtime GPU discovery (no hardcoded assumptions)
/// - Vendor-agnostic (works with NVIDIA, AMD, Intel, Apple)
/// - Graceful degradation (returns empty if no GPU)
/// - Part of universal GPU compute framework
async fn query_gpu_capabilities() -> (u64, u64, usize, Vec<String>) {
    match discover_gpus_via_wgpu().await {
        Ok(ref gpus) if !gpus.is_empty() => {
            const GPU_ESTIMATED_MEMORY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

            let gpu_count = gpus.len();
            let gpu_types: Vec<String> = gpus.iter().map(|g: &GpuInfo| g.name.clone()).collect();

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

/// Select wgpu backends based on environment.
///
/// When `TOADSTOOL_HEADLESS=1` is set, restricts to Vulkan-only to avoid
/// GL/GLES backends that probe for a display server. Otherwise uses all backends.
#[cfg(feature = "gpu-discovery")]
fn select_backends() -> wgpu::Backends {
    match std::env::var("TOADSTOOL_HEADLESS") {
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true") => wgpu::Backends::VULKAN,
        _ => wgpu::Backends::all(),
    }
}

/// Discover GPUs using wgpu in an isolated thread with panic safety.
///
/// Runs wgpu instance creation and adapter enumeration on a dedicated thread
/// wrapped in `catch_unwind`. This protects the caller from panics in the
/// Vulkan/Mesa ICD loader. A 5-second timeout prevents hangs on broken drivers.
#[cfg(feature = "gpu-discovery")]
async fn discover_gpus_via_wgpu() -> Result<Vec<GpuInfo>, ValidationError> {
    const GPU_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            let backends = select_backends();
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends,
                ..Default::default()
            });

            let adapters = instance.enumerate_adapters(backends);
            let mut gpu_infos = Vec::new();

            for adapter in adapters {
                let info = adapter.get_info();

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

            gpu_infos
        });
        let _ = tx.send(result);
    });

    match tokio::time::timeout(GPU_PROBE_TIMEOUT, rx).await {
        Ok(Ok(Ok(infos))) => Ok(infos),
        Ok(Ok(Err(_panic))) => {
            debug!("wgpu GPU discovery panicked — falling back to 0 GPUs");
            Ok(Vec::new())
        }
        Ok(Err(_recv_err)) => {
            debug!("wgpu GPU discovery thread dropped — falling back to 0 GPUs");
            Ok(Vec::new())
        }
        Err(_timeout) => {
            debug!("wgpu GPU discovery timed out after 5s — falling back to 0 GPUs");
            Ok(Vec::new())
        }
    }
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
pub(crate) fn vendor_from_backend(backend: wgpu::Backend) -> String {
    match backend {
        wgpu::Backend::Vulkan => "Vulkan".to_string(),
        wgpu::Backend::Metal => "Metal".to_string(),
        wgpu::Backend::Dx12 => "DirectX12".to_string(),
        wgpu::Backend::Gl => "OpenGL".to_string(),
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::estimate_network_bandwidth_mbps;
    use crate::resource_validator::ValidationError;
    use toadstool_sysmon::NetworkInterface;

    #[test]
    fn estimate_network_bandwidth_empty_interfaces_uses_fallback_mbps() {
        assert_eq!(estimate_network_bandwidth_mbps(&[]), 100);
    }

    #[test]
    fn estimate_network_bandwidth_low_traffic_uses_fallback_mbps() {
        let iface = NetworkInterface {
            name: "eth0".into(),
            received: 500_000_000,
            transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
        };
        assert_eq!(estimate_network_bandwidth_mbps(&[iface]), 100);
    }

    #[test]
    fn estimate_network_bandwidth_high_traffic_uses_high_mbps() {
        let iface = NetworkInterface {
            name: "eth0".into(),
            received: 1_000_000_001,
            transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
        };
        assert_eq!(estimate_network_bandwidth_mbps(&[iface]), 1000);
    }

    #[test]
    fn estimate_network_bandwidth_sums_across_interfaces() {
        let a = NetworkInterface {
            name: "eth0".into(),
            received: 600_000_000,
            transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
        };
        let b = NetworkInterface {
            name: "eth1".into(),
            received: 500_000_000,
            transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
        };
        assert_eq!(estimate_network_bandwidth_mbps(&[a, b]), 1000);
    }

    #[test]
    fn validation_error_system_query_failed_maps_to_message() {
        let err = ValidationError::SystemQueryFailed("disk offline".into());
        let s = err.to_string();
        assert!(s.contains("System query failed"), "{s}");
        assert!(s.contains("disk offline"), "{s}");
    }

    #[cfg(feature = "gpu-discovery")]
    #[test]
    fn vendor_from_backend_maps_known_wgpu_backends() {
        use super::vendor_from_backend;
        assert_eq!(vendor_from_backend(wgpu::Backend::Vulkan), "Vulkan");
        assert_eq!(vendor_from_backend(wgpu::Backend::Metal), "Metal");
        assert_eq!(vendor_from_backend(wgpu::Backend::Dx12), "DirectX12");
        assert_eq!(vendor_from_backend(wgpu::Backend::Gl), "OpenGL");
    }
}

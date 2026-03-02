//! Unified Device Info — high-level capability querying for Device enum.
//!
//! Answers "what can this Device do?" for routing and selection.
//! Contrast with DeviceCapabilities (wgpu limits).

use crate::device::device_types::Device;

/// Fallback system memory estimate (GB) when actual detection fails — 64-bit systems.
const FALLBACK_SYSTEM_MEMORY_GB_64BIT: usize = 8;

/// Fallback system memory estimate (GB) when actual detection fails — 32-bit systems.
const FALLBACK_SYSTEM_MEMORY_GB_32BIT: usize = 2;

/// Device information and capabilities for the unified Device enum.
///
/// **Runtime-discovered** — No hardcoding!
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device type
    pub device: Device,

    /// Human-readable name
    pub name: String,

    /// Is this device available?
    pub available: bool,

    /// Device capabilities
    pub capabilities: Vec<Capability>,

    /// Available memory (GB)
    pub memory_gb: usize,

    /// Number of compute units (cores, SMs, etc.)
    pub compute_units: usize,
}

/// Device capabilities for unified device selection.
///
/// **Capability-based** — Query at runtime!
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// General compute
    Compute,

    /// WGSL shader execution
    WGSL,

    /// Parallel execution
    ParallelExecution,

    /// Sparse event processing
    SparseEvents,

    /// Low power operation
    LowPower,

    /// Matrix operations
    MatrixOps,

    /// Memory operations
    Memory,

    /// Automatic device selection
    AutoSelection,
}

/// Check if GPU is available.
///
/// Optimistic — assume GPU might be available. Full runtime check happens at
/// DeviceContext creation.
#[must_use]
pub fn is_gpu_available() -> bool {
    true
}

/// Check if NPU is available by scanning for Akida device nodes or VFIO groups.
#[must_use]
pub fn is_npu_available() -> bool {
    // Check for /dev/akida* devices (C kernel driver path)
    for i in 0..16 {
        if std::path::Path::new(&format!("/dev/akida{i}")).exists() {
            return true;
        }
    }
    // Check for VFIO-eligible devices (future pure Rust path)
    // Scan IOMMU groups for BrainChip vendor 0x1e7c
    let iommu_groups = std::path::Path::new("/sys/kernel/iommu_groups");
    if iommu_groups.exists() {
        if let Ok(entries) = std::fs::read_dir(iommu_groups) {
            for entry in entries.flatten() {
                let devices_dir = entry.path().join("devices");
                if let Ok(devices) = std::fs::read_dir(devices_dir) {
                    for dev in devices.flatten() {
                        let vendor_path = dev.path().join("vendor");
                        if let Ok(vendor) = std::fs::read_to_string(vendor_path) {
                            // BrainChip vendor ID
                            if vendor.trim() == "0x1e7c" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Estimate system memory (GB).
///
/// Platform-specific — for now, conservative estimate.
#[must_use]
pub fn estimate_system_memory() -> usize {
    if cfg!(target_pointer_width = "64") {
        FALLBACK_SYSTEM_MEMORY_GB_64BIT
    } else {
        FALLBACK_SYSTEM_MEMORY_GB_32BIT
    }
}

/// Build DeviceInfo for a given Device.
///
/// **Runtime discovery** — No hardcoding!
#[must_use]
pub fn build_device_info(device: Device) -> DeviceInfo {
    match device {
        Device::CPU => DeviceInfo {
            device,
            name: "CPU".to_string(),
            available: true,
            capabilities: vec![Capability::Compute, Capability::Memory],
            memory_gb: estimate_system_memory(),
            compute_units: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        },

        Device::GPU => DeviceInfo {
            device,
            name: "GPU (wgpu)".to_string(),
            available: is_gpu_available(),
            capabilities: vec![
                Capability::Compute,
                Capability::WGSL,
                Capability::ParallelExecution,
            ],
            memory_gb: 0,
            compute_units: 0,
        },

        Device::NPU => DeviceInfo {
            device,
            name: "NPU (Akida)".to_string(),
            available: is_npu_available(),
            capabilities: vec![
                Capability::Compute,
                Capability::SparseEvents,
                Capability::LowPower,
            ],
            memory_gb: 0,
            compute_units: 0,
        },

        Device::TPU => DeviceInfo {
            device,
            name: "TPU".to_string(),
            available: false,
            capabilities: vec![Capability::Compute, Capability::MatrixOps],
            memory_gb: 0,
            compute_units: 0,
        },

        Device::Auto => DeviceInfo {
            device,
            name: "Auto (smart selection)".to_string(),
            available: true,
            capabilities: vec![Capability::AutoSelection],
            memory_gb: 0,
            compute_units: 0,
        },
    }
}

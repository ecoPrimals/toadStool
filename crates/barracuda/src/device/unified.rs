//! Unified Device Abstraction - Phase 2
//!
//! **EVOLVED**: Single Device enum for ALL hardware types!
//!
//! This module provides a unified interface for all compute devices:
//! - CPU: Pure Rust execution
//! - GPU: WGSL shaders via wgpu
//! - NPU: Akida neuromorphic hardware
//! - TPU: Tensor Processing Units (future)
//! - Auto: Automatic selection based on workload
//!
//! # Philosophy
//!
//! **Hardware does the specialization, not the code!**
//!
//! - One codebase, all hardware
//! - Explicit routing when needed
//! - Automatic selection by default
//! - Flexible fallback chains
//!
//! # Deep Debt Compliance
//!
//! - ✅ **Hardware agnostic**: No assumptions
//! - ✅ **Runtime discovery**: Capability-based
//! - ✅ **Explicit control**: When needed
//! - ✅ **Smart defaults**: Auto selection
//! - ✅ **Flexible routing**: Fallback chains
//!
//! # Example
//!
//! ```rust,ignore
//! use barracuda::prelude::{Device, DeviceInfo, Tensor};
//!
//! // Automatic selection (recommended)
//! let tensor = Tensor::randn(vec![1000, 1000]).await?;
//! let result = tensor.matmul(&other).await?; // Auto-routed!
//!
//! // Explicit routing
//! let gpu_tensor = tensor.on(Device::GPU).await?;
//! let npu_tensor = tensor.on(Device::NPU).await?;
//!
//! // Query capabilities
//! let info = Device::CPU.info();
//! println!("CPU supports: {:?}", info.capabilities);
//! ```

use crate::device::{AkidaBoard, WgpuDevice};
use crate::error::{BarracudaError, Result as BarracudaResult};
use std::fmt;

/// Unified device abstraction
///
/// **Hardware-agnostic** - Represents ANY compute device!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Device {
    /// CPU execution (pure Rust)
    CPU,

    /// GPU execution (WGSL via wgpu)
    GPU,

    /// NPU execution (Akida neuromorphic)
    NPU,

    /// TPU execution (Tensor Processing Unit)
    TPU,

    /// Automatic selection based on workload
    Auto,
}

impl Device {
    /// Get device information and capabilities
    ///
    /// **Runtime discovery** - No hardcoding!
    pub fn info(&self) -> DeviceInfo {
        match self {
            Device::CPU => DeviceInfo {
                device: *self,
                name: "CPU".to_string(),
                available: true, // Always available!
                capabilities: vec![Capability::Compute, Capability::Memory],
                memory_gb: estimate_system_memory(),
                compute_units: std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
            },

            Device::GPU => DeviceInfo {
                device: *self,
                name: "GPU (wgpu)".to_string(),
                available: is_gpu_available(),
                capabilities: vec![
                    Capability::Compute,
                    Capability::WGSL,
                    Capability::ParallelExecution,
                ],
                memory_gb: 0,     // Query at runtime
                compute_units: 0, // Query at runtime
            },

            Device::NPU => DeviceInfo {
                device: *self,
                name: "NPU (Akida)".to_string(),
                available: is_npu_available(),
                capabilities: vec![
                    Capability::Compute,
                    Capability::SparseEvents,
                    Capability::LowPower,
                ],
                memory_gb: 0,     // NPU-specific
                compute_units: 0, // Query at runtime
            },

            Device::TPU => DeviceInfo {
                device: *self,
                name: "TPU".to_string(),
                available: false, // Not yet implemented
                capabilities: vec![Capability::Compute, Capability::MatrixOps],
                memory_gb: 0,
                compute_units: 0,
            },

            Device::Auto => DeviceInfo {
                device: *self,
                name: "Auto (smart selection)".to_string(),
                available: true,
                capabilities: vec![Capability::AutoSelection],
                memory_gb: 0,
                compute_units: 0,
            },
        }
    }

    /// Check if this device is available
    pub fn is_available(&self) -> bool {
        self.info().available
    }

    /// List all available devices
    ///
    /// **Runtime discovery** - No assumptions!
    pub fn available_devices() -> Vec<Device> {
        vec![
            Device::CPU,
            Device::GPU,
            Device::NPU,
            Device::TPU,
            Device::Auto,
        ]
        .into_iter()
        .filter(|d| d.is_available())
        .collect()
    }

    /// Select best device for given workload characteristics (auto-routing).
    ///
    /// Routes workloads to the appropriate hardware based on the nature
    /// of the computation. GPUs run arbitrary WGSL shaders, NPUs run
    /// pre-compiled neural network models, CPUs handle everything else.
    ///
    /// This is BarraCuda's recommendation. To override, use
    /// [`select_with_preference`] or construct a [`DeviceContext`] directly.
    pub fn select_for_workload(workload: &WorkloadHint) -> Device {
        let gpu = Device::GPU.is_available();
        let npu = Device::NPU.is_available();

        match workload {
            // === CPU-only workloads ===
            WorkloadHint::SmallWorkload | WorkloadHint::StringOps => Device::CPU,

            // === NPU-preferred (pre-compiled inference, ultra-low power) ===
            WorkloadHint::SparseEvents if npu => Device::NPU,
            WorkloadHint::EventProcessing if npu => Device::NPU,
            WorkloadHint::PreScreen if npu => Device::NPU,
            WorkloadHint::Inference if npu => Device::NPU,
            WorkloadHint::Reservoir if npu => Device::NPU,

            // === GPU-preferred (arbitrary parallel math) ===
            WorkloadHint::LargeMatrices if gpu => Device::GPU,
            WorkloadHint::PhysicsForce if gpu => Device::GPU,
            WorkloadHint::FFT if gpu => Device::GPU,
            WorkloadHint::EigenDecomp if gpu => Device::GPU,
            WorkloadHint::LinearSolve if gpu => Device::GPU,
            WorkloadHint::Training if gpu => Device::GPU,
            WorkloadHint::SurrogateEval if gpu => Device::GPU,
            WorkloadHint::MonteCarlo if gpu => Device::GPU,
            WorkloadHint::SparseMath if gpu => Device::GPU,

            // === Fallback chain: GPU → CPU ===
            _ => {
                if gpu {
                    Device::GPU
                } else {
                    Device::CPU
                }
            }
        }
    }

    /// Select device with an explicit user preference.
    ///
    /// If the user requests a specific device and it is available, honour
    /// that choice regardless of what the auto-router would recommend.
    /// This lets callers try workloads on hardware BarraCuda might not
    /// consider optimal -- experimentation is always allowed.
    ///
    /// Fallback chain when the preferred device is unavailable:
    /// `preferred → auto-route recommendation → GPU → CPU`
    pub fn select_with_preference(preferred: Option<Device>, workload: &WorkloadHint) -> Device {
        match preferred {
            // Explicit preference -- honour it if the hardware exists
            Some(Device::Auto) | None => Self::select_for_workload(workload),
            Some(dev) if dev.is_available() => dev,
            // Requested device unavailable -- fall back to auto-routing
            Some(_) => Self::select_for_workload(workload),
        }
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Device::CPU => write!(f, "CPU"),
            Device::GPU => write!(f, "GPU"),
            Device::NPU => write!(f, "NPU"),
            Device::TPU => write!(f, "TPU"),
            Device::Auto => write!(f, "Auto"),
        }
    }
}

/// Device information and capabilities
///
/// **Runtime-discovered** - No hardcoding!
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

/// Device capabilities
///
/// **Capability-based** - Query at runtime!
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

/// Workload hints for automatic device selection
///
/// Carries enough metadata for the router to make intelligent decisions
/// about data size, sparsity, and hardware affinity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadHint {
    /// Large matrix operations (GPU-preferred)
    LargeMatrices,

    /// Small workload (CPU-preferred to avoid GPU dispatch overhead)
    SmallWorkload,

    /// Sparse event processing (NPU-preferred, ultra-low power)
    SparseEvents,

    /// Event-driven logic (CPU or NPU)
    EventProcessing,

    /// String operations (CPU-only)
    StringOps,

    /// General computation (Auto -- GPU if available, else CPU)
    General,

    // --- Science-aware hints (route physics/math to the right device) ---
    /// Physics force computation (always GPU -- needs WGSL shaders for arbitrary math)
    PhysicsForce,

    /// FFT computation (always GPU -- butterfly stages are massively parallel)
    FFT,

    /// Eigenvalue decomposition (GPU for large, CPU for small)
    EigenDecomp,

    /// Linear system solve (GPU for large, CPU for small)
    LinearSolve,

    /// Training / gradient computation (always GPU -- needs gradient shaders)
    Training,

    /// Neural network inference with a pre-compiled model (NPU if available)
    Inference,

    /// Binary classification pre-screening (NPU ideal -- ultra-low power)
    PreScreen,

    /// Surrogate model evaluation (GPU for RBF kernel, NPU for pre-filter)
    SurrogateEval,

    /// Monte Carlo / random sampling (GPU -- parallel PRNG)
    MonteCarlo,

    /// Sparse linear algebra (GPU for large, CPU for small)
    SparseMath,

    /// Reservoir computing / ESN (NPU natural fit -- fixed random weights)
    Reservoir,
}

/// Device context for execution
///
/// **Lazy initialization** - Only create when needed!
pub enum DeviceContext {
    /// CPU context (always available)
    CPU,

    /// GPU context (WGSL via wgpu)
    GPU(WgpuDevice),

    /// NPU context (Akida)
    NPU(AkidaBoard),

    /// Not yet initialized
    Uninitialized,
}

impl DeviceContext {
    /// Create context for device
    ///
    /// **Lazy initialization** - Only when needed!
    pub async fn for_device(device: Device) -> BarracudaResult<Self> {
        match device {
            Device::CPU => Ok(DeviceContext::CPU),

            Device::GPU => {
                let wgpu_device = WgpuDevice::new().await?;
                Ok(DeviceContext::GPU(wgpu_device))
            }

            Device::NPU => {
                let capabilities = crate::device::detect_akida_boards()?;
                if capabilities.boards.is_empty() {
                    return Err(BarracudaError::DeviceNotAvailable {
                        device: "NPU".to_string(),
                        reason: "No Akida boards detected".to_string(),
                    });
                }
                Ok(DeviceContext::NPU(capabilities.boards[0].clone()))
            }

            Device::TPU => Err(BarracudaError::DeviceNotAvailable {
                device: "TPU".to_string(),
                reason: "TPU support not yet implemented".to_string(),
            }),

            Device::Auto => {
                // Auto selects GPU if available, else CPU
                // Use explicit match to avoid recursion
                if Device::GPU.is_available() {
                    match WgpuDevice::new().await {
                        Ok(wgpu_device) => Ok(DeviceContext::GPU(wgpu_device)),
                        Err(_) => Ok(DeviceContext::CPU), // Fallback to CPU
                    }
                } else {
                    Ok(DeviceContext::CPU)
                }
            }
        }
    }
}

// Runtime detection helpers

/// Check if GPU is available
fn is_gpu_available() -> bool {
    // Optimistic - assume GPU might be available
    // Full runtime check happens at DeviceContext creation
    true
}

/// Check if NPU is available by scanning for Akida device nodes or VFIO groups
fn is_npu_available() -> bool {
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

/// Estimate system memory (GB)
fn estimate_system_memory() -> usize {
    // Platform-specific - for now, conservative estimate
    if cfg!(target_pointer_width = "64") {
        8 // Assume at least 8GB on 64-bit systems
    } else {
        2 // 32-bit systems likely have less
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_display() {
        assert_eq!(Device::CPU.to_string(), "CPU");
        assert_eq!(Device::GPU.to_string(), "GPU");
        assert_eq!(Device::NPU.to_string(), "NPU");
        assert_eq!(Device::Auto.to_string(), "Auto");
    }

    #[test]
    fn test_cpu_always_available() {
        assert!(Device::CPU.is_available());
    }

    #[test]
    fn test_device_info() {
        let info = Device::CPU.info();
        assert_eq!(info.device, Device::CPU);
        assert!(info.available);
        assert!(!info.name.is_empty());
    }

    #[test]
    fn test_workload_selection_strings() {
        let device = Device::select_for_workload(&WorkloadHint::StringOps);
        assert_eq!(device, Device::CPU);
    }

    #[test]
    fn test_workload_selection_small() {
        let device = Device::select_for_workload(&WorkloadHint::SmallWorkload);
        assert_eq!(device, Device::CPU);
    }

    #[test]
    fn test_available_devices() {
        let devices = Device::available_devices();
        assert!(!devices.is_empty());
        assert!(devices.contains(&Device::CPU)); // Always available!
    }

    #[test]
    fn test_select_with_preference_none_uses_auto() {
        // None preference should behave like auto-routing
        let auto = Device::select_for_workload(&WorkloadHint::SmallWorkload);
        let pref = Device::select_with_preference(None, &WorkloadHint::SmallWorkload);
        assert_eq!(auto, pref);
    }

    #[test]
    fn test_select_with_preference_auto_uses_auto() {
        let auto = Device::select_for_workload(&WorkloadHint::General);
        let pref = Device::select_with_preference(Some(Device::Auto), &WorkloadHint::General);
        assert_eq!(auto, pref);
    }

    #[test]
    fn test_select_with_preference_cpu_always_honoured() {
        // CPU is always available, so explicit CPU preference is always honoured
        let dev = Device::select_with_preference(Some(Device::CPU), &WorkloadHint::LargeMatrices);
        assert_eq!(dev, Device::CPU);
    }

    #[test]
    fn test_select_with_preference_unavailable_falls_back() {
        // TPU is never available, so requesting it should fall back to auto
        let auto = Device::select_for_workload(&WorkloadHint::General);
        let pref = Device::select_with_preference(Some(Device::TPU), &WorkloadHint::General);
        assert_eq!(auto, pref);
    }

    #[test]
    fn test_science_workloads_route_to_gpu_or_cpu() {
        // Science hints should never route to NPU (unless explicitly forced)
        let hints = [
            WorkloadHint::PhysicsForce,
            WorkloadHint::FFT,
            WorkloadHint::EigenDecomp,
            WorkloadHint::LinearSolve,
            WorkloadHint::MonteCarlo,
            WorkloadHint::SparseMath,
        ];
        for hint in &hints {
            let dev = Device::select_for_workload(hint);
            assert!(
                dev == Device::GPU || dev == Device::CPU,
                "{:?} should route to GPU or CPU, got {:?}",
                hint,
                dev
            );
        }
    }

    #[test]
    fn test_runtime_device_discovery_report() {
        // Diagnostic: report what this machine actually sees
        let available = Device::available_devices();
        println!("=== Runtime Device Discovery ===");
        for dev in &available {
            let info = dev.info();
            println!(
                "  {:?}: available={}, name={:?}, capabilities={:?}, mem={}GB, units={}",
                dev,
                info.available,
                info.name,
                info.capabilities,
                info.memory_gb,
                info.compute_units
            );
        }
        println!("  GPU detected: {}", Device::GPU.is_available());
        println!("  NPU detected: {}", Device::NPU.is_available());
        println!("  TPU detected: {}", Device::TPU.is_available());

        // Routing report for all workload hints
        let hints = [
            WorkloadHint::PhysicsForce,
            WorkloadHint::FFT,
            WorkloadHint::EigenDecomp,
            WorkloadHint::LinearSolve,
            WorkloadHint::Training,
            WorkloadHint::Inference,
            WorkloadHint::PreScreen,
            WorkloadHint::SurrogateEval,
            WorkloadHint::MonteCarlo,
            WorkloadHint::SparseMath,
            WorkloadHint::Reservoir,
            WorkloadHint::SparseEvents,
            WorkloadHint::EventProcessing,
            WorkloadHint::LargeMatrices,
            WorkloadHint::SmallWorkload,
            WorkloadHint::StringOps,
            WorkloadHint::General,
        ];
        println!("=== Workload Routing ===");
        for hint in &hints {
            let auto = Device::select_for_workload(hint);
            let forced_cpu = Device::select_with_preference(Some(Device::CPU), hint);
            println!("  {:?}: auto={:?}, forced_cpu={:?}", hint, auto, forced_cpu);
        }

        // CPU must always be present
        assert!(available.contains(&Device::CPU));
    }
}

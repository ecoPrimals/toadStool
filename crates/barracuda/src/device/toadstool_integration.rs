//! ToadStool ↔ BarraCUDA Hardware Integration
//!
//! ToadStool discovers hardware. BarraCUDA runs math on it.
//!
//! Architecture:
//! - GPU/CPU: Run ANY operation via WGSL shaders (same code, different speed)
//! - NPU: Runs pre-compiled SNN models for inference (not general compute)
//!
//! ToadStool routes workloads to the right hardware based on what each can do.
//! Hardware guides its own performance - same WGSL on GPU vs CPU gives
//! identical results at different speeds.

use anyhow::Result;
use toadstool_core::{HardwareManager, HardwareType};

// ─── Discovery ───────────────────────────────────────────────────────────────

/// Discover all available compute devices via ToadStool
///
/// Returns a `HardwareManager` with every device ToadStool can find:
/// GPUs (via sysfs), NPUs (via PCIe scan), CPU (always available).
pub fn discover_devices() -> Result<HardwareManager> {
    HardwareManager::discover()
}

/// Check if GPU compute is available
pub fn has_gpu() -> bool {
    HardwareManager::discover()
        .map(|hw| hw.has_gpu())
        .unwrap_or(false)
}

/// Check if NPU compute is available
pub fn has_npu() -> bool {
    HardwareManager::discover()
        .map(|hw| hw.has_npu())
        .unwrap_or(false)
}

// ─── Workload Classification ─────────────────────────────────────────────────

/// What kind of work needs to be done (hardware routing level)
///
/// ToadStool uses this to pick the right hardware.
/// Different hardware excels at different workloads.
///
/// Note: This is distinct from `capabilities::WorkloadType` which classifies
/// individual operations. This classifies entire workload *domains* for
/// hardware routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareWorkload {
    /// Dense tensor math (matmul, conv, etc.) → GPU preferred, CPU fallback
    TensorOps,
    /// Neural network training/inference → GPU preferred, CPU fallback
    NeuralNetwork,
    /// Spiking neural network inference → NPU preferred (native SNN), GPU fallback
    SpikingNetwork,
    /// Reservoir computing / echo state → NPU preferred (event-driven), GPU fallback
    ReservoirComputing,
    /// Genomics (k-mer filtering, alignment) → NPU for sparse patterns, GPU for dense
    Genomics,
    /// Bioinformatics pipelines → NPU for filtering, GPU for compute
    Bioinformatics,
    /// Scientific computing (Cholesky, RBF, FFT) → GPU preferred, CPU fallback
    ScientificCompute,
    /// Homomorphic encryption (NTT, key switch) → GPU preferred, CPU fallback
    HomomorphicEncryption,
}

// ─── Device Selection ────────────────────────────────────────────────────────

/// Which hardware to target and how
///
/// This is not just a label - it describes what the hardware can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSelection {
    /// GPU via WGPU/WGSL - runs any operation, hardware-accelerated
    Gpu,
    /// CPU via WGPU software rasterizer - runs any operation, slower
    Cpu,
    /// NPU via Akida driver - runs pre-compiled SNN models only
    ///
    /// Cannot run arbitrary WGSL. For general math, falls back to GPU/CPU.
    /// Excels at: sparse inference, event-driven processing, low-power operation.
    Npu,
}

impl DeviceSelection {
    /// Can this device run arbitrary WGSL compute shaders?
    ///
    /// GPU and CPU: yes (via WGPU).
    /// NPU: no (inference-only chip, pre-compiled models only).
    pub fn supports_wgsl(self) -> bool {
        matches!(self, Self::Gpu | Self::Cpu)
    }

    /// Is this device best for sparse/event-driven workloads?
    pub fn is_event_driven(self) -> bool {
        matches!(self, Self::Npu)
    }
}

// ─── Routing Logic ───────────────────────────────────────────────────────────

/// Select best device for a workload
///
/// The routing logic is honest about what each device can do:
/// - GPU: Best for dense parallel compute (WGSL shaders)
/// - CPU: Fallback for everything (WGSL via software rasterizer)
/// - NPU: Best for sparse SNN inference (pre-compiled models only)
///
/// For general math (matmul, Cholesky, RBF, FFT), always routes to GPU/CPU
/// because NPU cannot run arbitrary compute.
pub fn select_best_device(workload_type: HardwareWorkload) -> Result<DeviceSelection> {
    let hw = HardwareManager::discover()?;

    match workload_type {
        // Dense compute: GPU > CPU (NPU can't run WGSL)
        HardwareWorkload::TensorOps
        | HardwareWorkload::NeuralNetwork
        | HardwareWorkload::ScientificCompute
        | HardwareWorkload::HomomorphicEncryption => {
            if hw.has_gpu() {
                Ok(DeviceSelection::Gpu)
            } else {
                Ok(DeviceSelection::Cpu)
            }
        }

        // Event-driven: NPU > GPU > CPU
        // NPU runs pre-compiled SNN models natively
        // If no NPU, GPU runs SNN simulation via WGSL
        HardwareWorkload::SpikingNetwork | HardwareWorkload::ReservoirComputing => {
            if hw.has_npu() {
                Ok(DeviceSelection::Npu)
            } else if hw.has_gpu() {
                Ok(DeviceSelection::Gpu)
            } else {
                Ok(DeviceSelection::Cpu)
            }
        }

        // Mixed workloads: NPU for sparse filtering, GPU for dense compute
        // Route to NPU if available (sparse patterns), else GPU
        HardwareWorkload::Genomics | HardwareWorkload::Bioinformatics => {
            if hw.has_npu() {
                Ok(DeviceSelection::Npu)
            } else if hw.has_gpu() {
                Ok(DeviceSelection::Gpu)
            } else {
                Ok(DeviceSelection::Cpu)
            }
        }
    }
}

/// Select device with explicit hardware preference
///
/// Tries the requested hardware first, falls back if unavailable.
/// Always returns a usable device - never fails on working systems.
pub fn select_device_prefer(preferred: DeviceSelection) -> Result<DeviceSelection> {
    let hw = HardwareManager::discover()?;

    match preferred {
        DeviceSelection::Gpu => {
            if hw.has_gpu() {
                Ok(DeviceSelection::Gpu)
            } else {
                Ok(DeviceSelection::Cpu)
            }
        }
        DeviceSelection::Npu => {
            if hw.has_npu() {
                Ok(DeviceSelection::Npu)
            } else if hw.has_gpu() {
                Ok(DeviceSelection::Gpu)
            } else {
                Ok(DeviceSelection::Cpu)
            }
        }
        DeviceSelection::Cpu => Ok(DeviceSelection::Cpu),
    }
}

/// Get a full hardware report
///
/// Returns what ToadStool found and what BarraCUDA can target.
pub fn hardware_report() -> Result<HardwareReport> {
    let hw = HardwareManager::discover()?;
    let wgpu_adapters = super::WgpuDevice::enumerate_adapters();

    let gpu_count = hw.devices_by_type(HardwareType::Gpu).len();
    let npu_count = hw.devices_by_type(HardwareType::Npu).len();
    let wgpu_gpu_count = wgpu_adapters
        .iter()
        .filter(|a| {
            matches!(
                a.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            )
        })
        .count();
    let wgpu_cpu_count = wgpu_adapters
        .iter()
        .filter(|a| a.device_type == wgpu::DeviceType::Cpu)
        .count();

    Ok(HardwareReport {
        toadstool_devices: hw.device_count(),
        gpus_discovered: gpu_count,
        npus_discovered: npu_count,
        wgpu_adapters: wgpu_adapters.len(),
        wgpu_gpu_adapters: wgpu_gpu_count,
        wgpu_cpu_adapters: wgpu_cpu_count,
        can_run_wgsl_on_gpu: wgpu_gpu_count > 0,
        can_run_wgsl_on_cpu: wgpu_cpu_count > 0,
        can_run_npu_inference: npu_count > 0,
        adapter_names: wgpu_adapters.iter().map(|a| a.name.clone()).collect(),
    })
}

/// Summary of available hardware
#[derive(Debug, Clone)]
pub struct HardwareReport {
    /// Total devices ToadStool found (GPUs + NPUs + CPU)
    pub toadstool_devices: usize,
    /// GPU devices found via sysfs
    pub gpus_discovered: usize,
    /// NPU devices found via PCIe scan
    pub npus_discovered: usize,
    /// Total WGPU adapters (GPU + CPU software)
    pub wgpu_adapters: usize,
    /// WGPU adapters that are real GPUs
    pub wgpu_gpu_adapters: usize,
    /// WGPU adapters that are CPU software rasterizers
    pub wgpu_cpu_adapters: usize,
    /// Can run WGSL on GPU hardware?
    pub can_run_wgsl_on_gpu: bool,
    /// Can run WGSL on CPU (software rasterizer)?
    pub can_run_wgsl_on_cpu: bool,
    /// Can run pre-compiled SNN models on NPU?
    pub can_run_npu_inference: bool,
    /// Names of all WGPU adapters
    pub adapter_names: Vec<String>,
}

impl std::fmt::Display for HardwareReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ToadStool + BarraCUDA Hardware Report")?;
        writeln!(f, "  ToadStool devices: {}", self.toadstool_devices)?;
        writeln!(f, "    GPUs: {}", self.gpus_discovered)?;
        writeln!(f, "    NPUs: {}", self.npus_discovered)?;
        writeln!(f, "  WGPU adapters: {}", self.wgpu_adapters)?;
        writeln!(f, "    GPU adapters: {}", self.wgpu_gpu_adapters)?;
        writeln!(f, "    CPU adapters: {}", self.wgpu_cpu_adapters)?;
        writeln!(f, "  Capabilities:")?;
        writeln!(f, "    WGSL on GPU: {}", self.can_run_wgsl_on_gpu)?;
        writeln!(f, "    WGSL on CPU: {}", self.can_run_wgsl_on_cpu)?;
        writeln!(f, "    NPU inference: {}", self.can_run_npu_inference)?;
        for name in &self.adapter_names {
            writeln!(f, "    - {}", name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_discovery() {
        let hw = discover_devices().expect("Failed to discover devices");
        // CPU always available
        assert!(!hw.devices().is_empty());
    }

    #[test]
    fn test_device_selection_tensor_ops() {
        let selection = select_best_device(HardwareWorkload::TensorOps).unwrap();
        // Should route to GPU or CPU, never NPU (can't run WGSL)
        assert!(selection.supports_wgsl());
    }

    #[test]
    fn test_device_selection_scientific() {
        let selection = select_best_device(HardwareWorkload::ScientificCompute).unwrap();
        assert!(selection.supports_wgsl());
    }

    #[test]
    fn test_device_selection_spiking() {
        let selection = select_best_device(HardwareWorkload::SpikingNetwork);
        // Routes to NPU if available, else GPU/CPU - always succeeds
        assert!(selection.is_ok());
    }

    #[test]
    fn test_wgsl_support() {
        assert!(DeviceSelection::Gpu.supports_wgsl());
        assert!(DeviceSelection::Cpu.supports_wgsl());
        assert!(!DeviceSelection::Npu.supports_wgsl());
    }

    #[test]
    fn test_event_driven() {
        assert!(!DeviceSelection::Gpu.is_event_driven());
        assert!(!DeviceSelection::Cpu.is_event_driven());
        assert!(DeviceSelection::Npu.is_event_driven());
    }

    #[test]
    fn test_prefer_gpu() {
        let selection = select_device_prefer(DeviceSelection::Gpu).unwrap();
        // If GPU exists, get GPU. Otherwise CPU.
        assert!(selection.supports_wgsl());
    }

    #[test]
    fn test_prefer_cpu() {
        let selection = select_device_prefer(DeviceSelection::Cpu).unwrap();
        assert_eq!(selection, DeviceSelection::Cpu);
    }

    #[test]
    fn test_hardware_report() {
        let report = hardware_report().unwrap();
        println!("{}", report);
        assert!(report.toadstool_devices > 0, "Should find at least CPU");
        assert!(
            report.wgpu_adapters > 0,
            "Should find at least one WGPU adapter"
        );
    }
}

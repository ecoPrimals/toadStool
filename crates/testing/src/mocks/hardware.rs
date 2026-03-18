// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mock hardware backends for headless CI parity.
//!
//! Provides synthetic GPU adapter info and NPU responses that match the API
//! shape of real hardware. Tests that depend on `GpuAdapterInfo` fields or
//! NPU inference results can use these mocks instead of `#[ignore]`.

use serde::{Deserialize, Serialize};

/// Mock GPU adapter matching the shape of `GpuAdapterInfo` in
/// `toadstool-runtime-universal`. Tests use this for assertions on adapter
/// fields without requiring actual GPU hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockGpuAdapter {
    pub name: String,
    pub driver: String,
    pub driver_info: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend: String,
    pub device_type: String,
    pub max_compute_workgroups_per_dimension: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    pub max_buffer_size: u64,
    pub supports_shader_f64: bool,
    pub f64_compute_unreliable: bool,
    pub f64_shared_memory_reliable: bool,
    pub f64_zeros_risk: bool,
    pub min_subgroup_size: u32,
    pub max_subgroup_size: u32,
    pub safe_allocation_limit: u64,
    pub vram_bytes: u64,
    pub pcie_gen: u32,
    pub pcie_width: u32,
}

impl MockGpuAdapter {
    /// AMD RX 6950 XT (RDNA2) — strandgate card0 profile.
    #[must_use]
    pub fn amd_rx6950xt() -> Self {
        Self {
            name: "AMD Radeon RX 6950 XT".into(),
            driver: "radv".into(),
            driver_info: "Mesa 24.0".into(),
            vendor_id: 0x1002,
            device_id: 0x73A5,
            backend: "Vulkan".into(),
            device_type: "DiscreteGpu".into(),
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 1024,
            max_compute_workgroup_size_y: 1024,
            max_compute_workgroup_size_z: 1024,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: false,
            f64_compute_unreliable: false,
            f64_shared_memory_reliable: false,
            f64_zeros_risk: false,
            min_subgroup_size: 64,
            max_subgroup_size: 64,
            safe_allocation_limit: 4_294_967_296,
            vram_bytes: 16_000_000_000,
            pcie_gen: 4,
            pcie_width: 16,
        }
    }

    /// NVIDIA RTX 3090 (GA102, Ampere) — strandgate card1 profile.
    #[must_use]
    pub fn nvidia_rtx3090() -> Self {
        Self {
            name: "NVIDIA GeForce RTX 3090".into(),
            driver: "nvidia".into(),
            driver_info: "535.183.01".into(),
            vendor_id: 0x10DE,
            device_id: 0x2204,
            backend: "Vulkan".into(),
            device_type: "DiscreteGpu".into(),
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 1024,
            max_compute_workgroup_size_y: 1024,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: true,
            f64_compute_unreliable: false,
            f64_shared_memory_reliable: false,
            f64_zeros_risk: false,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            safe_allocation_limit: 4_294_967_296,
            vram_bytes: 24_000_000_000,
            pcie_gen: 4,
            pcie_width: 16,
        }
    }

    /// NVIDIA Titan V (SM70, Volta) — f64 native but unreliable via NVK.
    #[must_use]
    pub fn nvidia_titan_v_nvk() -> Self {
        Self {
            name: "NVIDIA TITAN V".into(),
            driver: "nvk".into(),
            driver_info: "Mesa NVK".into(),
            vendor_id: 0x10DE,
            device_id: 0x1D81,
            backend: "Vulkan".into(),
            device_type: "DiscreteGpu".into(),
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 1024,
            max_compute_workgroup_size_y: 1024,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 1_200_000_000,
            supports_shader_f64: true,
            f64_compute_unreliable: true,
            f64_shared_memory_reliable: false,
            f64_zeros_risk: true,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            safe_allocation_limit: 1_200_000_000,
            vram_bytes: 12_000_000_000,
            pcie_gen: 3,
            pcie_width: 16,
        }
    }

    /// CPU/software rasterizer — wgpu CPU backend.
    #[must_use]
    pub fn cpu_software() -> Self {
        Self {
            name: "llvmpipe (LLVM 18.1.8, 256 bits)".into(),
            driver: "llvmpipe".into(),
            driver_info: "Mesa 24.0".into(),
            vendor_id: 0x10005,
            device_id: 0x0000,
            backend: "Vulkan".into(),
            device_type: "Cpu".into(),
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 1024,
            max_compute_workgroup_size_y: 1024,
            max_compute_workgroup_size_z: 1024,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: false,
            f64_compute_unreliable: false,
            f64_shared_memory_reliable: false,
            f64_zeros_risk: false,
            min_subgroup_size: 0,
            max_subgroup_size: 0,
            safe_allocation_limit: 4_294_967_296,
            vram_bytes: 0,
            pcie_gen: 0,
            pcie_width: 0,
        }
    }

    /// Full strandgate fleet: AMD + NVIDIA + CPU.
    #[must_use]
    pub fn strandgate_fleet() -> Vec<Self> {
        vec![
            Self::amd_rx6950xt(),
            Self::nvidia_rtx3090(),
            Self::cpu_software(),
        ]
    }

    /// Whether this adapter is a discrete GPU (not CPU/integrated).
    #[must_use]
    pub fn is_discrete(&self) -> bool {
        self.device_type == "DiscreteGpu"
    }

    /// Whether this adapter supports native f64 reliably.
    #[must_use]
    pub const fn has_reliable_f64(&self) -> bool {
        self.supports_shader_f64 && !self.f64_compute_unreliable
    }
}

/// Mock NPU (Akida AKD1000) backend for headless CI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockNpuBackend {
    pub device_name: String,
    pub pci_slot: String,
    pub num_npes: u32,
    pub firmware_version: String,
    pub power_watts: f64,
    pub temperature_celsius: f64,
}

impl MockNpuBackend {
    /// Akida AKD1000 — strandgate NPU profile.
    #[must_use]
    pub fn akida_akd1000() -> Self {
        Self {
            device_name: "BrainChip Akida AKD1000".into(),
            pci_slot: "0000:e2:00.0".into(),
            num_npes: 80,
            firmware_version: "1.0.0".into(),
            power_watts: 0.5,
            temperature_celsius: 35.0,
        }
    }

    /// Simulate an inference result.
    #[must_use]
    pub fn mock_inference(&self, _input: &[u8]) -> MockNpuInferenceResult {
        MockNpuInferenceResult {
            outputs: vec![0.1, 0.7, 0.15, 0.05],
            latency_us: 500,
            power_draw_mw: 480,
        }
    }
}

/// Mock NPU inference result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockNpuInferenceResult {
    /// Output probabilities / activations.
    pub outputs: Vec<f64>,
    /// Inference latency in microseconds.
    pub latency_us: u64,
    /// Power consumed during inference in milliwatts.
    pub power_draw_mw: u64,
}

/// Mock hardware fleet for comprehensive testing.
#[derive(Debug, Clone)]
pub struct MockHardwareFleet {
    pub gpus: Vec<MockGpuAdapter>,
    pub npus: Vec<MockNpuBackend>,
}

impl Default for MockHardwareFleet {
    fn default() -> Self {
        Self::strandgate()
    }
}

impl MockHardwareFleet {
    /// strandgate fleet: 2 GPUs + 1 NPU.
    #[must_use]
    pub fn strandgate() -> Self {
        Self {
            gpus: MockGpuAdapter::strandgate_fleet(),
            npus: vec![MockNpuBackend::akida_akd1000()],
        }
    }

    /// Headless CI fleet: CPU software rasterizer only.
    #[must_use]
    pub fn headless_ci() -> Self {
        Self {
            gpus: vec![MockGpuAdapter::cpu_software()],
            npus: Vec::new(),
        }
    }

    /// Find GPU by vendor ID.
    #[must_use]
    pub fn find_gpu_by_vendor(&self, vendor_id: u32) -> Option<&MockGpuAdapter> {
        self.gpus.iter().find(|g| g.vendor_id == vendor_id)
    }

    /// All discrete GPUs.
    #[must_use]
    pub fn discrete_gpus(&self) -> Vec<&MockGpuAdapter> {
        self.gpus.iter().filter(|g| g.is_discrete()).collect()
    }

    /// Total VRAM across all GPUs.
    #[must_use]
    pub fn total_vram_bytes(&self) -> u64 {
        self.gpus.iter().map(|g| g.vram_bytes).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amd_adapter_profile() {
        let gpu = MockGpuAdapter::amd_rx6950xt();
        assert_eq!(gpu.vendor_id, 0x1002);
        assert!(!gpu.supports_shader_f64);
        assert!(gpu.is_discrete());
        assert_eq!(gpu.pcie_gen, 4);
    }

    #[test]
    fn test_nvidia_adapter_profile() {
        let gpu = MockGpuAdapter::nvidia_rtx3090();
        assert_eq!(gpu.vendor_id, 0x10DE);
        assert!(gpu.supports_shader_f64);
        assert!(gpu.has_reliable_f64());
        assert!(gpu.is_discrete());
    }

    #[test]
    fn test_titan_v_unreliable_f64() {
        let gpu = MockGpuAdapter::nvidia_titan_v_nvk();
        assert!(gpu.supports_shader_f64);
        assert!(gpu.f64_compute_unreliable);
        assert!(!gpu.has_reliable_f64());
        assert!(gpu.f64_zeros_risk);
    }

    #[test]
    fn test_cpu_software_not_discrete() {
        let gpu = MockGpuAdapter::cpu_software();
        assert!(!gpu.is_discrete());
        assert_eq!(gpu.vram_bytes, 0);
    }

    #[test]
    fn test_strandgate_fleet() {
        let fleet = MockGpuAdapter::strandgate_fleet();
        assert_eq!(fleet.len(), 3);
        assert_eq!(fleet[0].vendor_id, 0x1002);
        assert_eq!(fleet[1].vendor_id, 0x10DE);
    }

    #[test]
    fn test_akida_npu_mock() {
        let npu = MockNpuBackend::akida_akd1000();
        assert_eq!(npu.num_npes, 80);
        let result = npu.mock_inference(&[0u8; 32]);
        assert_eq!(result.outputs.len(), 4);
        assert!(result.latency_us > 0);
    }

    #[test]
    fn test_hardware_fleet_strandgate() {
        let fleet = MockHardwareFleet::strandgate();
        assert_eq!(fleet.gpus.len(), 3);
        assert_eq!(fleet.npus.len(), 1);
        assert_eq!(fleet.discrete_gpus().len(), 2);
        assert!(fleet.total_vram_bytes() > 0);
    }

    #[test]
    fn test_hardware_fleet_headless() {
        let fleet = MockHardwareFleet::headless_ci();
        assert_eq!(fleet.gpus.len(), 1);
        assert!(fleet.npus.is_empty());
        assert!(fleet.discrete_gpus().is_empty());
    }

    #[test]
    fn test_find_gpu_by_vendor() {
        let fleet = MockHardwareFleet::strandgate();
        let amd = fleet.find_gpu_by_vendor(0x1002);
        assert!(amd.is_some());
        assert_eq!(amd.unwrap().driver, "radv");

        let intel = fleet.find_gpu_by_vendor(0x8086);
        assert!(intel.is_none());
    }
}

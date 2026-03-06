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
    /// Whether f64 compute is known to be unreliable on this adapter.
    /// NVK on Volta (SM70) reports f64 support but produces zeros.
    pub f64_compute_unreliable: bool,
    /// Minimum subgroup size (warp size). 0 if unknown.
    pub min_subgroup_size: u32,
    /// Maximum subgroup size. 0 if unknown.
    pub max_subgroup_size: u32,
    /// Hardware fingerprint for backend-agnostic capability comparison.
    pub fingerprint: HardwareFingerprint,
    /// Safe allocation ceiling in bytes (guards against NVK PTE faults).
    pub safe_allocation_limit: u64,
}

/// Backend-agnostic hardware fingerprint for capability comparison
/// across heterogeneous substrates. Aligned with metalForge's
/// substrate characterization model.
#[derive(Debug, Clone)]
pub struct HardwareFingerprint {
    /// Estimated single-precision TFLOPS.
    pub estimated_tflops_f32: f64,
    /// Estimated double-precision TFLOPS (0.0 if no f64 support).
    pub estimated_tflops_f64: f64,
    /// Whether the sovereign pipeline (coralReef + coralDriver) can
    /// drive this GPU without vendor toolchains.
    pub sovereign_capable: bool,
    /// Substrate capabilities discovered at runtime.
    pub capabilities: Vec<SubstrateCapabilityKind>,
}

/// Substrate capability kinds aligned with metalForge's 12-variant model.
///
/// Each capability represents a concrete compute primitive that the
/// substrate can execute. Discovered at runtime, not hardcoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstrateCapabilityKind {
    /// Native f64 arithmetic in shaders.
    F64Native,
    /// DF64 (double-float f32 pairs) emulation.
    Df64Emulation,
    /// Sparse matrix operations (SpMV, SpMM).
    Spmv,
    /// Dense eigenvalue solvers.
    Eigen,
    /// Conjugate gradient / iterative solvers.
    Cg,
    /// FFT / spectral operations.
    Fft,
    /// Molecular dynamics force kernels.
    MdForce,
    /// Monte Carlo / stochastic operations.
    MonteCarlo,
    /// Neural network inference (matmul, activation).
    NnInference,
    /// Reservoir computing (ESN update).
    ReservoirCompute,
    /// Homomorphic encryption primitives (NTT, bootstrap).
    Fhe,
    /// Subgroup / warp-level operations.
    SubgroupOps,
}

impl GpuAdapterInfo {
    /// Whether this adapter is safe to allocate `size` bytes on.
    ///
    /// Guards against NVK PTE faults and driver-reported lies about
    /// `max_buffer_size`.
    #[must_use]
    pub fn is_allocation_safe(&self, size_bytes: u64) -> bool {
        size_bytes <= self.safe_allocation_limit
    }

    /// Whether the sovereign compute pipeline can drive this GPU.
    #[must_use]
    pub fn is_sovereign_capable(&self) -> bool {
        self.fingerprint.sovereign_capable
    }

    /// Whether this adapter uses the NVK (Nouveau Vulkan) driver.
    #[must_use]
    pub fn is_nvk(&self) -> bool {
        self.driver.contains("nvk") || self.driver.contains("nouveau")
    }

    /// Whether f64 compute actually works (supported AND reliable).
    #[must_use]
    pub fn has_reliable_f64(&self) -> bool {
        self.supports_shader_f64 && !self.f64_compute_unreliable
    }

    /// Maximum safe 2D dispatch dimensions (x * y must fit workgroup limit).
    /// Returns (max_x, max_y) for 2D compute dispatch.
    #[must_use]
    pub fn max_2d_dispatch(&self) -> (u32, u32) {
        let max = self.max_compute_workgroups_per_dimension;
        (max, max)
    }
}

impl HardwareFingerprint {
    /// Build a fingerprint from wgpu adapter info.
    ///
    /// TFLOPS estimates use workgroup count as a proxy for shader core count.
    /// Real benchmarks should replace these estimates — this provides a
    /// conservative baseline for capability-based routing.
    pub(crate) fn from_adapter_info(
        info: &wgpu::AdapterInfo,
        device_type: GpuDeviceType,
        supports_f64: bool,
        f64_compute_unreliable: bool,
        max_workgroups: u32,
    ) -> Self {
        let is_nvk = info.driver.contains("nvk") || info.driver.contains("nouveau");

        // Estimate TFLOPS from device type and workgroup count.
        // Discrete GPUs: ~10-80 TFLOPS f32, ~0.3-40 TFLOPS f64
        // Integrated: ~1-4 TFLOPS f32
        let estimated_tflops_f32 = match device_type {
            GpuDeviceType::Discrete => (max_workgroups as f64 / 65535.0) * 40.0,
            GpuDeviceType::Integrated => (max_workgroups as f64 / 65535.0) * 4.0,
            _ => 0.5,
        };

        let f64_reliable = supports_f64 && !f64_compute_unreliable;
        let estimated_tflops_f64 = if f64_reliable {
            estimated_tflops_f32 / 2.0
        } else {
            0.0
        };

        // Sovereign capable = can be driven by WGSL→SPIR-V without vendor tools.
        // Currently: all Vulkan adapters are sovereign-capable via wgpu+naga.
        // NVK has limitations (PTE faults, NAK f64 crashes) but the sovereign
        // compiler pipeline (naga→SPIR-V passthrough) bypasses NAK entirely.
        let sovereign_capable = !info.driver.is_empty();

        let mut capabilities = vec![SubstrateCapabilityKind::NnInference];

        if f64_reliable {
            capabilities.push(SubstrateCapabilityKind::F64Native);
        }
        capabilities.push(SubstrateCapabilityKind::Df64Emulation);

        if matches!(device_type, GpuDeviceType::Discrete) {
            capabilities.extend_from_slice(&[
                SubstrateCapabilityKind::Fft,
                SubstrateCapabilityKind::MonteCarlo,
                SubstrateCapabilityKind::Spmv,
            ]);
        }

        if is_nvk || info.driver.contains("nvidia") {
            capabilities.push(SubstrateCapabilityKind::MdForce);
            capabilities.push(SubstrateCapabilityKind::Eigen);
            capabilities.push(SubstrateCapabilityKind::Cg);
        }

        Self {
            estimated_tflops_f32,
            estimated_tflops_f64,
            sovereign_capable,
            capabilities,
        }
    }
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

        let supports_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
        let is_nvk = info.driver.contains("nvk") || info.driver.contains("nouveau");

        let is_nvk_volta = is_nvk
            && (info.name.contains("Titan V")
                || info.name.contains("Tesla V100")
                || info.name.contains("Quadro GV100"));
        let f64_compute_unreliable = is_nvk_volta;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_fingerprint(
        device_type: GpuDeviceType,
        supports_f64: bool,
        f64_compute_unreliable: bool,
        driver: &str,
        name: &str,
    ) -> HardwareFingerprint {
        let info = wgpu::AdapterInfo {
            name: name.to_owned(),
            vendor: 0x10de,
            device: 0x2684,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: driver.to_owned(),
            driver_info: "test".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        HardwareFingerprint::from_adapter_info(
            &info,
            device_type,
            supports_f64,
            f64_compute_unreliable,
            65535,
        )
    }

    #[test]
    fn test_hardware_fingerprint_discrete_f64() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvidia", "Test GPU");
        assert!(fp.estimated_tflops_f32 > 0.0);
        assert!(fp.estimated_tflops_f64 > 0.0);
        assert!(fp.sovereign_capable);
        assert!(fp
            .capabilities
            .contains(&SubstrateCapabilityKind::F64Native));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::MdForce));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Fft));
    }

    #[test]
    fn test_hardware_fingerprint_integrated_no_f64() {
        let fp = make_test_fingerprint(GpuDeviceType::Integrated, false, false, "anv", "Test GPU");
        assert!(fp.estimated_tflops_f32 > 0.0);
        assert_eq!(fp.estimated_tflops_f64, 0.0);
        assert!(!fp
            .capabilities
            .contains(&SubstrateCapabilityKind::F64Native));
        assert!(fp
            .capabilities
            .contains(&SubstrateCapabilityKind::Df64Emulation));
    }

    #[test]
    fn test_hardware_fingerprint_nvk_has_md_force() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvk", "Test GPU");
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::MdForce));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Eigen));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Cg));
    }

    #[test]
    fn test_gpu_adapter_info_allocation_guard() {
        let info = GpuAdapterInfo {
            name: "Test".to_owned(),
            driver: "nvk".to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Discrete,
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: true,
            f64_compute_unreliable: false,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvk", "Test"),
            safe_allocation_limit: 1_200_000_000,
        };

        assert!(info.is_allocation_safe(1_000_000_000));
        assert!(!info.is_allocation_safe(2_000_000_000));
        assert!(info.is_nvk());
        assert!(info.is_sovereign_capable());
    }

    #[test]
    fn test_gpu_adapter_info_non_nvk() {
        let info = GpuAdapterInfo {
            name: "Test".to_owned(),
            driver: "nvidia".to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Discrete,
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: true,
            f64_compute_unreliable: false,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Discrete,
                true,
                false,
                "nvidia",
                "Test",
            ),
            safe_allocation_limit: 4_294_967_296,
        };

        assert!(info.is_allocation_safe(4_000_000_000));
        assert!(!info.is_nvk());
    }

    #[test]
    fn test_gpu_device_type_variants() {
        assert_eq!(GpuDeviceType::Discrete, GpuDeviceType::Discrete);
        assert_ne!(GpuDeviceType::Discrete, GpuDeviceType::Integrated);
    }

    #[test]
    fn test_substrate_capability_kind_equality() {
        assert_eq!(
            SubstrateCapabilityKind::F64Native,
            SubstrateCapabilityKind::F64Native
        );
        assert_ne!(
            SubstrateCapabilityKind::F64Native,
            SubstrateCapabilityKind::Df64Emulation
        );
    }

    #[test]
    fn test_f64_compute_unreliable_nvk_volta() {
        let fp =
            make_test_fingerprint(GpuDeviceType::Discrete, true, true, "nvk", "NVIDIA Titan V");
        assert!(!fp
            .capabilities
            .contains(&SubstrateCapabilityKind::F64Native));
        assert!(fp
            .capabilities
            .contains(&SubstrateCapabilityKind::Df64Emulation));
    }

    #[test]
    fn test_f64_compute_unreliable_nvk_non_volta() {
        let fp = make_test_fingerprint(
            GpuDeviceType::Discrete,
            true,
            false,
            "nvk",
            "NVIDIA GeForce RTX 3080",
        );
        assert!(fp
            .capabilities
            .contains(&SubstrateCapabilityKind::F64Native));
        assert!(fp
            .capabilities
            .contains(&SubstrateCapabilityKind::Df64Emulation));
    }

    #[test]
    fn test_has_reliable_f64_nvk_volta() {
        let info = GpuAdapterInfo {
            name: "NVIDIA Titan V".to_owned(),
            driver: "nvk".to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Discrete,
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: true,
            f64_compute_unreliable: true,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Discrete,
                true,
                true,
                "nvk",
                "NVIDIA Titan V",
            ),
            safe_allocation_limit: 1_200_000_000,
        };
        assert!(info.supports_shader_f64);
        assert!(info.f64_compute_unreliable);
        assert!(!info.has_reliable_f64());
    }

    #[test]
    fn test_subgroup_size_fields() {
        let info_zero = GpuAdapterInfo {
            name: "Test".to_owned(),
            driver: "anv".to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Integrated,
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: false,
            f64_compute_unreliable: false,
            min_subgroup_size: 0,
            max_subgroup_size: 0,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Integrated,
                false,
                false,
                "anv",
                "Test",
            ),
            safe_allocation_limit: 4_294_967_296,
        };
        assert_eq!(info_zero.min_subgroup_size, 0);
        assert_eq!(info_zero.max_subgroup_size, 0);

        let info_populated = GpuAdapterInfo {
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            ..info_zero.clone()
        };
        assert_eq!(info_populated.min_subgroup_size, 32);
        assert_eq!(info_populated.max_subgroup_size, 32);
    }

    #[test]
    fn test_max_2d_dispatch() {
        let info = GpuAdapterInfo {
            name: "Test".to_owned(),
            driver: "nvidia".to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Discrete,
            max_compute_workgroups_per_dimension: 4096,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: true,
            f64_compute_unreliable: false,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Discrete,
                true,
                false,
                "nvidia",
                "Test",
            ),
            safe_allocation_limit: 4_294_967_296,
        };
        let (max_x, max_y) = info.max_2d_dispatch();
        assert_eq!(max_x, 4096);
        assert_eq!(max_y, 4096);
    }
}

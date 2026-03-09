// SPDX-License-Identifier: AGPL-3.0-only
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
    /// Whether f64 shared-memory reductions produce correct results.
    ///
    /// groundSpring V84-V85 discovered that naga/SPIR-V f64 shared-memory
    /// reductions return zeros on ALL tested GPUs (NVIDIA proprietary + NVK).
    /// DF64 paths and f32 shared-memory work correctly.
    /// Currently `false` for all adapters via naga/SPIR-V pipeline.
    pub f64_shared_memory_reliable: bool,
    /// Whether fused f64 reductions risk returning zeros on this adapter.
    ///
    /// `true` for NVK + full/throttled FP64 devices and Ada Lovelace +
    /// proprietary driver where shared-memory f64 reductions silently fail.
    /// Springs and barraCuda use this to guard or skip fused reduction tests.
    pub f64_zeros_risk: bool,
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
    /// Whether a coralDriver binary submission path exists for this GPU.
    /// `true` when coralReef can compile SPIR-V to native binaries
    /// and coralDriver can submit them. Currently `false` for all GPUs
    /// until coralDriver reaches production readiness.
    pub sovereign_binary_capable: bool,
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
    /// Sovereign compile pipeline (coralReef SPIR-V → native without vendor toolchains).
    /// groundSpring V100: `SubstrateKind::Sovereign` recognition.
    SovereignCompile,
}

/// Precision routing advice for f64 workloads.
///
/// Callers (barraCuda, springs) use this to select the correct compute
/// path without needing to understand driver-level quirks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecisionRoutingAdvice {
    /// Native f64 is reliable for all operations including shared memory.
    F64Native,
    /// f64 arithmetic works but shared-memory reductions fail (return zeros).
    /// Use DF64 for reductions, native f64 for element-wise ops.
    F64NativeNoSharedMem,
    /// f64 is unreliable — use DF64 (double-float f32 pairs) for all operations.
    Df64Only,
    /// No f64 support at all — f32 only.
    F32Only,
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

    /// Route f64 workloads to the correct precision path.
    ///
    /// Encapsulates the groundSpring V84-V85 discovery: naga/SPIR-V f64
    /// shared-memory reductions return zeros on all tested GPUs. This
    /// method tells callers exactly which path to use.
    ///
    /// Ada Lovelace (RTX 40xx) on proprietary drivers is classified as
    /// `F64NativeNoSharedMem` per groundSpring V98 + neuralSpring V90.
    #[must_use]
    pub fn precision_routing(&self) -> PrecisionRoutingAdvice {
        if !self.supports_shader_f64 {
            return PrecisionRoutingAdvice::F32Only;
        }
        if self.f64_compute_unreliable {
            return PrecisionRoutingAdvice::Df64Only;
        }
        if !self.f64_shared_memory_reliable {
            return PrecisionRoutingAdvice::F64NativeNoSharedMem;
        }
        PrecisionRoutingAdvice::F64Native
    }

    /// Whether fused f64 operations are safe on this adapter.
    ///
    /// Returns `false` when shared-memory f64 reductions risk returning
    /// zeros (NVK FP64 devices, Ada Lovelace proprietary). Callers should
    /// run a variance canary probe or skip fused reductions.
    #[must_use]
    pub fn fused_ops_healthy(&self) -> bool {
        !self.f64_zeros_risk
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
        const MAX_WORKGROUPS_NORMALIZER: f64 = 65535.0;
        const DISCRETE_PEAK_TFLOPS_F32: f64 = 40.0;
        const INTEGRATED_PEAK_TFLOPS_F32: f64 = 4.0;
        const FALLBACK_TFLOPS_F32: f64 = 0.5;

        let is_nvk = info.driver.contains("nvk") || info.driver.contains("nouveau");

        let estimated_tflops_f32 = match device_type {
            GpuDeviceType::Discrete => {
                (max_workgroups as f64 / MAX_WORKGROUPS_NORMALIZER) * DISCRETE_PEAK_TFLOPS_F32
            }
            GpuDeviceType::Integrated => {
                (max_workgroups as f64 / MAX_WORKGROUPS_NORMALIZER) * INTEGRATED_PEAK_TFLOPS_F32
            }
            _ => FALLBACK_TFLOPS_F32,
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

        if sovereign_capable {
            capabilities.push(SubstrateCapabilityKind::SovereignCompile);
        }

        Self {
            estimated_tflops_f32,
            estimated_tflops_f64,
            sovereign_capable,
            sovereign_binary_capable: false,
            capabilities,
        }
    }
}

/// Detect Ada Lovelace architecture from adapter name.
///
/// Matches RTX 40xx series, L40, A6000 Ada, and explicit "Ada" mentions.
fn is_nvidia_ada_lovelace(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("rtx 40")
        || lower.contains("rtx40")
        || lower.contains("l40")
        || lower.contains("ada")
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
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
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

    fn make_test_adapter_info(
        name: &str,
        driver: &str,
        supports_f64: bool,
        f64_unreliable: bool,
        f64_shared_mem: bool,
        safe_alloc: u64,
    ) -> GpuAdapterInfo {
        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_ada = is_nvidia_ada_lovelace(name);
        let is_prop_nv = driver.contains("nvidia") && !driver.contains("nvk");
        let zeros_risk = (is_nvk && supports_f64) || (is_ada && is_prop_nv);
        GpuAdapterInfo {
            name: name.to_owned(),
            driver: driver.to_owned(),
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
            supports_shader_f64: supports_f64,
            f64_compute_unreliable: f64_unreliable,
            f64_shared_memory_reliable: f64_shared_mem,
            f64_zeros_risk: zeros_risk,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Discrete,
                supports_f64,
                f64_unreliable,
                driver,
                name,
            ),
            safe_allocation_limit: safe_alloc,
        }
    }

    #[test]
    fn test_gpu_adapter_info_allocation_guard() {
        let info = make_test_adapter_info("Test", "nvk", true, false, false, 1_200_000_000);

        assert!(info.is_allocation_safe(1_000_000_000));
        assert!(!info.is_allocation_safe(2_000_000_000));
        assert!(info.is_nvk());
        assert!(info.is_sovereign_capable());
    }

    #[test]
    fn test_gpu_adapter_info_non_nvk() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);

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
        let info =
            make_test_adapter_info("NVIDIA Titan V", "nvk", true, true, false, 1_200_000_000);
        assert!(info.supports_shader_f64);
        assert!(info.f64_compute_unreliable);
        assert!(!info.has_reliable_f64());
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::Df64Only);
    }

    #[test]
    fn test_subgroup_size_fields() {
        let mut info_zero =
            make_test_adapter_info("Test", "anv", false, false, false, 4_294_967_296);
        info_zero.device_type = GpuDeviceType::Integrated;
        info_zero.min_subgroup_size = 0;
        info_zero.max_subgroup_size = 0;

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
        let mut info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        info.max_compute_workgroups_per_dimension = 4096;
        let (max_x, max_y) = info.max_2d_dispatch();
        assert_eq!(max_x, 4096);
        assert_eq!(max_y, 4096);
    }

    #[test]
    fn test_precision_routing_f32_only() {
        let info = make_test_adapter_info("Intel iGPU", "anv", false, false, false, 4_294_967_296);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::F32Only);
    }

    #[test]
    fn test_precision_routing_df64_only() {
        let info =
            make_test_adapter_info("NVIDIA Titan V", "nvk", true, true, false, 1_200_000_000);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::Df64Only);
    }

    #[test]
    fn test_precision_routing_no_shared_mem() {
        let info = make_test_adapter_info(
            "NVIDIA RTX 4070",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_precision_routing_full_native() {
        let info = make_test_adapter_info("Future GPU", "nvidia", true, false, true, 4_294_967_296);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::F64Native);
    }

    #[test]
    fn test_f64_shared_memory_reliable_field() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        assert!(!info.f64_shared_memory_reliable);
        assert!(info.has_reliable_f64());
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_sovereign_binary_capable_field() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        assert!(!info.fingerprint.sovereign_binary_capable);
        assert!(info.fingerprint.sovereign_capable);
    }

    #[test]
    fn test_ada_lovelace_proprietary_f64_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 4070",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert!(
            info.f64_zeros_risk,
            "Ada Lovelace + proprietary should have f64_zeros_risk"
        );
        assert!(
            !info.fused_ops_healthy(),
            "fused ops should not be healthy on Ada Lovelace proprietary"
        );
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_ada_lovelace_nvk_f64_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 4090",
            "nvk",
            true,
            false,
            false,
            1_200_000_000,
        );
        assert!(info.f64_zeros_risk, "NVK + f64 should have f64_zeros_risk");
        assert!(!info.fused_ops_healthy());
    }

    #[test]
    fn test_non_ada_proprietary_no_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 3090",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert!(
            !info.f64_zeros_risk,
            "Ampere + proprietary should not have f64_zeros_risk"
        );
        assert!(info.fused_ops_healthy());
    }

    #[test]
    fn test_sovereign_compile_capability_present() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvidia", "Test GPU");
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::SovereignCompile),
            "sovereign-capable adapters should have SovereignCompile capability"
        );
    }

    #[test]
    fn test_sovereign_compile_absent_for_empty_driver() {
        let info = wgpu::AdapterInfo {
            name: "Unknown".to_owned(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::Cpu,
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
        };
        let fp = HardwareFingerprint::from_adapter_info(&info, GpuDeviceType::Cpu, false, false, 1);
        assert!(
            !fp.capabilities
                .contains(&SubstrateCapabilityKind::SovereignCompile),
            "empty-driver adapters should not have SovereignCompile"
        );
        assert!(!fp.sovereign_capable);
    }

    #[test]
    fn test_is_nvidia_ada_lovelace_detection() {
        assert!(is_nvidia_ada_lovelace("NVIDIA GeForce RTX 4070"));
        assert!(is_nvidia_ada_lovelace("NVIDIA GeForce RTX 4090"));
        assert!(is_nvidia_ada_lovelace("NVIDIA L40"));
        assert!(is_nvidia_ada_lovelace("NVIDIA RTX 4000 Ada Generation"));
        assert!(!is_nvidia_ada_lovelace("NVIDIA GeForce RTX 3090"));
        assert!(!is_nvidia_ada_lovelace("NVIDIA Titan V"));
        assert!(!is_nvidia_ada_lovelace("AMD Radeon RX 6950 XT"));
    }

    /// GPU f64 reduction smoke test (P1 — groundSpring V84-V100).
    ///
    /// Validates that all adapter configurations correctly flag
    /// f64 shared-memory as unreliable via the naga/SPIR-V pipeline
    /// and that precision routing steers callers to safe paths.
    #[test]
    fn test_f64_reduction_smoke_all_adapters() {
        let configs = [
            ("NVIDIA RTX 4090", "nvidia", true, false),
            ("NVIDIA RTX 4070", "nvidia", true, false),
            ("NVIDIA RTX 3090", "nvidia", true, false),
            ("NVIDIA Titan V", "nvk", true, true),
            ("NVIDIA RTX 3080", "nvk", true, false),
            ("Intel Arc A770", "anv", false, false),
            ("AMD RX 7900 XTX", "radv", false, false),
        ];

        for (name, driver, f64_support, f64_unreliable) in configs {
            let info = make_test_adapter_info(
                name,
                driver,
                f64_support,
                f64_unreliable,
                false,
                4_294_967_296,
            );

            assert!(
                !info.f64_shared_memory_reliable,
                "{name}: f64 shared-memory must be unreliable via naga/SPIR-V"
            );

            let routing = info.precision_routing();
            match (f64_support, f64_unreliable) {
                (false, _) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::F32Only,
                    "{name}: no f64 → F32Only"
                ),
                (true, true) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::Df64Only,
                    "{name}: unreliable f64 → Df64Only"
                ),
                (true, false) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::F64NativeNoSharedMem,
                    "{name}: f64 OK but shared-mem broken → F64NativeNoSharedMem"
                ),
            }
        }
    }

    /// Validates that fused_ops_healthy correctly tracks f64_zeros_risk
    /// across NVK, Ada Lovelace proprietary, and safe configurations.
    #[test]
    fn test_fused_ops_healthy_matrix() {
        let cases = [
            ("NVIDIA RTX 4070", "nvidia", true, false, true), // Ada + proprietary → risk
            ("NVIDIA RTX 3090", "nvidia", true, false, false), // Ampere + proprietary → no risk
            ("NVIDIA RTX 4090", "nvk", true, false, true),    // NVK + f64 → risk
            ("NVIDIA RTX 3090", "nvk", true, false, true),    // NVK + f64 → risk
            ("Intel Arc A770", "anv", false, false, false),   // No f64 → no risk
        ];

        for (name, driver, f64_support, f64_unreliable, expect_risk) in cases {
            let info = make_test_adapter_info(
                name,
                driver,
                f64_support,
                f64_unreliable,
                false,
                4_294_967_296,
            );
            assert_eq!(
                info.f64_zeros_risk, expect_risk,
                "{name}/{driver}: f64_zeros_risk mismatch"
            );
            assert_eq!(
                info.fused_ops_healthy(),
                !expect_risk,
                "{name}/{driver}: fused_ops_healthy mismatch"
            );
        }
    }
}

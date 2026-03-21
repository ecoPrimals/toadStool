// SPDX-License-Identifier: AGPL-3.0-only
//! wgpu adapter types — GpuAdapterInfo, HardwareFingerprint, capability enums.

/// GPU device type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuDeviceType {
    /// Discrete GPU (PCIe, dedicated VRAM).
    Discrete,
    /// Integrated GPU (shared memory with CPU).
    Integrated,
    /// Virtual GPU (VM passthrough).
    Virtual,
    /// CPU software renderer.
    Cpu,
    /// Other or unknown device type.
    Other,
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
    /// Max compute workgroup size Y dimension.
    pub max_compute_workgroup_size_y: u32,
    /// Max compute workgroup size Z dimension.
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
    /// Per-unit silicon capabilities discovered on this GPU die.
    ///
    /// Enumerates which functional units (tensor cores, RT cores, TMUs,
    /// ROPs, rasterizer, tessellator, video encoder) are present and their
    /// generation. `None` until silicon discovery is performed.
    pub silicon: Option<toadstool_core::SiliconCapabilities>,
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

/// Substrate capability kinds — compute primitives and silicon unit access.
///
/// Each capability represents a concrete compute primitive or hardware unit
/// that the substrate can execute. Discovered at runtime, not hardcoded.
/// Expanded in S159 with fixed-function GPU unit capabilities per the
/// ludoSpring V24 all-silicon audit.
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
    SovereignCompile,
    /// Tensor core matrix multiply-accumulate (MMA).
    /// CG solver, pairwise distances, convolution, FFT butterfly.
    TensorCoreMma,
    /// RT core BVH spatial query.
    /// MD neighbor search, Monte Carlo transport, line-of-sight.
    RtCoreBvh,
    /// Texture unit interpolated lookup.
    /// EOS tables, activation functions, exp/log at reduced precision.
    TextureUnitLookup,
    /// ROP/alpha-blend scatter-add.
    /// Histograms, particle deposition, Beer-Lambert transmittance.
    RopScatterAdd,
    /// Rasterizer spatial binning.
    /// Voxelization, FEM cell assignment, PIC/SPH binning.
    RasterizerBinning,
    /// Depth buffer min-reduction.
    /// Voronoi diagrams, distance fields, nearest-neighbor.
    DepthBufferMinReduce,
    /// Hardware video encode/decode.
    /// Simulation frame compression, motion-estimation registration.
    VideoEncodeDecode,
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

impl GpuAdapterInfo {
    /// Whether this adapter is safe to allocate `size` bytes on.
    ///
    /// Guards against NVK PTE faults and driver-reported lies about
    /// `max_buffer_size`.
    #[must_use]
    pub const fn is_allocation_safe(&self, size_bytes: u64) -> bool {
        size_bytes <= self.safe_allocation_limit
    }

    /// Whether the sovereign compute pipeline can drive this GPU.
    #[must_use]
    pub const fn is_sovereign_capable(&self) -> bool {
        self.fingerprint.sovereign_capable
    }

    /// Whether this adapter uses the NVK (Nouveau Vulkan) driver.
    #[must_use]
    pub fn is_nvk(&self) -> bool {
        self.driver.contains("nvk") || self.driver.contains("nouveau")
    }

    /// Whether f64 compute actually works (supported AND reliable).
    #[must_use]
    pub const fn has_reliable_f64(&self) -> bool {
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
    pub const fn precision_routing(&self) -> PrecisionRoutingAdvice {
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
    pub const fn fused_ops_healthy(&self) -> bool {
        !self.f64_zeros_risk
    }

    /// Maximum safe 2D dispatch dimensions (x * y must fit workgroup limit).
    /// Returns (max_x, max_y) for 2D compute dispatch.
    #[must_use]
    pub const fn max_2d_dispatch(&self) -> (u32, u32) {
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
pub fn is_nvidia_ada_lovelace(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("rtx 40")
        || lower.contains("rtx40")
        || lower.contains("l40")
        || lower.contains("ada")
}
